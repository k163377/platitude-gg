//! The task-chip guard (`spawn_task` / `dismiss_task`): the number a chip
//! leads with, the paths two chips may not both claim, and the ledger
//! both are judged against.
//!
//! A chip is read in a list and worked in a session of its own, so two
//! things about it are not the chip's own business: **where it stands**
//! among the others, and **what it will touch** that another chip has
//! already claimed. Both were left to attention and both slipped — chips
//! arrived at the bottom of a list they should have led, and two chips
//! arrived for one file, whose sessions then had to be untangled.
//!
//! So the harness holds them. Every chip this session stacked is
//! remembered beside the primary checkout's `.git`, and:
//!
//! * a title leads with its priority (`1. …`), 1 being the most
//!   important chip live right now;
//! * a chip that asks the user to decide wears `[任意]` after its number
//!   — it weighs less than one recommending work, and the list is read
//!   for which is which;
//! * a chip may not claim a path a live chip already claims — the two
//!   are one chip, or the first stacks the second itself once its own
//!   work (or the decision it is waiting on) is done;
//! * a turn does not end while the live set fails to read 1..N;
//! * and a turn does not end saying in prose that work is left undone
//!   while stacking no chip at all (`leftovers`).
//!
//! The ledger is this session's alone. A chip the user has since started
//! or dropped by hand is invisible from here, and a session that ends
//! takes its ledger with it — which is what keeps a dead session's chips
//! from numbering a live one's.

use std::collections::BTreeSet;

mod guard;
mod ledger;
mod leftovers;

pub(crate) use guard::{post_chip, pre_spawn, stop as numbering};
pub(crate) use ledger::session_end;
pub(crate) use leftovers::stop as leftovers;

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
    /// decide, rather than one recommending work.
    optional: bool,
    /// The paths its words named — what the next chip may not claim too.
    targets: BTreeSet<String>,
}
