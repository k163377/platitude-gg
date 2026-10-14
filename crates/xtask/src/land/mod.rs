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
    needs: &["the branch checked out in a seat, with nothing uncommitted"],
    permission: Permission::Permit,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&LAND];
