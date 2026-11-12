//! The ZSA keyboard's layer names, out of Keymapp's own database.
//!
//!     jayos-keymapp-layers                 read the database, print the envelope
//!     jayos-keymapp-layers names           the same thing, named
//!     jayos-keymapp-layers parse           read `sqlite3 -json` rows on stdin,
//!                                       run nothing
//!     jayos-keymapp-layers name <n>        one layer's name, from the database
//!     jayos-keymapp-layers name <n> --stdin
//!                                       the same lookup against an envelope
//!                                       read on stdin
//!     jayos-keymapp-layers layout          every layer's keys, for the Super panel
//!
//! Anything else is `emit::unknown_command`.
//!
//! Its callers are JayOS's: the shell's moonlander.sh, the lock screen, the
//! held-Super panel and the overlay, and this package's jayos-moonlander-osd.
//! It moved here from JayOS's `rust/crates/jayos-hw` on 2026-11-12
//! (docs/MOONLANDER.md).
//!
//! WHY THIS EXISTS
//!
//! `kontroll status` reports a layer NUMBER, so everything downstream says
//! "layer 3" where the keyboard's own software says "Symbols". Keymapp keeps
//! the layout it flashed in a local SQLite file, and reading that is the only
//! way to get the names without asking Oryx over the network -- which would
//! mean an account and a round trip to answer a question about hardware on this
//! desk. The idea came from omarchy's keymapp plugin; this is the piece of it
//! worth taking.
//!
//! WHAT IT PRINTS
//!
//! ```text
//! {"names":["Base","Symbols","Numbers"],"error":""}
//! ```
//!
//! Always both keys, always that shape. A failure fills in `error` and leaves
//! `names` empty rather than printing something else: 50-hardware.sh greps
//! `"error":"..."` out of this to say *why* names are unavailable, and
//! moonlander.sh indexes `names` without looking at anything else, so an
//! envelope that changes shape when it fails is an envelope neither can read.
//!
//! `name` is the other documented format -- the raw name on one line, exactly
//! what `jq -r` wrote, so `$(...)` in the shell yields the name and nothing
//! else. Its failure shape is the same shape: an empty line, with the reason on
//! stderr. The name is NOT replaced by the number when it is unknown; the
//! caller already has the number, and a name that is really a number pretending
//! to be one is worse than an absent name.
//!
//! WHICH REVISION
//!
//! Keymapp keeps more than one revision, one row each in `revision`, keyed by
//! the revision's Oryx hash. The names that are true are the ones in the
//! revision on the keyboard, and an Oryx build says which that is: its USB
//! serial is `<layout hash>/<revision hash>` (`SERIAL_NUMBER` in the
//! `config.h` Oryx generates), which sysfs shows as `serial` beside the
//! `idVendor` that says ZSA. So the row is, in order:
//!
//! 1. the revision a plugged-in ZSA keyboard's serial names, by `revisionId`;
//! 2. with no such keyboard, or one Keymapp holds no row for, the newest
//!    `layout.revision.createdAt`, which Oryx writes as
//!    `2025-12-09 14:22:24 UTC` -- fixed width and all UTC, so it sorts as
//!    text;
//! 3. among rows with no `createdAt`, the one written last, by `rowid`.
//!
//! This used to be `ORDER BY revisionId DESC`, which is the newest only while
//! the id counts up. It is a hash, so that order was alphabetical, and on a
//! third-party fixture of three revisions it chose the oldest. A keyboard
//! re-flashed to an older revision is the case only the serial gets right.
//!
//! WHERE THE NAMES ARE
//!
//! `layout.revision.layers[]`, each `{title, position, color, keys}`, put in
//! `position` order, since `position` is the layer's number in the firmware and
//! the envelope is indexed by that number. This used to look only at the top of
//! the document, for `layers`, `layerNames` or `layer_names`, which no Keymapp
//! anyone has looked at writes, so on a real database every name was absent and
//! every popup said "Layer 3". The top is still read where the nested array is
//! missing or empty, which costs nothing and keeps every fixture below that
//! uses it.
//!
//! READ, NOT RUN
//!
//! Nothing here has opened a real Keymapp database or read a real keyboard's
//! serial. The schema, `revision(revisionId TEXT NOT NULL UNIQUE, data BLOB)`,
//! is the one AlexKrupa/qmk-keymap@0edf1c0 builds its fixture with, and that
//! tool reads `createdAt` and the layers' titles from `layout.revision`;
//! ajmasia/gnome-shell-extension-zsa-helper@dad3777 and
//! vibewrks/zsa-overlay@2892e2d choose the row by `WHERE revisionId = ?`, and
//! the second orders the layers by `position`. The serial and the format of
//! `createdAt` are Oryx's own: the source it generates for ZSA's default
//! Moonlander layout, and its public API's answer for that layout, asked on
//! 2026-10-19. The tests build databases of that shape; point 6 of
//! docs/plans/30-moonlander-layers-on-super.md is the read at the machine that
//! would settle it.
//!
//! QS_USB_ROOT moves `/` for /sys, as it does for jayos-usb and jayos-moonlander-osd.
//!
//! THE LAYOUT, FOR THE HELD SUPER
//!
//! `layout` prints the same revision's layers with their keys, for the panel a
//! held Super shows (docs/plans/30-moonlander-layers-on-super.md):
//!
//! ```text
//! {"plugged":true,"geometry":"moonlander","layers":[{"title":"Base",
//!  "color":"#b30000","keys":[{"tap":{"code":"KC_GRAVE"},
//!  "hold":{"code":"MO","layer":1}}, ...]}],"error":""}
//! ```
//!
//! Each key's four steps keep Oryx's names, `tap`, `hold`, `doubleTap` and
//! `tapHold`, each reduced to its keycode and the layer a layer key goes to; a
//! step the key does not have is left out, so `{}` is a key with nothing on it
//! and `{"tap":{"code":"KC_TRANSPARENT"}}` one that falls through to the layer
//! below -- the two Oryx's API kept apart in ZSA's default layout, asked on
//! 2026-10-21, where layer 1's left inner key is `null` and its right one
//! transparent. The titles are cut as the names are, so the panel and the
//! popups say one thing; a colour is `#rrggbb` or null. `plugged` is whether a
//! ZSA keyboard is on the bus, read fresh, so the panel can refuse to show for
//! a board that is not there whatever the popup daemon last wrote -- and a
//! board USBGuard has not authorised is not there. Read evidence, as the rest
//! of this is: no real Keymapp database has been read.
//!
//! WHY `parse` EXISTS
//!
//! Not a convenience. The machine this is built and tested on has no Keymapp,
//! no keyboard and no database, so `parse` is how the parser is exercised at
//! all, and it is how the differential harness feeds identical bytes to this
//! and to the jq it replaces. It takes what `sqlite3 -json` prints on stdin:
//!
//! ```text
//! [{"data":"{\"layout\":{\"revision\":{\"layers\":[{\"title\":\"Base\",\"position\":0}]}}}"}]
//! ```
//!
//! THE JSON IS NESTED TWO LEVELS
//!
//! sqlite3 gives rows, and the row's `data` column is itself a JSON string that
//! has to be parsed again. `names` skips the outer level -- rusqlite hands back
//! the column, not a rendering of it -- but `parse` still reads sqlite3's own
//! output, so the two-level parse is kept for it and is what the fixtures below
//! drive.
//!
//! READ-ONLY, ALWAYS
//!
//! Keymapp may be running. Opening its database read-write while it holds a
//! lock is how you corrupt someone's layout, and nothing here needs to write,
//! so this opens `SQLITE_OPEN_READ_ONLY` as the `sqlite3 -readonly` it replaces
//! did. The cost is that a database with a hot journal cannot be recovered on
//! open and reports as unreadable instead. That is the right trade here and the
//! opposite of the one jayos-focus-stats makes, because that database is ours and
//! this one is somebody else's.
//!
//! WHAT THE PORT DELIBERATELY CHANGES
//!
//! sqlite3 is no longer a dependency, so the branch's "sqlite3 is not
//! installed" answer cannot arise any more -- SQLite is linked in.
//!
//! Where the shell reported every unsuccessful `sqlite3` run as "no layout in
//! the database" -- no rows, no such table, a locked file, all of them -- this
//! separates them: an empty table still says "no layout in the database", and a
//! database that could not be opened or queried says so and carries SQLite's
//! own reason, because 50-hardware.sh prints that string to a person trying to
//! work out what is wrong.
//!
//! The jq indexed `.data` on the first row without checking it was an object,
//! so rows of `[1,2]` made it exit 5 having printed nothing -- and the shell
//! then handed its caller an empty string where an object was expected. That is
//! the envelope like every other failure now.
//!
//! `parse` given nothing at all answers "no layout in the database" rather
//! than "unreadable layout data". That is the shell BRANCH's answer rather
//! than the jq's: sqlite3 prints nothing, not `[]`, for a query that matched
//! no rows, and the branch tested for exactly that before it reached jq. It
//! disagrees with the bare jq program in order to agree with the script.
//!
//! Input that is only WHITESPACE agrees with neither, and that is the one
//! place this overshoots. `[[ -n "   " ]]` is true, so the branch handed a run
//! of spaces to jq and jq called it "unreadable layout data"; the blank test
//! here is a trim, so it says "no layout" instead. Left as it is because
//! sqlite3 cannot produce that input -- a query matching no rows prints
//! nothing at all, and `$(...)` had already stripped the trailing newline of
//! one that matched -- but recorded rather than claimed as fidelity.

use jayos_moonlander::support::{cli::Args, emit, paths, sysfs};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::Serialize;
use serde_json::{Map, Value};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

/// The revision a keyboard's serial names.
///
/// An index lookup rather than a scan, since `revisionId` is UNIQUE in the
/// schema the module comment cites, and it is the one query that is right
/// after the keyboard is re-flashed to an older revision.
const FLASHED_QUERY: &str = "SELECT data FROM revision WHERE revisionId = ?1 LIMIT 1";

/// The newest revision, for when no keyboard says which.
///
/// `createdAt` is when Oryx made the revision. `rowid` only says when Keymapp
/// wrote the row, a weaker guess at the same thing, and the one left for a
/// document with no `createdAt`, which the GNOME extension's fixture is.
///
/// Compared as text, not through `julianday()`: SQLite's date functions do not
/// read the ` UTC` Oryx writes, and answer NULL for every row.
///
/// The `json_valid` guard is there because `json_extract` over a malformed
/// document is an error for the whole statement, so one damaged row would hide
/// every good one; guarded, it sorts as a row with no date.
///
/// No CAST, though `data` is declared BLOB. SQLite's JSON functions read a
/// BLOB as text, a rule 3.45.1 restored for compatibility, except one whose
/// length is what its first byte would declare as a header of SQLite's own
/// binary JSONB: for a document opening with `{`, eight bytes, too few to hold
/// a date. Both lengths were tried under SQLite 3.45.1, and a CAST was written
/// and taken out again when removing it failed no test. The fixture behind
/// `the_newest_revision_is_not_the_hash_that_sorts_last` stores BLOBs, so a
/// SQLite that stopped reading them as text fails it.
const NEWEST_QUERY: &str = "SELECT data FROM revision \
    ORDER BY CASE WHEN json_valid(data) \
    THEN json_extract(data, '$.layout.revision.createdAt') END DESC, \
    rowid DESC LIMIT 1";

/// ZSA Technology Labs' USB vendor id, as sysfs writes it.
const ZSA_VENDOR: &str = "3297";

/// Where the names live in the layout document, after `layout.revision.layers`,
/// in the order they are tried.
///
/// This port's first guess at the shape, kept as a fallback: no Keymapp anyone
/// has looked at writes these, but a document that does still reads as it did.
/// The first key holding a non-empty array wins, which is why an empty `layers`
/// falls through to `layerNames` instead of answering with nothing.
const NAME_KEYS: [&str; 3] = ["layers", "layerNames", "layer_names"];

/// How much of a name is kept.
///
/// Carried over from the jq's `.[0:24]` unchanged. Nothing records why 24 in
/// particular, and it is not this port's number to revise: the panel already
/// shows names cut at 24, so widening it here would change what is on screen
/// for anyone whose layout has a longer name, and narrowing it would cut names
/// that fit today.
const MAX_NAME_CHARS: usize = 24;

/// How long to wait for a writer to let go.
///
/// Short, because this is read on the status poll: a second and a half of a
/// stalled panel is tolerable, and the alternative -- rusqlite's default of no
/// timeout at all -- is a widget that renders nothing every time Keymapp
/// happens to be mid-flash.
const BUSY_TIMEOUT: Duration = Duration::from_millis(1500);

/// Malformed data. The one answer that means "there is something here and it is
/// not a layout".
const UNREADABLE: &str = "unreadable layout data";

/// An empty `revision` table, or a query that returned nothing.
const NO_LAYOUT: &str = "no layout in the database";

/// No database at all, which is the ordinary state of a machine where Keymapp
/// has been installed but nothing flashed yet.
const NO_DATABASE: &str = "no Keymapp database; flash a layout from Keymapp first";

/// A layout with no `layout.revision.layers`: names may still be read from the
/// top of it, but there are no keys to draw.
const NO_KEYS: &str = "the layout keeps no keys under layout.revision.layers";

/// How much of a layout `layout` carries. Oryx's largest board has 72 keys and
/// a layout has tens of layers at most; the caps keep a damaged document from
/// becoming a megabyte the panel parses on every held Super.
const MAX_LAYERS: usize = 32;
const MAX_KEYS: usize = 128;

/// How much of a keycode is kept. Oryx's are `KC_A`, `MO`, `LCTL(KC_A)`.
const MAX_CODE_CHARS: usize = 48;

/// The envelope every JSON verb prints.
///
/// `names` first and `error` second, matching the object the jq built, because
/// 50-hardware.sh reads this with `grep -oP '"names":\s*\[\K[^]]*'` and a
/// reordering would silently stop it counting.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
struct Layers {
    names: Vec<String>,
    error: String,
}

impl Layers {
    fn failed(reason: impl Into<String>) -> Self {
        Layers { names: Vec::new(), error: reason.into() }
    }
}

// ── parsing the layout ───────────────────────────────────────────────────────

/// One layer entry's name.
///
/// An entry may be a bare string OR an object, and the object may carry the name
/// under `name` or under `title`. All three are handled because the jq handled
/// all three, and a jq that only handled one would not have grown the other two.
///
/// `.name // .title // ""` in jq, and `//` there treats `false` as absent just
/// as it treats `null`: an entry of `{"name": false}` falls through to `title`.
/// Reproduced deliberately. A `0` does NOT fall through -- zero is truthy in jq
/// -- so a layer literally named `0` keeps that name.
fn layer_name(entry: &Value) -> String {
    match entry {
        Value::String(name) => name.clone(),
        Value::Object(fields) => {
            for key in ["name", "title"] {
                match fields.get(key) {
                    None | Some(Value::Null) | Some(Value::Bool(false)) => continue,
                    Some(found) => return jq_tostring(found),
                }
            }
            String::new()
        }
        // A number, a boolean, a null or a nested array where a name was
        // expected. Blank rather than a rendering of it, and blank rather than
        // dropped -- see `names_in` for why the entry has to stay.
        _ => String::new(),
    }
}

/// jq's `tostring`: a string is itself, everything else is its JSON text.
///
/// Only reachable through the `name`/`title` fields, where a layout with a
/// structured name would otherwise be rendered as Rust's `Debug`.
///
/// NOT byte-identical to jq for every number, and deliberately left that way.
/// The differential harness found three: jq writes `1e30` as `1E+30` and `1e-7`
/// as `1E-7` where serde_json writes `1e+30` and `1e-7`, and jq writes `-0`
/// where serde_json writes `-0.0`. Matching those would mean reimplementing
/// jq's own number printer to make a layer whose name is a raw JSON number in
/// exponent form -- which no Keymapp version writes -- render identically.
/// Integers, ordinary decimals and `3.0` do already agree.
///
/// An object name is rendered with its keys in sorted order rather than in the
/// order the document had them, because serde_json's `Map` is a `BTreeMap`
/// unless the whole workspace turns on `preserve_order`. Same trade: the input
/// is not a shape Keymapp produces, and the alternative changes a dependency
/// every other binary in the tree shares.
fn jq_tostring(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// One leading byte-order mark, skipped.
///
/// jq's `fromjson` skips one and serde_json does not, so a layout document
/// stamped with a BOM parsed under the shell and lands on "unreadable layout
/// data" here -- and that failure is total: every name disappears and the panel
/// is back to saying "layer 3". Found by feeding both sides the same bytes.
///
/// Exactly one, and only at the very first character, because that is what jq
/// 1.7 accepts: `\u{feff}\u{feff}{...}` and `  \u{feff}{...}` are errors there
/// too, and being more permissive than the program this replaces would let a
/// genuinely corrupt document through as a layout.
fn without_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

/// Trim, then cut to [`MAX_NAME_CHARS`].
///
/// That order is the jq's, and it matters: a name whose 25th character onwards
/// is whitespace keeps the trailing spaces the cut leaves behind, because only
/// the whole string was trimmed.
///
/// `trim` and not an ASCII-only trim: jq 1.7's `\s` is Unicode-aware under
/// oniguruma -- checked against the real jq, which strips U+00A0 and U+3000 and
/// leaves U+200B alone, which is exactly `char::is_whitespace`.
///
/// CHARACTERS, not bytes. jq slices strings by codepoint, so `.[0:24]` on a
/// layout named in emoji keeps 24 of them; `&name[..24]` would keep six and
/// panic on the seventh for landing inside one.
fn tidy(name: &str) -> String {
    name.trim().chars().take(MAX_NAME_CHARS).collect()
}

/// The layers in the order their `position` numbers them.
///
/// `position` is the layer's number in the firmware, which is what `kontroll
/// status` reports and what the envelope is indexed by. It is trusted only
/// where every entry has one and together they number 0 to n-1 once each;
/// anything else -- one missing, a repeat, a gap -- keeps the array's own
/// order, which is the order Oryx's API listed ZSA's default layout in, and
/// the only other order there is.
fn by_position(entries: &[Value]) -> Vec<&Value> {
    let mut slots: Vec<Option<&Value>> = vec![None; entries.len()];
    for entry in entries {
        let slot = entry
            .get("position")
            .and_then(Value::as_u64)
            .and_then(|p| usize::try_from(p).ok())
            .and_then(|p| slots.get_mut(p));
        match slot {
            Some(slot) if slot.is_none() => *slot = Some(entry),
            _ => return entries.iter().collect(),
        }
    }
    slots.into_iter().flatten().collect()
}

/// The names out of a layout document.
///
/// Blank entries are kept in place rather than dropped: the index IS the layer
/// number, and compacting would rename every layer above a blank one.
fn names_in(layout: &Map<String, Value>) -> Vec<String> {
    // Where Keymapp keeps them, by every reader of its database this tree has
    // found. An empty array falls through to the top, for the same reason an
    // empty `layers` falls through to `layerNames` below.
    let nested = layout.get("layout").and_then(|l| l.get("revision")).and_then(|r| r.get("layers"));
    if let Some(Value::Array(entries)) = nested {
        if !entries.is_empty() {
            return by_position(entries)
                .into_iter()
                .map(|entry| tidy(&layer_name(entry)))
                .collect();
        }
    }

    for key in NAME_KEYS {
        // `select(type == "array" and length > 0)` in the jq, and both halves
        // of that test earn their keep: a key that is present but empty, or
        // present and not an array, is treated as absent and the next spelling
        // is tried. Stopping at the first key that merely EXISTS answers with
        // no names at all on a layout that keeps them under the next one.
        if let Some(Value::Array(entries)) = layout.get(key) {
            if !entries.is_empty() {
                return entries.iter().map(|entry| tidy(&layer_name(entry))).collect();
            }
        }
    }
    // A layout with no names at all is a real answer, not a failure.
    Vec::new()
}

/// The inner level: the text of a `data` column.
fn layers_from_layout(data: &str) -> Layers {
    match serde_json::from_str::<Value>(without_bom(data)) {
        Ok(Value::Object(layout)) => Layers { names: names_in(&layout), error: String::new() },
        // Anything that is not an object -- a bare array, a number, a truncated
        // document -- is data we cannot read, which is a failure and says so.
        _ => Layers::failed(UNREADABLE),
    }
}

/// The outer level: what `sqlite3 -json` prints, rows and all.
///
/// Only `parse` needs this. The real path gets the column from rusqlite and
/// starts at [`layers_from_layout`].
fn layers_from_rows(text: &str) -> Layers {
    // The shell tested `[[ -n "$KM_JSON" ]]` before it ever reached jq, because
    // sqlite3 prints nothing at all -- not `[]` -- for a query that matched no
    // rows. Blank rather than strictly empty, so that the trailing newline
    // sqlite3 writes (which `$(...)` used to strip before jq saw it) still lands
    // on the branch's own message instead of on "unreadable".
    //
    // A trim overshoots the branch for input that is only spaces -- see the
    // module comment and `whitespace_alone_overshoots_the_branch`. sqlite3
    // cannot write that, so the newline is worth more than the fidelity.
    if text.trim().is_empty() {
        return Layers::failed(NO_LAYOUT);
    }

    // The BOM is skipped AFTER the blank test, not before it: the shell's
    // `[[ -n "$KM_JSON" ]]` saw the raw bytes, so a document that is nothing but
    // a BOM reached jq and came back "unreadable". It still does.
    let Ok(Value::Array(rows)) = serde_json::from_str::<Value>(without_bom(text)) else {
        return Layers::failed(UNREADABLE);
    };

    // No rows is not a failure: it is a keyboard with no stored layout, and the
    // Python read that as an empty object rather than as an error. Only a
    // sqlite3 that prints `[]` for an empty result reaches this.
    let Some(first) = rows.first() else {
        return Layers::default();
    };

    match first.get("data") {
        Some(Value::String(data)) => layers_from_layout(data),
        // A NULL `data` column, a row that is not an object, a row with no
        // `data` at all. jq errored out on the second of those and wrote
        // nothing, which left the shell substituting an empty string where the
        // caller expected an object; every one of them is the same envelope
        // here.
        _ => Layers::failed(UNREADABLE),
    }
}

// ── looking one name up ──────────────────────────────────────────────────────

/// Which layer `raw` asks for.
///
/// A value that is not a run of digits reads layer 0 rather than nothing. That
/// is the jq's `if test("^[0-9]+$") then tonumber else 0 end` and it is kept:
/// the state file is written by `watch` and read with `cat`, so an empty or
/// half-written file asks for the base layer, which is the layer the keyboard is
/// on when nothing has switched it.
///
/// Note that `-1` is therefore layer 0 and not the last layer -- jq's negative
/// indexing counts from the end, and the digit test is what stops a truncated
/// state file naming the topmost layer.
fn layer_index(raw: &str) -> usize {
    // jq's `$` matches before a final newline, so `"3\n"` counted as numeric
    // there. `$(...)` strips it before the shell can pass it, but the harness
    // calls this binary directly.
    let digits = raw.strip_suffix('\n').unwrap_or(raw);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return 0;
    }
    // Saturating rather than wrapping: a number too large for a machine integer
    // is past the end of any list, which is the answer jq gave for `.[1e30]`.
    digits.parse().unwrap_or(usize::MAX)
}

/// The name of one layer, or "" when there is not one.
fn name_at(names: &[String], index: &str) -> String {
    names.get(layer_index(index)).cloned().unwrap_or_default()
}

/// The same lookup against an envelope this program printed earlier.
///
/// The shell piped `layer-names` into a second jq to do this. Kept as a verb
/// because it is the only way to drive the lookup from a fixture, and because
/// the differential harness needs somewhere to send the identical bytes.
fn name_from_envelope(text: &str, index: &str) -> String {
    let Ok(envelope) = serde_json::from_str::<Value>(text) else {
        // `(try fromjson catch {})`. An envelope that did not parse has no
        // names, which is an empty line -- and an empty line is what the caller
        // already handles, because that is what a keyboard with no stored
        // layout gives it.
        return String::new();
    };

    // `.names` must be an array. The jq wrote `select(type == "array")`, which
    // produces nothing at all for anything else, and a jq that produces nothing
    // prints nothing -- so a `names` that is a string was already an empty line.
    let Some(Value::Array(names)) = envelope.get("names") else {
        return String::new();
    };

    match names.get(layer_index(index)) {
        None | Some(Value::Null) | Some(Value::Bool(false)) => String::new(),
        Some(found) => jq_tostring(found),
    }
}

// ── the database ─────────────────────────────────────────────────────────────

/// Which directory holds `keymapp.sqlite3`.
///
/// Two candidates, and the fallback is deliberately NOT XDG-aware: the shell
/// wrote `"$HOME/.config/keymapp"` literally, so a machine with XDG_CONFIG_HOME
/// pointed somewhere else looks in the dotted directory under it and then in
/// `~/.config` regardless. Reproduced rather than tidied, because tidying it
/// would move the file this reads on exactly the machines that set that
/// variable, and the names would quietly stop appearing there.
///
/// `dotted_exists` is passed in rather than tested here so the resolution can be
/// checked without a filesystem.
fn db_dir(config_root: &Path, home: &Path, dotted_exists: bool) -> PathBuf {
    if dotted_exists {
        config_root.join(".keymapp")
    } else {
        home.join(".config/keymapp")
    }
}

/// The database this reads, with the usual environment override.
fn db_path() -> PathBuf {
    // Every path in this tree is overridable from the environment; this one is
    // also how the port was checked against a database that is not the running
    // user's.
    if let Some(explicit) = std::env::var_os("QS_KEYMAPP_DB").filter(|v| !v.is_empty()) {
        return PathBuf::from(explicit);
    }

    let config_root = paths::config_root();
    let dotted = config_root.join(".keymapp");
    db_dir(&config_root, &paths::home(), dotted.is_dir()).join("keymapp.sqlite3")
}

/// Where `/` is, for /sys. QS_USB_ROOT moves it, as it does for jayos-usb and
/// jayos-moonlander-osd.
fn usb_root() -> PathBuf {
    std::env::var_os("QS_USB_ROOT")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// The revision hash out of an Oryx build's serial, `<layout>/<revision>`.
///
/// Strict, because the serial is whatever the firmware says: a keyboard built
/// from QMK rather than Oryx has its own, or none, and a serial that is not
/// exactly two non-empty parts names no revision Keymapp could hold.
fn revision_in(serial: &str) -> Option<&str> {
    let (layout, revision) = serial.split_once('/')?;
    (!layout.is_empty() && !revision.is_empty() && !revision.contains('/')).then_some(revision)
}

/// The revision each plugged-in ZSA keyboard is running, in bus order.
///
/// A device, not an interface: an entry with a colon is one of a device's
/// interfaces and has no `serial` of its own. More than one ZSA keyboard is
/// read in bus order and the first whose revision Keymapp holds wins, which
/// is a guess -- Keymapp's own answer for which board it is connected to is
/// over its API, which this does not speak.
fn flashed_revisions(root: &Path) -> Vec<String> {
    let dir = root.join("sys/bus/usb/devices");
    let mut out = Vec::new();
    for name in sysfs::entry_names(&dir) {
        if name.contains(':') {
            continue;
        }
        let at = dir.join(&name);
        if sysfs::read(at.join("idVendor")).as_deref() != Some(ZSA_VENDOR) {
            continue;
        }
        if let Some(revision) = sysfs::read(at.join("serial")).as_deref().and_then(revision_in) {
            out.push(revision.to_string());
        }
    }
    out
}

/// The row whose layout this reads: the first of `revisions` Keymapp holds,
/// else the newest. The module comment has the order and why.
fn layout_row(conn: &Connection, revisions: &[String]) -> rusqlite::Result<Option<SqlValue>> {
    for revision in revisions {
        let found = conn
            .query_row(FLASHED_QUERY, [revision], |row| row.get::<_, SqlValue>(0))
            .optional()?;
        if found.is_some() {
            return Ok(found);
        }
    }
    conn.query_row(NEWEST_QUERY, [], |row| row.get::<_, SqlValue>(0)).optional()
}

/// Read the layout out of the database: the revision on the keyboard where
/// one of `revisions` is stored, else the newest.
fn layers_from_db(path: &Path, revisions: &[String]) -> Layers {
    match document_from_db(path, revisions) {
        Ok(data) => layers_from_layout(&data),
        Err(reason) => Layers::failed(reason),
    }
}

/// The document `names` and `layout` read: the `data` column of the row
/// [`layout_row`] chooses, as text, or why there is none.
fn document_from_db(path: &Path, revisions: &[String]) -> Result<String, String> {
    // Readable, not merely present: the shell's test was `[[ ! -r "$KM_DB" ]]`,
    // and the message it printed tells someone with no layout what to do about
    // it. Left to SQLite this would be "unable to open database file", which is
    // the same sentence for a file that is missing, a file owned by root and a
    // directory.
    if std::fs::File::open(path).is_err() {
        return Err(NO_DATABASE.to_string());
    }

    // READ-ONLY. Keymapp may be running, and opening its database read-write
    // while it holds a lock is how you corrupt someone's layout.
    //
    // NO_MUTEX because this connection never leaves the thread that made it.
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = match Connection::open_with_flags(path, flags) {
        Ok(conn) => conn,
        Err(e) => return Err(format!("could not open the Keymapp database: {e}")),
    };
    if let Err(e) = conn.busy_timeout(BUSY_TIMEOUT) {
        // Not fatal: the default is no wait at all, so the only consequence is
        // that a concurrent flash makes this fail sooner than it might have.
        eprintln!("jayos-keymapp-layers: could not set the busy timeout: {e}");
    }

    // The column as a `Value` rather than as a `String`. A NULL `data` is not a
    // read error, it is a row whose payload is not text -- which is the
    // `type != "string"` arm of the jq -- and asking rusqlite for a String there
    // reports it as a type error instead, with SQLite's wording rather than
    // ours.
    match layout_row(&conn, revisions) {
        Ok(None) => Err(NO_LAYOUT.to_string()),
        Ok(Some(SqlValue::Text(data))) => Ok(data),
        // A BLOB holding the document, which is how the schema the module
        // comment cites declares the column, and what SQLite keeps even in a
        // TEXT column when the writer bound a buffer rather than a string.
        // `sqlite3 -json` printed a blob column as a JSON *string*, so the jq
        // took its `type == "string"` arm and produced the names; treating it
        // as "not a layout" here loses every one of them on a machine the shell
        // handled. Found by running both sides over the same database.
        //
        // Lossy for the same reason `read_stdin` is: a layer name is whatever
        // its author typed, and one bad byte should not empty the envelope.
        Ok(Some(SqlValue::Blob(bytes))) => Ok(String::from_utf8_lossy(&bytes).into_owned()),
        // A NULL, an integer or a float column. sqlite3 rendered each of those
        // as a JSON non-string, which is the jq's `type != "string"` arm.
        Ok(Some(_)) => Err(UNREADABLE.to_string()),
        Err(e) => Err(format!("could not read the Keymapp database: {e}")),
    }
}

// ── the layout, for the held Super ───────────────────────────────────────────

/// One step of a key -- its tap, hold, double tap or tap-then-hold -- as the
/// panel reads it: the keycode, and the layer a layer key goes to.
#[derive(Debug, PartialEq, Eq, Serialize)]
struct Step {
    code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    layer: Option<u32>,
}

/// One key, by Oryx's names for its steps; a step it does not have is left out.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
struct Key {
    #[serde(skip_serializing_if = "Option::is_none")]
    tap: Option<Step>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hold: Option<Step>,
    #[serde(rename = "doubleTap", skip_serializing_if = "Option::is_none")]
    double_tap: Option<Step>,
    #[serde(rename = "tapHold", skip_serializing_if = "Option::is_none")]
    tap_hold: Option<Step>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct BoardLayer {
    title: String,
    color: Option<String>,
    keys: Vec<Key>,
}

/// What `layout` prints: always these four keys, in this order, as the names
/// envelope is always its two.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
struct Board {
    plugged: bool,
    geometry: String,
    layers: Vec<BoardLayer>,
    error: String,
}

impl Board {
    fn failed(reason: impl Into<String>) -> Self {
        Board { error: reason.into(), ..Board::default() }
    }
}

/// A step, or none for a null, a blank code or a code that is not a string.
/// A target that is not a whole number from 0 is dropped and the code kept.
fn step_in(value: Option<&Value>) -> Option<Step> {
    let fields = value?.as_object()?;
    let code = fields.get("code")?.as_str()?.trim();
    if code.is_empty() {
        return None;
    }
    let layer = fields.get("layer").and_then(Value::as_u64).and_then(|l| u32::try_from(l).ok());
    Some(Step { code: code.chars().take(MAX_CODE_CHARS).collect(), layer })
}

fn key_in(value: &Value) -> Key {
    Key {
        tap: step_in(value.get("tap")),
        hold: step_in(value.get("hold")),
        double_tap: step_in(value.get("doubleTap")),
        tap_hold: step_in(value.get("tapHold")),
    }
}

/// `#rrggbb`, lower-cased, or none: the colour is drawn, so anything that is
/// not one -- a name, three digits, a number -- is no colour at all.
fn colour_in(value: Option<&Value>) -> Option<String> {
    let hex = value?.as_str()?.trim().strip_prefix('#')?;
    (hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()))
        .then(|| format!("#{}", hex.to_ascii_lowercase()))
}

/// The layout out of one revision's document. `plugged` is left for the
/// caller, which knows the bus.
fn board_from_layout(data: &str) -> Board {
    let Ok(Value::Object(document)) = serde_json::from_str::<Value>(without_bom(data)) else {
        return Board::failed(UNREADABLE);
    };
    let layout = document.get("layout");
    let geometry: String = layout
        .and_then(|l| l.get("geometry"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .chars()
        .take(MAX_CODE_CHARS)
        .collect();
    let entries = layout
        .and_then(|l| l.get("revision"))
        .and_then(|r| r.get("layers"))
        .and_then(Value::as_array)
        .filter(|entries| !entries.is_empty());
    let Some(entries) = entries else {
        return Board { geometry, ..Board::failed(NO_KEYS) };
    };
    let layers = by_position(entries)
        .into_iter()
        .take(MAX_LAYERS)
        .map(|entry| BoardLayer {
            title: tidy(&layer_name(entry)),
            color: colour_in(entry.get("color")),
            keys: entry
                .get("keys")
                .and_then(Value::as_array)
                .map(|keys| keys.iter().take(MAX_KEYS).map(key_in).collect())
                .unwrap_or_default(),
        })
        .collect();
    Board { plugged: false, geometry, layers, error: String::new() }
}

/// Whether a ZSA keyboard is on the bus: a device, not an interface, of ZSA's
/// vendor, that USBGuard -- or anything else writing `authorized` -- has not
/// blocked. A blocked board sends nothing, so a panel shown for it would draw
/// layers no key can reach.
fn zsa_plugged(root: &Path) -> bool {
    let dir = root.join("sys/bus/usb/devices");
    sysfs::entry_names(&dir).iter().filter(|name| !name.contains(':')).any(|name| {
        let at = dir.join(name);
        sysfs::read(at.join("idVendor")).as_deref() == Some(ZSA_VENDOR)
            && sysfs::read(at.join("authorized")).as_deref() != Some("0")
    })
}

/// What `layout` reads: the database's revision for the keyboards on `root`'s
/// bus, and whether one is there.
fn board_from(path: &Path, root: &Path) -> Board {
    let mut board = match document_from_db(path, &flashed_revisions(root)) {
        Ok(data) => board_from_layout(&data),
        Err(reason) => Board::failed(reason),
    };
    board.plugged = zsa_plugged(root);
    board
}

// ── entry ────────────────────────────────────────────────────────────────────

/// Bytes then a lossy decode, not `read_to_string`.
///
/// A layer name is whatever its author typed into Oryx and is not required to be
/// UTF-8 once it has been through a `data` column; `read_to_string` fails the
/// whole read on one bad byte, which would turn one odd layout into an empty
/// envelope. `jq -R` substitutes U+FFFD for the same input, so this also keeps
/// the two comparable.
fn read_stdin() -> Option<String> {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf).ok()?;
    Some(String::from_utf8_lossy(&buf).into_owned())
}

/// Print one name on one line, which is what `jq -r` wrote.
fn print_line(name: &str) -> ExitCode {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    match writeln!(lock, "{name}").and_then(|()| lock.flush()) {
        Ok(()) => ExitCode::SUCCESS,
        // The caller stopped reading, which `head -1` and a closed popup both
        // do. Not an error worth reporting, and emit::json takes the same view.
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("jayos-keymapp-layers: could not write output: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Say why on stderr, for the person running this in a terminal.
///
/// Both the envelope and this line, deliberately: the widget and the doctor
/// need the structured answer, and someone who ran the helper by hand needs a
/// sentence. `name` prints nothing a person can read at all, so without this it
/// would answer an empty line and never say what went wrong.
fn warn(layers: &Layers) {
    if !layers.error.is_empty() {
        eprintln!("jayos-keymapp-layers: {}", layers.error);
    }
}

/// Print the envelope.
///
/// Exit ZERO even when `error` is filled in, which is not this tree's usual
/// rule. The branch this replaces ended every one of its failure paths with
/// `exit 0`, and its shell callers rely on it: 50-hardware.sh reads the error
/// string out of a `$(...)` and moonlander.sh pipes the envelope onwards.
/// jayos-moonlander-osd reads the names out of whatever it prints. A non-zero
/// status here would mean "the helper is missing", which is a different thing
/// and the one thing a caller should react to.
fn emit_layers(layers: &Layers) -> ExitCode {
    warn(layers);
    emit::json(layers)
}

fn cmd_name(args: &Args) -> ExitCode {
    // No index is layer 0, exactly as an unreadable state file was: `watch`
    // writes that file and `status` reads it with `cat`, so "absent" and "not a
    // number" have always meant the base layer.
    let index = args.positional(0).unwrap_or_default();

    if args.has("stdin") {
        return match read_stdin() {
            Some(text) => print_line(&name_from_envelope(&text, index)),
            None => {
                // An empty line on stdout so the caller still gets the shape it
                // expects, but a NON-ZERO exit: this verb is the harness and
                // test route, and a harness that cannot read its own input
                // otherwise looks exactly like a keyboard with no names.
                eprintln!("jayos-keymapp-layers: could not read the envelope on stdin");
                let _ = print_line("");
                ExitCode::FAILURE
            }
        };
    }

    let layers = layers_here();
    warn(&layers);
    print_line(&name_at(&layers.names, index))
}

/// What `names` and `name` read: the database, for the keyboards on the bus.
fn layers_here() -> Layers {
    layers_from_db(&db_path(), &flashed_revisions(&usb_root()))
}

fn main() -> ExitCode {
    let args = Args::from_env();

    // Only `name` reads a flag, so only `name` accepts one. Declared per verb
    // rather than as one list: `names --stdin` reads nothing from stdin either
    // way, and accepting the flag there would let someone believe it did.
    //
    // Refusing is safe -- no caller in the tree passes a flag: moonlander.sh
    // runs it bare and as `name <n>`, jayos-moonlander-osd bare, and the
    // held-Super panel as `layout`. `--stdin` is the harness and terminal
    // route, and ignoring it opens Keymapp's database instead of reading the
    // envelope it was handed, which answers about the wrong keyboard layout
    // entirely.
    let known: &[&str] = if args.command_or("names") == "name" { &["stdin"] } else { &[] };
    if let Some(code) = args.reject_unknown(known) {
        return code;
    }

    match args.command_or("names") {
        "names" => emit_layers(&layers_here()),

        "parse" => match read_stdin() {
            Some(text) => emit_layers(&layers_from_rows(&text)),
            None => {
                eprintln!("jayos-keymapp-layers: could not read the sqlite3 rows on stdin");
                let _ = emit::json(&Layers::failed("could not read stdin"));
                ExitCode::FAILURE
            }
        },

        "name" => cmd_name(&args),

        // Exit zero with `error` filled in, as `names` does and for the same
        // reason: the panel reads the document, and a status would only say
        // the helper ran.
        "layout" => {
            let board = board_from(&db_path(), &usb_root());
            if !board.error.is_empty() {
                eprintln!("jayos-keymapp-layers: {}", board.error);
            }
            emit::json(&board)
        }

        other => emit::unknown_command(&args.program, other, &["names", "parse", "name", "layout"]),
    }
}

// ── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // ── fixtures ─────────────────────────────────────────────────────────────

    /// What `sqlite3 -json "$KM_DB" "SELECT data ..."` actually prints: an array
    /// of one row, whose `data` column is a JSON document escaped INSIDE a JSON
    /// string. Two levels, and the inner one is text.
    const ROWS: &str = r#"[{"data":"{\"layers\":[{\"name\":\"Base\"},{\"name\":\"Symbols\"},{\"name\":\"Numbers\"}],\"title\":\"Moonlander\"}"}]"#;

    /// The same thing from a Keymapp old enough to write bare strings.
    const ROWS_STRING_LAYERS: &str = r#"[{"data":"{\"layerNames\":[\"Base\",\"Symbols\"]}"}]"#;

    /// sqlite3 printing an empty result set as `[]`. Not every build does --
    /// most print nothing at all -- which is why both are handled.
    const NO_ROWS: &str = "[]\n";

    /// Wrap a layout document the way sqlite3 would, so a test can be written in
    /// terms of the layout rather than of two levels of backslashes.
    fn rows_of(layout: &str) -> String {
        Value::Array(vec![Value::Object(
            [("data".to_string(), Value::String(layout.to_string()))].into_iter().collect(),
        )])
        .to_string()
    }

    fn names(text: &str) -> Vec<String> {
        layers_from_rows(text).names
    }

    /// A directory per test. Cargo runs tests on many threads in one process,
    /// so a name built from the pid alone is not unique.
    fn tmpdir(test: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("qs-keymapp-{}-{test}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::create_dir_all(&dir);
        dir
    }

    /// A Keymapp database, in the schema the module comment cites, holding
    /// `rows` written in the order given -- which is the `rowid` order.
    fn db_with(test: &str, rows: &[(&str, SqlValue)]) -> PathBuf {
        let path = tmpdir(test).join("keymapp.sqlite3");
        let conn = Connection::open(&path).expect("a fresh database opens");
        conn.execute("CREATE TABLE revision (revisionId TEXT NOT NULL UNIQUE, data BLOB)", [])
            .expect("the schema is a constant");
        for (id, data) in rows {
            conn.execute("INSERT INTO revision (revisionId, data) VALUES (?1, ?2)", (id, data))
                .expect("the insert is a constant");
        }
        drop(conn);
        path
    }

    /// One revision's document in the shape Keymapp keeps, its layers numbered
    /// in the order given. With no `created` the key is left out rather than
    /// written null, as the GNOME extension's fixture leaves it out.
    fn doc(hash: &str, created: Option<&str>, titles: &[&str]) -> String {
        let layers: Vec<Value> = titles
            .iter()
            .enumerate()
            .map(|(position, title)| {
                serde_json::json!({"title": title, "position": position, "color": null, "keys": []})
            })
            .collect();
        let mut revision = serde_json::json!({
            "hashId": hash, "title": "Edited", "model": "mk1", "layers": layers
        });
        if let Some(created) = created {
            revision["createdAt"] = Value::String(created.to_string());
        }
        serde_json::json!({
            "layout": {"hashId": "xbJpL", "title": "Moonlander", "geometry": "moonlander",
                       "revision": revision}
        })
        .to_string()
    }

    /// A document as the bytes of a BLOB, which is how the schema declares it.
    fn blob(document: String) -> SqlValue {
        SqlValue::Blob(document.into_bytes())
    }

    /// Three revisions whose hashes sort the wrong way, written in neither
    /// date order nor hash order, so that only `createdAt` puts them right.
    /// Byte order puts lowercase after uppercase, so `ORDER BY revisionId
    /// DESC` reads q8Rm2a, cW9nTe, Lz4bXo -- oldest first.
    fn three_revisions(test: &str) -> PathBuf {
        db_with(
            test,
            &[
                (
                    "Lz4bXo",
                    blob(doc("Lz4bXo", Some("2026-03-02 09:15:00 UTC"), &["Base", "Newest"])),
                ),
                (
                    "q8Rm2a",
                    blob(doc("q8Rm2a", Some("2026-01-05 18:40:12 UTC"), &["Base", "Oldest"])),
                ),
                (
                    "cW9nTe",
                    blob(doc("cW9nTe", Some("2026-02-11 07:02:59 UTC"), &["Base", "Middle"])),
                ),
            ],
        )
    }

    /// A /sys holding USB devices, each `(name, idVendor, serial)`, with the
    /// trailing newline the kernel writes.
    fn bus(test: &str, devices: &[(&str, &str, Option<&str>)]) -> PathBuf {
        let root = tmpdir(test);
        for (name, vendor, serial) in devices {
            let at = root.join("sys/bus/usb/devices").join(name);
            fs::create_dir_all(&at).expect("the temporary is writable");
            fs::write(at.join("idVendor"), format!("{vendor}\n"))
                .expect("the temporary is writable");
            if let Some(serial) = serial {
                fs::write(at.join("serial"), format!("{serial}\n"))
                    .expect("the temporary is writable");
            }
        }
        root
    }

    /// What `flashed_revisions` would answer for keyboards running these.
    fn on_the_bus(revisions: &[&str]) -> Vec<String> {
        revisions.iter().map(|r| r.to_string()).collect()
    }

    // ── the two levels ───────────────────────────────────────────────────────

    #[test]
    fn the_shape_sqlite3_actually_prints_is_read() {
        assert_eq!(
            layers_from_rows(ROWS),
            Layers {
                names: vec!["Base".into(), "Symbols".into(), "Numbers".into()],
                error: String::new()
            }
        );
    }

    #[test]
    fn the_outer_and_inner_documents_are_parsed_separately() {
        // The inner document is TEXT inside the row. Parsing the outer one once
        // and reading `.layers` off the row -- which is what a one-level parse
        // does -- finds nothing here, because the row has no `layers` key at
        // all; it has a string that contains one.
        let rows: Value = serde_json::from_str(ROWS).expect("the fixture is valid JSON");
        assert!(rows[0].get("layers").is_none());
        assert!(rows[0]["data"].is_string());
        assert_eq!(names(ROWS).len(), 3);
    }

    #[test]
    fn a_layout_that_is_itself_a_rows_array_is_not_mistaken_for_the_outer_level() {
        // A `data` column holding `[{"data":"..."}]` is unreadable, not a
        // second attempt at the outer parse: the inner document must be an
        // object.
        let doubled = rows_of(ROWS);
        assert_eq!(layers_from_rows(&doubled), Layers::failed(UNREADABLE));
    }

    #[test]
    fn no_rows_at_all_is_not_a_failure() {
        // A keyboard with no stored layout. The Python read this as an empty
        // object, so `error` stays empty and only the list is empty -- anything
        // else makes 50-hardware.sh print a warning at a machine that is fine.
        assert_eq!(layers_from_rows(NO_ROWS), Layers::default());
        assert_eq!(layers_from_rows(NO_ROWS).error, "");
    }

    #[test]
    fn nothing_on_stdin_is_the_branchs_own_message() {
        // sqlite3 prints NOTHING, not `[]`, for a query that matched no rows,
        // and the shell tested for that before it ever reached jq. The
        // newline is here because `parse` reads stdin raw, where the shell's
        // `$(...)` had already stripped it.
        for blank in ["", "\n"] {
            assert_eq!(layers_from_rows(blank), Layers::failed(NO_LAYOUT), "{blank:?}");
        }
    }

    #[test]
    fn whitespace_alone_overshoots_the_branch() {
        // NOT the branch's answer, and the test that used to assert this
        // claimed it was. `[[ -n "   " ]]` is true, so the shell handed a run
        // of spaces to jq -- checked against jq 1.7 itself, which answers
        // {"names":[],"error":"unreadable layout data"} for it.
        //
        // The behaviour is left alone: sqlite3 writes no spaces, so the input
        // is unreachable outside a hand-written `parse` fixture, and narrowing
        // the blank test to reproduce it would put the trailing newline of a
        // real query back on "unreadable". Pinned so the overshoot stays
        // deliberate.
        for spaces in [" ", "   \n", "\t"] {
            assert_eq!(layers_from_rows(spaces), Layers::failed(NO_LAYOUT), "{spaces:?}");
        }
    }

    #[test]
    fn a_malformed_rows_document_is_unreadable() {
        // sqlite3 can print an error where the rows should be, and jq given a
        // malformed document exits having written nothing -- which left the
        // shell substituting an empty string where the caller expected an
        // object. Every failure lands on the envelope instead.
        for junk in
            ["Error: no such table: revision", r#"[{"data": "#, "null", "{\"data\":\"{}\"}", "42"]
        {
            assert_eq!(layers_from_rows(junk), Layers::failed(UNREADABLE), "{junk}");
        }
    }

    #[test]
    fn a_data_column_that_is_not_a_string_is_unreadable() {
        // `($rows[0].data | type) != "string"`. A NULL column arrives as JSON
        // null and an integer column as a number; neither is a layout.
        for row in [r#"[{"data":null}]"#, r#"[{"data":42}]"#, r#"[{"data":{"layers":[]}}]"#] {
            assert_eq!(layers_from_rows(row), Layers::failed(UNREADABLE), "{row}");
        }
    }

    #[test]
    fn a_row_with_no_data_column_is_unreadable() {
        assert_eq!(layers_from_rows(r#"[{"layout":"x"}]"#), Layers::failed(UNREADABLE));
    }

    #[test]
    fn a_row_that_is_not_an_object_is_the_same_envelope_and_not_a_crash() {
        // The jq indexed `.data` on the row unconditionally, so `[1,2]` made it
        // exit 5 with nothing on stdout -- and the shell then handed its caller
        // an empty string where an object was expected. Deliberately changed:
        // the same shape comes out for this as for every other failure.
        assert_eq!(layers_from_rows("[1,2]"), Layers::failed(UNREADABLE));
        assert_eq!(layers_from_rows(r#"["x"]"#), Layers::failed(UNREADABLE));
    }

    #[test]
    fn a_malformed_inner_document_is_unreadable() {
        assert_eq!(layers_from_rows(&rows_of("{\"layers\": [")), Layers::failed(UNREADABLE));
        assert_eq!(layers_from_rows(&rows_of("")), Layers::failed(UNREADABLE));
    }

    #[test]
    fn an_inner_document_that_is_not_an_object_is_unreadable() {
        // `($d | type) != "object"`. A layout that is a bare array has no keys
        // to try, and reporting it as "no names" would hide a real corruption.
        for layout in ["[]", "\"Base\"", "7", "null"] {
            assert_eq!(layers_from_rows(&rows_of(layout)), Layers::failed(UNREADABLE), "{layout}");
        }
    }

    // ── a byte-order mark ────────────────────────────────────────────────────

    #[test]
    fn a_byte_order_mark_on_the_layout_document_is_skipped() {
        // jq's `fromjson` skips one and serde_json does not, so this document
        // read as a layout under the shell and came back "unreadable layout
        // data" here -- losing every name rather than one of them. Found by the
        // differential harness.
        let layout = rows_of("\u{feff}{\"layers\":[\"Base\",\"Symbols\"]}");
        assert_eq!(names(&layout), ["Base", "Symbols"]);
    }

    /// U+FEFF, spelled once so the tests below can interpolate it: `\u{...}`
    /// cannot be written inside a `format!` literal, where `{{` is an escaped
    /// brace before it is ever a codepoint.
    const BOM: &str = "\u{feff}";

    #[test]
    fn a_byte_order_mark_on_the_rows_document_is_skipped() {
        assert_eq!(names(&format!("{BOM}{ROWS}")), ["Base", "Symbols", "Numbers"]);
    }

    #[test]
    fn only_the_first_byte_order_mark_and_only_at_the_start() {
        // jq 1.7 accepts exactly one, at the very first character. Being more
        // permissive than the program this replaces would let a document that
        // is genuinely corrupt through as a layout.
        assert_eq!(layers_from_rows(&format!("{BOM}{BOM}{ROWS}")), Layers::failed(UNREADABLE));
        assert_eq!(layers_from_rows(&format!("  {BOM}{ROWS}")), Layers::failed(UNREADABLE));
        assert_eq!(layers_from_rows(&format!("{ROWS}{BOM}")), Layers::failed(UNREADABLE));
        // A BOM and nothing else is non-blank, so it reaches the parse and
        // fails there -- which is where the shell's `[[ -n ... ]]` sent it too.
        assert_eq!(layers_from_rows("\u{feff}"), Layers::failed(UNREADABLE));
    }

    // ── where serde_json stops agreeing with jq ──────────────────────────────

    #[test]
    fn a_number_out_of_double_range_makes_the_whole_layout_unreadable() {
        // NOT what the jq did: it kept `1e400` as a literal and answered with
        // the names. serde_json refuses the document, so one out-of-range
        // number ANYWHERE in the layout -- not only in a name -- empties the
        // envelope.
        //
        // Left as it is on purpose. The fix is serde_json's `arbitrary_precision`
        // feature, which is set per-workspace and would change the Value type
        // every other binary in this tree parses with, to make a layout Keymapp
        // cannot write parse. Recorded here so the limit is known rather than
        // discovered.
        let layout = rows_of(r#"{"layers":["Base"],"keys":[1e400]}"#);
        assert_eq!(layers_from_rows(&layout), Layers::failed(UNREADABLE));
    }

    #[test]
    fn a_layout_nested_deeper_than_serde_jsons_limit_is_unreadable() {
        // The same trade as the one above: jq has no depth limit and read this,
        // serde_json stops at 128 and calls the document unreadable. Lifting it
        // needs the `unbounded_depth` feature, workspace-wide. A Keymapp layout
        // is four levels deep.
        let deep =
            format!("{{\"layers\":[\"Base\"],\"t\":{}1{}}}", "[".repeat(200), "]".repeat(200));
        assert_eq!(layers_from_rows(&rows_of(&deep)), Layers::failed(UNREADABLE));
        // ...and a layout at a depth Keymapp actually reaches is fine.
        let shallow =
            format!("{{\"layers\":[\"Base\"],\"t\":{}1{}}}", "[".repeat(50), "]".repeat(50));
        assert_eq!(names(&rows_of(&shallow)), ["Base"]);
    }

    // ── where Keymapp keeps the names ────────────────────────────────────────

    /// A document whose `layout.revision.layers` is `layers`, as JSON text.
    fn nested(layers: &str) -> String {
        rows_of(&format!(r#"{{"layout":{{"revision":{{"layers":{layers}}}}}}}"#))
    }

    #[test]
    fn the_names_are_the_titles_under_layout_revision() {
        // The defect this fixes: every reader of Keymapp's database finds
        // them here, and this looked only at the top of the document.
        let layout = rows_of(&doc("x9m4dK", None, &["Base", "Symbols", "Media"]));
        assert_eq!(names(&layout), ["Base", "Symbols", "Media"]);
    }

    #[test]
    fn the_names_are_put_in_position_order() {
        let layout = nested(
            r#"[{"title":"Media","position":2},{"title":"Base","position":0},
                {"title":"Symbols","position":1}]"#,
        );
        assert_eq!(names(&layout), ["Base", "Symbols", "Media"]);
    }

    #[test]
    fn positions_that_do_not_number_every_layer_once_keep_the_array_order() {
        for layers in [
            r#"[{"title":"A","position":1},{"title":"B"}]"#,
            r#"[{"title":"A","position":1},{"title":"B","position":1}]"#,
            r#"[{"title":"A","position":0},{"title":"B","position":2}]"#,
            r#"[{"title":"A","position":1},{"title":"B","position":-1}]"#,
            r#"[{"title":"A","position":1.0},{"title":"B","position":0}]"#,
            r#"[{"title":"A","position":"1"},{"title":"B","position":"0"}]"#,
            r#"["A",{"title":"B","position":0}]"#,
        ] {
            assert_eq!(names(&nested(layers)), ["A", "B"], "{layers}");
        }
    }

    #[test]
    fn a_layer_with_no_title_is_blank_in_its_place() {
        // The index is the layer number here as at the top: the layer at
        // position 1 stays at 1 whatever it is called.
        let layout = nested(
            r#"[{"title":"Base","position":0},{"title":null,"position":1},
                {"title":"  Media ","position":2}]"#,
        );
        assert_eq!(names(&layout), ["Base", "", "Media"]);
    }

    #[test]
    fn the_nested_names_win_over_the_top_of_the_document() {
        let layout = rows_of(
            r#"{"layers":["Top"],"layout":{"revision":{"layers":[{"title":"Nested","position":0}]}}}"#,
        );
        assert_eq!(names(&layout), ["Nested"]);
    }

    #[test]
    fn a_nested_array_that_is_empty_or_absent_falls_through_to_the_top() {
        for layout in [
            r#"{"layout":{"revision":{"layers":[]}},"layers":["Top"]}"#,
            r#"{"layout":{"revision":{"layers":"Base"}},"layers":["Top"]}"#,
            r#"{"layout":{"revision":{}},"layers":["Top"]}"#,
            r#"{"layout":"x","layers":["Top"]}"#,
        ] {
            assert_eq!(names(&rows_of(layout)), ["Top"], "{layout}");
        }
    }

    // ── which key holds the names, at the top ───────────────────────────────

    #[test]
    fn the_keys_are_tried_in_order() {
        // The port's first guess at the shape, which no reader of a real
        // database has seen: `layers` first, then `layerNames`, then
        // `layer_names`.
        let all = rows_of(r#"{"layers":["A"],"layerNames":["B"],"layer_names":["C"]}"#);
        assert_eq!(names(&all), ["A"]);

        let two = rows_of(r#"{"layerNames":["B"],"layer_names":["C"]}"#);
        assert_eq!(names(&two), ["B"]);

        assert_eq!(names(&rows_of(r#"{"layer_names":["C"]}"#)), ["C"]);
        assert_eq!(names(ROWS_STRING_LAYERS), ["Base", "Symbols"]);
    }

    #[test]
    fn an_empty_array_under_the_first_key_falls_through_to_the_next() {
        // `select(type == "array" and length > 0)`. A version that writes
        // `"layers": []` beside a populated `layerNames` would otherwise answer
        // with no names on exactly the machines that have them.
        let layout = rows_of(r#"{"layers":[],"layerNames":["Base","Symbols"]}"#);
        assert_eq!(names(&layout), ["Base", "Symbols"]);
    }

    #[test]
    fn a_key_that_is_not_an_array_falls_through_to_the_next() {
        let layout = rows_of(r#"{"layers":"Base","layer_names":["Base"]}"#);
        assert_eq!(names(&layout), ["Base"]);
    }

    #[test]
    fn a_layout_with_no_recognised_key_is_a_real_answer() {
        // Not an error: a layout can genuinely have no names, and the numbers
        // still work.
        let layout = rows_of(r#"{"title":"Moonlander","keys":[1,2,3]}"#);
        assert_eq!(layers_from_rows(&layout), Layers::default());
    }

    // ── one entry's name ─────────────────────────────────────────────────────

    #[test]
    fn an_entry_may_be_a_string_or_an_object() {
        let layout = rows_of(r#"{"layers":["Base",{"name":"Symbols"},{"title":"Numbers"}]}"#);
        assert_eq!(names(&layout), ["Base", "Symbols", "Numbers"]);
    }

    #[test]
    fn name_wins_over_title() {
        assert_eq!(layer_name(&serde_json::json!({"name": "N", "title": "T"})), "N");
    }

    #[test]
    fn a_null_name_falls_through_to_title() {
        assert_eq!(layer_name(&serde_json::json!({"name": null, "title": "T"})), "T");
    }

    #[test]
    fn a_false_name_falls_through_to_title() {
        // jq's `//` treats `false` as absent exactly as it treats `null`. An
        // entry of `{"name": false}` is not a layer named "false".
        assert_eq!(layer_name(&serde_json::json!({"name": false, "title": "T"})), "T");
        assert_eq!(layer_name(&serde_json::json!({"name": false, "title": false})), "");
    }

    #[test]
    fn a_zero_name_is_kept() {
        // Zero is truthy in jq, so `0` does NOT fall through: a layer named "0"
        // keeps that name rather than borrowing the title.
        assert_eq!(layer_name(&serde_json::json!({"name": 0, "title": "T"})), "0");
    }

    #[test]
    fn a_structured_name_is_its_json_text() {
        // `tostring`. Not useful, but it is what the jq produced and it is not
        // a crash.
        assert_eq!(layer_name(&serde_json::json!({"name": {"en": "Base"}})), r#"{"en":"Base"}"#);
        assert_eq!(layer_name(&serde_json::json!({"name": 3.5})), "3.5");
    }

    #[test]
    fn an_entry_that_is_neither_a_string_nor_an_object_is_blank() {
        for entry in [serde_json::json!(7), serde_json::json!(null), serde_json::json!(["x"])] {
            assert_eq!(layer_name(&entry), "");
        }
    }

    #[test]
    fn blank_entries_are_kept_in_place() {
        // The index IS the layer number. Compacting would rename every layer
        // above a blank one, so `set-layer 2` would light up the layer the
        // panel calls 3.
        let layout = rows_of(r#"{"layers":["Base",{},"Numbers",null]}"#);
        assert_eq!(names(&layout), ["Base", "", "Numbers", ""]);
    }

    // ── trimming and cutting ─────────────────────────────────────────────────

    #[test]
    fn names_are_trimmed_at_both_ends() {
        assert_eq!(tidy("  Base \t\n"), "Base");
        assert_eq!(tidy("   "), "");
        // Interior whitespace is untouched: the trim is anchored in the jq.
        assert_eq!(tidy(" Base Layer "), "Base Layer");
    }

    #[test]
    fn unicode_spaces_are_trimmed_and_a_zero_width_space_is_not() {
        // Checked against jq 1.7 itself: oniguruma's `\s` is Unicode-aware
        // there, so U+00A0 and U+3000 go and U+200B stays -- which is exactly
        // what `str::trim` does. An ASCII-only trim would leave a leading blank
        // on any name pasted out of a document.
        assert_eq!(tidy("\u{a0}Base\u{a0}"), "Base");
        assert_eq!(tidy("\u{3000}Base"), "Base");
        assert_eq!(tidy("\u{200b}Base"), "\u{200b}Base");
    }

    #[test]
    fn names_are_cut_to_twenty_four_characters() {
        assert_eq!(tidy(&"x".repeat(30)), "x".repeat(24));
        assert_eq!(tidy("short"), "short");
    }

    #[test]
    fn the_cut_counts_characters_and_not_bytes() {
        // jq slices by codepoint. Slicing bytes would keep six emoji and panic
        // for landing inside the seventh.
        let cut = tidy(&"😀".repeat(30));
        assert_eq!(cut.chars().count(), 24);
        assert_eq!(cut, "😀".repeat(24));
    }

    #[test]
    fn trimming_happens_before_the_cut() {
        // The jq's order, and it shows: the leading spaces are gone before the
        // 24 are counted, so a padded name keeps 24 characters of name.
        assert_eq!(tidy(&format!("    {}", "x".repeat(30))), "x".repeat(24));
        // ...and trailing whitespace INSIDE the first 24 survives, because only
        // the whole string was trimmed.
        assert_eq!(
            tidy(&format!("{}   {}", "x".repeat(21), "y".repeat(20))),
            format!("{}   ", "x".repeat(21))
        );
    }

    // ── the index ────────────────────────────────────────────────────────────

    #[test]
    fn the_index_is_the_layer_number() {
        let names: Vec<String> =
            ["Base", "Symbols", "Numbers"].iter().map(|s| s.to_string()).collect();
        assert_eq!(name_at(&names, "0"), "Base");
        assert_eq!(name_at(&names, "2"), "Numbers");
    }

    #[test]
    fn an_index_past_the_end_is_an_empty_name() {
        // Not the number, and not "layer 9": the caller already has the number,
        // and a name that is really a number pretending to be one is worse than
        // an absent name.
        let names = vec!["Base".to_string()];
        assert_eq!(name_at(&names, "9"), "");
        assert_eq!(name_at(&[], "0"), "");
    }

    #[test]
    fn a_non_numeric_index_reads_layer_zero() {
        // The jq's `else 0`. `watch` writes the state file and `status` reads it
        // with `cat`, so an empty or half-written file asks for the base layer
        // rather than for nothing.
        let names = vec!["Base".to_string(), "Symbols".to_string()];
        for junk in ["", "one", " 1", "1.0", "0x1", "-1"] {
            assert_eq!(name_at(&names, junk), "Base", "{junk:?}");
        }
    }

    #[test]
    fn a_trailing_newline_on_the_index_still_counts() {
        // jq's `$` matches before a final newline, so `test("^[0-9]+$")` was
        // true for "1\n". Two newlines were not.
        let names = vec!["Base".to_string(), "Symbols".to_string()];
        assert_eq!(name_at(&names, "1\n"), "Symbols");
        assert_eq!(name_at(&names, "1\n\n"), "Base");
    }

    #[test]
    fn an_index_too_large_for_a_machine_integer_is_empty_rather_than_wrapped() {
        // jq answered `null` for `.[1e30]`, which is an empty name. Wrapping
        // would name a real layer for a number that asked for none.
        let names = vec!["Base".to_string(), "Symbols".to_string()];
        assert_eq!(layer_index("999999999999999999999999"), usize::MAX);
        assert_eq!(name_at(&names, "999999999999999999999999"), "");
    }

    #[test]
    fn leading_zeroes_are_still_a_number() {
        let names = vec!["Base".to_string(), "Symbols".to_string()];
        assert_eq!(name_at(&names, "01"), "Symbols");
    }

    // ── the index, against an envelope ───────────────────────────────────────

    #[test]
    fn the_envelope_this_prints_is_the_envelope_the_lookup_reads() {
        let envelope = serde_json::to_string(&layers_from_rows(ROWS)).expect("serialises");
        assert_eq!(name_from_envelope(&envelope, "1"), "Symbols");
        assert_eq!(name_from_envelope(&envelope, "7"), "");
    }

    #[test]
    fn an_envelope_that_reports_an_error_yields_no_name() {
        let envelope = serde_json::to_string(&Layers::failed(NO_DATABASE)).expect("serialises");
        assert_eq!(name_from_envelope(&envelope, "0"), "");
    }

    #[test]
    fn a_malformed_envelope_yields_no_name() {
        // `(try fromjson catch {})`. The shell reached this by piping a helper
        // that had printed nothing at all.
        for junk in ["", "not json", "{\"names\":"] {
            assert_eq!(name_from_envelope(junk, "0"), "", "{junk:?}");
        }
    }

    #[test]
    fn an_envelope_whose_names_is_not_an_array_yields_no_name() {
        // `select(type == "array")`, which produced no output at all in jq --
        // and a jq that produces nothing prints nothing, which the shell read
        // as an absent name.
        assert_eq!(name_from_envelope(r#"{"names":"Base"}"#, "0"), "");
        assert_eq!(name_from_envelope(r#"{"names":null}"#, "0"), "");
        assert_eq!(name_from_envelope("[1,2]", "0"), "");
    }

    #[test]
    fn a_null_entry_in_an_envelope_yields_no_name() {
        // `$names[$k] // ""`, which turns both null and false into "".
        assert_eq!(name_from_envelope(r#"{"names":[null,"Symbols"]}"#, "0"), "");
        assert_eq!(name_from_envelope(r#"{"names":[false]}"#, "0"), "");
        assert_eq!(name_from_envelope(r#"{"names":[7]}"#, "0"), "7");
    }

    // ── the database ─────────────────────────────────────────────────────────

    // ── which revision ───────────────────────────────────────────────────────

    #[test]
    fn the_newest_revision_is_not_the_hash_that_sorts_last() {
        // The defect this fixes. The query it replaced, over the same file,
        // proves the fixture would have caught it: alphabetical, and the
        // oldest of the three.
        let path = three_revisions("bydate");
        let conn = Connection::open(&path).expect("the fixture opens");
        let replaced: Vec<u8> = conn
            .query_row("SELECT data FROM revision ORDER BY revisionId DESC LIMIT 1", [], |row| {
                row.get(0)
            })
            .expect("the fixture has rows");
        drop(conn);
        assert!(String::from_utf8_lossy(&replaced).contains("Oldest"));

        assert_eq!(layers_from_db(&path, &[]).names, ["Base", "Newest"]);
    }

    #[test]
    fn the_revision_on_the_keyboard_wins_over_the_newest() {
        let path = three_revisions("byserial");
        assert_eq!(layers_from_db(&path, &on_the_bus(&["cW9nTe"])).names, ["Base", "Middle"]);
        // A keyboard flashed back to an older revision, which no ordering of
        // the rows can get right.
        assert_eq!(layers_from_db(&path, &on_the_bus(&["q8Rm2a"])).names, ["Base", "Oldest"]);
    }

    #[test]
    fn a_revision_keymapp_does_not_hold_falls_back_to_the_newest() {
        // A board flashed from Oryx's site, say, before Keymapp has seen it.
        let path = three_revisions("unheld");
        assert_eq!(layers_from_db(&path, &on_the_bus(&["zzzzzz"])).names, ["Base", "Newest"]);
        // With several boards plugged in, the first whose revision is held.
        let several = on_the_bus(&["zzzzzz", "q8Rm2a", "cW9nTe"]);
        assert_eq!(layers_from_db(&path, &several).names, ["Base", "Oldest"]);
    }

    #[test]
    fn with_no_dates_the_row_written_last_wins() {
        // Written in this order, so `rowid` says Last, and the hashes sort
        // the other way, so `revisionId DESC` would have said First.
        let path = db_with(
            "nodates",
            &[
                ("q8Rm2a", blob(doc("q8Rm2a", None, &["First"]))),
                ("cW9nTe", blob(doc("cW9nTe", None, &["Second"]))),
                ("Lz4bXo", blob(doc("Lz4bXo", None, &["Last"]))),
            ],
        );
        assert_eq!(layers_from_db(&path, &[]).names, ["Last"]);
    }

    #[test]
    fn a_revision_with_a_date_beats_one_without() {
        // NULL sorts last under DESC, so a newer row with no date loses to an
        // older one with one. Pinned as the order the module comment gives,
        // since the serial, not this, is what is right after a re-flash.
        let path = db_with(
            "somedates",
            &[
                ("q8Rm2a", blob(doc("q8Rm2a", Some("2026-01-05 18:40:12 UTC"), &["Dated"]))),
                ("Lz4bXo", blob(doc("Lz4bXo", None, &["Undated"]))),
            ],
        );
        assert_eq!(layers_from_db(&path, &[]).names, ["Dated"]);
    }

    #[test]
    fn a_damaged_row_does_not_hide_the_good_ones() {
        // Unguarded, `json_extract` over the second row is "malformed JSON"
        // for the whole statement, and the good revision goes with it.
        let path = db_with(
            "damaged",
            &[
                ("q8Rm2a", blob(doc("q8Rm2a", Some("2026-01-05 18:40:12 UTC"), &["Good"]))),
                ("Lz4bXo", SqlValue::Blob(br#"{"layout": "#.to_vec())),
            ],
        );
        assert_eq!(
            layers_from_db(&path, &[]),
            Layers { names: vec!["Good".into()], error: "".into() }
        );
        // Named by the keyboard, it is the ordinary failure and not a read
        // error: the row is there and is not a layout.
        assert_eq!(layers_from_db(&path, &on_the_bus(&["Lz4bXo"])), Layers::failed(UNREADABLE));
    }

    #[test]
    fn a_document_stored_as_text_is_dated_the_same_way() {
        // The schema declares BLOB, and SQLite stores whatever the writer
        // bound; which one Keymapp binds is not known.
        let path = db_with(
            "textdocs",
            &[
                (
                    "q8Rm2a",
                    SqlValue::Text(doc("q8Rm2a", Some("2026-03-02 09:15:00 UTC"), &["New"])),
                ),
                (
                    "Lz4bXo",
                    SqlValue::Text(doc("Lz4bXo", Some("2026-01-05 18:40:12 UTC"), &["Old"])),
                ),
            ],
        );
        assert_eq!(layers_from_db(&path, &[]).names, ["New"]);
        assert_eq!(layers_from_db(&path, &on_the_bus(&["Lz4bXo"])).names, ["Old"]);
    }

    #[test]
    fn the_schema_this_port_first_assumed_still_reads() {
        // `revisionId INTEGER PRIMARY KEY` was the guess before any reader of
        // the real database was found. There `rowid` IS the id, so the
        // highest id still wins, and a hash from the bus matches nothing
        // without failing the query.
        let path = tmpdir("intschema").join("keymapp.sqlite3");
        let conn = Connection::open(&path).expect("a fresh database opens");
        conn.execute("CREATE TABLE revision (revisionId INTEGER PRIMARY KEY, data TEXT)", [])
            .expect("the schema is a constant");
        for (id, data) in [
            (1, r#"{"layers":["Old"]}"#),
            (7, r#"{"layers":["New"]}"#),
            (3, r#"{"layers":["Middle"]}"#),
        ] {
            conn.execute("INSERT INTO revision (revisionId, data) VALUES (?1, ?2)", (id, data))
                .expect("the insert is a constant");
        }
        drop(conn);
        assert_eq!(layers_from_db(&path, &[]).names, ["New"]);
        assert_eq!(layers_from_db(&path, &on_the_bus(&["x9m4dK"])).names, ["New"]);
    }

    // ── the keyboard's serial ────────────────────────────────────────────────

    #[test]
    fn the_revision_is_what_follows_the_slash_in_an_oryx_serial() {
        // `#define SERIAL_NUMBER "xbJpL/x9m4dK"` in the config.h Oryx generated
        // for ZSA's default Moonlander layout.
        assert_eq!(revision_in("xbJpL/x9m4dK"), Some("x9m4dK"));
        for serial in ["", "x9m4dK", "/x9m4dK", "xbJpL/", "/", "a/b/c", "0"] {
            assert_eq!(revision_in(serial), None, "{serial:?}");
        }
    }

    #[test]
    fn only_a_zsa_device_with_an_oryx_serial_names_a_revision() {
        let root = bus(
            "bus",
            &[
                ("1-4", "3297", Some("xbJpL/x9m4dK")),
                // An interface. A real one has neither file; given both, it is
                // still not a device.
                ("1-4:1.0", "3297", Some("xbJpL/aaaaaa")),
                ("1-5", "046d", Some("xbJpL/bbbbbb")),
                ("1-6", "3297", None),
                ("1-7", "3297", Some("0")),
                ("2-1", "3297", Some("Kd4xP/Qm7xLa")),
            ],
        );
        assert_eq!(flashed_revisions(&root), ["x9m4dK", "Qm7xLa"]);
    }

    #[test]
    fn no_bus_at_all_names_no_revision() {
        assert!(flashed_revisions(&tmpdir("nobus")).is_empty());
    }

    #[test]
    fn the_keyboard_on_the_bus_chooses_the_row() {
        // The two halves together, as `names` runs them.
        let path = three_revisions("busrow");
        let root = bus("busrowroot", &[("3-2", "3297", Some("xbJpL/q8Rm2a"))]);
        assert_eq!(layers_from_db(&path, &flashed_revisions(&root)).names, ["Base", "Oldest"]);
    }

    // ── the rest of the database ─────────────────────────────────────────────

    #[test]
    fn an_empty_revision_table_says_there_is_no_layout() {
        let path = db_with("empty", &[]);
        assert_eq!(layers_from_db(&path, &[]), Layers::failed(NO_LAYOUT));
        assert_eq!(layers_from_db(&path, &on_the_bus(&["x9m4dK"])), Layers::failed(NO_LAYOUT));
    }

    #[test]
    fn a_null_data_column_is_unreadable_rather_than_a_read_error() {
        // The `type != "string"` arm, reached from the real database rather than
        // from sqlite3's rendering of it.
        let path = db_with("nulldata", &[("x9m4dK", SqlValue::Null)]);
        assert_eq!(layers_from_db(&path, &[]), Layers::failed(UNREADABLE));
        let path = db_with("intdata", &[("x9m4dK", SqlValue::Integer(42))]);
        assert_eq!(layers_from_db(&path, &[]), Layers::failed(UNREADABLE));
    }

    #[test]
    fn a_blob_data_column_is_read_as_the_document() {
        // The schema declares `data BLOB`, and `sqlite3 -json` printed a blob
        // column as a JSON *string* -- so the jq took its `type == "string"`
        // arm and produced the names. Rejecting it as "not a layout" lost every
        // name on a machine the shell handled. Found by running both sides over
        // the same database, when this port still guessed the column was TEXT.
        let path = db_with(
            "blob",
            &[("x9m4dK", SqlValue::Blob(br#"{"layers":["Base","Symbols"]}"#.to_vec()))],
        );
        assert_eq!(layers_from_db(&path, &[]).names, ["Base", "Symbols"]);

        // A blob that is not a layout is still the ordinary failure.
        let path =
            db_with("blobjunk", &[("x9m4dK", SqlValue::Blob(b"\xff\xfe not json".to_vec()))]);
        assert_eq!(layers_from_db(&path, &[]), Layers::failed(UNREADABLE));
    }

    #[test]
    fn a_missing_database_says_what_to_do_about_it() {
        let path = tmpdir("missing").join("keymapp.sqlite3");
        assert_eq!(layers_from_db(&path, &[]), Layers::failed(NO_DATABASE));
    }

    #[test]
    fn a_file_that_is_not_a_database_is_a_read_error_and_not_a_panic() {
        let path = tmpdir("notadb").join("keymapp.sqlite3");
        fs::write(&path, b"this is not a database").expect("the temporary is writable");
        let layers = layers_from_db(&path, &[]);
        assert!(layers.names.is_empty());
        assert!(!layers.error.is_empty(), "a file that is not a database must say so");
    }

    #[test]
    fn a_database_with_no_revision_table_is_a_read_error() {
        // What `SELECT data FROM revision` does to a database Keymapp has not
        // created yet. The shell reported this as "no layout in the database",
        // which sent people looking for a layout rather than for the table.
        let path = tmpdir("notable").join("keymapp.sqlite3");
        let conn = Connection::open(&path).expect("a fresh database opens");
        conn.execute("CREATE TABLE other (id INTEGER)", []).expect("the schema is a constant");
        drop(conn);
        let layers = layers_from_db(&path, &[]);
        assert!(layers.names.is_empty());
        assert!(layers.error.contains("revision"), "the reason should name the table: {layers:?}");
    }

    #[test]
    fn the_database_is_opened_read_only() {
        // Keymapp may be running, and opening its database read-write while it
        // holds a lock is how you corrupt someone's layout. Proved rather than
        // asserted in a comment: the connection this program makes cannot
        // write.
        let path = db_with(
            "readonly",
            &[("x9m4dK", SqlValue::Text(r#"{"layers":["Base"]}"#.to_string()))],
        );
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let conn = Connection::open_with_flags(&path, flags).expect("the fixture opens");
        assert!(conn.execute("DELETE FROM revision", []).is_err());
    }

    // ── where the database is ────────────────────────────────────────────────

    #[test]
    fn the_dotted_directory_under_the_config_root_wins_when_it_exists() {
        let root = Path::new("/home/u/.config-alt");
        let home = Path::new("/home/u");
        assert_eq!(db_dir(root, home, true), Path::new("/home/u/.config-alt/.keymapp"));
    }

    #[test]
    fn the_fallback_is_not_xdg_aware_and_that_is_deliberate() {
        // The shell wrote `"$HOME/.config/keymapp"` literally, so a machine
        // with XDG_CONFIG_HOME pointed elsewhere falls back into ~/.config
        // regardless. Tidying it would move the file this reads on exactly the
        // machines that set that variable.
        let root = Path::new("/home/u/.config-alt");
        let home = Path::new("/home/u");
        assert_eq!(db_dir(root, home, false), Path::new("/home/u/.config/keymapp"));
    }

    // ── the envelope's own shape ─────────────────────────────────────────────

    #[test]
    fn the_envelope_is_names_then_error() {
        // 50-hardware.sh greps `"names":\s*\[\K[^]]*` and `"error":\s*"\K[^"]*`
        // out of this. Both keys are always present, and in this order.
        let text = serde_json::to_string(&layers_from_rows(ROWS)).expect("serialises");
        assert_eq!(text, r#"{"names":["Base","Symbols","Numbers"],"error":""}"#);

        let text = serde_json::to_string(&Layers::failed(UNREADABLE)).expect("serialises");
        assert_eq!(text, r#"{"names":[],"error":"unreadable layout data"}"#);
    }

    // ── the layout, for the held Super ───────────────────────────────────────

    /// A revision written in the shape Oryx's API answered for ZSA's default
    /// Moonlander layout -- by hand, not copied: two layers listed out of
    /// position order, a layer-tap, a toggle, a transparent key and an empty
    /// one, the step fields Oryx sends beside `code` and `layer`.
    fn keyed_doc() -> String {
        serde_json::json!({"layout": {"hashId": "xbJpL", "title": "Mine", "geometry": "moonlander",
            "revision": {"hashId": "r1", "layers": [
                {"title": "Symbols", "position": 1, "color": "#B30000", "keys": [
                    {"tap": {"code": "KC_TRANSPARENT", "layer": null, "color": null}},
                    {"tap": null, "hold": null},
                    {"tap": {"code": "TG", "layer": 1, "modifiers": null}}]},
                {"title": "Base", "position": 0, "color": null, "keys": [
                    {"tap": {"code": "KC_GRAVE", "layer": null}, "hold": {"code": "MO", "layer": 1}},
                    {"tap": {"code": "KC_A"}, "doubleTap": {"code": "TO", "layer": 1},
                     "tapHold": {"code": "OSL", "layer": 1}},
                    {"tap": {"code": "TG", "layer": 1}}]}]}}})
        .to_string()
    }

    #[test]
    fn the_layout_is_the_revisions_layers_in_position_order_with_their_keys() {
        let board = board_from_layout(&keyed_doc());
        assert_eq!(board.error, "");
        assert_eq!(board.geometry, "moonlander");
        let titles: Vec<&str> = board.layers.iter().map(|l| l.title.as_str()).collect();
        assert_eq!(titles, ["Base", "Symbols"]);
        let base = &board.layers[0].keys;
        assert_eq!(base[0].tap, Some(Step { code: "KC_GRAVE".into(), layer: None }));
        assert_eq!(base[0].hold, Some(Step { code: "MO".into(), layer: Some(1) }));
        assert_eq!(base[1].double_tap, Some(Step { code: "TO".into(), layer: Some(1) }));
        assert_eq!(base[1].tap_hold, Some(Step { code: "OSL".into(), layer: Some(1) }));
        let symbols = &board.layers[1].keys;
        assert_eq!(symbols[0].tap, Some(Step { code: "KC_TRANSPARENT".into(), layer: None }));
        // An empty key is a key, kept in its place, with nothing on it.
        assert_eq!(symbols[1], Key::default());
        assert_eq!(symbols.len(), 3);
    }

    #[test]
    fn the_document_prints_only_the_steps_a_key_has_in_oryxs_names() {
        let board = board_from_layout(&keyed_doc());
        let text = serde_json::to_string(&board.layers[0].keys).expect("serialises");
        assert_eq!(
            text,
            concat!(
                r#"[{"tap":{"code":"KC_GRAVE"},"hold":{"code":"MO","layer":1}},"#,
                r#"{"tap":{"code":"KC_A"},"doubleTap":{"code":"TO","layer":1},"#,
                r#""tapHold":{"code":"OSL","layer":1}},{"tap":{"code":"TG","layer":1}}]"#
            )
        );
        assert_eq!(serde_json::to_string(&Key::default()).expect("serialises"), "{}");
    }

    #[test]
    fn a_step_with_no_code_or_a_code_that_is_not_text_is_none() {
        assert_eq!(step_in(None), None);
        assert_eq!(step_in(Some(&Value::Null)), None);
        assert_eq!(step_in(Some(&serde_json::json!({"code": ""}))), None);
        assert_eq!(step_in(Some(&serde_json::json!({"code": "   "}))), None);
        assert_eq!(step_in(Some(&serde_json::json!({"code": 4}))), None);
        assert_eq!(step_in(Some(&serde_json::json!({"layer": 1}))), None);
        assert_eq!(step_in(Some(&serde_json::json!("KC_A"))), None);
    }

    #[test]
    fn a_target_is_a_whole_number_from_zero_or_nothing() {
        let at = |layer: Value| step_in(Some(&serde_json::json!({"code": "MO", "layer": layer})));
        assert_eq!(at(serde_json::json!(3)), Some(Step { code: "MO".into(), layer: Some(3) }));
        assert_eq!(at(serde_json::json!(-1)), Some(Step { code: "MO".into(), layer: None }));
        assert_eq!(at(serde_json::json!("1")), Some(Step { code: "MO".into(), layer: None }));
        assert_eq!(at(serde_json::json!(1.5)), Some(Step { code: "MO".into(), layer: None }));
        assert_eq!(at(Value::Null), Some(Step { code: "MO".into(), layer: None }));
    }

    #[test]
    fn a_colour_is_six_hex_digits_or_none() {
        let colour = |v: Value| colour_in(Some(&v));
        assert_eq!(colour(serde_json::json!("#B30000")), Some("#b30000".to_string()));
        assert_eq!(colour(serde_json::json!(" #1e90ff ")), Some("#1e90ff".to_string()));
        assert_eq!(colour(serde_json::json!("#fff")), None);
        assert_eq!(colour(serde_json::json!("red")), None);
        assert_eq!(colour(serde_json::json!("#gg0000")), None);
        assert_eq!(colour(serde_json::json!(16711680)), None);
        assert_eq!(colour(Value::Null), None);
        assert_eq!(colour_in(None), None);
    }

    #[test]
    fn a_layout_with_no_layers_under_its_revision_has_no_keys_to_draw() {
        // Names may still be read from the top of such a document; keys not.
        let board = board_from_layout(r#"{"layers":[{"name":"Base"}]}"#);
        assert_eq!(board.error, NO_KEYS);
        assert!(board.layers.is_empty());
        let board = board_from_layout(&doc("r", None, &[]));
        assert_eq!(board.error, NO_KEYS);
        assert_eq!(board.geometry, "moonlander");
    }

    #[test]
    fn a_damaged_document_is_the_same_envelope_and_says_so() {
        for text in ["{", "[1,2]", "7", ""] {
            let board = board_from_layout(text);
            assert_eq!(board.error, UNREADABLE, "{text:?}");
            assert!(board.layers.is_empty() && board.geometry.is_empty(), "{text:?}");
        }
        let text = serde_json::to_string(&Board::failed(UNREADABLE)).expect("serialises");
        assert_eq!(
            text,
            r#"{"plugged":false,"geometry":"","layers":[],"error":"unreadable layout data"}"#
        );
    }

    #[test]
    fn a_document_too_big_to_be_a_layout_is_cut_to_what_a_board_can_have() {
        let key = serde_json::json!({"tap": {"code": "K".repeat(200)}});
        let layer = serde_json::json!({"title": "L", "keys": vec![key; MAX_KEYS + 40]});
        let text =
            serde_json::json!({"layout": {"revision": {"layers": vec![layer; MAX_LAYERS + 9]}}})
                .to_string();
        let board = board_from_layout(&text);
        assert_eq!(board.layers.len(), MAX_LAYERS);
        assert!(board.layers.iter().all(|l| l.keys.len() == MAX_KEYS));
        let code = &board.layers[0].keys[0].tap.as_ref().expect("a tap").code;
        assert_eq!(code.chars().count(), MAX_CODE_CHARS);
    }

    #[test]
    fn plugged_is_a_zsa_device_on_the_bus_that_nothing_has_blocked() {
        let root = bus("plugged", &[("1-4", "3297", None), ("1-5", "046d", None)]);
        assert!(zsa_plugged(&root));
        fs::write(root.join("sys/bus/usb/devices/1-4/authorized"), "1\n").expect("writable");
        assert!(zsa_plugged(&root), "authorised is plugged");
        fs::write(root.join("sys/bus/usb/devices/1-4/authorized"), "0\n").expect("writable");
        assert!(!zsa_plugged(&root), "a board USBGuard blocked sends nothing");
        let root = bus("plugged-iface", &[("1-4:1.0", "3297", None), ("1-5", "046d", None)]);
        assert!(!zsa_plugged(&root), "an interface is not a device");
        assert!(!zsa_plugged(&tmpdir("plugged-none")), "no bus is no keyboard");
    }

    #[test]
    fn the_layout_is_the_revision_on_the_keyboard_and_says_whether_it_is_plugged() {
        let path = db_with(
            "board-flashed",
            &[
                ("old", blob(doc("old", Some("2026-01-01 00:00:00 UTC"), &["Old"]))),
                ("new", blob(doc("new", Some("2026-03-01 00:00:00 UTC"), &["New"]))),
            ],
        );
        let root = bus("board-flashed-bus", &[("1-4", "3297", Some("xbJpL/old"))]);
        let board = board_from(&path, &root);
        assert!(board.plugged);
        assert_eq!(board.layers.iter().map(|l| l.title.as_str()).collect::<Vec<_>>(), ["Old"]);
        let board = board_from(&path, &tmpdir("board-flashed-empty"));
        assert!(!board.plugged);
        assert_eq!(board.layers[0].title, "New", "with no keyboard, the newest");
    }

    #[test]
    fn no_database_is_the_same_envelope_with_the_reason_and_the_bus_still_read() {
        let root = bus("board-no-db", &[("1-4", "3297", None)]);
        let board = board_from(&tmpdir("board-no-db-dir").join("absent.sqlite3"), &root);
        assert_eq!(board.error, NO_DATABASE);
        assert!(board.plugged && board.layers.is_empty());
    }

    #[test]
    fn the_names_and_the_layout_read_one_document() {
        // The panel and the popups must agree, so both read through
        // document_from_db and cut titles the same way.
        let long = "A layer name far longer than twenty-four";
        let path = db_with("board-same", &[("r", blob(doc("r", None, &["Base", long])))]);
        let root = tmpdir("board-same-bus");
        let names = layers_from_db(&path, &flashed_revisions(&root)).names;
        let titles: Vec<String> =
            board_from(&path, &root).layers.into_iter().map(|l| l.title).collect();
        assert_eq!(names, titles);
        assert_eq!(titles[1].chars().count(), MAX_NAME_CHARS);
    }
}
