//! What leaves the board, and when.
//!
//! The board is the surface a change is worked on: what stands on it
//! should be the pictures somebody is looking at now. Left to
//! hand-pruning it drifts — 263 runs and 612 pictures piled up in half
//! a day, one label standing twelve times over, and 88 pictures whose
//! run file somebody had deleted from under them (measured). Three
//! rules keep it a working surface, and none of them reaches a session
//! that is still using its pictures:
//!
//! * **A picture retaken replaces the one before it** — one seat's runs
//!   under one label collapse to the newest. The attempts that were
//!   tried and undone leave with the attempts.
//! * **A seat's pictures leave when its work does** — when the branch
//!   lands on main (`land`), and when the seat is handed to a fresh
//!   stretch of work having landed nothing (`seats::start_at_main`).
//!   Those two are the whole of it: a run stands for exactly as long as
//!   the work it was taken for.
//!
//! **A session ending leaves the board standing.** The machine sleeps
//! and every open conversation is handed a SessionEnd, then goes on
//! working at the next wake — so a board that swept there would empty
//! itself under a reader just told to press F5 on it. The seat
//! outlives its sessions anyway: which one took the picture says
//! nothing about whether the work it argues for is finished.
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
/// only because its own run is still being written is left where it
/// is. Ten minutes is far past the milliseconds that window actually
/// is, and nothing is lost by waiting.
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
    /// Every run, whoever took it — reached only by a caller who
    /// asks for it in words.
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
            Whose::Everything => true,
        };
        whose && self.label.as_ref().is_none_or(|label| *label == run.label)
    }
}

/// Takes runs off the board — their pictures and the file that names
/// them — and rebuilds the page. Answers how many went.
///
/// Every seat writes to one board (`board_dir`), so **only what the
/// caller names is touched**: the runs beside yours belong to a
/// session that may be showing them right now, and a sweep that took
/// the board back to empty would be one seat throwing away another's
/// work. `Whose::Everything` is the deliberate exception.
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

/// A seat's pictures go when its work does (CLAUDE.md ビルド・テスト,
/// 席を外す): the branch is on main, so the pictures that argued for it
/// are nobody's evidence any more. Answers how many went.
pub(crate) fn seat_freed(seat: &str) -> Result<(usize, PathBuf), String> {
    prune(&Scope::seats(vec![seat.to_string()]))
}

/// The same, for a seat starting a stretch of work
/// (`seats::start_at_main`): whatever is still on the board from this
/// letter belongs to work that is over — landed, or given up on — and
/// the run that argued for it has nobody left to argue to.
///
/// Quiet in every failure, and quiet about how many
/// went: a seat is handed out whatever the board does —
/// missing, or refusing to be written.
pub(crate) fn seat_reused(seat: &str) {
    if let Err(_unheard) = seat_freed(seat) {
        // The seat is the answer being given; the board is a note beside
        // it. `cargo xtask shots prune --seat <letter>` is the way back.
    }
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
/// that changes the board ends here, so what the page shows and what
/// the directory holds agree within one command.
pub(super) fn rebuild(board: &Path) -> Result<PathBuf, String> {
    let collected = collect_orphans(board);
    if collected > 0 {
        println!("board: collected {collected} picture(s) no run points at");
    }
    let page = board.join("index.html");
    // Staged and moved into place: six seats rebuild
    // this page, and a reader pressing F5 meets a whole
    // one. Under this process's own name — there is no
    // lock between the seats, and two staging one name
    // would move each other's halves into place.
    let staging = board.join(format!("index.html.{}.part", std::process::id()));
    std::fs::write(&staging, page::render(&load_runs(&board.join("runs"))))
        .map_err(|e| format!("could not write {}: {e}", shown(&staging)))?;
    std::fs::rename(&staging, &page)
        .map_err(|e| format!("could not place {}: {e}", shown(&page)))?;
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
/// Failures are counted out: this runs on the way past every rebuild,
/// and a picture that would not delete leaves the command that was
/// actually asked for standing.
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

/// How long since a file was written, and zero when the filesystem
/// will not say — zero keeps a picture whose age nobody can read.
fn age(path: &Path, now: SystemTime) -> Duration {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|written| now.duration_since(written).ok())
        .unwrap_or(Duration::ZERO)
}

/// Every picture named by anything in the runs directory.
///
/// Read as text, and from every file in the directory:
/// a run that no longer parses still names the pictures
/// it holds, and one being staged under .tsv.part names
/// the pictures its `record` has already copied.
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

/// Deletes a file, and calls a file that is already
/// gone done. The board is written by six seats at
/// once, so the picture this run named may have been
/// swept a moment ago — that is the outcome asked for.
/// Anything else is reported (CLAUDE.md Rust 規約).
fn remove(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("could not remove {}: {e}", shown(path))),
    }
}

#[cfg(test)]
mod tests;
