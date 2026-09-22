//! The repeat guard: the same shell line, asked again while nothing has
//! answered it.
//!
//! A line and its output stay in the conversation, and every call after
//! them reads that conversation back whole — so a wait spelled as `tail`
//! or `grep -c` every few seconds is charged the entire context per
//! tick, for an answer the harness hands over by itself the moment the
//! work finishes. Four identical lines inside ten minutes is the shape
//! of a wait, not of work.
//!
//! Cargo is let through whatever it does: a build, a test and a verb
//! cost minutes and wait for themselves, which is the one thing a poll
//! never does.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::payload::{deny, string_field};

/// How far back earlier runs of the same line are counted.
const WINDOW: u64 = 10 * 60;

/// How many identical lines inside the window are still work. The one
/// after them is held.
const ALLOWED: usize = 3;

/// What the held line is told: the two ways of waiting that cost one
/// call instead of one per tick.
const WAITING: &str = "Three of these have already run inside ten minutes, which is a wait, \
     not work. A line and its output stay in the conversation and every call after them reads \
     it back whole, so a wait spelled as a shell line is charged the context in full, per \
     tick. Let the work say when it is done instead: start it with run_in_background and the \
     notification comes when it exits, or hand Monitor the condition to watch. Asking the \
     same question in other words is the same wait. Cargo is not held this way — a build or a \
     verb waits for itself.";

/// PreToolUse(Bash|PowerShell), last of the shell guards so the ledger
/// holds only the lines the guards before it let through. Answers
/// whether it refused, like `pre_git`.
pub(super) fn pre_shell(input: &str) -> Result<bool, String> {
    let Some(command) = string_field(input, "command") else {
        return Ok(false);
    };
    if super::shell::runs_cargo(&command) {
        return Ok(false);
    }
    let Some(path) = ledger_path(input) else {
        return Ok(false);
    };
    let now = now();
    let asking = fingerprint(&command);
    let recent = within_window(&recorded(&path), now);
    if held(&recent, asking) {
        deny(WAITING);
        return Ok(true);
    }
    record(&path, &recent, now, asking);
    Ok(false)
}

/// SessionEnd: the ledger goes with the session that wrote it.
pub(super) fn session_end(input: &str) {
    let Some(path) = ledger_path(input) else {
        return;
    };
    if let Err(_unheard) = std::fs::remove_file(&path) {
        // Never written, or already gone. Nobody is left to tell.
    }
}

/// Whether this line has already been asked its share of times.
fn held(recent: &[(u64, u64)], asking: u64) -> bool {
    recent.iter().filter(|(_, line)| *line == asking).count() >= ALLOWED
}

/// A line reduced to what makes it the same question: its tokens, in
/// order. Spacing and line breaks are all that separate two spellings of
/// one wait, and the whole line is read, so two long lines that share a
/// prefix stay different questions.
fn fingerprint(command: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for token in command.split_whitespace() {
        for byte in token.bytes().chain(std::iter::once(b' ')) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

fn within_window(runs: &[(u64, u64)], now: u64) -> Vec<(u64, u64)> {
    runs.iter()
        .copied()
        .filter(|(at, _)| now.saturating_sub(*at) < WINDOW)
        .collect()
}

/// This session's ledger: `.repeats/<session>.tsv` beside the primary
/// checkout, as the chip ledger is — a session types in the checkout
/// while it is being handed a seat and in the seat afterwards, and a
/// ledger that moved with it would leave the first half behind for
/// nobody to delete (`tree::primary_root`).
fn ledger_path(input: &str) -> Option<PathBuf> {
    let session: String = string_field(input, "session_id")?
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if session.is_empty() {
        return None;
    }
    let root = crate::tree::primary_root(&string_field(input, "cwd")?)?;
    Some(root.join(".repeats").join(format!("{session}.tsv")))
}

/// Field separator inside a ledger line. Both fields are numbers, so the
/// format has no escape rules to get wrong (chips/ledger.rs).
const SEP: char = '\t';

fn recorded(path: &Path) -> Vec<(u64, u64)> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines().filter_map(parse_run).collect()
}

fn parse_run(line: &str) -> Option<(u64, u64)> {
    let (at, asked) = line.split_once(SEP)?;
    Some((at.parse().ok()?, asked.parse().ok()?))
}

/// Writes the window back with this line appended. It is advisory: a
/// session whose ledger cannot be written keeps working, and the guard
/// simply sees no earlier runs.
fn record(path: &Path, recent: &[(u64, u64)], now: u64, asking: u64) {
    let Some(dir) = path.parent() else {
        return;
    };
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let text: String = recent
        .iter()
        .chain(std::iter::once(&(now, asking)))
        .map(|(at, line)| format!("{at}{SEP}{line}\n"))
        .collect();
    if let Err(_unheard) = std::fs::write(path, text) {
        // Nobody to tell from inside a hook; the guard falls back to
        // judging whatever the file still holds.
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

#[cfg(test)]
mod tests {
    use super::{ALLOWED, WINDOW, fingerprint, held, record, recorded, within_window};

    /// The count survives the file it is kept in: the guard reads the
    /// window back on every line, from a fresh process each time.
    #[test]
    fn a_ledger_written_and_read_back_holds_the_fourth_ask() {
        let dir = crate::verify::claim_dir(&std::env::temp_dir().join("pgg-hook"), "repeats")
            .expect("a directory of its own");
        let path = dir.join(".repeats").join("session.tsv");
        let asking = fingerprint("tail -2 target/gate.log");
        let mut now = 1_000;
        for _ in 0..ALLOWED {
            let recent = within_window(&recorded(&path), now);
            assert!(!held(&recent, asking));
            record(&path, &recent, now, asking);
            now += 5;
        }
        assert!(held(&within_window(&recorded(&path), now), asking));
        // A window later the rows on file have aged out of it.
        assert!(!held(
            &within_window(&recorded(&path), now + WINDOW),
            asking
        ));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn one_wait_spelled_two_ways_is_one_question() {
        assert_eq!(
            fingerprint("tail -2  target/gate.log"),
            fingerprint("tail -2 target/gate.log\n")
        );
        assert_ne!(
            fingerprint("tail -2 target/gate.log"),
            fingerprint("tail -3 target/gate.log")
        );
        // Two long lines that share everything but their tail: the whole
        // line is read, so the ledger does not confuse them.
        assert_ne!(
            fingerprint("cargo xtask verify-ui nav-jump tag:0 --preset worktrees --no-board"),
            fingerprint("cargo xtask verify-ui nav-jump tag:1 --preset worktrees --no-board")
        );
    }

    #[test]
    fn the_fourth_identical_line_is_held_and_the_third_is_not() {
        let asking = fingerprint("grep -c '' target/gate.log");
        let other = fingerprint("git status --short");
        let runs: Vec<(u64, u64)> = (0..ALLOWED).map(|n| (100 + n as u64, asking)).collect();
        assert!(held(&runs, asking));
        assert!(!held(&runs[1..], asking));
        assert!(!held(&runs, other));
    }

    #[test]
    fn runs_older_than_the_window_stop_counting() {
        let asking = fingerprint("date");
        let runs = [(100, asking), (200, asking), (300, asking)];
        let now = 300 + WINDOW - 1;
        // Only the last is still inside the window, so a fourth ask a
        // window later is a first ask.
        assert_eq!(within_window(&runs, now).len(), 1);
        assert!(!held(&within_window(&runs, now), asking));
    }
}
