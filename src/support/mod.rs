//! Paths, output, processes and sysfs, for the two binaries.
//!
//! WHY A COPY AND NOT A DEPENDENCY
//!
//! These six files are jayos's `rust/crates/jayos-core/src/{cli,emit,paths,
//! proc,store,sysfs}.rs` at jayos commit fdde9c4, copied whole with their
//! tests, when the Moonlander's code moved out of that tree on 2026-11-12 by
//! the owner's call, "Move all zsa moonlander keyboard related stuff out of
//! JayOS and into jayos-moonlander". A git dependency on jayos's workspace
//! would make this repository's build depend on the one it moved out of --
//! the coupling the move exists to remove -- and jayos-bluetooth,
//! jayos-sony-xm5 and jayos-audio made the same choice for the same reason.
//!
//! `paths` matters most: the daemon's `status`, `layer` and `pid` files, the
//! `announced` note the shell's moonlander.sh writes, and the shell's
//! `osd.sh`, which the daemon runs to show a popup, all have to be where
//! jayos's Paths.qml and paths.sh put them, so the copy keeps jayos's
//! resolution order exactly rather than a simplified one that would agree on
//! most machines. The edits are two module paths -- `crate::emit` in `cli`,
//! and a doc link in `emit` -- since the modules sit under `support` here.
//!
//! A fix to one copy is a fix to one copy: a change to any of these files in
//! jayos is carried here by hand, diffed from the commit above.

pub mod cli;
pub mod emit;
pub mod paths;
pub mod proc;
pub mod store;
pub mod sysfs;

/// Seconds since the Unix epoch, as jayos-core's `now_unix` (its lib.rs).
///
/// Before 1970 means the clock is badly wrong; zero then makes every "how
/// long ago" read as a very long time, which is the safe direction.
pub fn now_unix() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}
