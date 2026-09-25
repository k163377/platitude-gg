//! What leaves the board, and when: it should hold only the pictures
//! somebody is looking at now, and hand-pruning lets it pile up. None of
//! these rules reaches a session still using its pictures:
//!
//! * A picture retaken replaces the one before it — one seat's runs
//!   under one label collapse to the newest.
//! * A seat's pictures leave when its work does — when the branch lands
//!   on main (`land`), when the seat is handed to a fresh stretch of work
//!   (`seats::start_at_main`), and when a letter whose tree went is
//!   created anew (`seats::create_seat`).
//! * A session ending leaves the board standing. A sleeping machine hands
//!   every open conversation a SessionEnd and they go on at the next
//!   wake, so sweeping there would empty the board under a reader just
//!   told to press F5.
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

/// A run copies its pictures before it writes the file that names them,
/// and seats write to one board concurrently: a picture younger than
/// this may belong to a run still being written. Far past that window,
/// and nothing is lost by waiting.
const GRACE: Duration = Duration::from_secs(10 * 60);

/// Which runs a sweep reaches.
pub(super) struct Scope {
    pub(super) whose: Whose,
    /// One label within that — how a session drops an abandoned approach
    /// without touching the rest of its work.
    pub(super) label: Option<String>,
}

pub(super) enum Whose {
    /// Runs taken in these trees: roster letters, or `main`.
    Seats(Vec<String>),
    /// Every run, whoever took it — only when the caller asks for it in
    /// words.
    Everything,
}

impl Scope {
    /// Everything these trees took.
    pub(super) fn seats(seats: Vec<String>) -> Self {
        Self {
            whose: Whose::Seats(seats),
            label: None,
        }
    }

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
/// Only what the caller names is touched: the board is shared, and the
/// runs beside yours may be on somebody's screen right now. A run file
/// that cannot be parsed is left: removing it would mean guessing which
/// pictures it held.
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

/// A seat's pictures go when its work does — its branch is on main.
/// Answers how many went.
pub(crate) fn seat_freed(seat: &str) -> Result<(usize, PathBuf), String> {
    prune(&Scope::seats(vec![seat.to_string()]))
}

/// The same, for a letter created anew (`seats::create_seat`): what
/// still stands under its name belongs to work that went with its tree.
/// Quiet in every failure: a seat is handed out whatever the board does.
pub(crate) fn seat_reused(seat: &str) {
    if let Err(_unheard) = seat_freed(seat) {
        // `cargo xtask shots prune --seat <letter>` is the way back.
    }
}

/// Takes the same seat's earlier runs under the same label off the
/// board; answers how many. The seat is half the key because the board
/// is shared; the label is the other, so a comparison meant to be kept
/// goes under a label of its own, or into one run of several pictures.
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

fn replaced(old: &Run, fresh: &Run) -> bool {
    old.seat == fresh.seat && old.label == fresh.label && old.at < fresh.at
}

/// Rewrites the page and collects the pictures nothing points at. Every
/// path that changes the board ends here, so page and directory agree
/// within one command.
pub(super) fn rebuild(board: &Path) -> Result<PathBuf, String> {
    let collected = collect_orphans(board);
    if collected > 0 {
        println!("board: collected {collected} picture(s) no run points at");
    }
    let page = board.join("index.html");
    // Staged under this process's own name and renamed into place: seats
    // rebuild this page with no lock between them, and F5 has to meet a
    // whole one.
    let staging = board.join(format!("index.html.{}.part", std::process::id()));
    std::fs::write(&staging, page::render(&load_runs(&board.join("runs"))))
        .map_err(|e| format!("could not write {}: {e}", shown(&staging)))?;
    std::fs::rename(&staging, &page)
        .map_err(|e| format!("could not place {}: {e}", shown(&page)))?;
    Ok(page)
}

/// Deletes the pictures nothing points at — left by a run file deleted
/// by hand, or a `record` that failed after copying. Answers how many
/// went. A picture that will not delete is skipped: this runs on the way
/// past every rebuild and must not fail the command actually asked for.
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

fn collectable(name: &str, referenced: &BTreeSet<String>, age: Duration) -> bool {
    age >= GRACE && !referenced.contains(&format!("img/{name}"))
}

/// Zero when the filesystem will not say, which keeps the picture.
fn age(path: &Path, now: SystemTime) -> Duration {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|written| now.duration_since(written).ok())
        .unwrap_or(Duration::ZERO)
}

/// Read as text from every file in the directory: a run that no longer
/// parses still names its pictures, and one staged under .tsv.part names
/// what its `record` has already copied.
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

fn read_run(path: &Path) -> Option<Run> {
    parse_run(&std::fs::read_to_string(path).ok()?)
}

/// Pictures first, then the file that named them: interrupted between,
/// this leaves orphans `collect_orphans` sweeps, not a run pointing at
/// pictures that are gone.
fn take(board: &Path, run: &Run, path: &Path) -> Result<(), String> {
    for shot in &run.shots {
        remove(&board.join(&shot.file))?;
    }
    remove(path)
}

/// A file already gone is done: another seat may have swept it a moment
/// ago. Anything else is reported.
fn remove(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("could not remove {}: {e}", shown(path))),
    }
}

#[cfg(test)]
mod tests;
