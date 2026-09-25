//! Reading and writing: the name a repository is filed under, the two
//! file operations, and the per-key coercions every reader falls back on.

use std::path::Path;

use toml::{Table, Value};

use super::{AUTO_WIDTH, StoreError};

/// The name a repository is filed under: separators the way git writes
/// them, and no trailing one. Every path either file holds goes through
/// this, so the same repository reads the same wherever it is written
/// down and nothing needs a backslash escape.
pub fn repo_key(path: &str) -> String {
    // The `\\?\` / `\\.\` prefix turns off Windows path parsing; rewriting
    // it addresses somewhere else.
    if path.starts_with(r"\\?\") || path.starts_with(r"\\.\") {
        return path.to_string();
    }
    let key = path.replace('\\', "/");
    let trimmed = key.trim_end_matches('/');
    // A drive root and the filesystem root keep their slash.
    if trimmed.is_empty() || trimmed.ends_with(':') {
        key
    } else {
        trimmed.to_string()
    }
}

/// Whatever could be parsed. Missing, unreadable and malformed all read as
/// an empty table.
pub(super) fn read_table(path: Option<&Path>) -> Table {
    let Some(path) = path else {
        return Table::new();
    };
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Table::new(),
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "could not read; using defaults");
            return Table::new();
        }
    };
    match text.parse::<Table>() {
        Ok(table) => table,
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "could not parse; using defaults");
            Table::new()
        }
    }
}

/// Writes through a neighbouring temporary file and renames over the
/// target: a write cut short parses into different values (a truncated
/// `180` reads as `18`), which per-key tolerance cannot catch.
pub(super) fn write_atomically(path: &Path, text: &str) -> Result<(), StoreError> {
    let failed = |source: std::io::Error| StoreError::Write {
        path: path.to_path_buf(),
        source,
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(failed)?;
    }
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let tmp = path.with_file_name(format!("{name}.{}.tmp", std::process::id()));

    let write = (|| -> std::io::Result<()> {
        use std::io::Write;
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()
    })();
    if let Err(error) = write {
        remove_leftover(&tmp);
        return Err(failed(error));
    }

    // If this fails (on Windows something else may hold the file open),
    // the old file stays intact — losing the newest layout beats losing
    // the file.
    if let Err(error) = std::fs::rename(&tmp, path) {
        remove_leftover(&tmp);
        return Err(failed(error));
    }
    Ok(())
}

fn remove_leftover(tmp: &Path) {
    if let Err(error) = std::fs::remove_file(tmp) {
        tracing::debug!(path = %tmp.display(), %error, "settings temp file not removed");
    }
}

pub(super) fn sub_table<'a>(table: &'a Table, key: &str) -> Option<&'a Table> {
    table.get(key).and_then(Value::as_table)
}

pub(super) fn flag(table: &Table, key: &str, fallback: bool) -> bool {
    table.get(key).and_then(Value::as_bool).unwrap_or(fallback)
}

/// An integer that has to make sense. Out-of-range is treated exactly like
/// the wrong type: a saved pane width of zero is as unusable as `"wide"`.
pub(super) fn int_in(
    table: &Table,
    key: &str,
    range: std::ops::RangeInclusive<i64>,
    fallback: i32,
) -> i32 {
    table
        .get(key)
        .and_then(Value::as_integer)
        .filter(|v| range.contains(v))
        .and_then(|v| i32::try_from(v).ok())
        .unwrap_or(fallback)
}

/// A column width inside the graph: `AUTO_WIDTH` for "nobody moved this
/// one", or a width wide enough to be one. Anything between falls back
/// like a wrong type.
pub(super) fn width_or_auto(table: &Table, key: &str) -> i32 {
    /// Narrower than this and the column cannot hold one lane or a chip.
    const NARROWEST: i64 = 24;
    table
        .get(key)
        .and_then(Value::as_integer)
        .filter(|v| *v == i64::from(AUTO_WIDTH) || (NARROWEST..=4_000).contains(v))
        .and_then(|v| i32::try_from(v).ok())
        .unwrap_or(AUTO_WIDTH)
}

/// A window coordinate, which may legitimately be negative (a second
/// monitor to the left) but not absurd.
pub(super) fn coord(table: &Table, key: &str) -> Option<i32> {
    table
        .get(key)
        .and_then(Value::as_integer)
        .filter(|v| (-32_000..=32_000).contains(v))
        .and_then(|v| i32::try_from(v).ok())
}

/// An interval in minutes. A number past the ceiling is clamped by
/// `session::auto_fetch_minutes`, so the file and the settings screen
/// cannot decide it differently.
pub(super) fn minutes(table: &Table, key: &str) -> Option<u32> {
    table
        .get(key)
        .and_then(Value::as_integer)
        .filter(|v| *v >= 0)
        // Saturating: falling back to the default would answer the most
        // aggressive interval to the value that asked for the least.
        .map(|v| u32::try_from(v).unwrap_or(u32::MAX))
        .map(crate::session::auto_fetch_minutes)
}

/// The commits a graph opens with: `0` is the whole history, and anything
/// else is at least the floor `session::log_limit` puts under it.
///
/// The outer `Option` is "the key said nothing usable" (→ default), the
/// inner one the answer "no window at all". Flattened, a hand-written
/// `initial_commits = 0` would mean the default.
pub(super) fn initial_commits(table: &Table, key: &str) -> Option<Option<u32>> {
    table
        .get(key)
        .and_then(Value::as_integer)
        .filter(|v| *v >= 0)
        .map(|v| match u32::try_from(v) {
            Ok(0) => None,
            Ok(count) => Some(crate::session::log_limit(count)),
            // Saturating, like `minutes`: the largest count is nearer to
            // what was asked than the default is.
            Err(_) => Some(u32::MAX),
        })
}

/// A written-down path, trimmed. An empty string is an answer ("whatever
/// `PATH` resolves") and survives the read; only a non-string falls back.
pub(super) fn text(table: &Table, key: &str) -> Option<String> {
    table
        .get(key)
        .and_then(Value::as_str)
        .map(|s| s.trim().to_string())
}

/// How many git processes run at once. A number outside the range is
/// clamped by `process::concurrency`, like `minutes`.
pub(super) fn concurrency(table: &Table, key: &str) -> Option<u32> {
    table
        .get(key)
        .and_then(Value::as_integer)
        .filter(|v| *v >= 0)
        .map(|v| u32::try_from(v).unwrap_or(u32::MAX))
        .map(crate::process::concurrency)
}

/// Seconds between passes over the other working copies: `0` is off,
/// anything else clamped by `session::copies_interval_secs`, like
/// `minutes`.
pub(super) fn copies_interval(table: &Table, key: &str) -> Option<u32> {
    table
        .get(key)
        .and_then(Value::as_integer)
        .filter(|v| *v >= 0)
        .map(|v| u32::try_from(v).unwrap_or(u32::MAX))
        .map(crate::session::copies_interval_secs)
}

pub(super) fn timeout_secs(table: &Table, key: &str) -> Option<u64> {
    table
        .get(key)
        .and_then(Value::as_integer)
        .filter(|v| (1..=86_400).contains(v))
        .and_then(|v| u64::try_from(v).ok())
}

pub(super) fn clamp_to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::testkit::dir_store;
    use crate::settings::{STATE_FILE, Settings, State, TabRecord, TabsState};

    /// One tab standing in the repository's own working copy.
    fn one_tab(path: &str) -> TabsState {
        TabsState {
            tabs: vec![TabRecord {
                path: path.to_string(),
                repo: path.to_string(),
            }],
            active: 0,
        }
    }

    #[test]
    fn windows_paths_are_written_without_escaping() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = dir_store(dir.path());
        let state = State {
            tabs: one_tab(r"C:\Users\me\proj"),
            ..State::default()
        };
        store.save_state(&state).expect("save");
        let text = std::fs::read_to_string(dir.path().join(STATE_FILE)).expect("read");
        assert!(
            text.contains("\"C:/Users/me/proj\""),
            "separators as git writes them:\n{text}"
        );
        assert!(
            !text.contains('\\'),
            "nothing in the file needs an escape:\n{text}"
        );
        assert_eq!(
            store
                .load_state()
                .tabs
                .tabs
                .iter()
                .map(|tab| tab.path.clone())
                .collect::<Vec<_>>(),
            vec!["C:/Users/me/proj".to_string()],
            "and what comes back is what a repository is named elsewhere"
        );
    }

    /// Saving normalises paths, so callers hand over [`repo_key`] output.
    /// Pins that one round trip settles a raw path, so a held state
    /// compared against the file does not differ every time.
    #[test]
    fn a_raw_path_settles_after_one_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = dir_store(dir.path());
        let raw = State {
            tabs: one_tab(r"C:\Users\me\proj"),
            ..State::default()
        };
        store.save_state(&raw).expect("save");
        let settled = store.load_state();
        assert_ne!(settled, raw, "the separators moved");

        store.save_state(&settled).expect("save again");
        assert_eq!(store.load_state(), settled, "and then stop moving");
    }

    #[test]
    fn an_extended_length_path_keeps_its_backslashes() {
        let raw = r"\\?\C:\Users\me\proj";
        assert_eq!(repo_key(raw), raw);
    }

    #[test]
    fn nonsense_file_starts_from_defaults() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(STATE_FILE), "this is not toml {{{").expect("write");
        assert_eq!(dir_store(dir.path()).load_state(), State::default());
    }

    #[test]
    fn missing_files_start_from_defaults() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = dir_store(&dir.path().join("never-created"));
        assert_eq!(store.load_state(), State::default());
        assert_eq!(store.load_settings(), Settings::default());
    }

    #[test]
    fn a_replaced_file_never_appears_half_written() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = dir_store(dir.path());
        store.save_state(&State::default()).expect("first");

        let mut state = State::default();
        state.layout.details_width = 999;
        store.save_state(&state).expect("second");

        let left_behind: Vec<_> = std::fs::read_dir(dir.path())
            .expect("read_dir")
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".tmp"))
            .collect();
        assert!(
            left_behind.is_empty(),
            "no scratch left over: {left_behind:?}"
        );
        assert_eq!(store.load_state().layout.details_width, 999);
    }
}
