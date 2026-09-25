//! What may run at once on this machine, across every seat: one budget,
//! one queue, and a landing at the head of it
//! (internal-docs/反映前テストの機械化.md §機械の予算と優先キュー).
//!
//! Every unit a gate runs — one step (`gate::step::run_one`), on either
//! side — takes a **ticket** out of one machine-wide budget before it
//! starts and gives it back when it ends. Counting the verbs alone would
//! leave the cargo, the tests and the container's half outside the count,
//! and seats gating together would stack them.
//!
//! A ticket's weight is admission control, not a bound on a rustc's
//! threads ([`weight_of`]). The budget is one gate's worth, computed the
//! same way in every process ([`demand`]), so a gate alone never waits
//! for itself.
//!
//! A landing's units sort first ([`queue`]), but nothing is preempted:
//! room comes from running units finishing, so a landing waits them out
//! (`gate::record`). Landings queue against each other on a FIFO turn
//! outside the budget ([`Pool::turn`]).
//!
//! ## The ledger
//!
//! Beside the repository's own `.git`, which every worktree shares:
//!
//! ```text
//! pgg-budget/
//!   admit.lock        one decision at a time; every read and write of
//!                     the ledger happens under it
//!   seq               the arrival counter
//!   served            each seat's running total, by rank
//!   t-<seq>           one live ticket
//!   t-<seq>.lock      held by its owner for the ticket's whole life
//! ```
//!
//! Liveness is the lock, as in `still` and `lanes`: a ticket whose lock
//! nobody holds belongs to a gone process, and the next process to read
//! the ledger takes it away. No manager process exists whose death could
//! strand the machine.
//!
//! ## What waits for what
//!
//! The tree's one gate (`lanes::sole`, which refuses) → a landing's turn
//! → a ticket → the announcement to a measurement (`still::busy`, inside
//! `check::run_step`). `still::hold`'s side takes no ticket, and a unit
//! takes exactly one ticket whole, so there is no cycle and nothing waits
//! holding part of what it needs.
//!
//! A child of an admitted unit is under its parent's ticket and takes
//! none of its own ([`under`]).
//!
//! [`queue`] is the rule and touches no file; [`ledger`] is the pool, the
//! tickets and their locks; [`unit`] is what a unit weighs, holds and says
//! about the process it started; [`cli`] is `cargo xtask budget`.

mod cli;
mod ledger;
mod queue;
#[cfg(test)]
mod tests;
mod unit;

use crate::command::{self, Permission, Where};

pub(crate) static BUDGET: command::Command = command::Command {
    id: "budget.standing",
    call: "budget",
    purpose: "what the machine's one budget is doing, and the queue behind it",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&BUDGET];

pub use cli::run;
pub(crate) use ledger::{Pool, probe};
pub(crate) use queue::Rank;
pub(crate) use unit::{
    Admitted, Ask, COMPILE, HELD, LIGHT, child_started, demand, standalone, under, watched,
    weight_of,
};
