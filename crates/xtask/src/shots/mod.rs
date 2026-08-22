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

mod board;
mod cli;
mod page;
mod sweep;

pub(crate) use board::{record, record_dir, shown};
pub(crate) use cli::run;

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
    /// Milliseconds since the epoch. The page formats it — it has a
    /// calendar, and xtask depends on std alone (CLAUDE.md 技術スタック).
    pub(crate) at: u128,
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
    pub(crate) width: u32,
    pub(crate) height: u32,
}
