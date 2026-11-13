#!/usr/bin/env bash
#
# The build, check and install steps of the jayos-moonlander package.
#
#   steps.sh fetch               cargo fetch, locked, for this host only
#   steps.sh build               the release build of both programs
#   steps.sh check               what they link, the crate's tests, and
#                                tests/shell's suites over the build
#   steps.sh package <pkgdir>    install everything under <pkgdir>
#
# Every step works on the tree this script sits in. Exit 0 when the step did
# what it says, 1 when it did not, with a line on stderr saying why.
#
# WHY A SCRIPT, AND NOT THE STEPS WRITTEN INTO A RECIPE
#
# Two recipes build this package. packaging/arch/PKGBUILD builds the checkout
# it sits in; JayOS's pkgbuilds/jayos-moonlander/PKGBUILD clones the release
# it pins -- a signed tag, and the commit that tag named when it was pinned
# (docs/RELEASES.md) -- and builds that. Two recipes that each spelled out the
# cargo flags and the file list would drift, and the drift that matters is
# silent: a udev rule installed by one recipe and not the other is a keyboard
# Keymapp cannot open on one machine and can on the next, with nothing
# failing. So both call this, and the only thing either recipe owns is where
# its sources come from (and the install file, which makepkg reads from beside
# the recipe alone; packaging/arch/jayos-moonlander.install says so).
# jayos-bluetooth's packaging/steps.sh is the model, for the same reason.
#
# The verbs and their arguments are the interface JayOS's recipe calls, and
# contract/steps.txt lists them; renaming one breaks JayOS's build. What
# `package` installs is contract/files.txt, which JayOS reads to know where
# the programs are, and tests/shell/test-packaging.sh holds this verb to that
# list exactly, both ways.

set -uo pipefail

ROOT="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")/.." && pwd)"
BINS=(jayos-keymapp-layers jayos-moonlander-osd)
PKG=jayos-moonlander

die() { echo "steps.sh: $*" >&2; exit 1; }

usage() {
    echo "usage: steps.sh fetch | build | check | package <pkgdir>" >&2
    exit 1
}

# The toolchain name is for rustup, which is what JayOS's toolchain stage
# installs; a plain rust package ignores it. CARGO_TARGET_DIR under the tree
# even when the building user set one globally, so check and package find the
# binaries where they look and a `makepkg -C` cleans them.
cargo_env() {
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR="$ROOT/target"
    [[ -f "$ROOT/Cargo.toml" && -f "$ROOT/Cargo.lock" ]] \
        || die "no Cargo.toml and Cargo.lock in $ROOT, so nothing can be built"
}

# MOLD LINKS WHAT THIS BUILDS, the release build and the tests, by JayOS's
# owner's call of 2026-11-13, "use https://github.com/rui314/mold for building
# rust". As a link argument to the C compiler rustc drives, which takes over
# from Rust's own LLD (.cargo/config.toml says how, and what was run to show
# it), and through RUSTFLAGS rather than that file: makepkg.conf sets
# RUSTFLAGS, and cargo takes that in place of every config's flags. Appended to
# what is there, once, so makepkg's own flags stay and a flag somebody set
# already is not repeated.
#
# Refused before cargo where mold is missing: both recipes name it among their
# makedepends, so a build without it is one makepkg did not prepare, and it
# would fail at its first link. Only the steps that link ask; fetch downloads
# and links nothing.
MOLD_FLAG="-C link-arg=-fuse-ld=mold"
link_with_mold() {
    command -v ld.mold >/dev/null 2>&1 \
        || die "mold is not installed, and this build links with it (pacman -S mold)"
    [[ " ${RUSTFLAGS:-} " == *" $MOLD_FLAG "* ]] || export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }$MOLD_FLAG"
}

step_fetch() {
    local host
    cargo_env
    # The lockfile is the pin for everything the release's commit does not
    # cover: --locked refuses to resolve anything Cargo.lock does not already
    # name, and the host target keeps the fetch to what this machine builds,
    # so build and check can run --frozen where makepkg gives them no network.
    host=$(rustc -vV 2>/dev/null | sed -n 's/^host: //p')
    [[ -n "$host" ]] || die "rustc did not say which target this host is; is a toolchain selected? (rustup default stable)"
    ( cd "$ROOT" && cargo fetch --locked --target "$host" ) || die "cargo fetch failed"
}

step_build() {
    local b args=()
    cargo_env
    link_with_mold
    for b in "${BINS[@]}"; do args+=(--bin "$b"); done
    # Cargo.toml's release profile, JayOS's settings when these lived there:
    # fat LTO, one codegen unit and panic = "abort", for a daemon the shell
    # restarts when it ends and a reader run once per question.
    ( cd "$ROOT" && cargo build --frozen --release "${args[@]}" ) || die "cargo build failed"
}

# What the binaries need at run time is what depends= installs, and nothing
# more: glibc's libc, libm and loader, and gcc-libs' libgcc_s for the
# unwinder std still links with panic = "abort". SQLite is compiled in
# (rusqlite's `bundled`), so no libsqlite3 -- a NEEDED entry for one would be
# a dependency that arrived with a crate update unannounced, and a program
# that links a library depends= does not install runs on the build machine
# and nowhere else. readelf is binutils', which base-devel brings and makepkg
# requires.
links_only_depends() { # links_only_depends <binary>
    local needed outside
    command -v readelf >/dev/null 2>&1 || die "readelf is not installed (binutils), so what ${1##*/} links cannot be read"
    needed=$(readelf -d "$1" | sed -n 's/.*(NEEDED).*\[\(.*\)\]$/\1/p') || true
    if [[ -z "$needed" ]]; then
        echo "readelf read no NEEDED entry from ${1##*/}" >&2
        return 1
    fi
    outside=$(grep -vxE 'libc\.so\.6|libm\.so\.6|libgcc_s\.so\.1|ld-linux-x86-64\.so\.2|ld-linux-aarch64\.so\.1' <<<"$needed") || true
    if [[ -n "$outside" ]]; then
        echo "${1##*/} links what depends= does not install: ${outside//$'\n'/ }" >&2
        return 1
    fi
}

step_check() {
    local b suite status
    cargo_env
    link_with_mold
    for b in "${BINS[@]}"; do
        [[ -x "$ROOT/target/release/$b" ]] || die "no $ROOT/target/release/$b; build first"
        links_only_depends "$ROOT/target/release/$b" || exit 1
    done
    # The dev profile's tests, not the release build's: tests ignore
    # panic = "abort", and fat LTO over a test harness would double the step
    # for nothing it tests.
    ( cd "$ROOT" && cargo test --frozen ) || die "the crate's tests failed"
    # And the release binaries and the CLI themselves, run the way JayOS runs
    # them: against a stand-in kontroll, a fake USB tree and a fake Keymapp,
    # never the machine's. A 2 is a failure here -- a suite saying it examined
    # nothing, over binaries that were just built -- since python3, which the
    # fake Keymapp needs, is in this recipe's checkdepends.
    for suite in test-cli.sh test-layers.sh test-osd.sh; do
        [[ -f "$ROOT/tests/shell/$suite" ]] || die "no tests/shell/$suite to run the build against"
        JAYOS_MOONLANDER_BIN_DIR="$ROOT/target/release" bash "$ROOT/tests/shell/$suite"
        status=$?
        (( status == 0 )) || die "tests/shell/$suite exited $status against the built package"
    done
}

step_package() {
    local pkgdir share
    [[ -n "${1:-}" ]] || usage
    pkgdir=$1
    share="$pkgdir/usr/share"

    # /usr/bin itself: JayOS's shell runs them by that path (its paths.sh's
    # QS_PKG_BIN, and Paths.qml's pkgBin), as it runs jayos-bluetooth's.
    install -Dm755 "${BINS[@]/#/$ROOT/target/release/}" -t "$pkgdir/usr/bin/" \
        || die "could not install ${BINS[*]}; build first"
    install -Dm755 "$ROOT/src/jayos-moonlander" "$pkgdir/usr/bin/jayos-moonlander" \
        || die "could not install src/jayos-moonlander"

    # The rule that opens the board, in udev's package directory: read after
    # /etc and /run, so a person's file of the same name there masks it, and
    # never a path another package owns (share/udev/70-zsa-uaccess.rules says
    # why that name).
    install -Dm644 "$ROOT/share/udev/70-zsa-uaccess.rules" "$pkgdir/usr/lib/udev/rules.d/70-zsa-uaccess.rules" \
        || die "could not install share/udev/70-zsa-uaccess.rules"

    # The contract, for a machine that wants to compare what it runs against
    # what a desktop built on this vendored.
    install -d "$share/$PKG/contract" || die "could not make the contract directory"
    install -m644 "$ROOT"/contract/* "$share/$PKG/contract/" || die "could not install the contract"

    install -Dm644 "$ROOT/LICENSE" -t "$share/licenses/$PKG/" || die "could not install the licence"
}

case "${1:-}" in
    fetch)   step_fetch ;;
    build)   step_build ;;
    check)   step_check ;;
    package) step_package "${2:-}" ;;
    *)       usage ;;
esac
