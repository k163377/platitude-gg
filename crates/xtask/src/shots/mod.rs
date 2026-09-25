//! `cargo xtask shots` — the shot board: one page per checkout that
//! outlives the window it was read in.
//!
//! A chat pane shrinks a pasted screenshot to its width and cannot zoom
//! it; the board is a page in a window of its own that magnifies to exact
//! integer ratios and stays after it is closed.
//!
//! Every seat writes to the same board, beside the primary checkout's
//! `.git`, and each run names the seat that took it — a picture from
//! another seat's tree says nothing about this one. It is read in one
//! window: the page is rewritten in place, so an open window is one F5
//! from the latest (`window.rs`).
//!
//! Runs are read top down in the order they were put up (verify-ui
//! SKILL.md「board は上から下へ、載せた順に読む」).

mod board;
mod cli;
mod crop;
mod page;
mod sweep;
mod window;

pub(crate) use board::{record, record_dir, shown, written_label};
pub(crate) use cli::run;
pub(crate) use sweep::{seat_freed, seat_reused};

use crate::command::{self, Permission, Where};

pub(crate) static OPEN: command::Command = command::Command {
    id: "shots.open",
    call: "shots open",
    purpose: "the board's one window — already open, it is one F5 from the latest run",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static PRUNE: command::Command = command::Command {
    id: "shots.prune",
    call: "shots prune",
    purpose: "take this seat's runs off the board when its own rules will not",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&OPEN, &PRUNE];

/// One `add`: the pictures taken for one thing, under the name that says
/// what to look at in them.
pub(crate) struct Run {
    /// What these pictures show, in the board's language
    /// (`board::written_label`). Required: pictures under no name leave
    /// nobody able to say which file was which change.
    pub(crate) label: String,
    /// The verify-ui verb behind them, when there was one.
    pub(crate) verb: String,
    /// A roster letter a-f, or `main` for the primary checkout — the whole
    /// of who a run belongs to. Not the session: a seat outlives its
    /// sessions, and a run stands for the work it was taken for (`sweep`).
    pub(crate) seat: String,
    /// Milliseconds since the epoch; the page formats it (it has a
    /// calendar, xtask is std only).
    pub(crate) at: u128,
    /// Read abreast in one view under one zoom — what a before/after
    /// needs: one after the other, the difference is whatever the reader
    /// remembers.
    pub(crate) side_by_side: bool,
    pub(crate) shots: Vec<Shot>,
}

/// One picture on the board.
pub(crate) struct Shot {
    /// Board-relative: the page points at the files, so index.html stays
    /// small however many runs pile up behind it.
    pub(crate) file: String,
    /// The file's name where it was taken (app.png, overlay.png).
    pub(crate) from: String,
    /// The word a run read abreast puts over this picture (`before` /
    /// `after`, or a `--part` word). Empty otherwise: a caption over every
    /// shot would only repeat the label.
    pub(crate) caption: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
}
