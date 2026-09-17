//! In a file of its own because seats.rs stands at the length backstop
//! (.claude/rules/structure.md §長さの閾値); a seat's operations
//! belong to the seat module.

use crate::command::{self, Permission, Where};

pub(crate) static TAKE: command::Command = command::Command {
    id: "seat.take",
    call: "seat",
    purpose: "hand this session a worktree seat, claimed behind the lock",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static RELEASE: command::Command = command::Command {
    id: "seat.release",
    call: "seat release",
    purpose: "hand this session's seat back without landing it",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static ROSTER: command::Command = command::Command {
    id: "seat.roster",
    call: "seats",
    purpose: "where the six seats stand — for reading, never for choosing one",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&TAKE, &RELEASE, &ROSTER];
