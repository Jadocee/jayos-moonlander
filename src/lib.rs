//! ZSA Moonlander support for the JayOS shell.
//!
//! Two programs, in `src/bin/`: `jayos-keymapp-layers`, which reads the
//! layout Keymapp flashed out of Keymapp's own database, and
//! `jayos-moonlander-osd`, the daemon that says when a ZSA keyboard is plugged
//! in, unplugged or changes layer. Each carries its own unit tests; this
//! library is only what they share, which is `support`.

pub mod support;
