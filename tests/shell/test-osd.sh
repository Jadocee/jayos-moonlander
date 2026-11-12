#!/usr/bin/env bash
#
# jayos-moonlander-osd, the built daemon: what it sees on a fake USB bus,
# what a fake Keymapp tells it over gRPC, the popups it asks for, and the
# files contract/runtime.txt says it keeps.
#
#   tests/shell/test-osd.sh
#
# Exit 0 when every case passed, 1 when one failed, 2 when nothing could be
# examined: no jq, no python3 (which plays Keymapp), or no built daemon.
#
# The cases came from JayOS's tools/test-moonlander.sh at fdde9c4, where the
# daemon was started through the shell's hypr/moonlander-osd.sh. That
# lifecycle -- the setting it obeys, its pid checks, the layer file it removes
# on stop -- is JayOS's and stays in its suite, against a stand-in daemon;
# here the daemon is started the way that script starts it, in the
# background with the shell's variables, and stopped with SIGTERM. A fake
# USB tree under QS_USB_ROOT, a fake Keymapp speaking HTTP/2 and gRPC on a
# Unix socket, and an osd.sh that writes down what it is asked to show.
#
# JAYOS_MOONLANDER_BIN_DIR is where the build is, target/release unless set
# (tools/check.sh points it at target/debug).

set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BIN_DIR="${JAYOS_MOONLANDER_BIN_DIR:-$REPO/target/release}"
# Absolute, since the suite links the programs into directories of its own,
# and a relative target would dangle there.
BIN_DIR=$(realpath -m -- "$BIN_DIR")
REAL="$BIN_DIR/jayos-moonlander-osd"

command -v jq >/dev/null 2>&1 || { echo "jq is required"; exit 2; }
command -v python3 >/dev/null 2>&1 || { echo "python3, which plays Keymapp here, is required"; exit 2; }
[[ -x "$REAL" ]] || { echo "no built $REAL (cargo build --release)"; exit 2; }

WORK="$(mktemp -d "${TMPDIR:-/tmp}/jayos-moonlander-osd.XXXXXX")" || exit 2
mkdir -p "$WORK/home" "$WORK/run"
RUN="$WORK/run/quickshell/moonlander-osd"
# Every background process this file starts, so the trap ends all of them.
KIDS=()
cleanup() {
    local p
    exec 2>/dev/null
    for p in "${KIDS[@]:-}"; do [[ -n "$p" ]] && kill -9 "$p"; done
    [[ -s "$RUN/pid" ]] && kill -9 "$(cat "$RUN/pid")"
    wait
    rm -rf "$WORK"
}
trap cleanup EXIT

PASS=0
FAIL=0
SKIP=0
pass() { printf '  \033[32mok\033[0m   %s\n' "$1"; PASS=$((PASS + 1)); }
bad()  { printf '  \033[31mfail\033[0m %s\n' "$1"; FAIL=$((FAIL + 1)); }
skip() { printf '  skip %s\n' "$1"; SKIP=$((SKIP + 1)); }
check() {
    if [[ "$2" == "$3" ]]; then pass "$1"; else
        bad "$1"
        printf '         want: %s\n         got:  %s\n' "$2" "$3"
    fi
}
has() { if grep -qE -- "$3" <<< "$2"; then pass "$1"; else bad "$1"; printf '%s\n' "$2" | sed 's/^/         | /' | head -8; fi; }
step() { printf '\n\033[1m%s\033[0m\n' "$1"; }

step "jayos-moonlander-osd says what is plugged in, and which layer stays"

ROOT="$WORK/usb"
DEV="$ROOT/sys/bus/usb/devices"
mkdir -p "$ROOT/dev/bus/usb/001" "$DEV"
plug()   { mkdir -p "$DEV/1-4"; echo 3297 > "$DEV/1-4/idVendor"; echo "Moonlander Mark I" > "$DEV/1-4/product"; : > "$ROOT/dev/bus/usb/001/005"; }
unplug() { rm -rf "$DEV/1-4"; rm -f "$ROOT/dev/bus/usb/001/005"; }
# Another vendor's device, which is never a popup.
mkdir -p "$DEV/1-5"; echo 046d > "$DEV/1-5/idVendor"; echo "USB Receiver" > "$DEV/1-5/product"
plug

# Keymapp: GetStatus over HTTP/2 on a Unix socket, the layer from a
# file read per call -- a number, `none` for no keyboard, `fail` for a
# trailers-only error. What each request carried is written down.
cat > "$WORK/keymapp.py" <<'PY'
import os, socket, struct, sys
path, layer_file, log_file = sys.argv[1:4]
try:
    os.unlink(path)
except FileNotFoundError:
    pass
server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
server.bind(path)
server.listen(8)

def frame(kind, flags, stream, payload):
    return len(payload).to_bytes(3, "big") + bytes([kind, flags]) + stream.to_bytes(4, "big") + payload

def varint(v):
    v &= (1 << 64) - 1
    out = bytearray()
    while v >= 0x80:
        out.append((v & 0x7F) | 0x80)
        v >>= 7
    out.append(v)
    return bytes(out)

def field(n, b):
    return varint(n << 3 | 2) + varint(len(b)) + b

def exact(conn, n):
    buf = b""
    while len(buf) < n:
        chunk = conn.recv(n - len(buf))
        if not chunk:
            raise EOFError
        buf += chunk
    return buf

while True:
    conn, _ = server.accept()
    try:
        conn.settimeout(2)
        if exact(conn, 24) != b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n":
            continue
        conn.sendall(frame(4, 0, 0, b""))
        block = b""
        while True:
            head = exact(conn, 9)
            payload = exact(conn, int.from_bytes(head[:3], "big"))
            if head[3] == 1:
                block += payload
            if head[3] == 0 and head[4] & 1:
                break
        with open(log_file, "a") as log:
            log.write(("GetStatus" if b"/api.KeyboardService/GetStatus" in block else "other")
                      + (" grpc" if b"application/grpc" in block else " not-grpc") + "\n")
        text = open(layer_file).read().strip()
        if text == "fail":
            conn.sendall(frame(1, 5, 1, b"\x88"))
            continue
        message = field(1, b"9.9.9-suite")
        if text != "none":
            board = field(1, b"Moonlander MK1") + field(2, b"suite")
            if int(text):
                board += varint(3 << 3) + varint(int(text))
            message += field(2, board)
        body = b"\x00" + struct.pack(">I", len(message)) + message
        conn.sendall(frame(1, 4, 1, b"\x88") + frame(0, 0, 1, body) + frame(1, 5, 1, b"\x88"))
    except Exception:
        pass
    finally:
        conn.close()
PY
SOCK="$WORK/keymapp.sock"
LAYER="$WORK/layer"
REQUESTS="$WORK/requests.log"
# The layer the stand-in reports, written whole: beside the file and
# renamed over it, so a request answered while it changes reads the old
# layer or the new one, never an empty file. `echo 1 > "$LAYER"`
# truncates first and writes after, and a request that read between the
# two found "", which int() refuses: the stand-in closed the connection
# unanswered, the daemon took that for a gap and forgot its layer, and
# adopted the next reading without a popup -- "a layer the note does not
# name is said as ever" failed on Jadocee/jayos#169's second gate run and
# passed on its first. Run here with the file held empty for one poll,
# that case failed and no other.
set_layer() { printf '%s\n' "$1" > "$LAYER.new" && mv -f "$LAYER.new" "$LAYER"; }
set_layer 0
keymapp() { python3 -I "$WORK/keymapp.py" "$SOCK" "$LAYER" "$REQUESTS" >/dev/null 2>&1 & KM=$!; KIDS+=("$KM"); }
keymapp

# osd.sh, as the daemon finds it through QS_SCRIPTS_DIR: it writes down
# what it was asked to show, one popup a line.
SCR="$WORK/scripts"; mkdir -p "$SCR"
POPUPS="$WORK/popups.log"; : > "$POPUPS"
printf '#!/usr/bin/env bash\nshift\nIFS="|"; printf "%%s\\n" "$*" >> %q\n' "$POPUPS" > "$SCR/osd.sh"
# The daemon beside the layer-name helper it looks for, as the package puts
# both in /usr/bin: a stand-in helper here, and the real one at the end.
RBIN="$WORK/realbin"; mkdir -p "$RBIN"
ln -sf "$REAL" "$RBIN/jayos-moonlander-osd"
printf '#!/usr/bin/env bash\necho %q\n' '{"names":["Base","Symbols","Media"],"error":""}' > "$RBIN/jayos-keymapp-layers"
chmod +x "$RBIN/jayos-keymapp-layers"
# real start|stop|status: the daemon run as JayOS's moonlander-osd.sh runs
# it -- in the background, with the shell's variables -- and stopped with
# SIGTERM. The lifecycle itself, its pid checks and its removal of the layer
# file on stop, are JayOS's, and its tools/test-moonlander.sh holds them.
# DAEMON_ENV adds variables, for the real helper's database at the end.
DAEMON_ENV=()
real() {
    local pid
    case "$1" in
        start)
            env -i HOME="$WORK/home" PATH="/usr/bin:/bin" XDG_RUNTIME_DIR="$WORK/run" QS_PKG_BIN_DIR="$RBIN" \
                QS_SCRIPTS_DIR="$SCR" QS_USB_ROOT="$ROOT" KEYMAPP_SOCKET="$SOCK" "${DAEMON_ENV[@]}" \
                "$RBIN/jayos-moonlander-osd" >/dev/null 2>&1 </dev/null &
            KIDS+=("$!")
            if wait_for 3 test -s "$RUN/pid"; then echo started; else echo "no pid"; fi ;;
        stop)
            pid=$(cat "$RUN/pid" 2>/dev/null)
            [[ -n "$pid" ]] && kill "$pid" 2>/dev/null
            if [[ -n "$pid" ]] && wait_for 3 eval '! kill -0 "$pid" 2>/dev/null'; then echo stopped; else echo "still running"; fi
            rm -f "$RUN/pid" ;;
        status)
            pid=$(cat "$RUN/pid" 2>/dev/null)
            if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then echo running; else echo stopped; fi ;;
    esac
}
# wait_for <what> <seconds> <command...>: true once the command succeeds.
wait_for() {
    local tries=$(( $1 * 10 )); shift
    while (( tries-- > 0 )); do "$@" && return 0; sleep 0.1; done
    return 1
}
popup_is() { grep -qxF -- "$1" "$POPUPS"; }
status_is() { [[ "$(cat "$RUN/status" 2>/dev/null)" == "$1" ]]; }
status_has() { grep -qF -- "$1" "$RUN/status" 2>/dev/null; }
# The `layer` file the held-Super panel reads, one line of JSON.
layer_is() { [[ "$(cat "$RUN/layer" 2>/dev/null)" == "$1" ]]; }
layer_says() { if wait_for 3 layer_is "$2"; then pass "$1"; else bad "$1"; printf '         want: %s\n         got:  %s\n' "$2" "$(cat "$RUN/layer" 2>/dev/null)"; fi; }

rm -f "$RUN/status" "$RUN/pid"
check "the daemon starts, and writes its own pid" "started" "$(real start)"
check "which names the process the kernel calls jayos-moonlan, fifteen characters" "jayos-moonlande" \
    "$(cat "/proc/$(cat "$RUN/pid" 2>/dev/null)/comm" 2>/dev/null)"
if wait_for 4 status_is "running: Moonlander Mark I plugged in; layers from Keymapp"; then
    pass "it sees the keyboard on the bus and its layer from Keymapp"
else
    bad "it sees the keyboard on the bus and its layer from Keymapp (status: $(cat "$RUN/status" 2>/dev/null))"
fi
sleep 1
check "and announces neither: both were there before it started" "" "$(cat "$POPUPS")"
check "Keymapp was asked GetStatus, as gRPC" "GetStatus grpc" "$(sort -u "$REQUESTS")"
layer_says "the layer file: the layer Keymapp reads now" '{"layer":0,"plugged":true,"why":""}'

set_layer 1
if wait_for 3 popup_is "moonlander-layer|Symbols layer|||1200"; then
    pass "a layer that stays: announced by its name, for 1200 ms"
else
    bad "a layer that stays: announced by its name, for 1200 ms"; sed 's/^/         | /' "$POPUPS"
fi
layer_says "and the layer file follows it" '{"layer":1,"plugged":true,"why":""}'
: > "$POPUPS"
set_layer 2; sleep 0.15; set_layer 1
sleep 1.5
check "a layer gone again within 150 ms: never announced" "" "$(cat "$POPUPS")"

# A switch moonlander.sh made and said itself, with why: its note names
# the layer, and the daemon does not say it a second time.
mkdir -p "$WORK/run/quickshell/moonlander"
printf '2 %s\n' "$(date +%s)" > "$WORK/run/quickshell/moonlander/announced"
set_layer 2
layer_says "a layer moonlander.sh has just said: the layer file follows it" '{"layer":2,"plugged":true,"why":""}'
sleep 1.5
check "and it is not said again" "" "$(cat "$POPUPS")"
set_layer 1
if wait_for 3 popup_is "moonlander-layer|Symbols layer|||1200"; then
    pass "a layer the note does not name is said as ever"
else
    bad "a layer the note does not name is said as ever"; sed 's/^/         | /' "$POPUPS"
fi
rm -f "$WORK/run/quickshell/moonlander/announced"

unplug
if wait_for 3 popup_is "moonlander-off|Moonlander Mark I disconnected|||2000"; then
    pass "unplugged: said, in red's name, for 2000 ms"
else
    bad "unplugged: said, in red's name, for 2000 ms"; sed 's/^/         | /' "$POPUPS"
fi
wait_for 2 status_is "running: no ZSA keyboard plugged in" && pass "and the status says none is plugged in" \
    || bad "and the status says none is plugged in (status: $(cat "$RUN/status" 2>/dev/null))"
layer_says "and the layer file says none is, so the panel stays down" \
    '{"layer":null,"plugged":false,"why":"no ZSA keyboard plugged in"}'
: > "$POPUPS"; : > "$REQUESTS"
sleep 1
check "with nothing plugged in, Keymapp is not asked" "" "$(cat "$REQUESTS")"

# A board USBGuard has not allowed: its `authorized` is 0, it types
# nothing, and it is not plugged in until it is allowed. Written before
# the node, as the kernel has it by the time the node exists.
set_layer 2
mkdir -p "$DEV/1-4"; echo 0 > "$DEV/1-4/authorized"; plug
sleep 1.5
check "a board USBGuard holds back: no popup" "" "$(cat "$POPUPS")"
status_is "running: no ZSA keyboard plugged in" && pass "and not plugged in, to the status" \
    || bad "and not plugged in, to the status (status: $(cat "$RUN/status" 2>/dev/null))"
# Allowing it writes the flag and makes no node, so inotify hears nothing:
# within three seconds is the daemon looking again on its own, where it
# used to wait for its thirty-second backstop.
echo 1 > "$DEV/1-4/authorized"
if wait_for 3 popup_is "moonlander|Moonlander Mark I connected|||2000"; then
    pass "allowed: said as plugged in then, without a node made for inotify"
else
    bad "allowed: said as plugged in then, without a node made for inotify"; sed 's/^/         | /' "$POPUPS"
fi
sleep 1.5
check "and the layer it came up on is not announced after it" "moonlander|Moonlander Mark I connected|||2000" "$(cat "$POPUPS")"

kill "$KM" 2>/dev/null; wait "$KM" 2>/dev/null; rm -f "$SOCK"
if wait_for 3 status_has "no layer popups: Keymapp's API does not answer at $SOCK"; then
    pass "Keymapp gone: the status says why there are no layer popups"
else
    bad "Keymapp gone: the status says why there are no layer popups (status: $(cat "$RUN/status" 2>/dev/null))"
fi
# Compared, not piped into grep -q: under pipefail a match can read as
# false (jayos-check shellcheck-lite says why).
api_gone() {
    [[ "$(jq -c '[.layer, .plugged]' "$RUN/layer" 2>/dev/null)" == "[null,true]" ]] \
        && [[ "$(jq -r .why "$RUN/layer" 2>/dev/null)" == "Keymapp's API does not answer"* ]]
}
if wait_for 3 api_gone; then
    pass "and the layer file: a board, no layer, and the same reason"
else
    bad "and the layer file: a board, no layer, and the same reason"; sed 's/^/         | /' "$RUN/layer"
fi
set_layer fail; keymapp
if wait_for 5 status_has "no layer popups: Keymapp answered the call with an error"; then
    pass "Keymapp answering with an error: said as that"
else
    bad "Keymapp answering with an error: said as that (status: $(cat "$RUN/status" 2>/dev/null))"
fi
: > "$POPUPS"; set_layer 2
wait_for 3 status_is "running: Moonlander Mark I plugged in; layers from Keymapp" && pass "and back" || bad "and back"
sleep 1
check "the first layer after a gap is adopted, not announced" "" "$(cat "$POPUPS")"
set_layer none
wait_for 3 status_has "Keymapp is connected to no keyboard" && pass "Keymapp with no keyboard: said as that" \
    || bad "Keymapp with no keyboard: said as that (status: $(cat "$RUN/status" 2>/dev/null))"
layer_says "and in the layer file" '{"layer":null,"plugged":true,"why":"Keymapp is connected to no keyboard"}'

check "the files it keeps while it runs: contract/runtime.txt's three" "layer|pid|status" \
    "$(ls "$RUN" 2>/dev/null | paste -sd'|')"
check "SIGTERM stops it" "stopped" "$(real stop)"
check "and status agrees" "stopped" "$(real status)"

# The probe: what it watches and what it hears, on stderr, and nothing
# written where `status` and doctor read. "for hotplug" is the inotify
# path; a daemon that had lost it would still pass every row above,
# only later, by reading the bus every two seconds.
# The layer file too: the shell's lifecycle removes it on stop, and here
# nothing did.
rm -f "$RUN/status" "$RUN/pid" "$RUN/layer"
set_layer 1
OUT=$(env -i HOME="$WORK/home" PATH="/usr/bin:/bin" XDG_RUNTIME_DIR="$WORK/run" QS_PKG_BIN_DIR="$RBIN" \
    QS_SCRIPTS_DIR="$SCR" QS_USB_ROOT="$ROOT" KEYMAPP_SOCKET="$SOCK" \
    timeout 1.5 "$REAL" --probe 2>&1 </dev/null)
has   "probe: it watches the bus for hotplug" "$OUT" "^watching $ROOT/dev/bus/usb for hotplug$"
has   "and says what Keymapp answers" "$OUT" '^  Keymapp 9\.9\.9-suite: Moonlander MK1 on layer 1$'
check "and writes none of its three files" "" "$(ls "$RUN" 2>/dev/null)"

# With no /dev/bus/usb to watch it reads the bus every two seconds: the
# same popup, later.
rm -rf "$ROOT/dev"
: > "$POPUPS"; rm -f "$RUN/status" "$RUN/pid"
check "started where inotify has nothing to watch" "started" "$(real start)"
wait_for 3 status_has "plugged in" || true
OUT=$(env -i HOME="$WORK/home" PATH="/usr/bin:/bin" XDG_RUNTIME_DIR="$WORK/run" QS_USB_ROOT="$ROOT" \
    KEYMAPP_SOCKET="$SOCK" timeout 0.5 "$REAL" --probe 2>&1 </dev/null)
has   "and its probe says it reads the bus instead" "$OUT" "^cannot watch $ROOT/dev/bus/usb; reading the bus every 2s$"
rm -rf "$DEV/1-4"
if wait_for 4 popup_is "moonlander-off|Moonlander Mark I disconnected|||2000"; then
    pass "and an unplug is still noticed, by reading the bus"
else
    bad "and an unplug is still noticed, by reading the bus"; sed 's/^/         | /' "$POPUPS"
fi
real stop >/dev/null

# The chain the helper's fix is for: the daemon with the real helper
# beside it, as the package installs them, over a database tests/shell/
# test-layers.sh's shape, and a Moonlander whose serial names the middle
# revision. Its layer 2 keeps Oryx's title for a layer nobody named, which
# says the number.
if [[ ! -x "$BIN_DIR/jayos-keymapp-layers" ]]; then
    skip "the popups' names from the real helper not exercised: no built jayos-keymapp-layers beside the daemon"
else
    KDB="$WORK/keymapp.sqlite3"
    python3 -I - "$KDB" <<'PY'
import json, sqlite3, sys
db = sqlite3.connect(sys.argv[1])
db.execute("CREATE TABLE revision (revisionId TEXT NOT NULL UNIQUE, data BLOB)")
for hash_id, created, second in [("Lz4bXo", "2026-03-02 09:15:00 UTC", "Newest"),
                                 ("cW9nTe", "2026-02-11 07:02:59 UTC", "Middle")]:
    layers = [{"title": t, "position": i, "keys": []} for i, t in enumerate(["Base", second, "Layer"])]
    revision = {"hashId": hash_id, "title": "Edited", "model": "mk1", "createdAt": created, "layers": layers}
    doc = {"layout": {"hashId": "xbJpL", "title": "Moonlander", "geometry": "moonlander", "revision": revision}}
    db.execute("INSERT INTO revision VALUES (?, ?)", (hash_id, json.dumps(doc).encode()))
db.commit()
db.close()
PY
    ln -sf "$BIN_DIR/jayos-keymapp-layers" "$RBIN/jayos-keymapp-layers"
    mkdir -p "$ROOT/dev/bus/usb/001"; plug; echo "xbJpL/cW9nTe" > "$DEV/1-4/serial"
    set_layer 0; : > "$POPUPS"; rm -f "$RUN/status" "$RUN/pid"
    DAEMON_ENV=(QS_KEYMAPP_DB="$KDB")
    check "started beside the real helper" "started" "$(real start)"
    wait_for 4 status_is "running: Moonlander Mark I plugged in; layers from Keymapp" || true
    sleep 1
    set_layer 1
    if wait_for 3 popup_is "moonlander-layer|Middle layer|||1200"; then
        pass "layer 1: named from the revision the keyboard's serial names"
    else
        bad "layer 1: named from the revision the keyboard's serial names"; sed 's/^/         | /' "$POPUPS"
    fi
    set_layer 2
    if wait_for 3 popup_is "moonlander-layer|Layer 2|||1200"; then
        pass "layer 2, titled only \"Layer\": said as its number"
    else
        bad "layer 2, titled only \"Layer\": said as its number"; sed 's/^/         | /' "$POPUPS"
    fi
    real stop >/dev/null
fi

echo
printf '%d passed, %d failed' "$PASS" "$FAIL"
(( SKIP )) && printf ', %d not exercised' "$SKIP"
printf '\n'
(( FAIL == 0 ))
