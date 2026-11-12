# The contract with JayOS

JayOS's shell runs this package's commands from `/usr/bin`: its
`moonlander.sh` hands the keyboard's verbs to `jayos-moonlander`, its
`moonlander-osd.sh` starts and stops `jayos-moonlander-osd`, and its lock
screen, held-Super panel and overlay run `jayos-keymapp-layers` and read the
daemon's files. Its Moonlander setup builds this package with a recipe of its
own, from another repository. These files are what the two sides agree on, in
a form each side can test against without the other's code. JayOS vendors a
copy of this directory and of the files beside it that it compares against
(the udev rule and the install file), records the release it came from and
each file's sha256, and holds its side to the copy -- a release's copy, since
JayOS builds the release its recipe pins and vendors the same one
(`docs/RELEASES.md`).

| File | What it is | Made by |
|---|---|---|
| `VERSION` | The schema of the files below: 1. A reader refuses a newer one. | hand |
| `verbs.txt` | The verbs of `jayos-moonlander`, in its dispatch's order. JayOS may run these and no others. | generated |
| `layers-verbs.txt` | The verbs of `jayos-keymapp-layers`. JayOS may run these and no others; with none, it prints what `names` does. | generated |
| `osd-icons.txt` | The icon names `jayos-moonlander-osd` hands the desktop's `osd.sh show`. JayOS's OSD draws a glyph and a colour for each, and its suite holds that to this list. | generated |
| `steps.txt` | The verbs of `packaging/steps.sh` and their arguments: everything JayOS's recipe calls. | generated |
| `files.txt` | Every path the package installs, relative to `/`. JayOS reads it for where the commands and the udev rule are; `tests/shell/test-packaging.sh` holds `steps.sh package` to it exactly, both ways. | hand |
| `runtime.txt` | The files `jayos-moonlander-osd` writes and reads under the runtime directory, and the one desktop program it runs. `tests/shell/test-osd.sh` holds the daemon to it. | hand |

The generated files are made by `tools/contract.sh`, which reads the code
rather than describing it, and which fails when any of them differs from
what the code now says (`tools/check.sh` runs it). After an intended change:

    tools/contract.sh --write

then commit the result, make a release, and pin it in JayOS
(`tools/package-release.sh jayos-moonlander <version>` there).

The `.txt` files carry a header of `#` lines; a reader skips them.
