//! The one window the board is read in.
//!
//! The page lives at a fixed path and is rewritten in place, so a window
//! already showing the board is one F5 from the run just taken; a second
//! window only leaves the reader unsure which one is current. So `open`
//! hands out a window only while none stands, whoever asks and from
//! whichever seat, and `--again` is the one way past it — for the case
//! the board cannot see, the reader having closed theirs.
//!
//! What stands is remembered beside the page, because every seat reads
//! one board (`board_dir`): a marker kept per session would let each open
//! a window of its own.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::board::{seat_here, shown};

/// Where the board remembers the window it handed out: beside the page,
/// out of the sweeps' reach (`runs/`, `img/`), since a window outlives
/// the run, session and seat that opened it.
fn marker(board: &Path) -> PathBuf {
    board.join("window")
}

/// A window handed out earlier and still standing.
pub(super) struct Standing {
    /// Milliseconds since the epoch.
    at: u128,
    /// The tree it was opened from.
    seat: String,
}

/// The window the board believes is open, if it believes in one.
pub(super) fn standing(board: &Path) -> Option<Standing> {
    read_standing(&std::fs::read_to_string(marker(board)).ok()?)
}

/// Puts the board in front of the reader; while a window stands, says
/// where and opens nothing. `again` is the reader's own word that they
/// closed it — nothing else can know, since the browser may hand the
/// window to an already-running process and leave no child to watch.
pub(super) fn show(board: &Path, page: &Path, again: bool) -> Result<(), String> {
    if !again && let Some(open) = standing(board) {
        println!(
            "board: a window is already open (seat {}, opened {}) — press F5 there \
             and read from the top down",
            open.named_seat(),
            ago(open.at),
        );
        println!("board: closed it? `cargo xtask shots open --again` opens one more");
        return Ok(());
    }
    let program = spawn(page)?;
    println!(
        "board: opened {} with {program} — F5 in that window from now on",
        shown(page)
    );
    // The window is open regardless: a marker that will not write is
    // reported, not an error.
    if let Err(message) = remember(board) {
        println!(
            "board: could not write down the window ({message}) — the next `open` will open another"
        );
    }
    Ok(())
}

impl Standing {
    /// The seat, or `?` for a marker written without one.
    fn named_seat(&self) -> &str {
        if self.seat.is_empty() {
            "?"
        } else {
            &self.seat
        }
    }
}

/// Where the pictures just put on the board are read — said right after
/// a run, the moment a second window would otherwise get opened.
pub(super) fn how_to_see_it(board: &Path) -> String {
    match standing(board) {
        Some(open) => format!(
            "a window is open (seat {}) — F5 there and read from the top down",
            open.named_seat()
        ),
        None => format!(
            "no window open yet — `{}` opens the one",
            super::OPEN.line()
        ),
    }
}

fn remember(board: &Path) -> Result<(), String> {
    let at = now().ok_or("the clock is before the epoch")?;
    let text = format!("at\t{at}\nseat\t{}\n", seat_here());
    let path = marker(board);
    std::fs::write(&path, text).map_err(|e| format!("could not write {}: {e}", shown(&path)))
}

/// The inverse of `remember`. An unreadable marker is no window, which
/// errs towards opening one.
fn read_standing(text: &str) -> Option<Standing> {
    let mut at = 0;
    let mut seat = String::new();
    for line in text.lines() {
        let mut parts = line.split('\t');
        match parts.next() {
            Some("at") => {
                at = parts
                    .next()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0)
            }
            Some("seat") => seat = parts.next().unwrap_or_default().to_string(),
            _ => {}
        }
    }
    (at > 0).then_some(Standing { at, seat })
}

fn now() -> Option<u128> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|since| since.as_millis())
}

/// How old the standing window is, in words: the age is how a reader
/// judges whether it was closed, which this command cannot see.
fn ago(at: u128) -> String {
    let Some(now) = now() else {
        return "at a time this clock cannot read".to_string();
    };
    words(now.saturating_sub(at))
}

/// A span of milliseconds as the coarsest true thing that can be said
/// about it.
fn words(since: u128) -> String {
    let minutes = since / 60_000;
    let plural = |n: u128, unit: &str| format!("{n} {unit}{} ago", if n == 1 { "" } else { "s" });
    match minutes {
        0 => "a moment ago".to_string(),
        1..=59 => plural(minutes, "minute"),
        60..=1439 => plural(minutes / 60, "hour"),
        _ => plural(minutes / 1440, "day"),
    }
}

/// Opens the board in a window of its own (a browser in `--app` mode),
/// answering what opened it. System image viewers are no candidate: they
/// smooth what they magnify.
fn spawn(page: &Path) -> Result<&'static str, String> {
    let url = format!("file:///{}", page.display().to_string().replace('\\', "/"));
    let app = format!("--app={url}");
    for (program, args) in browsers(&app, &url) {
        if std::process::Command::new(program)
            .args(&args)
            .spawn()
            .is_ok()
        {
            return Ok(program);
        }
    }
    Err(format!(
        "found no browser to open {}: open it by hand",
        page.display()
    ))
}

/// Candidates in order of preference, per OS.
fn browsers(app: &str, url: &str) -> Vec<(&'static str, Vec<String>)> {
    if cfg!(windows) {
        vec![
            (
                r"C:\Program Files\Google\Chrome\Application\chrome.exe",
                vec![app.to_string()],
            ),
            (
                r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
                vec![app.to_string()],
            ),
            (
                "cmd",
                vec![
                    "/c".to_string(),
                    "start".to_string(),
                    String::new(),
                    url.to_string(),
                ],
            ),
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            (
                "open",
                vec![
                    "-na".to_string(),
                    "Google Chrome".to_string(),
                    "--args".to_string(),
                    app.to_string(),
                ],
            ),
            ("open", vec![url.to_string()]),
        ]
    } else {
        vec![
            ("google-chrome", vec![app.to_string()]),
            ("chromium", vec![app.to_string()]),
            ("xdg-open", vec![url.to_string()]),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::{browsers, read_standing, remember, standing, words};

    #[test]
    fn a_window_written_down_is_the_one_the_next_command_finds() {
        let board = crate::yard::Yard::new("shots-window");
        assert!(
            standing(&board).is_none(),
            "a board nobody has opened stands no window"
        );
        remember(&board).expect("the window is written down");
        let open = standing(&board).expect("and read back by whoever asks next");
        assert!(
            !open.named_seat().is_empty(),
            "it says which tree opened it"
        );
    }

    #[test]
    fn a_standing_window_survives_the_round_trip() {
        let open = read_standing("at\t1700000000000\nseat\tc\n").expect("a whole marker reads");
        assert_eq!(open.at, 1_700_000_000_000);
        assert_eq!(open.named_seat(), "c");
    }

    /// Errs towards no window: a reader sent to a window that is not
    /// there has nowhere to press F5, where an extra one can be closed.
    #[test]
    fn a_marker_that_says_nothing_stands_for_no_window() {
        assert!(read_standing("").is_none());
        assert!(read_standing("seat\tc\n").is_none());
        assert!(read_standing("at\tnot a number\n").is_none());
        assert!(read_standing("at\t0\nseat\tc\n").is_none());
        // A window opened by hand, outside any seat, is still a window.
        let open = read_standing("at\t1700000000000\n").expect("a time is a window");
        assert_eq!(open.named_seat(), "?");
    }

    #[test]
    fn an_age_reads_as_one() {
        assert_eq!(words(0), "a moment ago");
        assert_eq!(words(59_000), "a moment ago");
        assert_eq!(words(60_000), "1 minute ago");
        assert_eq!(words(25 * 60_000), "25 minutes ago");
        assert_eq!(words(60 * 60_000), "1 hour ago");
        assert_eq!(words(5 * 60 * 60_000), "5 hours ago");
        assert_eq!(words(24 * 60 * 60_000), "1 day ago");
        assert_eq!(words(9 * 24 * 60 * 60_000), "9 days ago");
    }

    #[test]
    fn every_candidate_is_handed_the_board() {
        let candidates = browsers("--app=file:///b/index.html", "file:///b/index.html");
        assert!(!candidates.is_empty());
        for (_, args) in candidates {
            assert!(
                args.iter().any(|a| a.contains("file:///b/index.html")),
                "a candidate was handed no board: {args:?}"
            );
        }
    }
}
