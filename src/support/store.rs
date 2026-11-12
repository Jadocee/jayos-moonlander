//! Reading and writing the files these helpers keep.
//!
//! Two things, both of which the Python got wrong somewhere.
//!
//! ATOMIC WRITES. A helper that writes JSON with `open(p, "w")` truncates the
//! file, then writes. Interrupt it in between -- a logout, an OOM kill, a
//! second copy started by an impatient keybind -- and the file on disk is
//! empty or half a document. The ledger, the settings and the usage counts have
//! all been found truncated this way. Writing to a temporary file in the same
//! directory and renaming makes the replacement atomic: a reader sees either
//! the old file or the new one.
//!
//! TTL READS. Several helpers cache a network answer for a while. Each did its
//! own `time.time() - os.path.getmtime(p) < 900`, and two of them got the
//! comparison the wrong way round and cached forever.

use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{Duration, SystemTime};

/// Write bytes so that a reader never sees a partial file.
///
/// The temporary lands in the same directory as the destination, because
/// `rename` is only atomic within a filesystem and /tmp is frequently a
/// different one.
pub fn write_atomic(path: impl AsRef<Path>, contents: &[u8]) -> io::Result<()> {
    let path = path.as_ref();
    let dir = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(dir)?;

    // The pid in the name, so two copies of the same helper racing each other
    // do not write to the same temporary and produce a mixture of both.
    let tmp = dir.join(format!(
        ".{}.{}.tmp",
        path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default(),
        std::process::id()
    ));

    let result = (|| {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(contents)?;
        // Flushed to disk before the rename. Without it the rename can be
        // recorded while the contents are still in the page cache, and a power
        // loss in between leaves a file that exists and is empty -- which is
        // the one outcome this whole function is here to prevent.
        file.sync_all()?;
        fs::rename(&tmp, path)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Write a value as pretty-printed JSON, atomically.
///
/// Pretty rather than compact: these files are read by people debugging, and
/// a diff of a single-line JSON document says only that the line changed.
pub fn write_json<T: Serialize>(path: impl AsRef<Path>, value: &T) -> io::Result<()> {
    let text = serde_json::to_vec_pretty(value)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    write_atomic(path, &text)
}

/// Read a JSON file, or None if it is absent or unparseable.
///
/// Unparseable counts as absent deliberately. These files are caches and
/// ledgers; one corrupted by a crash should be rebuilt, not turned into a
/// startup failure for the widget that reads it. The corruption is reported on
/// stderr so it is not entirely silent.
pub fn read_json<T: DeserializeOwned>(path: impl AsRef<Path>) -> Option<T> {
    let path = path.as_ref();
    let text = fs::read_to_string(path).ok()?;
    match serde_json::from_str(&text) {
        Ok(v) => Some(v),
        Err(e) => {
            eprintln!("warning: ignoring unreadable {}: {e}", path.display());
            None
        }
    }
}

/// How long ago a file was modified.
pub fn age(path: impl AsRef<Path>) -> Option<Duration> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    // A file dated in the future -- a bad clock, a restored backup -- gives an
    // error from duration_since. Treated as brand new rather than as an error,
    // which is the safe direction: it means "do not refetch yet" rather than
    // "refetch on every call".
    Some(SystemTime::now().duration_since(modified).unwrap_or(Duration::ZERO))
}

/// Whether a cache file exists and is younger than `ttl`.
pub fn is_fresh(path: impl AsRef<Path>, ttl: Duration) -> bool {
    age(path).map(|a| a < ttl).unwrap_or(false)
}

/// A JSON cache, but only if it is younger than `ttl`.
pub fn read_json_fresh<T: DeserializeOwned>(path: impl AsRef<Path>, ttl: Duration) -> Option<T> {
    let path = path.as_ref();
    is_fresh(path, ttl).then(|| read_json(path)).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    // A directory PER TEST, not per process.
    //
    // These all shared one, and `leaves_no_temporary_behind` listed it for
    // *.tmp while a sibling test was part-way through a write -- so it failed
    // perhaps one run in three, reporting a bug in write_atomic that was
    // really a bug in the test. Cargo runs tests in one process on many
    // threads, so "unique per process" is not unique at all.
    fn tmpdir(test: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("qs-core-store-{}-{test}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn writes_and_reads_json() {
        let dir = tmpdir("writes_and_reads_json");
        let p = dir.join("a.json");
        write_json(&p, &serde_json::json!({"a": 1})).unwrap();
        let back: serde_json::Value = read_json(&p).unwrap();
        assert_eq!(back["a"], 1);
        fs::remove_file(&p).ok();
    }

    #[test]
    fn leaves_no_temporary_behind() {
        let dir = tmpdir("leaves_no_temporary_behind");
        let p = dir.join("b.json");
        write_json(&p, &serde_json::json!({"b": 2})).unwrap();
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "temporary files left: {leftovers:?}");
        fs::remove_file(&p).ok();
    }

    #[test]
    fn corrupt_json_reads_as_absent() {
        let dir = tmpdir("corrupt_json_reads_as_absent");
        let p = dir.join("c.json");
        write_atomic(&p, b"{not json").unwrap();
        assert!(read_json::<serde_json::Value>(&p).is_none());
        fs::remove_file(&p).ok();
    }

    #[test]
    fn freshness_is_the_right_way_round() {
        let dir = tmpdir("freshness_is_the_right_way_round");
        let p = dir.join("d.json");
        write_atomic(&p, b"{}").unwrap();
        assert!(is_fresh(&p, Duration::from_secs(60)), "a file just written is fresh");
        assert!(!is_fresh(&p, Duration::ZERO), "nothing is fresher than zero");
        assert!(!is_fresh(dir.join("missing.json"), Duration::from_secs(60)));
        fs::remove_file(&p).ok();
    }
}
