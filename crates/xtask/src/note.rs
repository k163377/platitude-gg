//! The small text notes this runner leaves for itself — a hold beside
//! `.git`, a build announced, what a measurement warmed — in one shape:
//! `key value` lines and a clock in whole seconds since the epoch. Two
//! writers (`still`, `perf::warmth`) and one reader for both, so a note
//! read back is parsed the way it was written.

use std::time::{SystemTime, UNIX_EPOCH};

/// Whole seconds since the epoch, 0 on a clock set before it.
pub(crate) fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// The value after `key` on the line that starts with it, trimmed.
/// `key` carries its own trailing space: `field(text, "pid ")`.
pub(crate) fn field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines()
        .find_map(|line| line.strip_prefix(key))
        .map(str::trim)
}

#[cfg(test)]
mod tests {
    use super::{field, now_secs};

    #[test]
    fn a_field_is_the_rest_of_its_line() {
        let text = "pid 7\nsince 11\nwhat cargo xtask perf\n";
        assert_eq!(field(text, "pid "), Some("7"));
        assert_eq!(field(text, "what "), Some("cargo xtask perf"));
        assert_eq!(field(text, "where "), None);
        // The key is matched at the start of a line, not anywhere in it.
        assert_eq!(field("what pid 9\n", "pid "), None);
    }

    #[test]
    fn the_clock_runs_forward() {
        assert!(now_secs() > 1_700_000_000);
    }
}
