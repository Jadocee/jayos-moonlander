# The Moonlander, from below the shell

What this package does, why each piece is the way it is, and what each claim
stands on. How the shell uses it -- the held-Super panel, the overlay, the
setup, the layer that follows the focused window -- is JayOS's
`docs/CONTROLLER.md`, *ZSA Moonlander*.

## What moved, and what stayed

On 2026-11-12 the owner asked: "Move all zsa moonlander keyboard related stuff
out of JayOS and into jayos-moonlander; add releases to jayos-moonlander".
Asked which reading, the owner chose the one the Bluetooth move of the same
day made -- everything but the UI and QML -- and that the package reach a
machine only through JayOS's Moonlander setup, as everything of the
Moonlander's has since the owner's call of 2026-10-26 that nothing of it is
installed until somebody opts in from the control room.

| Here, from JayOS at fdde9c4 | Was there |
|---|---|
| `src/bin/jayos-keymapp-layers.rs` | `rust/crates/jayos-hw/src/bin/jayos-keymapp-layers.rs`, whole |
| `src/bin/jayos-moonlander-osd.rs` | `rust/crates/jayos-hw/src/bin/jayos-moonlander-osd.rs`, whole but where it finds its helper |
| `src/support/` | six files of `rust/crates/jayos-core/src/`, copied |
| `src/jayos-moonlander` | the keyboard's verbs of `config/common/scripts/hypr/moonlander.sh` |
| `share/udev/70-zsa-uaccess.rules` | `system/udev/70-zsa-uaccess.rules`, written by the root helper's `zsa-rule` verb |
| `packaging/arch/jayos-moonlander.install` | that verb's udev reload and trigger |
| `tests/shell/test-cli.sh`, `test-layers.sh`, `test-osd.sh` | the matching sections of `tools/test-moonlander.sh` |

What stayed in JayOS is what draws, and what reads the shell: the settings,
the panel, the overlay, the setup wizard and its state, the lock screen's
chip, the doctor's section, and the parts of `moonlander.sh` that need the
shell -- `theme`, which takes the colour from the wallpaper's palette and the
control room's row and hands it to `jayos-moonlander color`; `watch`, which
follows Hyprland's focus; `qwerty`; `panel` and `overlay`, which call the
shell's IPC; and `setup` and `setup-state`. It hands every verb listed in
`contract/verbs.txt` on to `/usr/bin/jayos-moonlander`, and starts
`/usr/bin/jayos-moonlander-osd` through its own daemon lifecycle.

## The command

`jayos-moonlander` drives ZSA's kontroll, a command line over Keymapp's gRPC
API, rather than speaking gRPC from bash. Its header gives each kontroll call
as the 1.0.4 interface spells it, and the history: JayOS's script once called
`set-layer 2`, `set-rgb-all r g b 0`, `unset-layer` and `set-brightness`,
which kontroll 1.0.x refuses, and every call's output went to `/dev/null`, so
nothing changed and nothing said so; and it tested a TCP port that kontroll
1.0.1 and later never use on Linux.

- **`status` tells three failures apart**: kontroll missing or older than
  1.0.3, Keymapp not running, and Keymapp running with its API off, which is
  Keymapp's default. A Keymapp connected to no keyboard is a fourth. Each
  call is bounded at five seconds, since a Keymapp that accepts the
  connection and never answers held a call for good.
- **`color` takes a colour and chooses none.** JayOS chooses it, from the
  wallpaper's palette; this checks its shape, six hexadecimal digits, so that
  nothing else reaches kontroll's parser, and holds it with `--sustain 0`.
- **`access` counts a rule for what it grants**, not for naming ZSA's vendor
  id: uaccess, a mode that opens the node to every account, or a group that
  exists and the person is in. Keymapp's AUR package brings zsa-udev, whose
  rules give the board to a plugdev group Arch does not create, and JayOS's
  doctor once called that reachable. udev's four directories are read in its
  order, a file masking one of the same basename later, as udev does.

## The rule

`70-zsa-uaccess.rules` tags the board's raw HID node, its USB device and the
Moonlander's STM32 bootloader with uaccess, which logind's
`73-seat-late.rules` acts on: an ACL for the person at the seat, taken back at
logout, with no group to create. It is installed into `/usr/lib/udev/rules.d`
by the package, where until 2026-11-12 JayOS's root helper wrote it into
`/etc/udev/rules.d`. A machine that ran that helper keeps the `/etc` copy,
which masks this one by basename and says the same three lines.

The install file reloads udev's rules and sends a change event to the USB and
hidraw devices, so a board plugged in before the install is opened now rather
than at its next reconnect. It runs in post_install, before the
transaction's hooks, so it reloads itself rather than leaving the reload to
systemd's hook. A failure is said and never fails the install.

## The programs

`jayos-keymapp-layers` and `jayos-moonlander-osd` are as they were in JayOS,
and each file's header is the account: the database read read-only, since
Keymapp may hold it; the revision chosen by the board's USB serial, else the
newest; the daemon's inotify on `/dev/bus/usb`, its own HTTP/2 client for one
unary gRPC call four times a second while a board is plugged in, the half
second a layer must stay before it is said, and a board USBGuard holds back
not counted as plugged in. Two things changed:

- **Where the daemon finds jayos-keymapp-layers.** It looked in JayOS's
  helper directory, `$QS_BIN_DIR`, `$QS_BIN` or `~/.local/bin`, where
  install.sh built both. It looks in `$QS_PKG_BIN_DIR`, `$QS_PKG_BIN` or
  `/usr/bin` now, JayOS's order for a package's commands, since the package
  installs the two side by side.
- **What they share is a copy.** `src/support/` is six files of JayOS's
  jayos-core, copied whole with their tests, for the reason its `mod.rs`
  gives: a git dependency on JayOS's workspace would tie this build to the
  repository it moved out of.

The daemon's files under `$XDG_RUNTIME_DIR/quickshell/` and the shell's
program it runs are `contract/runtime.txt`; JayOS's lock screen, panel,
overlay and doctor read them, and its `moonlander.sh` writes the note the
daemon reads.

## What has run, and what has only been read

RUN, on 2026-11-12, in a container that is not Arch, with no Moonlander, no
Keymapp, no kontroll and no udev:

- `cargo test`: 178 tests, the six support modules' and the two programs'
  unit tests, as they ran in JayOS's workspace, all passing.
- `tests/shell/test-cli.sh`, 58 assertions: every verb against a stand-in
  kontroll that records what it is asked and refuses what 1.0.x refuses, and
  `access` over rules written there in the shapes zsa-udev's take.
- `tests/shell/test-layers.sh`, 13: the built `jayos-keymapp-layers` over a
  database python3's sqlite3 writes in the schema third-party readers of
  Keymapp's show, and `jayos-moonlander status` through it.
- `tests/shell/test-osd.sh`, 39: the built daemon against a fake USB tree, a
  fake Keymapp speaking HTTP/2 and gRPC on a Unix socket, and an `osd.sh`
  that writes down what it is asked.
- `tests/shell/test-packaging.sh` and `tests/shell/test-release.sh`
  (`docs/RELEASES.md` has those).
- Sixteen mutations, each restored byte for byte afterwards, and each failed
  its suite: in the command, `color`'s shape check loosened, a failed colour
  let through, `access` blind to uaccess, `layer` called unbounded, and Oryx's
  "Layer" taken for a name; in `packaging/steps.sh`, the rule installed under
  `/etc`, the NEEDED gate let through, and `test-osd.sh` left out of `check`;
  in the install file, the trigger before the reload; in the recipe, systemd
  dropped from `depends` and the install file not named; a path added to
  `contract/files.txt` that nothing installs; the command's version drifting
  from `VERSION`; a verb renamed with the contract left stale, which
  `tools/contract.sh` failed; the rule's hidraw line dropped; and, rebuilt,
  the daemon looking for its helper in JayOS's directory again.

READ, not run: a real Keymapp database, Keymapp's API, kontroll itself (its
interface was run once against a stand-in Keymapp in JayOS's history), a ZSA
keyboard on a real bus, udev and logind acting on the rule, pacman running
the install file, and makepkg.
