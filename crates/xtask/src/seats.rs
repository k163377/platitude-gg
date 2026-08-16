//! The worktree seat survey (`cargo xtask seats`), shared with the
//! session-start greeting.
//!
//! The greeting reports where the seats stood when the session began, and
//! that snapshot goes stale: a seat it called free was measured minutes
//! later holding another session's 13 uncommitted files (2026-08-11).
//! This module is the one place a seat is measured, so the command and
//! the greeting cannot drift apart — and the command is the live answer
//! to read before entering a seat (CLAUDE.md ビルド・テスト).

use std::time::{Duration, SystemTime};

/// The reusable worktree seats. Sessions rotate through these six instead
/// of minting a name per topic — a topical worktree is never reused, so
/// every one paid a cold target/ build and kept the gigabytes afterwards
/// (CLAUDE.md ビルド・テスト).
pub(crate) const SEATS: [&str; 6] = ["a", "b", "c", "d", "e", "f"];

/// The mark a session's seat claim carries in `git worktree lock`'s
/// reason, followed by the session id. The entry hooks and the post-write
/// re-claim write it; `land` and the session-end hook release it. A lock
/// without this mark is a person's, and nothing automatic touches it.
pub(crate) const SEAT_CLAIM: &str = "claude-seat";

/// The directory every worktree of this repository sits under.
const WORKTREES: &str = "/.claude/worktrees/";

/// One seat of the roster, surveyed.
pub(crate) struct Seat {
    pub name: &'static str,
    /// None while the seat's worktree has not been created.
    pub state: Option<SeatState>,
}

/// What a created seat holds right now.
pub(crate) struct SeatState {
    /// Branch name, empty while HEAD is detached — a rebase in flight
    /// detaches it, so an active session can read as branchless.
    pub branch: String,
    /// Whether `git worktree lock` holds it — a session's claim (the
    /// entry hooks write one; manual locks land here too).
    pub locked: bool,
    /// The lock's reason, empty when unlocked or given none.
    pub lock_reason: String,
    /// Commits main does not have (`main..HEAD`); None when git could not
    /// answer. Ranges run on HEAD, not the branch name, so a detached
    /// seat still counts.
    pub ahead: Option<u32>,
    /// The reverse (`HEAD..main`); zero of each means HEAD is main's tip.
    pub behind: Option<u32>,
    /// Lines of `status --porcelain`: every uncommitted change, untracked
    /// files included. None when git could not answer.
    pub dirty: Option<usize>,
    /// Time since the seat's own index was last written. Most git run in
    /// the seat refreshes it, so a fresh age means hands on the seat
    /// recently, whatever the counted columns say.
    pub index_age: Option<Duration>,
}

/// `cargo xtask seats`: one line per seat, and how to read them.
pub fn run(args: &[String]) -> Result<(), String> {
    if !args.is_empty() {
        return Err(format!("seats takes no arguments (got {args:?})"));
    }
    let root = crate::workspace_root();
    let seats = survey(&root.to_string_lossy())
        .ok_or("git worktree list failed — is git on PATH and this a repository?")?;
    print!("{}", render(&seats));
    Ok(())
}

/// Every roster seat, measured now. None when `git worktree list` itself
/// fails (not a repository). The survey only reads: status runs under
/// --no-optional-locks, because a plain status opportunistically rewrites
/// the index it refreshed, and that write would stamp the very index
/// mtimes this survey reports with the survey's own run.
pub(crate) fn survey(cwd: &str) -> Option<Vec<Seat>> {
    let listing = crate::git_query(cwd, &["worktree", "list", "--porcelain"])?;
    let entries = seat_entries(&listing);
    let now = SystemTime::now();
    // One thread per seat: the greeting takes this survey on every session
    // start, and six seats of sequential subprocess batches are the
    // difference between a beat and a second.
    Some(std::thread::scope(|scope| {
        let handles = SEATS.map(|name| {
            let entry = entries.iter().find(|entry| entry.seat == name);
            scope.spawn(move || entry.map(|entry| seat_state(entry, now)))
        });
        SEATS
            .into_iter()
            .zip(handles)
            .map(|(name, handle)| Seat {
                name,
                state: handle.join().unwrap_or(None),
            })
            .collect()
    }))
}

/// Measures one created seat. Each figure is None when its git call
/// fails, and the callers print those as unknowns rather than guess.
fn seat_state(entry: &SeatEntry, now: SystemTime) -> SeatState {
    let dir = entry.tree.path.as_str();
    SeatState {
        branch: entry.tree.branch.clone(),
        locked: entry.tree.locked,
        lock_reason: entry.tree.reason.clone(),
        ahead: commits_in(dir, "main..HEAD"),
        behind: commits_in(dir, "HEAD..main"),
        dirty: crate::git_query(dir, &["--no-optional-locks", "status", "--porcelain"])
            .map(|status| status.lines().filter(|line| !line.is_empty()).count()),
        index_age: index_age(dir, now),
    }
}

/// How long since the seat's own index was written. The path has to be
/// asked for: a worktree's admin directory is named after the directory
/// the tree was first created as, not after the seat — one seat here
/// sits on .git/worktrees/skillcare — so .git/worktrees/<seat>/index is
/// a guess that misses (measured 2026-08-11).
fn index_age(dir: &str, now: SystemTime) -> Option<Duration> {
    let index = crate::git_query(
        dir,
        &["rev-parse", "--path-format=absolute", "--git-path", "index"],
    )?;
    let written = std::fs::metadata(index).ok()?.modified().ok()?;
    Some(now.duration_since(written).unwrap_or(Duration::ZERO))
}

/// How many commits `git rev-list --count` sees in `range`, run in `dir`.
pub(crate) fn commits_in(dir: &str, range: &str) -> Option<u32> {
    crate::git_query(dir, &["rev-list", "--count", range]).and_then(|count| count.parse().ok())
}

/// One roster seat as `git worktree list --porcelain` shows it: the
/// listing's own block, and the roster letter its path names.
#[derive(Debug, PartialEq)]
pub(crate) struct SeatEntry {
    pub seat: &'static str,
    pub tree: WorktreeBlock,
}

/// One tree of `git worktree list --porcelain`, as the listing gives it.
///
/// Nothing is filtered and the order is the listing's, because the first
/// entry is the primary checkout and `land` turns on that.
#[derive(Debug, PartialEq)]
pub(crate) struct WorktreeBlock {
    /// The worktree's path, slashes forward, for `git -C`.
    pub path: String,
    /// Branch name, empty when HEAD is detached.
    pub branch: String,
    pub locked: bool,
    /// The lock's reason, empty when unlocked or given none.
    pub reason: String,
}

/// Every tree the listing shows, in listing order.
pub(crate) fn worktree_blocks(listing: &str) -> Vec<WorktreeBlock> {
    let mut blocks = Vec::new();
    for block in listing.split("\n\n") {
        let mut path = None;
        let mut branch = String::new();
        let mut locked = false;
        let mut reason = String::new();
        for line in block.lines() {
            if let Some(rest) = line.strip_prefix("worktree ") {
                path = Some(rest.replace('\\', "/"));
            } else if let Some(rest) = line.strip_prefix("branch refs/heads/") {
                branch = rest.to_string();
            } else if let Some(rest) = line.strip_prefix("locked") {
                locked = true;
                reason = rest.trim().to_string();
            }
        }
        if let Some(path) = path {
            blocks.push(WorktreeBlock {
                path,
                branch,
                locked,
                reason,
            });
        }
    }
    blocks
}

/// Every roster seat the listing shows, in listing order.
pub(crate) fn seat_entries(listing: &str) -> Vec<SeatEntry> {
    worktree_blocks(listing)
        .into_iter()
        .filter_map(|block| {
            let seat = worktree_root(&block.path).and_then(|root| {
                let name = root.rsplit('/').next()?.to_string();
                SEATS.iter().find(|seat| **seat == name).copied()
            })?;
            Some(SeatEntry { seat, tree: block })
        })
        .collect()
}

/// The worktree `cwd` sits in: the path down to the directory named under
/// .claude/worktrees/, and None for the primary checkout.
pub(crate) fn worktree_root(cwd: &str) -> Option<String> {
    let cwd = cwd.replace('\\', "/");
    let at = cwd.find(WORKTREES)? + WORKTREES.len();
    if at >= cwd.len() {
        return None;
    }
    let end = cwd[at..].find('/').map_or(cwd.len(), |slash| at + slash);
    Some(cwd[..end].to_string())
}

/// The reading, one line, the way CLAUDE.md ビルド・テスト has it.
const GUIDE: &str = "reading: dirty>0 or ahead>0 = in use; dirty=0 and ahead=0 = free \
    (start with `git reset --hard main` unless at-main is yes); a locked seat is held \
    by the session that locked it.";

/// The table: a header, one line per seat, and the reading. Pure so the
/// tests can hand it seats git never made.
fn render(seats: &[Seat]) -> String {
    let mut rows = vec![[
        "seat".to_string(),
        "branch".to_string(),
        "at-main".to_string(),
        "ahead".to_string(),
        "dirty".to_string(),
        "index-age".to_string(),
    ]];
    let mut notes = vec![String::new()];
    for seat in seats {
        let (row, note) = seat_row(seat);
        rows.push(row);
        notes.push(note);
    }
    let widths: [usize; 6] =
        std::array::from_fn(|column| rows.iter().map(|row| row[column].len()).max().unwrap_or(0));
    let mut out = String::new();
    for (row, note) in rows.iter().zip(&notes) {
        let mut line = String::new();
        for (cell, width) in row.iter().zip(widths) {
            line.push_str(&format!("{cell:<width$}  "));
        }
        out.push_str(line.trim_end());
        if !note.is_empty() {
            out.push_str("  ");
            out.push_str(note);
        }
        out.push('\n');
    }
    out.push_str(GUIDE);
    out.push('\n');
    out
}

/// One seat's cells, and the note appended past the columns ("locked").
fn seat_row(seat: &Seat) -> ([String; 6], String) {
    let name = seat.name.to_string();
    let Some(state) = &seat.state else {
        return (
            [
                name,
                "(not created)".to_string(),
                "-".to_string(),
                "-".to_string(),
                "-".to_string(),
                "-".to_string(),
            ],
            String::new(),
        );
    };
    let branch = if state.branch.is_empty() {
        "(detached)".to_string()
    } else {
        state.branch.clone()
    };
    let at_main = match (state.ahead, state.behind) {
        (Some(0), Some(0)) => "yes",
        (Some(_), Some(_)) => "no",
        _ => "?",
    };
    let count = |value: Option<u32>| value.map_or("?".to_string(), |value| value.to_string());
    let dirty = state
        .dirty
        .map_or("?".to_string(), |value| value.to_string());
    let note = match (state.locked, state.lock_reason.is_empty()) {
        (false, _) => String::new(),
        (true, true) => "locked".to_string(),
        (true, false) => format!("locked ({})", state.lock_reason),
    };
    (
        [
            name,
            branch,
            at_main.to_string(),
            count(state.ahead),
            dirty,
            format_age(state.index_age),
        ],
        note.to_string(),
    )
}

/// An age as the shortest round figure that still ranks seats: seconds
/// under a minute, then minutes, hours, days.
pub(crate) fn format_age(age: Option<Duration>) -> String {
    let Some(age) = age else {
        return "?".to_string();
    };
    let seconds = age.as_secs();
    match seconds {
        0..60 => format!("{seconds}s"),
        60..3600 => format!("{}m", seconds / 60),
        3600..86400 => format!("{}h", seconds / 3600),
        _ => format!("{}d", seconds / 86400),
    }
}

#[cfg(test)]
mod tests {
    use super::{Seat, SeatEntry, SeatState, WorktreeBlock, format_age, render, seat_entries};
    use std::time::Duration;

    #[test]
    fn reads_seats_out_of_a_worktree_listing() {
        let listing = "worktree C:/x/platitude-gg\nHEAD 1111\nbranch refs/heads/main\n\n\
                       worktree C:/x/platitude-gg/.claude/worktrees/a\nHEAD 2222\nbranch refs/heads/worktree-a\n\n\
                       worktree C:/x/platitude-gg/.claude/worktrees/tooltip\nHEAD 3333\nbranch refs/heads/worktree-tooltip\n\n\
                       worktree C:/x/platitude-gg/.claude/worktrees/b\nHEAD 4444\nbranch refs/heads/worktree-b\nlocked claude session b (pid 1)\n";
        let seats = seat_entries(listing);
        assert_eq!(
            seats,
            vec![
                SeatEntry {
                    seat: "a",
                    tree: WorktreeBlock {
                        path: "C:/x/platitude-gg/.claude/worktrees/a".to_string(),
                        branch: "worktree-a".to_string(),
                        locked: false,
                        reason: String::new(),
                    },
                },
                SeatEntry {
                    seat: "b",
                    tree: WorktreeBlock {
                        path: "C:/x/platitude-gg/.claude/worktrees/b".to_string(),
                        branch: "worktree-b".to_string(),
                        locked: true,
                        reason: "claude session b (pid 1)".to_string(),
                    },
                },
            ]
        );
    }

    #[test]
    fn rounds_ages_to_the_rank_that_orders_seats() {
        assert_eq!(format_age(None), "?");
        assert_eq!(format_age(Some(Duration::from_secs(59))), "59s");
        assert_eq!(format_age(Some(Duration::from_secs(60))), "1m");
        assert_eq!(format_age(Some(Duration::from_secs(3 * 3600))), "3h");
        assert_eq!(format_age(Some(Duration::from_secs(9 * 86400))), "9d");
    }

    #[test]
    fn renders_every_kind_of_seat_on_its_own_line() {
        let seats = vec![
            Seat {
                name: "a",
                state: Some(SeatState {
                    branch: "worktree-a".to_string(),
                    locked: false,
                    lock_reason: String::new(),
                    ahead: Some(1),
                    behind: Some(0),
                    dirty: Some(13),
                    index_age: Some(Duration::from_secs(16 * 60)),
                }),
            },
            Seat {
                name: "b",
                state: Some(SeatState {
                    branch: String::new(),
                    locked: true,
                    lock_reason: "claude-seat abc123".to_string(),
                    ahead: Some(0),
                    behind: Some(0),
                    dirty: Some(0),
                    index_age: None,
                }),
            },
            Seat {
                name: "c",
                state: Some(SeatState {
                    branch: "worktree-c".to_string(),
                    locked: false,
                    lock_reason: String::new(),
                    ahead: None,
                    behind: None,
                    dirty: None,
                    index_age: None,
                }),
            },
            Seat {
                name: "d",
                state: None,
            },
        ];
        let table = render(&seats);
        let lines: Vec<&str> = table.lines().collect();
        assert_eq!(lines.len(), 6, "{table}");
        assert!(lines[0].starts_with("seat  branch"), "{table}");
        assert!(
            lines[1].contains("worktree-a")
                && lines[1].contains("no")
                && lines[1].contains("13")
                && lines[1].contains("16m"),
            "{table}"
        );
        assert!(
            lines[2].contains("(detached)")
                && lines[2].contains("yes")
                && lines[2].ends_with("locked (claude-seat abc123)"),
            "{table}"
        );
        assert!(lines[3].contains('?'), "{table}");
        assert!(lines[4].contains("(not created)"), "{table}");
        assert!(lines[5].starts_with("reading:"), "{table}");
    }
}
