//! `cargo xtask land [<branch>]` — the one way a branch reaches main
//! (steps: internal-docs/反映前テストの機械化.md §land).
//!
//! A merge typed in the primary checkout inherits whatever HEAD is there:
//! a detached HEAD or a stray branch fast-forwards the wrong thing or
//! strands the commits. This verb reads where main actually is and picks
//! the safe move. Its fast-forward is what spends the permit
//! (hook/permit.rs).

use crate::command::{self, Permission, Where};

mod approval;
mod leftovers;
pub(crate) mod line;
mod record;
mod run;
mod tags;
mod versions;

pub(crate) use run::run;
// The landing's record reads the clock as the landing says it.
use run::clock;

pub(crate) static LAND: command::Command = command::Command {
    id: "land.branch",
    call: "land <branch>",
    purpose: "rebase, gate and fast-forward main onto a branch — the one sanctioned way",
    run_in: Where::Seat,
    needs: &[
        "the branch checked out in a seat, with nothing uncommitted",
        "this seat at the head of the line of landings",
    ],
    permission: Permission::Permit,
};

/// `cargo xtask land line`: the line and this session's place in it.
pub(crate) static LINE: command::Command = command::Command {
    id: "land.line",
    call: "land line",
    purpose: "the line of landings and this session's place in it",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

/// `cargo xtask land wait next`: ends when this seat is next or the head.
pub(crate) static WAIT_NEXT: command::Command = command::Command {
    id: "land.wait-next",
    call: "land wait next",
    purpose: "wait, spending nothing, until this seat is next in the line of landings",
    run_in: Where::Either,
    needs: &["the user's /land, which put this seat in the line"],
    permission: Permission::Plain,
};

/// `cargo xtask land wait head`: ends when this seat is the head.
pub(crate) static WAIT_HEAD: command::Command = command::Command {
    id: "land.wait-head",
    call: "land wait head",
    purpose: "wait, spending nothing, until this seat is the head of the line of landings",
    run_in: Where::Either,
    needs: &["the user's /land, which put this seat in the line"],
    permission: Permission::Plain,
};

/// `cargo xtask land leave <why>`: this seat leaves the line.
pub(crate) static LEAVE: command::Command = command::Command {
    id: "land.leave",
    call: "land leave <why>",
    purpose: "leave the line of landings — what stopped this seat is the user's to settle",
    run_in: Where::Either,
    needs: &["a landing this seat found it cannot make without the user"],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&LAND, &LINE, &WAIT_NEXT, &WAIT_HEAD, &LEAVE];
