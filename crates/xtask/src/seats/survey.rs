//! The roster measured and read (`cargo xtask seats`, the greeting's
//! survey): what each letter's tree holds now, one line per seat, and
//! what each letter is refusing on when the roster has nothing left to
//! give. Moving claims stays in `seats`.

use std::time::{Duration, SystemTime};

use super::{
    SEATS, SeatEntry, WorktreeBlock, commands, commits_in, dirty_lines, has_a_directory, rooted,
    seat_entries, whose,
};

/// One seat of the roster, surveyed.
pub(crate) struct Seat {
    pub name: &'static str,
    /// None for a letter with neither a tree nor a branch carrying work
    /// (unused — the roster creates it when it comes to it).
    pub state: Option<SeatState>,
}

pub(crate) struct SeatState {
    /// Empty while HEAD is detached — a rebase in flight does that, so an
    /// active session can read as branchless.
    pub branch: String,
    /// False when the tree went away under its branch: nothing can enter,
    /// land or read the letter until a takeover grows the tree back, and
    /// the claim on the record still stands.
    pub on_disk: bool,
    /// `git worktree lock` — a session's claim, or a manual lock.
    pub locked: bool,
    /// Empty when unlocked or given no reason.
    pub lock_reason: String,
    /// `main..HEAD` (so a detached seat still counts); None when git could
    /// not answer.
    pub ahead: Option<u32>,
    /// `HEAD..main`.
    pub behind: Option<u32>,
    /// Lines of `status --porcelain`, untracked included; None when git
    /// could not answer.
    pub dirty: Option<usize>,
    /// Since the seat's own index was last written. Most git run in the
    /// seat refreshes it, so a fresh age means recent hands on the seat
    /// whatever the other columns say.
    pub index_age: Option<Duration>,
}

/// What each letter is refusing on, for a full roster's refusal.
/// Surveys again rather than reusing the walk's reading: the report
/// should be the roster as it stands after the walk.
pub(super) fn in_the_way(primary: &str) -> String {
    let Some(survey) = survey(primary) else {
        return "  (git could not read the roster — `cargo xtask seats`)".to_string();
    };
    survey
        .iter()
        .map(|seat| format!("  {}", one_letter_refusing(seat)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn one_letter_refusing(seat: &Seat) -> String {
    let name = seat.name;
    let Some(state) = &seat.state else {
        return format!("{name}: no tree, and the roster could not make one");
    };
    if !state.on_disk {
        let held = match state.locked {
            true => format!("{}, and ", whose(&state.lock_reason)),
            false => String::new(),
        };
        return format!(
            "{name}: {held}no tree on disk while worktree-{name} carries {} commit(s) — a \
             takeover grows the tree back on that branch",
            state.ahead.unwrap_or(0)
        );
    }
    if state.locked {
        return format!("{name}: {}", whose(&state.lock_reason));
    }
    match (state.ahead, state.dirty) {
        (Some(ahead), Some(dirty)) if ahead > 0 || dirty > 0 => format!(
            "{name}: unclaimed, carrying {ahead} commit(s) main has not and {dirty} \
             uncommitted file(s)"
        ),
        (Some(_), Some(_)) => {
            format!("{name}: nothing — it was taken between the walk and this line")
        }
        _ => format!("{name}: something git would not answer for"),
    }
}

/// `cargo xtask seats`: one line per seat, and how to read them.
pub fn run(args: &[String]) -> Result<(), String> {
    let (root, args) = rooted(args)?;
    if !args.is_empty() {
        return Err(format!("seats takes no arguments (got {args:?})"));
    }
    let seats =
        survey(&root).ok_or("git worktree list failed — is git on PATH and this a repository?")?;
    print!("{}", render(&seats));
    Ok(())
}

/// Every roster seat, measured now; None when `git worktree list` fails
/// (not a repository). Reads only (`dirty_lines`).
pub(crate) fn survey(cwd: &str) -> Option<Vec<Seat>> {
    let listing = crate::subprocess::git_query(cwd, &["worktree", "list", "--porcelain"])?;
    let entries = seat_entries(&listing);
    let now = SystemTime::now();
    // One thread per seat: every session start waits on this survey, and
    // each seat is a batch of subprocesses.
    Some(std::thread::scope(|scope| {
        let handles = SEATS.map(|name| {
            let entry = entries.iter().find(|entry| entry.seat == name);
            scope.spawn(move || match entry {
                Some(entry) if has_a_directory(&entry.tree.path) => Some(seat_state(entry, now)),
                // Listed by git but gone from disk: its branch is asked
                // from the primary checkout.
                Some(entry) => Some(without_a_tree(
                    Some(&entry.tree),
                    name,
                    commits_in(cwd, &format!("main..worktree-{name}")),
                )),
                // No record either, but its branch carries work: shown,
                // or nobody lands it.
                None => {
                    stranded_work(cwd, name).map(|ahead| without_a_tree(None, name, Some(ahead)))
                }
            })
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

/// A letter whose tree is not on disk, read from the listing's record
/// (branch and claim) when git still has one.
fn without_a_tree(record: Option<&WorktreeBlock>, name: &str, ahead: Option<u32>) -> SeatState {
    SeatState {
        branch: record.map_or_else(
            || format!("worktree-{name}"),
            |record| record.branch.clone(),
        ),
        on_disk: false,
        locked: record.is_some_and(|record| record.locked),
        lock_reason: record
            .map(|record| record.reason.clone())
            .unwrap_or_default(),
        ahead,
        behind: None,
        dirty: None,
        index_age: None,
    }
}

/// What a letter with no tree (removed by hand, or pruned) still carries
/// on its own branch. Read as unused, that work would sit unseen while
/// the roster runs out of letters.
pub(super) fn stranded_work(cwd: &str, seat: &str) -> Option<u32> {
    commits_in(cwd, &format!("main..worktree-{seat}")).filter(|ahead| *ahead > 0)
}

/// Measures one seat whose tree is on disk.
fn seat_state(entry: &SeatEntry, now: SystemTime) -> SeatState {
    let dir = entry.tree.path.as_str();
    SeatState {
        branch: entry.tree.branch.clone(),
        on_disk: true,
        locked: entry.tree.locked,
        lock_reason: entry.tree.reason.clone(),
        ahead: commits_in(dir, "main..HEAD"),
        behind: commits_in(dir, "HEAD..main"),
        dirty: dirty_lines(dir),
        index_age: index_age(dir, now),
    }
}

/// The index path is asked of git: a worktree's admin directory is named
/// after the directory the tree was first created as, so
/// `.git/worktrees/<seat>/index` can miss.
fn index_age(dir: &str, now: SystemTime) -> Option<Duration> {
    let index = crate::subprocess::git_query(
        dir,
        &["rev-parse", "--path-format=absolute", "--git-path", "index"],
    )?;
    let written = std::fs::metadata(index).ok()?.modified().ok()?;
    Some(now.duration_since(written).unwrap_or(Duration::ZERO))
}

/// The reading, one line, the way CLAUDE.md ビルド・テスト has it.
fn guide() -> String {
    format!(
        "reading: a letter is free for somebody else when it is unclaimed with ahead=0 and \
         dirty=0, and `{}` is what takes one — it claims the letter and puts it at main's tip. \
         The tree the asking session is standing in comes back to it whatever it carries, \
         untouched. A claimed seat is its session's until that session lands, hands it back \
         (`{}`) or the user has it taken over (`{}`); no process is asked. A letter with a \
         branch but no tree is work nothing can reach until a takeover grows the tree back.",
        commands::TAKE.line(),
        commands::RELEASE.line(),
        commands::TAKEOVER.line()
    )
}

/// Pure so the tests can hand it seats git never made.
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
    out.push_str(&guide());
    out.push('\n');
    out
}

/// One seat's cells, and the note printed past the columns.
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
    let count = |value: Option<u32>| value.map_or("?".to_string(), |value| value.to_string());
    // A letter with no tree has no working copy: its figures are absent
    // (`-`), not unreadable (`?`), and it must not read "(not created)",
    // which hides its branch's work.
    let (at_main, dirty, age) = match state.on_disk {
        false => ("no".to_string(), "-".to_string(), "-".to_string()),
        true => (
            match (state.ahead, state.behind) {
                (Some(0), Some(0)) => "yes",
                (Some(_), Some(_)) => "no",
                _ => "?",
            }
            .to_string(),
            state
                .dirty
                .map_or("?".to_string(), |value| value.to_string()),
            format_age(state.index_age),
        ),
    };
    let mut note = if state.locked {
        whose(&state.lock_reason)
    } else {
        String::new()
    };
    if !state.on_disk {
        if !note.is_empty() {
            note.push_str("; ");
        }
        note.push_str(&format!(
            "no tree — the branch's work is out of reach until a takeover grows the tree back \
             (`{}`)",
            commands::TAKEOVER.line()
        ));
    }
    (
        [name, branch, at_main, count(state.ahead), dirty, age],
        note,
    )
}

/// The shortest round figure that still ranks seats.
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
    use super::{Seat, SeatState, format_age, one_letter_refusing, render};
    use std::time::Duration;

    fn seat(name: &'static str, state: SeatState) -> Seat {
        Seat {
            name,
            state: Some(state),
        }
    }

    /// A letter with a tree on disk.
    fn surveyed(branch: &str, lock_reason: &str, ahead: u32, dirty: usize) -> SeatState {
        SeatState {
            branch: branch.to_string(),
            on_disk: true,
            locked: !lock_reason.is_empty(),
            lock_reason: lock_reason.to_string(),
            ahead: Some(ahead),
            behind: Some(0),
            dirty: Some(dirty),
            index_age: None,
        }
    }

    /// A letter whose tree went away, as the survey reads it.
    fn treeless(name: &str, lock_reason: &str, ahead: u32) -> SeatState {
        SeatState {
            locked: !lock_reason.is_empty(),
            lock_reason: lock_reason.to_string(),
            ahead: Some(ahead),
            ..super::without_a_tree(None, name, Some(ahead))
        }
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
            seat(
                "a",
                SeatState {
                    index_age: Some(Duration::from_secs(16 * 60)),
                    ..surveyed("worktree-a", "", 1, 13)
                },
            ),
            seat("b", surveyed("", "claude-seat abc123 pid 42", 0, 0)),
            seat(
                "c",
                SeatState {
                    ahead: None,
                    behind: None,
                    dirty: None,
                    ..surveyed("worktree-c", "", 0, 0)
                },
            ),
            Seat {
                name: "d",
                state: None,
            },
            seat("e", treeless("e", "claude-seat s9 pid 7", 17)),
            seat("f", surveyed("worktree-f", "parked by hand", 0, 0)),
        ];
        let table = render(&seats);
        let lines: Vec<&str> = table.lines().collect();
        assert_eq!(lines.len(), 8, "{table}");
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
                && lines[2].ends_with("held by session abc123 (pid 42)"),
            "{table}"
        );
        assert!(lines[3].contains('?'), "{table}");
        assert!(lines[4].contains("(not created)"), "{table}");
        // A letter that lost its tree says whose it still is, what its
        // branch carries and how to get at it.
        assert!(
            lines[5].contains("worktree-e")
                && lines[5].contains("17")
                && lines[5].contains("held by session s9 (pid 7); no tree")
                && lines[5].contains("seat takeover"),
            "{table}"
        );
        assert!(
            lines[6].ends_with("locked by hand (parked by hand)"),
            "{table}"
        );
        assert!(lines[7].starts_with("reading:"), "{table}");
        assert!(lines[7].contains("no process is asked"), "{table}");
    }

    #[test]
    fn a_full_roster_says_what_each_letter_is_refusing_on() {
        assert_eq!(
            one_letter_refusing(&seat(
                "a",
                surveyed("worktree-a", "claude-seat s9 pid 7", 0, 0)
            )),
            "a: held by session s9 (pid 7)"
        );
        assert_eq!(
            one_letter_refusing(&seat("b", surveyed("worktree-b", "", 3, 2))),
            "b: unclaimed, carrying 3 commit(s) main has not and 2 uncommitted file(s)"
        );
        assert!(
            one_letter_refusing(&seat("c", surveyed("worktree-c", "", 0, 0)))
                .contains("taken between"),
        );
        // A lost tree still names whoever holds the letter.
        assert_eq!(
            one_letter_refusing(&seat("d", treeless("d", "", 4))),
            "d: no tree on disk while worktree-d carries 4 commit(s) — a takeover grows the \
             tree back on that branch"
        );
        assert!(
            one_letter_refusing(&seat("e", treeless("e", "claude-seat s9 pid 7", 4)))
                .starts_with("e: held by session s9 (pid 7), and no tree on disk")
        );
        assert!(
            one_letter_refusing(&Seat {
                name: "f",
                state: None,
            })
            .contains("could not make one")
        );
        assert!(
            one_letter_refusing(&seat("f", surveyed("worktree-f", "parked by hand", 0, 0)))
                .ends_with("locked by hand (parked by hand)")
        );
    }
}
