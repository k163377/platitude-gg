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
//! * **A session that ends marks its own runs**, in whatever tree it
//!   sat, and they leave a day later unless the session is heard from
//!   again. By session, never by seat: a seat outlives the sessions that
//!   pass through it, and the next one is already sitting there.
//!
//! The mark in that last rule is the whole of it: a session ending is
//! not a session being over. The machine sleeps, every open
//! conversation is handed a SessionEnd, and on the next wake the same
//! sessions carry on working — so an end that deleted would take the
//! board out from under a reader who was told to press F5 on it. Only
//! the ending is visible here, so it writes a mark, and what expires is
//! the mark (`FAREWELL`).
//!
//! What no rule reaches — a run taken by hand outside a session, or an
//! approach abandoned under a name nobody retakes — is what `prune` is
//! for.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::Run;
use super::board::{board_dir, load_runs, now_ms, parse_run, place, shown};
use super::page;

/// How long a picture is nobody's business but its own run's.
///
/// A run copies its pictures before it writes the file that names them,
/// and six seats write to one board — a picture that is unreferenced
/// only because its own run is still being written must not be collected
/// out from under it. Ten minutes is far past the milliseconds that
/// window actually is, and nothing is lost by waiting.
const GRACE: Duration = Duration::from_secs(10 * 60);

/// How long a marked run stands before the board lets it go.
///
/// It has to outlast the gap a mark can be written across while the
/// session it belongs to is only asleep — the machine goes down for the
/// night with the conversation open, and the SessionEnd that fires there
/// is answered by the SessionStart of the same session in the morning.
/// A day covers a night with room to spare, and it is short enough that
/// a session that really is over has its pictures off the board before
/// the next day's work goes up. Nothing expires while the machine is
/// asleep either way: this is read when a command touches the board, and
/// a session that comes back speaks for its runs first (`session_seen`).
const FAREWELL: Duration = Duration::from_secs(24 * 60 * 60);

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

/// A session that has ended marks its own runs, wherever it shot them:
/// they stand a day longer (`FAREWELL`), and go on standing for as long
/// as the session is heard from again. A session that took no picture —
/// or never reached a board at all — has nothing to answer for, so this
/// is quiet about both.
///
/// The mark is written rather than acted on because this event fires
/// over a sleep as readily as over a goodbye, and the two are told apart
/// only by what happens next.
pub(crate) fn session_ended(session: &str) -> Result<usize, String> {
    let Some(now) = now_ms() else {
        return Ok(0);
    };
    let (marked, board) = mark(session, now)?;
    // The page carries no mark, so this rebuild is for the collecting it
    // does on the way past — and that is worth doing whether or not this
    // session had pictures of its own: a session ending is the moment a
    // mark left standing since yesterday is finally acted on.
    if let Some(board) = board {
        rebuild(&board)?;
    }
    Ok(marked)
}

/// A sign of life from a session — its start, and every prompt after
/// that. Whatever mark its end left comes off: the pictures are being
/// worked with, whether the end was a sleep the machine came back from
/// or a resume of the same conversation.
///
/// Nothing is rebuilt here: the page never showed the mark, and this
/// runs at every prompt, where a line of housekeeping would be a line in
/// somebody's prompt.
pub(crate) fn session_seen(session: &str) -> Result<usize, String> {
    mark(session, 0).map(|(unmarked, _)| unmarked)
}

/// Writes `ended` onto every run of one session that does not carry it
/// already. Answers how many runs were marked — or unmarked, for an
/// `ended` of 0 — and the board they are on, for a caller that has a
/// reason to rebuild it.
fn mark(session: &str, ended: u128) -> Result<(usize, Option<PathBuf>), String> {
    if session.is_empty() {
        return Ok((0, None));
    }
    let Ok(board) = board_dir() else {
        return Ok((0, None));
    };
    if !board.join("runs").is_dir() {
        return Ok((0, None));
    }
    let marked = mark_in(&board, session, ended)?;
    Ok((marked, Some(board)))
}

/// The same, on a board somebody has already found — which is what the
/// rule can be asserted against, the board a session actually writes to
/// being wherever this process happens to be standing.
fn mark_in(board: &Path, session: &str, ended: u128) -> Result<usize, String> {
    let mut marked = 0;
    for path in run_files(&board.join("runs")) {
        let Some(mut run) = read_run(&path) else {
            continue;
        };
        if !taken_by(&run, session) || (run.ended == 0) == (ended == 0) {
            continue;
        }
        run.ended = ended;
        place(&path, &run)?;
        marked += 1;
    }
    Ok(marked)
}

/// Whether a run is one that session may speak for.
///
/// A run nobody stamped belongs to no session — a picture taken by hand
/// in a terminal carries none — so an id that is empty on either side
/// matches nothing: two blanks meeting would put every hand-taken
/// picture on the board on some session's clock.
fn taken_by(run: &Run, session: &str) -> bool {
    !session.is_empty() && run.session == session
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
    let expired = collect_expired(board);
    if expired > 0 {
        println!("board: {expired} run(s) went — their sessions ended a day ago");
    }
    let collected = collect_orphans(board);
    if collected > 0 {
        println!("board: collected {collected} picture(s) no run points at");
    }
    let page = board.join("index.html");
    std::fs::write(&page, page::render(&load_runs(&board.join("runs"))))
        .map_err(|e| format!("could not write {}: {e}", shown(&page)))?;
    Ok(page)
}

/// The runs whose mark has run out, off the board. Answers how many
/// went.
///
/// This is where a session's end is finally acted on, a day after the
/// event and only for a session that never came back (`FAREWELL`). It
/// runs on the way past every rebuild rather than on a clock of its own:
/// nothing runs while the machine is asleep, and the first command to
/// touch the board afterwards is early enough — the session that woke up
/// with it will have spoken for its own runs before then
/// (`session_seen`).
///
/// A run that will not delete is counted out rather than raised, for the
/// same reason `collect_orphans` does it: this is housekeeping on the
/// way to the command somebody actually asked for.
fn collect_expired(board: &Path) -> usize {
    let Some(now) = now_ms() else {
        return 0;
    };
    let mut gone = 0;
    for path in run_files(&board.join("runs")) {
        let Some(run) = read_run(&path) else {
            continue;
        };
        if !expired(&run, now) {
            continue;
        }
        if take(board, &run, &path).is_ok() {
            gone += 1;
        }
    }
    gone
}

/// Whether a run's mark has run out, `now` being the clock in
/// milliseconds. A run nobody marked never expires, and neither does one
/// marked in the future — a clock that disagrees with the mark is no
/// reason to delete a picture.
fn expired(run: &Run, now: u128) -> bool {
    run.ended > 0 && now.saturating_sub(run.ended) >= FAREWELL.as_millis()
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
mod tests;
