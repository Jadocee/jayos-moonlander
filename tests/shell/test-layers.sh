#!/usr/bin/env bash
#
# jayos-keymapp-layers, the built program, over a Keymapp database written
# here and a fake USB bus; then jayos-moonlander status through it.
#
#   tests/shell/test-layers.sh
#
# Exit 0 when every case passed, 1 when one failed, 2 when nothing could be
# examined: no jq, or no built program. A missing python3, which writes the
# database, is a counted skip of every case, printed as
# `  skip <what> not exercised: <why>`.
#
# The program's unit tests cover each function and none of main(); this runs
# main(), as JayOS's shell runs it. The cases came from JayOS's
# tools/test-moonlander.sh at fdde9c4. One case stayed there, as read
# evidence only: JayOS's panel model over this program's real `layout`,
# which needs JayOS's built MoonlanderLayers.js and this build at once.
#
# JAYOS_MOONLANDER_BIN_DIR is where the build is, target/release unless set
# (tools/check.sh points it at target/debug).

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BIN_DIR="${JAYOS_MOONLANDER_BIN_DIR:-$ROOT/target/release}"
# Absolute, since the suite links the programs into directories of its own,
# and a relative target would dangle there.
BIN_DIR=$(realpath -m -- "$BIN_DIR")
HELPER="$BIN_DIR/jayos-keymapp-layers"
CLI="$ROOT/src/jayos-moonlander"

command -v jq >/dev/null 2>&1 || { echo "jq is required"; exit 2; }
[[ -x "$HELPER" ]] || { echo "no built $HELPER (cargo build --release)"; exit 2; }
[[ -r "$CLI" ]] || { echo "missing $CLI" >&2; exit 2; }

WORK="$(mktemp -d "${TMPDIR:-/tmp}/jayos-moonlander-layers.XXXXXX")" || exit 2
trap 'rm -rf "$WORK"' EXIT

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
step() { printf '\n\033[1m%s\033[0m\n' "$1"; }

step "jayos-keymapp-layers reads the revision on the keyboard, and its layers' titles"

# A database in the schema third-party readers of Keymapp's show
# (jayos-keymapp-layers.rs says which), written by python3's sqlite3: three
# revisions whose hashes sort the wrong way, written in neither date nor hash
# order, each a BLOB as the schema declares, with its layers listed backwards
# so that only `position` puts them right. The helper used to take the hash
# that sorts last, the oldest here, and look for the names at the top of the
# document, so over this file it answered no names at all.
KDB="$WORK/keymapp/keymapp.sqlite3"
if ! command -v python3 >/dev/null 2>&1; then
    skip "the layer-name helper over a database not exercised: python3, which writes Keymapp's database here, is not installed"
else
    mkdir -p "$WORK/keymapp" "$WORK/home"
    python3 -I - "$KDB" <<'PY'
import json, sqlite3, sys
def document(hash_id, created, titles):
    # 72 keys a layer, for `layout`: the base all KC_A but a layer-tap on
    # key 27 that carries a field the helper must drop, the rest transparent,
    # and the second layer coloured in upper case.
    layers = []
    for i, t in enumerate(titles):
        keys = [{"tap": {"code": "KC_A" if i == 0 else "KC_TRANSPARENT"}} for _ in range(72)]
        if i == 0:
            keys[27] = {"tap": {"code": "KC_GRAVE"}, "hold": {"code": "MO", "layer": 1}, "customLabel": "x"}
        layers.append({"title": t, "position": i, "color": "#B30000" if i == 1 else None, "keys": keys})
    layers.reverse()
    revision = {"hashId": hash_id, "title": "Edited", "model": "mk1", "createdAt": created, "layers": layers}
    layout = {"hashId": "xbJpL", "title": "Moonlander", "geometry": "moonlander", "revision": revision}
    return json.dumps({"layout": layout}).encode()
db = sqlite3.connect(sys.argv[1])
db.execute("CREATE TABLE revision (revisionId TEXT NOT NULL UNIQUE, data BLOB)")
for hash_id, created, second in [("Lz4bXo", "2026-03-02 09:15:00 UTC", "Newest"),
                                 ("q8Rm2a", "2026-01-05 18:40:12 UTC", "Oldest"),
                                 ("cW9nTe", "2026-02-11 07:02:59 UTC", "Middle")]:
    db.execute("INSERT INTO revision VALUES (?, ?)", (hash_id, document(hash_id, created, ["Base", second, "Layer"])))
db.commit()
db.close()
PY
    KROOT="$WORK/kbus"; KDEV="$KROOT/sys/bus/usb/devices"; mkdir -p "$KDEV"
    helper() {
        env -i HOME="$WORK/home" PATH="/usr/bin:/bin" QS_KEYMAPP_DB="$KDB" QS_USB_ROOT="$KROOT" \
            "$HELPER" "$@" 2>/dev/null </dev/null
    }
    NEWEST='{"names":["Base","Newest","Layer"],"error":""}'
    check "no ZSA keyboard: the newest revision, in position order, not the hash that sorts last" \
        "$NEWEST" "$(helper names)"
    check "and no verb is names" "$NEWEST" "$(helper)"
    mkdir -p "$KDEV/1-5"; echo 046d > "$KDEV/1-5/idVendor"; echo "xbJpL/q8Rm2a" > "$KDEV/1-5/serial"
    check "another vendor's device names no revision, whatever its serial" "$NEWEST" "$(helper names)"
    mkdir -p "$KDEV/1-4"; echo 3297 > "$KDEV/1-4/idVendor"; echo "xbJpL/q8Rm2a" > "$KDEV/1-4/serial"
    check "a Moonlander flashed back to the oldest revision: that one, by its serial" \
        '{"names":["Base","Oldest","Layer"],"error":""}' "$(helper names)"
    check "and name looks layer 1 up in it" "Oldest" "$(helper name 1)"

    # `layout`, which JayOS's held-Super panel runs on each show: the same
    # revision, every layer's keys reduced to their steps, and whether a ZSA
    # keyboard is on the bus at all.
    OUT=$(helper layout)
    check "layout: plugged, the Moonlander's geometry, its layers in position order, no error" \
        "true|moonlander|Base,Oldest,Layer|" "$(jq -r '"\(.plugged)|\(.geometry)|\([.layers[].title] | join(","))|\(.error)"' <<< "$OUT")"
    check "72 keys a layer, a layer-tap as its two steps and nothing else of the key" \
        '72|{"tap":{"code":"KC_GRAVE"},"hold":{"code":"MO","layer":1}}|{"tap":{"code":"KC_A"}}' \
        "$(jq -r '.layers[0].keys | length' <<< "$OUT")|$(jq -c '.layers[0].keys[27]' <<< "$OUT")|$(jq -c '.layers[0].keys[0]' <<< "$OUT")"
    check "a colour lower-cased, and none where Oryx gave none" "#b30000|null" \
        "$(jq -r '"\(.layers[1].color)|\(.layers[0].color)"' <<< "$OUT")"
    echo 0 > "$KDEV/1-4/authorized"
    check "a board USBGuard has not allowed is not plugged in" "false" "$(helper layout | jq -r .plugged)"
    rm -f "$KDEV/1-4/authorized"
    OUT=$(env -i HOME="$WORK/home" PATH="/usr/bin:/bin" QS_KEYMAPP_DB="$WORK/keymapp/none.sqlite3" \
        QS_USB_ROOT="$KROOT" "$HELPER" layout 2>/dev/null </dev/null); ST=$?
    check "no database: exit 0, plugged as the bus says, no layers, and why" "0|true|0|yes" \
        "$ST|$(jq -r '"\(.plugged)|\(.layers | length)|\(if .error != "" then "yes" else "no" end)"' <<< "$OUT")"
    echo "xbJpL/zzzzzz" > "$KDEV/1-4/serial"
    check "a revision Keymapp does not hold: the newest again" "$NEWEST" "$(helper names)"
    OUT=$(helper sideways); ST=$?
    check "a verb contract/layers-verbs.txt does not list: refused as a usage error, 2" "2" "$ST"

    # jayos-moonlander, which runs the helper where the package puts it
    # (QS_PKG_BIN_DIR here), against a kontroll that says layer 1.
    FAKE="$WORK/fake"; mkdir -p "$FAKE"
    printf '#!/usr/bin/env bash\n[[ "$*" == "status --json" ]] || exit 2\n%s\n' \
        "echo '{\"keymapp_version\":\"1.3.4\",\"keyboard\":{\"friendly_name\":\"Moonlander MK1\",\"current_layer\":1}}'" \
        > "$FAKE/kontroll"
    chmod +x "$FAKE/kontroll"
    HBIN="$WORK/helperbin"; mkdir -p "$HBIN"; ln -sf "$HELPER" "$HBIN/jayos-keymapp-layers"
    echo "xbJpL/cW9nTe" > "$KDEV/1-4/serial"
    OUT=$(env -i HOME="$WORK/home" PATH="$FAKE:/usr/bin:/bin" QS_PKG_BIN_DIR="$HBIN" \
        QS_KEYMAPP_DB="$KDB" QS_USB_ROOT="$KROOT" bash "$CLI" status 2>/dev/null </dev/null)
    check "jayos-moonlander status: layer 1's title, from the revision on the keyboard" "Middle" "$(jq -r .layerName <<< "$OUT")"
fi

echo
printf '%d passed, %d failed' "$PASS" "$FAIL"
(( SKIP )) && printf ', %d not exercised' "$SKIP"
printf '\n'
(( FAIL == 0 ))
