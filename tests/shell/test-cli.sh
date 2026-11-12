#!/usr/bin/env bash
#
# jayos-moonlander, the command: every verb against a stand-in kontroll, and
# `access` over udev rules written here.
#
#   tests/shell/test-cli.sh
#
# Exit 0 when every case passed, 1 when one failed, 2 when nothing could be
# examined (no jq, or no CLI). Counted skips print `  skip <what> not
# exercised: <why>`, as JayOS's suites do.
#
# WHY THIS EXISTS. These verbs were JayOS's moonlander.sh until 2026-11-12,
# and every kontroll call in it had once stopped matching kontroll:
# `set-layer 2`, `set-rgb-all r g b 0`, `unset-layer` and `set-brightness` are
# usage errors to 1.0.x, and each call's output went to /dev/null, so nothing
# changed and nothing said so. And `status` tested for a TCP port kontroll
# 1.0.1 and later never use on Linux. Each row below is a call as kontroll
# 1.0.4 takes it, run once against the real kontroll and a stand-in Keymapp
# (docs/MOONLANDER.md) and held here against a stand-in kontroll that records
# what it is asked. The cases came from JayOS's tools/test-moonlander.sh at
# fdde9c4, with the shell's verbs -- theme, panel, watch, qwerty -- left there.
#
# JAYOS_MOONLANDER_CLI names the command to run, src/jayos-moonlander unless
# set; the layer names come from a stand-in jayos-keymapp-layers either way,
# since tests/shell/test-layers.sh runs the real one.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CLI="${JAYOS_MOONLANDER_CLI:-$ROOT/src/jayos-moonlander}"
RULE="$ROOT/share/udev/70-zsa-uaccess.rules"

for f in "$CLI" "$RULE"; do
    [[ -r "$f" ]] || { echo "missing $f" >&2; exit 2; }
done
command -v jq >/dev/null 2>&1 || { echo "jq is required"; exit 2; }

WORK="$(mktemp -d "${TMPDIR:-/tmp}/jayos-moonlander-cli.XXXXXX")" || exit 2
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
has() { if grep -qE -- "$3" <<< "$2"; then pass "$1"; else bad "$1"; printf '%s\n' "$2" | sed 's/^/         | /' | head -8; fi; }
step() { printf '\n\033[1m%s\033[0m\n' "$1"; }
strip_sh() { sed -E '/^[[:space:]]*#/d; s/[[:space:]]+#.*$//' "$1"; }

# ── the verbs, against a stand-in kontroll ───────────────────────────────────
step "jayos-moonlander asks kontroll 1.0.4 what it takes"

# kontroll: records its argv, one call a line, then answers. `status --json`
# prints $KANSWER, or fails with its words when it starts `fail:`; anything
# 1.0.x would refuse as a usage error is refused here too, with its status 2,
# so a call in the old spelling fails the way it failed on a real machine.
# KFAIL makes every other call fail as a Keymapp that does not answer would,
# and KHANG makes it hang.
FAKE="$WORK/fake"; mkdir -p "$FAKE" "$WORK/home"
KLOG="$WORK/kontroll.log"
KANSWER="$WORK/kontroll-answer"
cat > "$FAKE/kontroll" <<STUB
#!/usr/bin/env bash
printf '%s\n' "\$*" >> "$KLOG"
case "\$*" in
    "status --json")
        a=\$(cat "$KANSWER")
        [[ "\$a" == fail:* ]] && { echo "\${a#fail:}" >&2; exit 1; }
        [[ "\$a" == old ]] && { echo "error: unexpected argument '--json' found" >&2; exit 2; }
        printf '%s\n' "\$a"; exit 0 ;;
    "set-layer --index "[0-9]*|"set-rgb-all --color #"??????" --sustain 0"|connect-any|"connect --index "[0-9]*|disconnect|\
"increase-brightness --steps "[0-9]*|"decrease-brightness --steps "[0-9]*)
        [[ -n "\${KFAIL:-}" ]] && { echo "Keymapp socket not found" >&2; exit 1; }
        [[ -n "\${KHANG:-}" ]] && exec sleep 60
        exit 0 ;;
    *) echo "error: unexpected argument" >&2; exit 2 ;;
esac
STUB
# Keymapp's process, by name: present unless NO_KEYMAPP is set.
cat > "$FAKE/pgrep" <<'STUB'
#!/usr/bin/env bash
case "$*" in
    "-xi keymapp") [[ -z "${NO_KEYMAPP:-}" ]] ;;
    *) exit 1 ;;
esac
STUB
# The layer names, by index, as jayos-keymapp-layers' `name` prints them.
cat > "$FAKE/jayos-keymapp-layers" <<'STUB'
#!/usr/bin/env bash
[[ "${1:-}" == name ]] || { echo '{"names":["Base","Symbols","Media"],"error":""}'; exit 0; }
case "$2" in 0) echo Base ;; 1) echo Symbols ;; 2) printf 'Tab\there "q" back\\\n' ;; 3) echo Layer ;; *) echo ;; esac
STUB
chmod +x "$FAKE"/*

ml() { # ml <verb...> -> stdout, with $? the command's
    env -i HOME="$WORK/home" PATH="$FAKE:/usr/bin:/bin" QS_PKG_BIN_DIR="$FAKE" \
        ${NO_KEYMAPP:+NO_KEYMAPP=1} ${KFAIL:+KFAIL=1} ${KHANG:+KHANG=1} bash "$CLI" "$@" 2>/dev/null
}
calls() { cat "$KLOG" 2>/dev/null; : > "$KLOG"; }
kb() { printf '{"keymapp_version":"1.3.4","kontroll_version":"1.0.3","keyboard":%s}' "$1"; }

printf '%s' "$(kb '{"friendly_name":"Moonlander MK1","firmware_version":"x","current_layer":1}')" > "$KANSWER"
check "status: the live layer and its name" \
    '{"available":true,"connected":true,"keyboard":"Moonlander MK1","layer":1,"layerName":"Symbols"}' "$(ml status)"
check "asked as status --json, once" "status --json" "$(calls)"
check "and status is what no verb means" '{"available":true,"connected":true,"keyboard":"Moonlander MK1","layer":1,"layerName":"Symbols"}' "$(ml)"
calls >/dev/null

printf '%s' "$(kb '{"friendly_name":"Moon\\lander \"MK1\"","firmware_version":"x","current_layer":2}')" > "$KANSWER"
OUT=$(ml status); calls >/dev/null
check "a keyboard name with a backslash and quotes survives the JSON" 'Moon\lander "MK1"' "$(jq -r .keyboard <<< "$OUT")"
check "and a layer name with a tab, quotes and a trailing backslash" $'Tab\there "q" back\\' "$(jq -r .layerName <<< "$OUT")"

printf '%s' "$(kb '{"friendly_name":"Moonlander MK1","firmware_version":"x","current_layer":-1}')" > "$KANSWER"
check "a layer that is not a number reads 0" "0" "$(ml status | jq -r .layer)"
calls >/dev/null

printf '%s' "$(kb '{"friendly_name":"Moonlander MK1","firmware_version":"x","current_layer":3}')" > "$KANSWER"
check "a layer titled only \"Layer\", Oryx's title for one nobody named, has no name" "3|" \
    "$(ml status | jq -r '"\(.layer)|\(.layerName)"')"
calls >/dev/null

printf '%s' "$(kb null)" > "$KANSWER"
OUT=$(ml status); calls >/dev/null
check "Keymapp with no keyboard: unavailable" "false" "$(jq -r .available <<< "$OUT")"
has   "and says so" "$(jq -r .reason <<< "$OUT")" 'connected to no keyboard'

printf 'fail:Keymapp socket not found at /s/keymapp.sock, make sure Keymapp is running and the API is started.' > "$KANSWER"
OUT=$(NO_KEYMAPP=1 ml status); calls >/dev/null
check "no Keymapp process: Keymapp is not running" "Keymapp is not running" "$(jq -r .reason <<< "$OUT")"
OUT=$(ml status); calls >/dev/null
has   "Keymapp running, no answer: the API, with kontroll's own words" "$(jq -r .reason <<< "$OUT")" \
      "^Keymapp's API did not answer .*\(Keymapp socket not found at /s/keymapp\.sock"

printf 'old' > "$KANSWER"
OUT=$(ml status); calls >/dev/null
has   "a kontroll without status --json is named as too old" "$(jq -r .reason <<< "$OUT")" 'older than 1\.0\.3'

printf 'Keymapp version: 1.3.4' > "$KANSWER"
OUT=$(ml status); calls >/dev/null
check "an answer that is not JSON is no answer, not a keyboard" "false" "$(jq -r .available <<< "$OUT")"

OUT=$(env -i HOME="$WORK/home" PATH="/usr/bin:/bin" bash "$CLI" status 2>/dev/null)
check "no kontroll: says so" '{"available":false,"reason":"kontroll not installed"}' "$OUT"
OUT=$(env -i HOME="$WORK/home" PATH="/usr/bin:/bin" bash "$CLI" layer 2 2>&1); ST=$?
check "and a verb that sets something fails, saying why" "1|kontroll is not installed" "$ST|$OUT"

ml layer 3 >/dev/null; check "layer 3: set-layer --index 3" "set-layer --index 3" "$(calls)"
ml layer --stdin >/dev/null; ST=$?
check "layer --stdin: refused" "1" "$ST"
check "and kontroll never asked" "" "$(calls)"
KFAIL=1 ml layer 2 >/dev/null; ST=$?; calls >/dev/null
check "a set-layer kontroll fails is a failure, not a quiet success" "1" "$ST"
ml unset-layer >/dev/null; check "unset-layer: back to layer 0" "set-layer --index 0" "$(calls)"
ml brightness up 3 >/dev/null; check "brightness up 3" "increase-brightness --steps 3" "$(calls)"
ml brightness down >/dev/null; check "brightness down: one step" "decrease-brightness --steps 1" "$(calls)"
ml brightness up 08 >/dev/null; check "brightness up 08: eight steps, not an octal error" "increase-brightness --steps 8" "$(calls)"
for bad_args in "sideways" "up 0" "up 256" "up x"; do
    # shellcheck disable=SC2086
    ml brightness $bad_args >/dev/null; ST=$?
    check "brightness $bad_args: refused, nothing asked" "1|" "$ST|$(calls)"
done
ml connect >/dev/null; check "connect: connect-any" "connect-any" "$(calls)"
ml connect 1 >/dev/null; check "connect 1: connect --index 1" "connect --index 1" "$(calls)"
ml connect x >/dev/null; ST=$?; check "connect x: refused, nothing asked" "1|" "$ST|$(calls)"
ml disconnect >/dev/null; check "disconnect" "disconnect" "$(calls)"

check "color #89b4fa: held, and said" "#89b4fa|set-rgb-all --color #89b4fa --sustain 0" "$(ml color '#89b4fa')|$(calls)"
check "and without its #" "#cba6f7|set-rgb-all --color #cba6f7 --sustain 0" "$(ml color cba6f7)|$(calls)"
for bad_colour in "" "not-a-colour" "#12345" "#1234567" "--stdin" "#12345g"; do
    ml color "$bad_colour" >/dev/null; ST=$?
    check "color '$bad_colour': refused, nothing asked" "1|" "$ST|$(calls)"
done
KFAIL=1 ml color '#89b4fa' >/dev/null; ST=$?; calls >/dev/null
check "a colour kontroll fails to set is a failure" "1" "$ST"

ml sideways >/dev/null; ST=$?
check "any other verb: refused, nothing asked" "1|" "$ST|$(calls)"
check "version: VERSION's" "$(cat "$ROOT/VERSION")" "$(ml version)"

# A Keymapp that takes the connection and never answers: every call bounded,
# since JayOS runs `color` in the wallpaper's background and `layer` from a
# keybinding, where a stuck call is a process nobody sees.
T0=$SECONDS
KHANG=1 ml layer 2 >/dev/null; ST=$?; calls >/dev/null
check "a kontroll that never answers: given up on, as a failure, within its five seconds" "1|yes" \
    "$ST|$( (( SECONDS - T0 <= 8 )) && echo yes || echo "no, $(( SECONDS - T0 ))s")"
check "and every call but status's is made through the bounded one" "" \
    "$(grep -nE '(^|[^_[:alnum:]-])kontroll (set|connect|disconnect|increase|decrease)' <<< "$(strip_sh "$CLI")")"
check "and it reads nothing of the shell's: no settings.json, palette or session" "" \
    "$(grep -nE 'settings\.json|qs_settings|QS_COLORS|qs_colors|hyprctl|quickshell|SHELL_QML' <<< "$(strip_sh "$CLI")")"

# The layer names: through the package's own jayos-keymapp-layers, found
# where the package puts it, and the envelope even where it is missing.
check "layer-names: the helper's envelope" '{"names":["Base","Symbols","Media"],"error":""}' "$(ml layer-names)"
OUT=$(env -i HOME="$WORK/home" PATH="$FAKE:/usr/bin:/bin" QS_PKG_BIN_DIR="$WORK/nowhere" bash "$CLI" layer-names 2>/dev/null); ST=$?
check "and with the helper missing, the same envelope saying so, and 0" \
    '{"names":[],"error":"jayos-keymapp-layers is not installed"}|0' "$OUT|$ST"

# ── which udev rule lets you open a ZSA keyboard ─────────────────────────────
step "access says what the udev rules grant, not only that one names 3297"

# JayOS's doctor asked whether any rule named ZSA's vendor id, and so said
# "present" over zsa-udev's, which Keymapp's AUR package brings and which give
# the keyboard to a plugdev group Arch does not create. The rules below are
# written here, in the shapes those take: no file of zsa-udev's is copied.
UD="$WORK/udev"; mkdir -p "$UD/etc" "$UD/lib" "$WORK/zsabus/sys/bus/usb/devices"
access() {
    env -i HOME="$WORK/home" PATH="${ACCESS_PATH:-/usr/bin:/bin}" \
        ZSA_UDEV_DIRS="$UD/etc $UD/lib" QS_USB_ROOT="$WORK/zsabus" bash "$CLI" access 2>&1
    echo "[$?]"
}
check "no ZSA keyboard and no Keymapp: n/a, and nothing read" \
    "n/a: no ZSA keyboard is plugged in, and Keymapp is not installed|[0]" "$(access | paste -sd'|')"
mkdir -p "$FAKE/km"; printf '#!/usr/bin/env bash\n' > "$FAKE/km/keymapp"; chmod +x "$FAKE/km/keymapp"
check "Keymapp installed, and no rule: no, and why" "no: no udev rule names ZSA's vendor id, 3297|[1]" \
    "$(ACCESS_PATH="$FAKE/km:/usr/bin:/bin" access | paste -sd'|')"
mkdir -p "$WORK/zsabus/sys/bus/usb/devices/1-4"; echo 3297 > "$WORK/zsabus/sys/bus/usb/devices/1-4/idVendor"
NOGROUP="zsa-suite-nogroup-$$"
printf 'SUBSYSTEM=="usb", ATTR{idVendor}=="3297", GROUP="%s"\n' "$NOGROUP" > "$UD/lib/50-oryx.rules"
check "a board plugged in, and a rule giving it to a group this machine lacks: no" \
    "no: 50-oryx.rules gives it to the group $NOGROUP, which this machine does not have|[1]" "$(access | paste -sd'|')"
OTHER=$(getent group | cut -d: -f1 | grep -vxF -f <(id -nG | tr ' ' '\n') | head -1)
if [[ -n "$OTHER" ]]; then
    printf 'SUBSYSTEM=="usb", ATTR{idVendor}=="3297", GROUP="%s"\n' "$OTHER" > "$UD/lib/50-oryx.rules"
    check "to a group that exists and you are not in: no" \
        "no: 50-oryx.rules gives it to the group $OTHER, which you are not in|[1]" "$(access | paste -sd'|')"
else
    skip "a group you are not in not exercised: every group on this machine is one of yours"
fi
printf 'SUBSYSTEM=="usb", ATTR{idVendor}=="3297", GROUP="%s"\n' "$(id -gn)" > "$UD/lib/50-oryx.rules"
check "to a group you are in: ok" "ok: 50-oryx.rules gives it to $(id -gn), which you are in|[0]" "$(access | paste -sd'|')"
printf '# SUBSYSTEM=="usb", ATTR{idVendor}=="3297", TAG+="uaccess"\nSUBSYSTEMS=="usb", ATTRS{idVendor}=="3297", \\\n    ENV{ID_MM_DEVICE_IGNORE}="1"\n' \
    > "$UD/lib/50-oryx.rules"
check "a rule naming 3297 across a continued line, granting nothing, and a comment that would: no" \
    "no: 50-oryx.rules names ZSA's vendor id and opens nothing|[1]" "$(access | paste -sd'|')"
printf 'SUBSYSTEMS=="usb", ATTRS{idVendor}=="3297", MODE:="0666"\n' > "$UD/lib/60-wide.rules"
check "a mode that opens it to every account: ok" "ok: 60-wide.rules opens it to every account on the machine|[0]" \
    "$(access | paste -sd'|')"
rm -f "$UD/lib/60-wide.rules"
cp "$RULE" "$UD/lib/"
check "the rule this package ships: ok, to whoever is at the machine" \
    "ok: 70-zsa-uaccess.rules opens it to whoever is at the machine|[0]" "$(access | paste -sd'|')"
: > "$UD/etc/70-zsa-uaccess.rules"
check "and an empty file of its name in /etc masks it, as udev reads it" \
    "no: 50-oryx.rules names ZSA's vendor id and opens nothing|[1]" "$(access | paste -sd'|')"
rm -f "$UD/etc/70-zsa-uaccess.rules"
RULE_CODE=$(grep -vE '^[[:space:]]*#' "$RULE" | grep -v '^$')
check "the rule tags ZSA's raw HID node, its USB device and the Moonlander's bootloader, and nothing else" \
    'SUBSYSTEM=="hidraw", ATTRS{idVendor}=="3297", TAG+="uaccess"|SUBSYSTEM=="usb", ATTR{idVendor}=="3297", TAG+="uaccess"|SUBSYSTEM=="usb", ATTR{idVendor}=="0483", ATTR{idProduct}=="df11", TAG+="uaccess"' \
    "$(paste -sd'|' <<< "$RULE_CODE")"
check "and sorts before systemd's 73-seat-late.rules, which acts on the tag" "yes" \
    "$([[ "$(basename "$RULE")" < "73-seat-late.rules" ]] && echo yes || echo no)"

echo
printf '%d passed, %d failed' "$PASS" "$FAIL"
(( SKIP )) && printf ', %d not exercised' "$SKIP"
printf '\n'
(( FAIL == 0 ))
