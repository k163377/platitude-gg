//! What may run at once on this machine, across every seat: one budget,
//! one queue, and a landing at the head of it.
//!
//! A gate is not one process's worth of load. It builds, it runs the
//! workspace's tests, it runs the verify-ui verbs several at a time, and
//! it does all of that on both sides at once — and the seats gate
//! together, so the machine sees as many of those as there are seats.
//! The verbs alone used to be counted (the lanes this replaces): a lock
//! per side, one per verb, so however many gates ran, the side's verbs
//! were the machine's count between them. Everything else — the cargo
//! that compiles, the tests, the container's half — was outside the
//! count, and three seats put three chains of `cargo test` on the
//! machine beside three gates' worth of apps.
//!
//! So the count is over the whole of it. Every unit a gate runs takes a
//! **ticket** out of one machine-wide budget before it starts, and gives
//! it back when it ends; a unit is one step, which is the granularity
//! the runner already has (`gate::run_one`). Both sides draw on the same
//! budget, because both sides are the same machine.
//!
//! **What a ticket weighs** is what its unit takes of the machine, not
//! what it is allowed: a weight is admission control and never a cap
//! (cargo has its own parallelism inside, and nothing here bounds a
//! rustc's threads or a container's memory). Two weights are enough to
//! say what a gate runs ([`weight_of`]).
//!
//! **The budget is one gate's worth**, computed the same way in every
//! process on the machine ([`demand`]) — so a single gate never waits
//! for itself and nothing gets slower when it is alone, and the second
//! seat's gate shares that one gate's worth rather than adding another.
//!
//! **A landing goes first** (CLAUDE.md Git 運用): its units sort ahead
//! of every other seat's, and while one of them is waiting for room, no
//! ordinary unit is handed what frees — the rule and its consequences
//! are [`queue`]. Nothing is ever killed or suspended to make room: a
//! landing waits out the units already running, which is why how long
//! one unit runs is worth measuring (`gate::record`).
//!
//! **Landings queue against each other** on a turn ([`Pool::turn`]),
//! which is FIFO and outside the budget: one landing at a time goes
//! through rebase, gate, census and fast-forward.
//!
//! ## The ledger
//!
//! Beside the repository's own `.git`, which every worktree shares:
//!
//! ```text
//! pg-budget/
//!   admit.lock        one decision at a time; every read and write of
//!                     the ledger happens under it
//!   seq               the arrival counter
//!   t-<seq>.ticket    one live ticket
//!   t-<seq>.lock      held by its owner for the ticket's whole life
//! ```
//!
//! Liveness is the lock, as it is in `still` and `lanes`: a ticket whose
//! lock nobody holds belongs to a process that is gone, and the next
//! process to look at the ledger takes it away under that lock. So a
//! gate killed mid-run — or a whole session's — leaves nothing anybody
//! waits for and nothing double-counted, and there is no manager process
//! whose death would strand the machine: the ledger is the manager, and
//! every participant keeps it.
//!
//! ## What waits for what
//!
//! A unit takes its ticket **before** it announces itself to a
//! measurement (`still::busy`, inside `check::run_step`) and never the
//! other way about, so the order across the runner is: the tree's one
//! gate (`lanes::sole`, which refuses rather than waits) → a landing's
//! turn → a ticket → the measurement's hold. Nothing that holds a
//! ticket is waited for by `still::hold`'s side of it, so the order has
//! no cycle; and a unit holds exactly one ticket, taken whole, so
//! nothing ever waits while holding part of what it needs.
//!
//! A child of an admitted unit is under its parent's ticket and takes
//! none of its own ([`under`]).
//!
//! ## Where it lives
//!
//! [`queue`] is the rule and touches no file; [`ledger`] is what the
//! rule reads — the pool, the tickets and the locks under them;
//! [`unit`] is what asks — what a unit weighs, what it holds, and what
//! it says about the process it started; [`cli`] is `cargo xtask
//! budget`, and [`bench`] the measurement the rule was chosen with.

mod bench;
mod cli;
mod ledger;
mod queue;
#[cfg(test)]
mod tests;
mod unit;

pub use cli::run;
pub(crate) use ledger::{Pool, probe};
pub(crate) use queue::Rank;
pub(crate) use unit::{
    Admitted, Ask, COMPILE, HELD, LIGHT, child_started, demand, standalone, under, watched,
    weight_of,
};
