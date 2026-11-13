#!/usr/bin/env bash
#
# The package: its version, said in four places, what packaging/steps.sh
# installs against contract/files.txt, the gates its check puts in front of a
# package, and the recipe's calls of it. jayos-bluetooth's
# tests/shell/test-packaging.sh is the model, case for case where the two
# packages agree.
#
#   tests/shell/test-packaging.sh
#
# RUN, not read: `steps.sh package` installs into a scratch pkgdir from a
# scratch copy of this tree with stand-in binaries, and what lands is held to
# contract/files.txt exactly -- a file installed that the list does not name
# fails, and so does a path the list names that nothing installed -- because
# JayOS reads that list to find the commands, and a udev rule installed
# somewhere udev does not read is a keyboard nobody can open, with nothing
# failing. `steps.sh check` runs against a cargo and suites that only answer,
# over binaries readelf can read: /bin/true, which links libc alone, passes
# the NEEDED gate, and a program that links a library depends= does not
# install is refused. The recipe's functions run with a steps.sh that writes
# down what it was asked, and its install file's functions with a udevadm that
# does.
#
# Exit 0 when every case passes, 1 when any fails, 2 when it could not run.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
STEPS="$ROOT/packaging/steps.sh"
RECIPE="$ROOT/packaging/arch/PKGBUILD"
FILES="$ROOT/contract/files.txt"
INSTALL="$ROOT/packaging/arch/jayos-moonlander.install"
for f in "$STEPS" "$RECIPE" "$INSTALL" "$FILES" "$ROOT/VERSION" "$ROOT/Cargo.toml" "$ROOT/contract/steps.txt" \
         "$ROOT/src/jayos-moonlander"; do
    [[ -r "$f" ]] || { echo "test-packaging not exercised: cannot read ${f#"$ROOT"/}" >&2; exit 2; }
done

WORK="$(mktemp -d "${TMPDIR:-/tmp}/jayos-moonlander-packaging.XXXXXX")" || exit 2
trap 'rm -rf "$WORK"' EXIT

PASS=0
FAIL=0
SKIP=0
pass() { PASS=$((PASS + 1)); printf '  \033[32mok\033[0m   %s\n' "$1"; }
bad()  { FAIL=$((FAIL + 1)); printf '  \033[31mfail\033[0m %s\n' "$1"; }
skip() { printf '  skip %s\n' "$1"; SKIP=$((SKIP + 1)); }
check() { # check <what> <expected> <actual>
    if [[ "$2" == "$3" ]]; then pass "$1"; else
        bad "$1"; printf '         expected: %s\n' "$2"; printf '         actual:   %s\n' "$3"; fi
}
says() { if grep -qF -- "$2" <<< "$3"; then pass "$1"; else
             bad "$1"; printf '         wanted: %s\n' "$2"; printf '%s\n' "$3" | sed 's/^/         got:    /'; fi; }

# ── one version, four places ─────────────────────────────────────────────────
echo
echo "the version"
VERSION=$(head -n 1 "$ROOT/VERSION")
# The [package] table's own line: the first `version =` before any other
# table, so a dependency's version is not mistaken for the crate's.
CRATE=$(awk '/^\[/ { t = $0 } t == "[package]" && /^version[[:space:]]*=/ { gsub(/[" ]/, "", $0); sub(/^version=/, ""); print; exit }' "$ROOT/Cargo.toml")
PKGVER=$(sed -n 's/^pkgver=//p' "$RECIPE")
check "VERSION is MAJOR.MINOR.PATCH" "1" "$([[ "$VERSION" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]] && echo 1)"
check "Cargo.toml's version is VERSION's: the binary reports the one a release is" "$VERSION" "$CRATE"
check "and so is the checkout recipe's pkgver, on one line" "$VERSION" "$PKGVER"
check "and the command's, which \`jayos-moonlander version\` prints" "$VERSION" \
    "$(sed -n 's/^VERSION_TEXT=//p' "$ROOT/src/jayos-moonlander")"

# ── what package installs, against files.txt ─────────────────────────────────
echo
echo "steps.sh package, against contract/files.txt"
# A scratch tree: everything step_package reads, and a stand-in binary, so the
# case needs no build and cannot be satisfied by an old one in target/.
TREE="$WORK/tree"
mkdir -p "$TREE/target/release"
mkdir -p "$TREE/src"
cp -r "$ROOT/packaging" "$ROOT/share" "$ROOT/contract" "$TREE/"
cp "$ROOT/LICENSE" "$ROOT/Cargo.toml" "$ROOT/VERSION" "$TREE/"
cp "$ROOT/src/jayos-moonlander" "$TREE/src/"
for b in jayos-keymapp-layers jayos-moonlander-osd; do
    printf '#!/bin/sh\nexit 0\n' > "$TREE/target/release/$b"
    chmod 755 "$TREE/target/release/$b"
done

PKG="$WORK/pkg"
OUT=$(bash "$TREE/packaging/steps.sh" package "$PKG" 2>&1); ST=$?
check "package exits 0" 0 "$ST"
WANT=$(grep -vE '^[[:space:]]*(#|$)' "$FILES" | LC_ALL=C sort)
GOT=$( (cd "$PKG" && find . -type f -o -type l) | sed 's|^\./||' | LC_ALL=C sort)
check "it installs every path files.txt names, and nothing else" "$WANT" "$GOT"
EXTRA=$(comm -13 <(printf '%s\n' "$WANT") <(printf '%s\n' "$GOT"))
MISSING=$(comm -23 <(printf '%s\n' "$WANT") <(printf '%s\n' "$GOT"))
[[ -n "$EXTRA" ]] && printf '         installed, not listed: %s\n' $EXTRA
[[ -n "$MISSING" ]] && printf '         listed, not installed: %s\n' $MISSING
check "every file under share/ is one of them: a rule added there is installed" "" \
    "$(cd "$ROOT/share" && find . -type f | sed 's|^\./||' | while IFS= read -r f; do
        case "$f" in
            udev/*.rules) [[ -f "$PKG/usr/lib/udev/rules.d/${f#udev/}" ]] || echo "$f" ;;
            *) echo "$f (no rule installs it)" ;;
        esac
    done)"
check "and byte for byte, as is the command" "" \
    "$(for f in "$ROOT"/share/udev/*.rules; do
           cmp -s "$f" "$PKG/usr/lib/udev/rules.d/${f##*/}" || echo "${f##*/}"
       done
       cmp -s "$ROOT/src/jayos-moonlander" "$PKG/usr/bin/jayos-moonlander" || echo jayos-moonlander)"
check "the three commands are 755, everything else 644" "755 755 755|" \
    "$(stat -c %a "$PKG"/usr/bin/* | paste -sd' ' -)|$(cd "$PKG" && find . -type f ! -path './usr/bin/*' ! -perm 644 | paste -sd' ' -)"
check "the contract it installs lists itself: files.txt names every file in contract/" "" \
    "$(cd "$ROOT/contract" && for f in *; do grep -qxF "usr/share/jayos-moonlander/contract/$f" "$FILES" || echo "$f"; done)"

OUT=$(bash "$TREE/packaging/steps.sh" package 2>&1); ST=$?
check "package with no pkgdir: refused, with the usage" "1" "$ST"
says "naming the verbs" "usage: steps.sh fetch | build | check | package <pkgdir>" "$OUT"
rm -f "$TREE/target/release/jayos-moonlander-osd"
OUT=$(bash "$TREE/packaging/steps.sh" package "$WORK/pkg2" 2>&1); ST=$?
check "package before a build: refused" 1 "$ST"
says "saying so" "could not install jayos-keymapp-layers jayos-moonlander-osd; build first" "$OUT"
OUT=$(bash "$TREE/packaging/steps.sh" frobnicate 2>&1); ST=$?
check "a verb steps.sh does not have: refused" 1 "$ST"

# ── the gates check puts in front of a package ───────────────────────────────
echo
echo "steps.sh check"
if ! command -v readelf >/dev/null 2>&1; then
    skip "check's gates not exercised: readelf (binutils) is not installed"
else
    # A cargo that answers 0 and writes down what it was asked, and three
    # suites that answer as a file says: the gates are steps.sh's, and what
    # cargo or the suites find is their own business.
    mkdir -p "$WORK/bin" "$TREE/tests/shell"
    printf '#!/usr/bin/env bash\nprintf "%%s\\n" "$*" >> %q\nprintf "%%s\\n" "${RUSTFLAGS-<unset>}" >> %q\nexit 0\n' \
        "$WORK/cargo.calls" "$WORK/cargo.flags" > "$WORK/bin/cargo"
    # ld.mold, for steps.sh to find as the recipes' makedepends put it there;
    # nothing runs it, since the cargo links nothing.
    printf '#!/bin/sh\nexit 0\n' > "$WORK/bin/ld.mold"
    chmod +x "$WORK/bin/cargo" "$WORK/bin/ld.mold"
    for s in test-cli.sh test-layers.sh test-osd.sh; do
        printf '#!/usr/bin/env bash\nprintf "%%s %%s\\n" %q "$JAYOS_MOONLANDER_BIN_DIR" >> %q\nexit "$(cat %q 2>/dev/null || echo 0)"\n' \
            "$s" "$WORK/suite.bin" "$WORK/suite.status" > "$TREE/tests/shell/$s"
    done
    touch "$TREE/Cargo.lock"
    stepcheck() { (PATH="$WORK/bin:$PATH" bash "$TREE/packaging/steps.sh" check 2>&1); }

    rm -f "$TREE"/target/release/jayos-*
    OUT=$(stepcheck); ST=$?
    check "no build: refused" 1 "$ST"
    says "saying so" "build first" "$OUT"

    cp /bin/true "$TREE/target/release/jayos-keymapp-layers"
    cp /bin/true "$TREE/target/release/jayos-moonlander-osd"
    : > "$WORK/cargo.calls"; : > "$WORK/suite.bin"
    OUT=$(stepcheck); ST=$?
    check "binaries that link libc alone, tests and suites passing: exit 0" 0 "$ST"
    check "the tests ran frozen, in the dev profile" "test --frozen" "$(cat "$WORK/cargo.calls")"
    check "and each suite was handed the build just made" \
        "test-cli.sh $TREE/target/release|test-layers.sh $TREE/target/release|test-osd.sh $TREE/target/release" \
        "$(paste -sd'|' "$WORK/suite.bin")"

    # The first program here that links something outside glibc and
    # gcc-libs, read with the same readelf the gate uses.
    LINKS=""
    for c in /usr/bin/curl /usr/bin/python3 /usr/bin/xmllint /usr/bin/ssh; do
        [[ -x "$c" ]] || continue
        if readelf -d "$c" 2>/dev/null | sed -n 's/.*(NEEDED).*\[\(.*\)\]$/\1/p' \
            | grep -qvxE 'libc\.so\.6|libm\.so\.6|libgcc_s\.so\.1|ld-linux-x86-64\.so\.2|ld-linux-aarch64\.so\.1'; then
            LINKS=$c
            break
        fi
    done
    if [[ -n "$LINKS" ]]; then
        cp "$LINKS" "$TREE/target/release/jayos-moonlander-osd"
        OUT=$(stepcheck); ST=$?
        check "a binary that links a library depends= does not install: refused" 1 "$ST"
        says "naming the library" "links what depends= does not install" "$OUT"
        cp /bin/true "$TREE/target/release/jayos-moonlander-osd"
    else
        skip "the NEEDED refusal not exercised: no program here links a library outside glibc and gcc-libs"
    fi

    echo 1 > "$WORK/suite.status"
    OUT=$(stepcheck); ST=$?
    check "a suite failing: refused" 1 "$ST"
    echo 2 > "$WORK/suite.status"
    OUT=$(stepcheck); ST=$?
    check "and its 2 is a failure too, over binaries just built" 1 "$ST"
    says "saying the suite's status" "tests/shell/test-cli.sh exited 2" "$OUT"
    rm -f "$WORK/suite.status"

    # MOLD LINKS IT, through RUSTFLAGS: build and check start cargo with mold's
    # flag, after whatever makepkg.conf set and once; and with no ld.mold each
    # refuses before cargo, naming it. A cargo config would be read and
    # replaced by makepkg's RUSTFLAGS, which is why steps.sh says mold itself.
    : > "$WORK/cargo.flags"
    OUT=$(stepcheck); ST=$?
    check "check starts cargo with mold's flag in RUSTFLAGS" "0|-C link-arg=-fuse-ld=mold" \
        "$ST|$(paste -sd'|' "$WORK/cargo.flags")"
    stepbuild() { (env -u RUSTFLAGS PATH="$WORK/bin:$PATH" "$@" bash "$TREE/packaging/steps.sh" build 2>&1); }
    : > "$WORK/cargo.flags"; OUT=$(stepbuild); ST=$?
    check "and build does too" "0|-C link-arg=-fuse-ld=mold" "$ST|$(paste -sd'|' "$WORK/cargo.flags")"
    : > "$WORK/cargo.flags"; OUT=$(stepbuild RUSTFLAGS=-Cforce-frame-pointers=yes); ST=$?
    check "after the RUSTFLAGS makepkg set, which stay" "0|-Cforce-frame-pointers=yes -C link-arg=-fuse-ld=mold" \
        "$ST|$(paste -sd'|' "$WORK/cargo.flags")"
    : > "$WORK/cargo.flags"; OUT=$(stepbuild RUSTFLAGS="-C link-arg=-fuse-ld=mold"); ST=$?
    check "and not a second time over a RUSTFLAGS that has it already" "0|-C link-arg=-fuse-ld=mold" \
        "$ST|$(paste -sd'|' "$WORK/cargo.flags")"
    # A PATH of links to every command the fakes and this machine have but
    # ld.mold, so a machine that has mold runs these as one that does not.
    NOMOLD="$WORK/no-mold-bin"
    mkdir -p "$NOMOLD"
    IFS=: read -ra _dirs <<<"$WORK/bin:$PATH"
    for _d in "${_dirs[@]}"; do
        for _f in "$_d"/*; do
            [[ -x "$_f" && ! -d "$_f" && "${_f##*/}" != ld.mold && ! -e "$NOMOLD/${_f##*/}" ]] && ln -s "$_f" "$NOMOLD/${_f##*/}"
        done
    done
    for verb in build check; do
        : > "$WORK/cargo.calls"
        OUT=$(PATH="$NOMOLD" bash "$TREE/packaging/steps.sh" "$verb" 2>&1); ST=$?
        check "with no mold, $verb refuses before cargo is asked anything" "1|1|" \
            "$ST|$(grep -c 'mold is not installed, and this build links with it' <<<"$OUT")|$(cat "$WORK/cargo.calls")"
    done
fi

# ── the recipe calls the steps, and nothing else ─────────────────────────────
echo
echo "packaging/arch/PKGBUILD"
# Its functions, run as makepkg runs them -- each in its own errexit shell --
# with startdir in a scratch tree whose steps.sh writes down what it is asked.
RT="$WORK/recipe"
mkdir -p "$RT/packaging/arch"
cp "$RECIPE" "$RT/packaging/arch/PKGBUILD"
printf '#!/usr/bin/env bash\nprintf "%%s\\n" "$*" >> %q\n' "$WORK/steps.calls" > "$RT/packaging/steps.sh"
: > "$WORK/steps.calls"
for fn in prepare build check package; do
    (cd "$RT/packaging/arch" && startdir="$RT/packaging/arch" srcdir="$WORK/src" pkgdir="$WORK/pkgdir" \
        bash -eE -c "source ./PKGBUILD; $fn") || bad "the recipe's $fn() failed"
done
check "prepare, build, check and package call fetch, build, check and package <pkgdir>" \
    "fetch|build|check|package $WORK/pkgdir" "$(paste -sd'|' "$WORK/steps.calls")"
check "every verb it calls is one contract/steps.txt lists" "" \
    "$(cut -d' ' -f1 "$WORK/steps.calls" | while IFS= read -r v; do
        grep -vE '^#' "$ROOT/contract/steps.txt" | cut -d' ' -f1 | grep -qxF "$v" || echo "$v"; done)"
check "it names no source of its own: the checkout is the source" "" \
    "$(sed -n 's/^source=(\(.*\))$/\1/p' "$RECIPE")"
DEPS=$(sed -n "s/^depends=(\(.*\))$/\1/p" "$RECIPE" | tr -d "'")
check "depends: the binaries' libraries, the command's tools, and systemd's udevadm" \
    "glibc gcc-libs bash jq coreutils procps-ng gawk grep systemd" "$DEPS"
check "and the install file beside it is the recipe's" "jayos-moonlander.install" "$(sed -n 's/^install=//p' "$RECIPE")"

# ── the install file ─────────────────────────────────────────────────────────
echo
echo "packaging/arch/jayos-moonlander.install"
# post_install and post_upgrade, sourced as pacman sources the file, with a
# udevadm that writes down what it was asked and answers as a file says.
mkdir -p "$WORK/udevbin"
printf '#!/usr/bin/env bash\nprintf "%%s\\n" "$*" >> %q\nexit "$(cat %q 2>/dev/null || echo 0)"\n' \
    "$WORK/udevadm.calls" "$WORK/udevadm.status" > "$WORK/udevbin/udevadm"
chmod +x "$WORK/udevbin/udevadm"
for fn in post_install post_upgrade; do
    : > "$WORK/udevadm.calls"
    OUT=$(PATH="$WORK/udevbin:$PATH" bash -c "source $(printf %q "$INSTALL"); $fn" 2>&1); ST=$?
    check "$fn: udev reloaded, then told of the USB and hidraw devices, in that order" \
        "control --reload-rules|trigger --action=change --subsystem-match=usb --subsystem-match=hidraw|0" \
        "$(paste -sd'|' "$WORK/udevadm.calls")|$ST"
done
echo 1 > "$WORK/udevadm.status"; : > "$WORK/udevadm.calls"
OUT=$(PATH="$WORK/udevbin:$PATH" bash -c "source $(printf %q "$INSTALL"); post_install" 2>&1); ST=$?
check "a udevadm that fails: said, never a failed install, and no trigger after a failed reload" "0|control --reload-rules" \
    "$ST|$(paste -sd'|' "$WORK/udevadm.calls")"
says "and the board's reconnect named as the remedy" "plug the keyboard in again" "$OUT"
rm -f "$WORK/udevadm.status"

MAKEDEPS=$(sed -n "s/^makedepends=(\(.*\))$/\1/p" "$RECIPE" | tr -d "'")
check "makedepends: cargo, git and mold, which steps.sh links with" "cargo git mold" "$MAKEDEPS"

# ── the crate is linked with mold ────────────────────────────────────────────
echo
echo ".cargo/config.toml and tools/check.sh: mold links the crate"
# The file's one setting, as a TOML reader reads it: mold's flag for the x86_64
# target, which cargo run in this tree reads -- tools/check.sh's, and a build by
# hand. steps.sh appends the same flag to RUSTFLAGS itself, spelt alike.
MOLD_CFG="$ROOT/.cargo/config.toml"
if [[ ! -f "$MOLD_CFG" ]]; then
    bad ".cargo/config.toml is gone, and nothing links the crate with mold"
elif python3 -c 'import tomllib' 2>/dev/null; then
    check "it holds one setting: mold, as the x86_64 linker's argument" \
        '{"target": {"x86_64-unknown-linux-gnu": {"rustflags": ["-C", "link-arg=-fuse-ld=mold"]}}}' \
        "$(python3 -I -c 'import json, sys, tomllib; print(json.dumps(tomllib.load(open(sys.argv[1], "rb")), sort_keys=True))' "$MOLD_CFG" 2>&1)"
else
    skip ".cargo/config.toml as TOML not exercised: no python3 with tomllib"
fi
check "and packaging/steps.sh appends the same flag" "1" \
    "$(grep -vE '^[[:space:]]*#' "$STEPS" | grep -cxF 'MOLD_FLAG="-C link-arg=-fuse-ld=mold"')"
# tools/check.sh's crate step, read rather than run, since running it is a
# cargo build: its build and its tests each through the one array an empty
# RUSTFLAGS goes into, only where there is no ld.mold, with the counted skip.
RUST_STEP=$(sed -n '/^rust_checks() {$/,/^}$/p' "$ROOT/tools/check.sh" | grep -vE '^[[:space:]]*#')
check "tools/check.sh builds and tests through it, falling back only with no ld.mold, said as a skip" "2|1|1|1" \
    "$(grep -cE '^[[:space:]]+env "\$\{link\[@\]\}" cargo (build|test) --locked \|\| return 1$' <<<"$RUST_STEP")|$(grep -cE '^[[:space:]]+if ! command -v ld\.mold >/dev/null 2>&1; then$' <<<"$RUST_STEP")|$(grep -cE '^[[:space:]]+link=\(RUSTFLAGS=\)$' <<<"$RUST_STEP")|$(grep -cF 'skip linking the crate with mold not exercised' <<<"$RUST_STEP")"

echo
SKIPPED=""
(( SKIP > 0 )) && SKIPPED=", $SKIP not exercised"
if (( FAIL )); then
    printf '  \033[31m%d passed, %d failed%s\033[0m\n' "$PASS" "$FAIL" "$SKIPPED"
    exit 1
fi
printf '  \033[32m%d passed%s\033[0m\n' "$PASS" "$SKIPPED"
exit 0
