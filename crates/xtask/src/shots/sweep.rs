//! What leaves the board, and when.
//!
//! The board is the surface a change is worked on, not an archive of how
//! it was arrived at: what stands on it should be the pictures somebody
//! is looking at now. Left to hand-pruning it is neither — 263 runs and
//! 612 pictures piled up in half a day, one label standing twelve times
//! over, and 88 pictures whose run file somebody had deleted from under
//! them (measured). Three rules keep it a working surface,
//! and none of them reaches a session that is still using its pictures:
//!
//! * **A picture retaken replaces the one before it** — one seat's runs
//!   under one label collapse to the newest. The attempts that were
//!   tried and undone leave with the attempts.
//! * **A seat whose branch landed lets its pictures go** (`land`) — the
//!   work is on main and the evidence has been read.
//! * **A session that ends takes its own runs with it**, in whatever
//!   tree it sat. By session, never by seat: a seat outlives the
//!   sessions that pass through it, and the next one is already sitting
//!   there.
//!
//! What no rule reaches — a run taken by hand outside a session, or an
//! approach abandoned under a name nobody retakes — is what `prune` is
//! for.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::Run;
use super::board::{board_dir, load_runs, parse_run, shown};
use super::page;

/// How long a picture is nobody's business but its own run's.
///
/// A run copies its pictures before it writes the file that names them,
/// and six seats write to one board — a picture that is unreferenced
/// only because its own run is still being written must not be collected
/// out from under it. Ten minutes is far past the milliseconds that
/// window actually is, and nothing is lost by waiting.
const GRACE: Duration = Duration::from_secs(10 * 60);

/// Which runs a sweep reaches.
pub(super) struct Scope {
    /// Whose runs are in reach at all.
    pub(super) whose: Whose,
    /// One label within that, when the caller named one — how a session
    /// drops the pictures of an approach it gave up on without touching
    /// the rest of its work.
    pub(super) label: Option<String>,
}

/// The side of the board a sweep may take.
pub(super) enum Whose {
    /// Runs taken in these trees: roster letters, or `main`.
    Seats(Vec<String>),
    /// Runs taken by one session, wherever it sat.
    Session(String),
    /// Every run, whoever took it — asked for in words, never arrived at
    /// by a caller who named nothing.
    Everything,
}

impl Scope {
    /// Everything one tree took.
    pub(super) fn seats(seats: Vec<String>) -> Self {
        Self {
            whose: Whose::Seats(seats),
            label: None,
        }
    }

    /// Whether this run is one the caller asked for.
    fn wants(&self, run: &Run) -> bool {
        let whose = match &self.whose {
            Whose::Seats(seats) => seats.contains(&run.seat),
            // A run nobody stamped belongs to no session, so no session
            // sweep may take it — an empty id matching an empty stamp
            // would take every hand-taken picture on the board.
            Whose::Session(session) => !session.is_empty() && *session == run.session,
            Whose::Everything => true,
        };
        whose && self.label.as_ref().is_none_or(|label| *label == run.label)
    }
}

/// Takes runs off the board — their pictures and the file that names
/// them — and rebuilds the page. Answers how many went.
///
/// Every seat writes to one board (`board_dir`), so **what the caller
/// does not name is not touched**: the runs beside yours belong to a
/// session that may be showing them right now, and a sweep that took the
/// board back to empty would be one seat throwing away another's work.
/// `Whose::Everything` is the deliberate exception.
///
/// A run whose file cannot be parsed is left where it is. It is invisible
/// on the page already (`load_runs` skips it), and removing what cannot
/// be read would mean guessing which pictures it held.
pub(super) fn prune(scope: &Scope) -> Result<(usize, PathBuf), String> {
    let board = board_dir()?;
    let runs = board.join("runs");
    if !runs.is_dir() {
        return Err(format!("no board to prune at {}", shown(&runs)));
    }
    let mut gone = 0;
    for path in run_files(&runs) {
        let Some(run) = read_run(&path) else {
            continue;
        };
        if !scope.wants(&run) {
            continue;
        }
        take(&board, &run, &path)?;
        gone += 1;
    }
    Ok((gone, rebuild(&board)?))
}

/// A landed seat's pictures go with it (CLAUDE.md ビルド・テスト,
/// 席を外す): the seat's branch is on main, so the pictures that argued
/// for it are nobody's evidence any more. Answers how many went.
pub(crate) fn seat_freed(seat: &str) -> Result<(usize, PathBuf), String> {
    prune(&Scope::seats(vec![seat.to_string()]))
}

/// A session that has ended, taking its own runs with it wherever it
/// shot them. A session that took no picture — or never reached a board
/// at all — has nothing to answer for, so this is quiet about both.
pub(crate) fn session_ended(session: &str) -> Result<usize, String> {
    if session.is_empty() {
        return Ok(0);
    }
    let Ok(board) = board_dir() else {
        return Ok(0);
    };
    if !board.join("runs").is_dir() {
        return Ok(0);
    }
    prune(&Scope {
        whose: Whose::Session(session.to_string()),
        label: None,
    })
    .map(|(gone, _)| gone)
}

/// The run just written, against what the board already held: the same
/// seat's picture of the same thing, taken earlier, is what this one
/// replaces. Answers how many it replaced.
///
/// Retaking is the ordinary rhythm of a change — `verify-ui` puts a run
/// on the board for every run it makes, and `check --verb` makes two of
/// them per verb, per side — so a view worked on all afternoon stands on
/// the board a dozen times over, of which only the last is true. The
/// seat is half the key because the board is shared: the same verb
/// photographed in another tree says nothing about this one. So is the
/// label, which is why a comparison meant to be kept goes under names of
/// its own, or into one run of several pictures.
pub(super) fn supersede(board: &Path, fresh: &Run, stem: &str) -> Result<usize, String> {
    let runs = board.join("runs");
    let mut gone = 0;
    for path in run_files(&runs) {
        if path.file_stem().is_some_and(|name| name == stem) {
            continue;
        }
        let Some(run) = read_run(&path) else {
            continue;
        };
        if !replaced(&run, fresh) {
            continue;
        }
        take(board, &run, &path)?;
        gone += 1;
    }
    if gone > 0 {
        println!(
            "board: replaced {gone} earlier run(s) of \"{}\" in seat {}",
            fresh.label, fresh.seat
        );
    }
    Ok(gone)
}

/// Whether `old` is the picture `fresh` was taken to replace.
fn replaced(old: &Run, fresh: &Run) -> bool {
    old.seat == fresh.seat && old.label == fresh.label && old.at < fresh.at
}

/// The page, written from what the board holds now — and the pictures
/// nothing points at any more, collected on the way past. Every path
/// that changes the board ends here, so what the page shows and what the
/// directory holds never disagree for longer than one command.
pub(super) fn rebuild(board: &Path) -> Result<PathBuf, String> {
    let collected = collect_orphans(board);
    if collected > 0 {
        println!("board: collected {collected} picture(s) no run points at");
    }
    let page = board.join("index.html");
    std::fs::write(&page, page::render(&load_runs(&board.join("runs"))))
        .map_err(|e| format!("could not write {}: {e}", shown(&page)))?;
    Ok(page)
}

/// The pictures nothing points at any more, off the disk. Answers how
/// many went.
///
/// The board's two halves drift: a run file deleted by hand leaves its
/// pictures behind (88 of them were standing that way), and a `record`
/// that copies pictures and then fails writes none of the file that
/// names them. Nothing would ever look at those bytes again, and nothing
/// but this notices they are there.
///
/// Failures are counted out rather than reported: this runs on the way
/// past every rebuild, and a picture that would not delete is not a
/// reason to fail the command that was actually asked for.
fn collect_orphans(board: &Path) -> usize {
    let referenced = referenced_pictures(&board.join("runs"));
    let now = SystemTime::now();
    let Ok(entries) = std::fs::read_dir(board.join("img")) else {
        return 0;
    };
    let mut gone = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
        else {
            continue;
        };
        if !path.is_file() || !collectable(&name, &referenced, age(&path, now)) {
            continue;
        }
        if std::fs::remove_file(&path).is_ok() {
            gone += 1;
        }
    }
    gone
}

/// Whether a picture in img/ is one no run points at any more. `age` is
/// how long since it was written.
fn collectable(name: &str, referenced: &BTreeSet<String>, age: Duration) -> bool {
    age >= GRACE && !referenced.contains(&format!("img/{name}"))
}

/// How long since a file was written, and zero when the filesystem will
/// not say — an age nobody can read is not grounds for deleting.
fn age(path: &Path, now: SystemTime) -> Duration {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|written| now.duration_since(written).ok())
        .unwrap_or(Duration::ZERO)
}

/// Every picture named by anything in the runs directory.
///
/// Read as text, not as parsed runs, and from every file rather than the
/// .tsv ones: a run that no longer parses still names the pictures it
/// holds, and one being staged under .tsv.part names the pictures its
/// `record` has already copied.
fn referenced_pictures(runs: &Path) -> BTreeSet<String> {
    let Ok(entries) = std::fs::read_dir(runs) else {
        return BTreeSet::new();
    };
    entries
        .flatten()
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .flat_map(|text| pictures_named_in(&text))
        .collect()
}

/// The picture each `shot` line of a run file names.
fn pictures_named_in(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| line.strip_prefix("shot\t"))
        .filter_map(|rest| rest.split('\t').next())
        .map(str::to_string)
        .collect()
}

/// The run files of a runs directory, whatever else is beside them.
fn run_files(runs: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(runs) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|kind| kind == "tsv"))
        .collect()
}

/// One run file as a run, or None when it is not one.
fn read_run(path: &Path) -> Option<Run> {
    parse_run(&std::fs::read_to_string(path).ok()?)
}

/// One run off the board: its pictures first, then the file that named
/// them. That order is the safe one — interrupted between the two it
/// leaves pictures nothing points at, which `collect_orphans` sweeps,
/// where the reverse would leave a run pointing at pictures that are
/// gone.
fn take(board: &Path, run: &Run, path: &Path) -> Result<(), String> {
    for shot in &run.shots {
        remove(&board.join(&shot.file))?;
    }
    remove(path)
}

/// Deletes a file, and calls a file that is already gone done. The board
/// is written by six seats at once, so the picture this run named may
/// have been swept a moment ago — that is the outcome asked for, not a
/// failure. Anything else is reported rather than dropped (CLAUDE.md
/// Rust 規約).
fn remove(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("could not remove {}: {e}", shown(path))),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Duration, GRACE, Run, Scope, Whose, collectable, pictures_named_in, referenced_pictures,
        replaced,
    };
    use std::collections::BTreeSet;

    fn run(seat: &str, session: &str, label: &str, at: u128) -> Run {
        Run {
            label: label.to_string(),
            verb: String::new(),
            seat: seat.to_string(),
            session: session.to_string(),
            at,
            side_by_side: false,
            shots: Vec::new(),
        }
    }

    /// The one thing a sweep must never do: reach a seat nobody named.
    /// The board is shared, so the default has to be the narrow answer
    /// and the whole board has to be asked for.
    #[test]
    fn a_prune_reaches_only_the_seats_it_was_given() {
        let mine = Scope::seats(vec!["c".to_string()]);
        assert!(mine.wants(&run("c", "s1", "x", 1)));
        assert!(!mine.wants(&run("a", "s1", "x", 1)));
        assert!(!mine.wants(&run("main", "s1", "x", 1)));
        // Naming several is still exactly those.
        let two = Scope::seats(vec!["c".to_string(), "d".to_string()]);
        assert!(two.wants(&run("d", "s1", "x", 1)));
        assert!(!two.wants(&run("e", "s1", "x", 1)));
        // A caller that resolved to no seat asked for no run (`--seat`
        // with nothing behind it is an error at the command line, and
        // this is the second lock).
        assert!(!Scope::seats(Vec::new()).wants(&run("c", "s1", "x", 1)));
        // And the sweep has to be spelled out.
        let every = Scope {
            whose: Whose::Everything,
            label: None,
        };
        assert!(every.wants(&run("a", "s1", "x", 1)));
        assert!(every.wants(&run("main", "", "x", 1)));
    }

    /// A session's sweep follows the session, not the seat it sat in —
    /// and a run stamped with no session is nobody's to take.
    #[test]
    fn a_session_takes_its_own_runs_and_no_others() {
        let mine = Scope {
            whose: Whose::Session("s1".to_string()),
            label: None,
        };
        assert!(mine.wants(&run("a", "s1", "x", 1)));
        assert!(mine.wants(&run("main", "s1", "x", 1)), "wherever it sat");
        assert!(
            !mine.wants(&run("a", "s2", "x", 1)),
            "the next session in the same seat keeps its pictures"
        );
        assert!(!mine.wants(&run("a", "", "x", 1)), "a hand-taken run stays");
        let nameless = Scope {
            whose: Whose::Session(String::new()),
            label: None,
        };
        assert!(
            !nameless.wants(&run("a", "", "x", 1)),
            "an empty id must match no run at all"
        );
    }

    /// Dropping one abandoned approach leaves the rest of the seat's
    /// work standing.
    #[test]
    fn a_named_label_narrows_the_sweep_to_itself() {
        let scope = Scope {
            whose: Whose::Seats(vec!["a".to_string()]),
            label: Some("header mock 0".to_string()),
        };
        assert!(scope.wants(&run("a", "s1", "header mock 0", 1)));
        assert!(!scope.wants(&run("a", "s1", "header mock 2", 1)));
        assert!(!scope.wants(&run("b", "s1", "header mock 0", 1)));
    }

    /// What a retake replaces, and what it leaves alone.
    #[test]
    fn a_retake_replaces_the_same_view_and_nothing_else() {
        let fresh = run("a", "s1", "graph-head", 200);
        assert!(replaced(&run("a", "s1", "graph-head", 100), &fresh));
        assert!(
            replaced(&run("a", "s2", "graph-head", 100), &fresh),
            "the seat and the name are the view; who took it is not"
        );
        assert!(
            !replaced(&run("a", "s1", "graph-head — linux", 100), &fresh),
            "the other side of Done is another picture, not this one again"
        );
        assert!(
            !replaced(&run("b", "s1", "graph-head", 100), &fresh),
            "another tree's picture says nothing about this one"
        );
        assert!(
            !replaced(&run("a", "s1", "graph-head", 300), &fresh),
            "a run taken later is not one this replaces"
        );
    }

    /// Collection waits out the window in which a run's own pictures are
    /// written before the file that names them.
    #[test]
    fn only_an_old_picture_nothing_points_at_is_collected() {
        let referenced: BTreeSet<String> = ["img/kept.png".to_string()].into_iter().collect();
        let old = GRACE + Duration::from_secs(1);
        assert!(collectable("stray.png", &referenced, old));
        assert!(!collectable("kept.png", &referenced, old));
        assert!(
            !collectable("stray.png", &referenced, Duration::from_secs(1)),
            "a picture written a moment ago may be a run still writing"
        );
    }

    /// A run file that no longer parses still speaks for its pictures.
    #[test]
    fn a_reference_is_read_out_of_any_line_that_names_one() {
        let whole = "label\tx\nseat\ta\nat\t1\nshot\timg/a.png\tapp.png\t1\t1\n";
        assert_eq!(pictures_named_in(whole), vec!["img/a.png".to_string()]);
        // No label, no time: `parse_run` refuses it, and its pictures
        // are still spoken for.
        let broken = "shot\timg/b.png\tapp.png\t1\t1\nshot\timg/c.png\toverlay.png\t1\t1\n";
        assert_eq!(
            pictures_named_in(broken),
            vec!["img/b.png".to_string(), "img/c.png".to_string()]
        );
        assert!(pictures_named_in("label\tno pictures here\n").is_empty());
    }

    /// A runs directory that is not there is an empty set, not a panic:
    /// `collect_orphans` runs on every rebuild, including the first.
    #[test]
    fn a_board_without_runs_references_nothing() {
        assert!(referenced_pictures(std::path::Path::new("no/such/runs")).is_empty());
    }
}
