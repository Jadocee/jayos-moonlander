//! Reading /sys and /proc.
//!
//! Kernel files are small, ASCII, and full of trailing newlines. The Python
//! read them with `open(p).read().strip()` inside a bare `try/except`, which
//! is right -- a device that disappears between listing it and reading it is
//! normal, not exceptional, and the answer is "no value" rather than a crash.
//!
//! What that idiom hid is the difference between "the file is not there" and
//! "the file is there and says something unexpected". The first is a device
//! that was unplugged; the second is a kernel interface that changed. Both
//! return None here, but only the second is worth a line on stderr, so the
//! parsing helpers are separate from the reading one.

use std::fs;
use std::path::{Path, PathBuf};

/// The contents of a kernel file, trimmed, or None if it cannot be read.
pub fn read(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

/// A kernel file parsed as an integer.
pub fn read_i64(path: impl AsRef<Path>) -> Option<i64> {
    read(path)?.parse().ok()
}

/// A kernel file parsed as a float.
pub fn read_f64(path: impl AsRef<Path>) -> Option<f64> {
    read(path)?.parse().ok()
}

/// A kernel flag file, where "1" is true.
///
/// Also accepts "Y"/"y"/"enabled", which different subsystems use for the same
/// idea. Anything else is false rather than an error: a flag file with an
/// unrecognised value is still not a yes.
pub fn read_bool(path: impl AsRef<Path>) -> bool {
    match read(path) {
        Some(v) => {
            matches!(v.to_ascii_lowercase().as_str(), "1" | "y" | "yes" | "on" | "enabled" | "true")
        }
        None => false,
    }
}

/// The entries of a directory, sorted by name.
///
/// Sorted because readdir order is filesystem order, which is arbitrary and
/// varies between boots. A device list that reorders itself every boot makes
/// the widget that renders it jump around for no reason.
pub fn entries(dir: impl AsRef<Path>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).map(|e| e.path()).collect(),
        Err(_) => return Vec::new(),
    };
    out.sort();
    out
}

/// Entry names in a directory, sorted.
pub fn entry_names(dir: impl AsRef<Path>) -> Vec<String> {
    entries(dir)
        .into_iter()
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect()
}

/// Whether this looks like a laptop.
///
/// A battery whose `scope` is not `Device`, rather than the DMI chassis type.
/// A DualSense reports itself under `power_supply` too, so on a desktop with a
/// controller plugged in the naive check says "laptop" -- and then the shell
/// draws a battery indicator for a gamepad and hides the settings a desktop
/// wants. docs/HARDWARE.md records this; it is the same rule as `hw.sh has laptop`.
pub fn is_laptop() -> bool {
    for entry in entries("/sys/class/power_supply") {
        let kind = read(entry.join("type")).unwrap_or_default();
        if !kind.eq_ignore_ascii_case("battery") {
            continue;
        }
        // No `scope` file at all means a built-in battery: the attribute is
        // added by drivers for batteries belonging to a peripheral.
        match read(entry.join("scope")) {
            Some(scope) if scope.eq_ignore_ascii_case("device") => continue,
            _ => return true,
        }
    }
    false
}

/// The kernel release, as `uname -r` reports it.
pub fn kernel_release() -> Option<String> {
    read("/proc/sys/kernel/osrelease")
}

/// A value from `/etc/os-release`, unquoted.
pub fn os_release(key: &str) -> Option<String> {
    let text = fs::read_to_string("/etc/os-release").ok()?;
    for line in text.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix(key).and_then(|r| r.strip_prefix('=')) {
            return Some(value.trim_matches('"').trim_matches('\'').to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_parses_proc() {
        // /proc/sys/kernel/osrelease exists on every Linux and is a number
        // followed by text, which exercises the trim.
        let release = kernel_release().expect("no osrelease");
        assert!(!release.is_empty());
        assert!(!release.ends_with('\n'));
    }

    #[test]
    fn missing_files_are_none_not_a_panic() {
        assert_eq!(read("/sys/definitely/not/here"), None);
        assert_eq!(read_i64("/sys/definitely/not/here"), None);
        assert!(!read_bool("/sys/definitely/not/here"));
        assert!(entries("/sys/definitely/not/here").is_empty());
    }

    #[test]
    fn entries_come_back_sorted() {
        let names = entry_names("/proc/self");
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }
}
