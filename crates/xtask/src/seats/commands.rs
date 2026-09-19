//! The seat verbs as the catalogue declares them (.claude/rules-refs/
//! structure.md §コマンドの正本): every document and every refusal that
//! names one spells it from here.

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

pub(crate) static TAKEOVER: command::Command = command::Command {
    id: "seat.takeover",
    call: "seat takeover <letter>",
    purpose: "move a letter to this session whoever holds it, tree and board as they stand",
    run_in: Where::Either,
    needs: &["the user's instruction, in so many words, to take that letter over"],
    permission: Permission::Escape(crate::hook::TAKEOVER_APPROVAL_FLAG),
};

pub(crate) static ROSTER: command::Command = command::Command {
    id: "seat.roster",
    call: "seats",
    purpose: "where the six seats stand — for reading, never for choosing one",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&TAKE, &RELEASE, &TAKEOVER, &ROSTER];
