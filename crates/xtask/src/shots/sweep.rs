//! What leaves the board, and how much of it a caller may take.

use std::path::{Path, PathBuf};

use super::board::{board_dir, load_runs, parse_run, shown};
use super::page;

/// Takes runs off the board — their pictures and the file that names
/// them — and rebuilds the page. Answers how many went.
///
/// Every seat writes to one board (`board_dir`), so **what the caller
/// does not name is not touched**: the runs beside yours belong to a
/// session that may be showing them right now, and a sweep that took
/// the board back to empty would be one seat throwing away another's
/// work. `None` is the deliberate exception — every run, whoever took
/// it.
///
/// A run whose file cannot be parsed is left where it is. It is invisible
/// on the page already (`load_runs` skips it), and removing what cannot
/// be read would mean guessing which pictures it held.
pub(super) fn prune(seats: Option<&[String]>) -> Result<(usize, PathBuf), String> {
    let board = board_dir()?;
    let runs = board.join("runs");
    let entries = std::fs::read_dir(&runs)
        .map_err(|e| format!("no board to prune at {}: {e}", shown(&runs)))?;
    let mut gone = 0;
    for path in entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "tsv"))
    {
        let Some(run) = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| parse_run(&text))
        else {
            continue;
        };
        if !wanted(seats, &run.seat) {
            continue;
        }
        for shot in &run.shots {
            remove(&board.join(&shot.file))?;
        }
        remove(&path)?;
        gone += 1;
    }
    let page = board.join("index.html");
    std::fs::write(&page, page::render(&load_runs(&runs)))
        .map_err(|e| format!("could not write {}: {e}", shown(&page)))?;
    Ok((gone, page))
}

/// Whether a run taken in `seat` is one the caller asked for. `None` is
/// every seat; a named list is exactly those.
fn wanted(seats: Option<&[String]>, seat: &str) -> bool {
    seats.is_none_or(|named| named.iter().any(|name| name == seat))
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
    use super::wanted;

    /// The one thing a prune must never do: reach a seat nobody named.
    /// The board is shared, so the default has to be the narrow answer
    /// and `all` has to be asked for.
    #[test]
    fn a_prune_reaches_only_the_seats_it_was_given() {
        let mine = ["c".to_string()];
        assert!(wanted(Some(&mine), "c"));
        assert!(!wanted(Some(&mine), "a"));
        assert!(!wanted(Some(&mine), "main"));
        // Naming several is still exactly those.
        let two = ["c".to_string(), "d".to_string()];
        assert!(wanted(Some(&two), "d"));
        assert!(!wanted(Some(&two), "e"));
        // And nothing named at all is the sweep.
        assert!(wanted(None, "a"));
        assert!(wanted(None, "main"));
        // An empty list is not the sweep — a caller that resolved to no
        // seat asked for no run (`--seat` with nothing behind it is an
        // error at the command line, and this is the second lock).
        assert!(!wanted(Some(&[]), "c"));
    }
}
