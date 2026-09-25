//! The task-chip guard (`spawn_task` / `dismiss_task`): what may become
//! a chip at all, the number one leads with, the paths a chip holds
//! alone, and the ledger they are judged against.
//!
//! Every chip this session stacked is remembered beside the primary
//! checkout's `.git`, and:
//!
//! * what this session already has open is this session's to finish
//!   (`at_hand`);
//! * a title leads with its priority (`1. …`), 1 being the most
//!   important chip live right now;
//! * a chip that asks the user to decide wears `[任意]` after its number;
//! * a path a live chip already claims stays that chip's — the two
//!   are one chip, or the first stacks the second itself once its own
//!   work (or the decision it is waiting on) is done;
//! * a turn ends only once the live set reads 1..N.
//!
//! The ledger is this session's alone: a chip the user has since started
//! or dropped by hand is invisible from here, and a session that ends
//! takes its ledger with it, so a dead session's chips never number a
//! live one's.

use std::collections::BTreeSet;

mod at_hand;
mod guard;
mod ledger;

pub(crate) use guard::{post_chip, pre_spawn, stop as numbering};
pub(crate) use ledger::session_end;

/// One live chip, as the ledger remembers it.
struct Chip {
    /// The harness's task id — what `dismiss_task` takes.
    id: String,
    /// The number its title leads with; 1 is the most important.
    priority: usize,
    /// The title without its number and mark. A re-stack keeps the body
    /// and changes the rest, which is how it is told from a second chip.
    body: String,
    /// Whether it wears the weight mark (asks the user to decide).
    optional: bool,
    /// The paths its words named — held against the next chip's.
    targets: BTreeSet<String>,
}
