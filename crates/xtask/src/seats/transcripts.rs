//! What Claude Code's transcripts say of where its sessions stand: one
//! file per session under `projects/<place>/` of its config directory, a
//! JSON line per event, each carrying the session's working directory and
//! its time. The layout is Claude Code's own, and a transcript that cannot
//! be read says nothing.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// How much of a transcript's end is read for its last event: a line
/// carries a tool's output only up to the size Claude Code persists
/// elsewhere, so the last event that names a directory is well inside.
const TAIL: u64 = 64 * 1024;

/// Where a transcript last put its session, and when.
#[derive(Debug, PartialEq)]
pub(super) struct Stand {
    /// The session's working directory, as Claude Code spelled it.
    pub cwd: String,
    pub at: SystemTime,
}

/// Where `session` stands now, by the last event its transcripts hold —
/// the session's own directory, wherever the shell it runs a command from
/// was sent (`cd <checkout> && cargo xtask seat`).
pub(super) fn stand_of(session: &str) -> Option<Stand> {
    transcripts(session)
        .iter()
        .filter_map(|path| last_stand(path))
        .max_by_key(|stand| stand.at)
}

/// Where each session whose transcript holds an event after `since` last
/// stood. A file's time runs ahead of what it holds — Claude Code rewrites
/// transcripts in bulk — and never behind, so it only rules files out; the
/// time a stand is given comes from the lines.
pub(super) fn stands_since(since: SystemTime) -> Vec<Stand> {
    let Some(places) = projects().and_then(|projects| std::fs::read_dir(projects).ok()) else {
        return Vec::new();
    };
    places
        .flatten()
        .filter_map(|place| std::fs::read_dir(place.path()).ok())
        .flatten()
        .flatten()
        .filter(|file| file.path().extension().is_some_and(|ext| ext == "jsonl"))
        .filter(|file| {
            file.metadata()
                .and_then(|meta| meta.modified())
                .is_ok_and(|written| written > since)
        })
        .filter_map(|file| last_stand(&file.path()))
        .filter(|stand| stand.at > since)
        .collect()
}

/// The last working directory and the last time a transcript's end holds.
fn last_stand(path: &Path) -> Option<Stand> {
    let mut file = std::fs::File::open(path).ok()?;
    let length = file.metadata().ok()?.len();
    file.seek(SeekFrom::Start(length.saturating_sub(TAIL)))
        .ok()?;
    let mut tail = Vec::new();
    file.read_to_end(&mut tail).ok()?;
    let text = String::from_utf8_lossy(&tail);
    Some(Stand {
        // JSON doubles a Windows path's backslashes; a path holds no quote.
        cwd: last_value(&text, "cwd")?.replace("\\\\", "\\"),
        at: utc(last_value(&text, "timestamp")?)?,
    })
}

/// The last `"key":"value"` in `text`.
fn last_value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\":\"");
    let rest = &text[text.rfind(&needle)? + needle.len()..];
    rest.split('"').next()
}

/// A time as Claude Code writes it, in UTC: `2026-10-13T14:20:52.449Z`.
fn utc(text: &str) -> Option<SystemTime> {
    let (date, time) = text.strip_suffix('Z')?.split_once('T')?;
    let mut date = date.split('-').map(str::parse::<i64>);
    let (year, month, day) = (date.next()?.ok()?, date.next()?.ok()?, date.next()?.ok()?);
    let mut time = time.split(':');
    let (hour, minute) = (
        time.next()?.parse::<u64>().ok()?,
        time.next()?.parse::<u64>().ok()?,
    );
    let seconds = time.next()?.parse::<f64>().ok()?;
    // Days since 1970-01-01 of a proleptic Gregorian date (H. Hinnant's
    // days_from_civil), counting the year from March.
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let days = era * 146_097 + of_era * 365 + of_era / 4 - of_era / 100 + of_year - 719_468;
    let whole = u64::try_from(days).ok()? * 86_400 + hour * 3_600 + minute * 60;
    Some(
        SystemTime::UNIX_EPOCH
            + Duration::from_secs(whole)
            + Duration::try_from_secs_f64(seconds).ok()?,
    )
}

/// Every transcript file `session` has: one in the directory of each place
/// it worked from, since a session that enters a seat can go on writing
/// beside where it started.
pub(super) fn transcripts(session: &str) -> Vec<PathBuf> {
    projects().map_or_else(Vec::new, |projects| transcripts_in(&projects, session))
}

fn transcripts_in(projects: &Path, session: &str) -> Vec<PathBuf> {
    // The id becomes a file name.
    if session.is_empty()
        || !session
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Vec::new();
    }
    let name = format!("{session}.jsonl");
    let Ok(places) = std::fs::read_dir(projects) else {
        return Vec::new();
    };
    places
        .flatten()
        .map(|place| place.path().join(&name))
        .filter(|path| path.is_file())
        .collect()
}

/// Where Claude Code keeps transcripts: `projects/` under its config
/// directory, one directory per place a session started or worked from.
fn projects() -> Option<PathBuf> {
    std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
                .map(|home| PathBuf::from(home).join(".claude"))
        })
        .map(|config| config.join("projects"))
}

#[cfg(test)]
mod tests {
    use super::{Stand, last_stand, transcripts_in, utc};
    use std::time::{Duration, SystemTime};

    #[test]
    fn reads_the_times_claude_code_writes() {
        let at = |secs: u64, millis: u64| {
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(secs) + Duration::from_millis(millis))
        };
        assert_eq!(utc("1970-01-01T00:00:00.000Z"), at(0, 0));
        assert_eq!(utc("2026-10-13T14:20:52.449Z"), at(1_791_901_252, 449));
        // A leap day, and the turn of a year.
        assert_eq!(utc("2024-02-29T00:00:00.000Z"), at(1_709_164_800, 0));
        assert_eq!(utc("2025-12-31T23:59:59.000Z"), at(1_767_225_599, 0));
        assert_eq!(utc("2026-10-13 14:20:52"), None);
        assert_eq!(utc("not a time"), None);
    }

    /// The end of a transcript as Claude Code leaves it: events with a
    /// directory and a time, then the app's own lines with neither.
    #[test]
    fn a_transcript_says_where_its_session_last_stood() {
        let yard = crate::yard::Yard::new("transcripts");
        let place = yard.join("projects").join("repo");
        std::fs::create_dir_all(&place).unwrap();
        let lines = [
            r#"{"type":"user","cwd":"C:\\x\\repo","timestamp":"2026-10-13T01:00:00.000Z","sessionId":"s1"}"#,
            r#"{"type":"assistant","message":{"content":"said \"cwd\":\"elsewhere\""},"cwd":"C:\\x\\repo\\.claude\\worktrees\\b","timestamp":"2026-10-13T02:00:00.000Z","sessionId":"s1"}"#,
            r#"{"type":"last-prompt","lastPrompt":"go on","sessionId":"s1"}"#,
        ];
        let path = place.join("s1.jsonl");
        std::fs::write(&path, lines.join("\n") + "\n").unwrap();
        assert_eq!(
            last_stand(&path),
            Some(Stand {
                cwd: r"C:\x\repo\.claude\worktrees\b".to_string(),
                at: utc("2026-10-13T02:00:00.000Z").unwrap(),
            })
        );
        assert_eq!(transcripts_in(&yard.join("projects"), "s1"), [path]);
        assert!(transcripts_in(&yard.join("projects"), "nobody").is_empty());
        assert!(transcripts_in(&yard.join("projects"), "../repo/s1").is_empty());
    }
}
