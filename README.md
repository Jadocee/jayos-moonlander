# jayos-moonlander

ZSA Moonlander support behind the [JayOS](https://github.com/Jadocee/jayos)
shell: the layout Keymapp flashed, read out of Keymapp's own database; a
daemon that says when a ZSA keyboard is plugged in, unplugged or changes
layer; a command over kontroll for the board's layer, brightness, colour and
connection; and the udev rule that opens the board to whoever is at the
machine, without which Keymapp cannot see it. Built for JayOS, usable without
it.

It moved here from JayOS on 2026-11-12, by the owner's call "Move all zsa
moonlander keyboard related stuff out of JayOS and into jayos-moonlander; add
releases to jayos-moonlander", read with the exception that call's Bluetooth
counterpart made: the UI and QML stay in JayOS. The held-Super panel, the
overlay, the setup wizard, the lock screen's chip, the settings and the
shell's own glue -- the LED colour taken from the wallpaper, the layer that
follows the focused window, QWERTY for a game -- are JayOS's, and run the
commands here from `/usr/bin`. Nothing here has met a real Moonlander or a
real Keymapp: what runs, runs against a stand-in kontroll, a fake USB tree and
a fake Keymapp, and `docs/MOONLANDER.md` says which claims stand on what.

## What is here

| | |
|---|---|
| `src/bin/jayos-keymapp-layers.rs` | Keymapp's database, read-only: the revision on the keyboard, its layers' titles and keys. `names`, `parse`, `name <n>` and `layout`. |
| `src/bin/jayos-moonlander-osd.rs` | The popups' daemon: the USB bus through inotify, Keymapp's layer over its gRPC socket, the shell's `osd.sh` for a popup, and three files for the shell. |
| `src/support/` | Six files of JayOS's `jayos-core`, copied; its `mod.rs` says why a copy. |
| `src/jayos-moonlander` | The command: `status`, `layer-names`, `layer`, `unset-layer`, `brightness`, `color`, `connect`, `disconnect`, `access` and `version`. Bash over kontroll, as it was in JayOS. |
| `share/udev/70-zsa-uaccess.rules` | The rule that opens the board, installed to `/usr/lib/udev/rules.d/`. It says why this one and not ZSA's own. |
| `contract/` | What JayOS tests its side against: the verbs, the daemon's files, the packaging steps and every installed path. See `contract/README.md`. |
| `packaging/steps.sh` | Every build flag, proof and installed file, once, for both recipes. |
| `packaging/arch/` | The recipe for this checkout, and the install file both recipes carry, which puts the rule into effect for a board already plugged in. |
| `tests/shell/` | The command against a stand-in kontroll and udev rules written there; the two programs, built, against a Keymapp database, a fake bus and a fake Keymapp; the packaging; the releases. |
| `tools/` | `check.sh`, everything checked in one run; `contract.sh`; and the release scripts. |
| `docs/` | `MOONLANDER.md`, how it works and what each claim stands on; `RELEASES.md`, how a release is made. |

## Installing

On Arch, from a checkout:

    cd packaging/arch && makepkg -si

That installs `/usr/bin/jayos-keymapp-layers`, `/usr/bin/jayos-moonlander-osd`,
`/usr/bin/jayos-moonlander`, the udev rule and the contract
(`contract/files.txt` is the whole list). Keymapp and kontroll are ZSA's, in
the AUR as `zsa-keymapp-bin` and `zsa-kontroll`, and are optional
dependencies: without them `jayos-moonlander status` says what is missing,
and the plug popups need neither. JayOS carries a recipe of its own,
`pkgbuilds/jayos-moonlander/PKGBUILD` there, which builds a release of this
repository pinned by tag and by commit, and which only its Moonlander setup in
the control room runs -- nothing of the Moonlander's is installed until
somebody opts in there.

Without JayOS: `jayos-moonlander status` and the other verbs work from a
terminal once Keymapp's API is started in its settings, and
`jayos-moonlander-osd --probe` prints what the daemon sees. The daemon's
popups go through JayOS's `osd.sh`; without one, it watches and says nothing.

## Checking

    tools/check.sh

Runs a syntax check over every script, the contract against the code,
`cargo build`, `cargo test --locked` and `cargo fmt --check`, and every suite
in `tests/shell/`. Exit 0 is clean, 1 is a failure, 2 is "could not run" (no
cargo, no jq) -- which is not the same answer as clean and is not reported as
one.

## Releases

A release is a signed tag, `vMAJOR.MINOR.PATCH`, on `main`, whose `VERSION`
says its version; `tools/release.sh` makes it, in the owner's checkout, and
the release workflow checks and publishes it. JayOS builds a release and
nothing else. `docs/RELEASES.md` has how, and the owner's steps before the
first one.

## Licence

MIT; see `LICENSE`.
