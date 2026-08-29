//! `cargo xtask shots` — the shot board: one page per checkout that
//! outlives the window it was read in.
//!
//! A screenshot pasted into a chat pane arrives shrunk to the pane's
//! width and cannot be zoomed there, so a 1440x900 window is unreadable
//! and shooting it larger changes nothing — the pane fits whatever it is
//! given. The board is the way out: pictures land on a page that opens
//! in a window of its own, magnifies to exact integer ratios, and is
//! still there after it is closed.
//!
//! Every seat writes to the *same* board, beside the primary checkout's
//! `.git`, and each run carries the seat that took it. A picture from
//! another seat's tree says nothing about this one, so a board that did
//! not name the seat would be a board nobody could trust.
//!
//! And it is read in *one* window, however many seats and sessions put
//! pictures on it: the page is rewritten where it stands, so the window
//! already open is one F5 away from the newest run (`window.rs`).

mod board;
mod cli;
mod page;
mod sweep;
mod window;

pub(crate) use board::{record, record_dir, shown};
pub(crate) use cli::run;
pub(crate) use sweep::{seat_freed, session_ended};

/// One `add`: the pictures taken for one thing, under the name that says
/// what to look at in them.
pub(crate) struct Run {
    /// What these pictures show, in the words of whoever took them. The
    /// board refuses a run without one: several pictures under no name
    /// leave nobody able to say which file was which change.
    pub(crate) label: String,
    /// The verify-ui verb behind them, when there was one.
    pub(crate) verb: String,
    /// The tree that took them: a roster letter a-f, or `main` for the
    /// primary checkout.
    pub(crate) seat: String,
    /// The session that took them, empty when nobody was named. It is
    /// what a session's own runs are found by when it ends (`sweep`):
    /// the seat cannot answer for that, because a seat outlives every
    /// session that passes through it and the next one is already
    /// sitting there.
    pub(crate) session: String,
    /// Milliseconds since the epoch. The page formats it — it has a
    /// calendar, and xtask depends on std alone (CLAUDE.md 技術スタック).
    pub(crate) at: u128,
    /// Whether the pictures are read *abreast* — one view holding them
    /// all in a row under one zoom — rather than one at a time.
    ///
    /// What a before/after is for: shown one after the other, the reader
    /// carries the first picture in their head while looking at the
    /// second, and the difference is whatever they remember rather than
    /// whatever changed. Side by side under one
    /// magnifier there is nothing to remember.
    pub(crate) side_by_side: bool,
    pub(crate) shots: Vec<Shot>,
}

/// One picture on the board.
pub(crate) struct Shot {
    /// Board-relative path. The page points at the files rather than
    /// carrying them, so index.html stays a few KB however many runs
    /// pile up behind it.
    pub(crate) file: String,
    /// What the file was called where it was taken (app.png, overlay.png)
    /// — the name that ties it back to the run that produced it.
    pub(crate) from: String,
    /// What this one picture is, when the run holds more than one and
    /// says so (`before` / `after`). Empty everywhere else: a run whose
    /// pictures are read one at a time is named by its label, and a
    /// caption over every shot would only repeat it.
    pub(crate) caption: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
}
