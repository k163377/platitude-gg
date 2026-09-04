//! The one window the board is read in.
//!
//! The page lives at a fixed path and is rewritten in place, so a window
//! already showing the board is one keypress away from showing the run
//! that was just taken: F5, and the board is read from the top down in
//! the order the runs were put up (`board::load_runs`). A second
//! window gains nothing and costs the reader the only thing they have to
//! be sure of — which of the windows in front of them is the board as it
//! stands now.
//!
//! Prose did not hold it. The rule was written down twice — in the
//! verify-ui skill and in memory — and the windows piled up anyway,
//! because every session starts without having watched the last one open
//! one. So the count is kept here instead: `open` hands out a window
//! only while none stands, whoever asks and from whichever seat, and
//! `--again` is the one way past it — for the single case the board
//! cannot see, which is the reader having closed theirs.
//!
//! What stands is remembered beside the page rather than in the session,
//! because six seats read one board (`board_dir`): a marker each session
//! kept to itself would let six of them open a window apiece and each be
//! certain it had opened only one.

use std::path::{Path, PathBuf};

use super::board::{now_ms, seat_here, shown};

/// Where the board remembers the window it handed out.
///
/// Beside the page, not under `runs/` or `img/`: the sweeps reach those
/// two, and a window is not evidence — it outlives the run that was
/// opened to be looked at, the session that took it, and the seat that
/// landed.
fn marker(board: &Path) -> PathBuf {
    board.join("window")
}

/// A window handed out earlier, and never taken back.
pub(super) struct Standing {
    /// Milliseconds since the epoch, so a reader can be told how old the
    /// window they are being sent to is.
    at: u128,
    /// The tree it was opened from. Not a claim on it — the window shows
    /// the whole board — only the answer to "who opened this".
    seat: String,
}

/// The window the board believes is open, if it believes in one.
pub(super) fn standing(board: &Path) -> Option<Standing> {
    read_standing(&std::fs::read_to_string(marker(board)).ok()?)
}

/// Puts the board in front of the reader.
///
/// The ordinary answer is that it is already in front of them: say where
/// the window is and what refreshes it, and open nothing. `again` is the
/// reader's own word that they closed it — nothing else may set it,
/// because nothing else can know (the browser is spawned and forgotten,
/// and on every OS it may hand the window to a process that was already
/// running, so there is no child to watch).
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
    // The window is open whatever happens next, so a marker that would
    // not write is reported rather than raised: failing here would say
    // the board did not open when it did.
    if let Err(message) = remember(board) {
        println!(
            "board: could not write down the window ({message}) — the next `open` will open another"
        );
    }
    Ok(())
}

impl Standing {
    /// The seat, or a mark for a marker written without one — a window
    /// that stands is worth saying so even when nobody can be named for
    /// it.
    fn named_seat(&self) -> &str {
        if self.seat.is_empty() {
            "?"
        } else {
            &self.seat
        }
    }
}

/// Where the pictures that were just put on the board are read — said at
/// the moment somebody has taken one to show, which is the moment the
/// second window used to get opened.
pub(super) fn how_to_see_it(board: &Path) -> String {
    match standing(board) {
        Some(open) => format!(
            "a window is open (seat {}) — F5 there and read from the top down",
            open.named_seat()
        ),
        None => "no window open yet — `cargo xtask shots open` opens the one".to_string(),
    }
}

/// Writes down the window that was just handed out.
fn remember(board: &Path) -> Result<(), String> {
    let at = now_ms().ok_or("the clock is before the epoch")?;
    let text = format!("at\t{at}\nseat\t{}\n", seat_here());
    let path = marker(board);
    std::fs::write(&path, text).map_err(|e| format!("could not write {}: {e}", shown(&path)))
}

/// The inverse. None when the file says nothing a window can be read out
/// of — an unreadable marker is no window, which errs towards opening
/// one rather than towards a reader sent to a window that is not there.
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

/// How old the standing window is, in words.
///
/// The one thing this command cannot see is a window that was closed, so
/// the age is what a reader judges that by: "opened 4 days ago" is the
/// line that gets `--again` typed, where a bare timestamp is one more
/// number to work out.
fn ago(at: u128) -> String {
    let Some(now) = now_ms() else {
        return "at a time this clock cannot read".to_string();
    };
    words(now.saturating_sub(at))
}

/// A span of milliseconds as the coarsest true thing that can be said
/// about it.
pub(super) fn words(since: u128) -> String {
    let minutes = since / 60_000;
    let plural = |n: u128, unit: &str| format!("{n} {unit}{} ago", if n == 1 { "" } else { "s" });
    match minutes {
        0 => "a moment ago".to_string(),
        1..=59 => plural(minutes, "minute"),
        60..=1439 => plural(minutes / 60, "hour"),
        _ => plural(minutes / 1440, "day"),
    }
}

/// Opens the board in a window of its own, answering what opened it.
///
/// A browser in `--app` mode is a bare window — no tabs, no address bar —
/// which is what the board wants to be. The system image viewers are the
/// alternative and they smooth what they magnify, so they cannot answer
/// the question the board exists for. Falling back to the ordinary
/// handler is still better than nothing: a tab in the default browser
/// zooms just as exactly, it merely brings its furniture along.
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

/// Candidates in order of preference, per OS. Named rather than searched
/// for: xtask carries std alone, and the differences between the three
/// targets belong in code, not in parallel scripts (CLAUDE.md ビルド・テスト).
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

    /// The rule is carried between two runs of the command by a file, so
    /// the file is the rule: a window written down here has to be the
    /// window the *next* command finds — the next seat's, the next
    /// session's, tomorrow's.
    #[test]
    fn a_window_written_down_is_the_one_the_next_command_finds() {
        let board = std::env::temp_dir().join(format!("pg-shots-window-{}", std::process::id()));
        std::fs::create_dir_all(&board).expect("a board to write into");
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
        std::fs::remove_dir_all(&board).expect("the temporary board goes");
    }

    /// What the marker has to survive: it is the whole difference
    /// between "press F5" and a second window.
    #[test]
    fn a_standing_window_survives_the_round_trip() {
        let open = read_standing("at\t1700000000000\nseat\tc\n").expect("a whole marker reads");
        assert_eq!(open.at, 1_700_000_000_000);
        assert_eq!(open.named_seat(), "c");
    }

    /// A marker nothing can be read out of is no window. The error has
    /// to fall this way: a reader sent to a window that is not there has
    /// nowhere to press F5, where one extra window is the thing they can
    /// close.
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

    /// The age is what gets `--again` typed, so it has to read as an age
    /// at every scale the board is left standing for.
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

    /// Every candidate carries the board, one way or the other: a bare
    /// window where the browser offers one, the plain URL where it does
    /// not.
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
