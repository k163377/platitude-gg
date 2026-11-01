//! `cargo xtask shots` — the shot board: one page per checkout that
//! outlives the window it was read in.
//!
//! A chat pane shrinks a pasted screenshot to its width and cannot zoom
//! it; the board is a page in a window of its own that magnifies to exact
//! integer ratios and stays after it is closed.
//!
//! Every seat writes to the same board, beside the primary checkout's
//! `.git`, and each run names the seat that took it — a picture from
//! another seat's tree says nothing about this one. `open` puts it in a
//! window when the user asks for it (`window.rs`, `hook::board`).
//!
//! Runs are read top down in the order they were put up (verify-ui
//! SKILL.md「board は上から下へ、載せた順に読む」).

mod board;
mod cli;
mod crop;
mod page;
mod sweep;
mod taken;
mod window;

pub(crate) use board::{put_up_since, record, record_dir, shown, written_label};
pub(crate) use cli::run;
pub(crate) use sweep::{seat_freed, seat_reused};
pub(crate) use taken::{Run, Shot};

use crate::command::{self, Permission, Where};

pub(crate) static OPEN: command::Command = command::Command {
    id: "shots.open",
    call: "shots open",
    purpose: "the board's window, when the user asks for it",
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
