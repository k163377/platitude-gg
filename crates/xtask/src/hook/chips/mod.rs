//! The task-chip guard (`spawn_task` / `dismiss_task`): what may become
//! a chip at all, the number one leads with, the paths a chip holds
//! alone, and the ledger they are judged against.
//!
//! A chip is read in a list and worked in a session of its own, so three
//! things about it are not the chip's own business: **whether it is
//! anybody else's work**, **where it stands** among the others, and
//! **what it will touch** that another chip has already claimed. All
//! three were left to attention and all three slipped — the work a
//! session was in the middle of went out as chips for other sessions to
//! redo, chips arrived at the bottom of a list they should have led, and
//! two chips arrived for one file, whose sessions then had to be
//! untangled.
//!
//! So the harness holds them. Every chip this session stacked is
//! remembered beside the primary checkout's `.git`, and:
//!
//! * what this session already has open is this session's to finish — a
//!   second branch over the same file buys a conflict and a rebase for
//!   nothing (`at_hand`);
//! * a title leads with its priority (`1. …`), 1 being the most
//!   important chip live right now;
//! * a chip that asks the user to decide wears `[任意]` after its number
//!   — it weighs less than one recommending work, and the list is read
//!   for which is which;
//! * a path a live chip already claims stays that chip's — the two
//!   are one chip, or the first stacks the second itself once its own
//!   work (or the decision it is waiting on) is done;
//! * a turn ends only once the live set reads 1..N.
//!
//! The ledger is this session's alone. A chip the user has since started
//! or dropped by hand is invisible from here, and a session that ends
//! takes its ledger with it — which is what keeps a dead session's chips
//! from numbering a live one's.

use std::collections::BTreeSet;

mod at_hand;
mod guard;
mod ledger;

pub(crate) use guard::{post_chip, pre_spawn, stop as numbering};
pub(crate) use ledger::session_end;

/// One live chip, as the ledger remembers it.
struct Chip {
    /// The harness's task id — what `dismiss_task` takes, and the only
    /// handle a re-stack has on the chip it replaces.
    id: String,
    /// The number its title leads with; 1 is the most important.
    priority: usize,
    /// The title with its number and its mark taken off: what the chip
    /// *is*, held apart from where it stands and from what it weighs. A
    /// re-stack keeps the body and changes the rest, which is how it is
    /// told from a second chip for the same work.
    body: String,
    /// Whether it wears the weight mark: a chip asking the user to
    /// decide.
    optional: bool,
    /// The paths its words named — held against the next chip's.
    targets: BTreeSet<String>,
}
