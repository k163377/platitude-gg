//! The application's own half of a write: what the screen holds from the
//! press until the answer, and which answer puts it down.
//!
//! **Three owners, three questions.** What git may do with a repository is
//! core's to decide; what this window does with a press while the answer
//! is still out is decided here; where the focus goes and what is drawn is
//! QML's. Nothing in this module knows Qt or git — no `qtbridge`, no
//! session, no repository — so the transitions run under `cargo test` with
//! no engine to start and no repository to make.
//!
//! **A write is named by the id the queue accepted it under**
//! (`platitude_core::OperationId`, as the bridge carries it: a number that
//! is never zero, and `None` where the queue accepted nothing at all).
//! Nothing here counts answers or reads anything into the order they
//! arrive in — what answered in between is somebody else's.
//!
//! **Where an owner is held is decided by what it is holding.** The rows
//! a delete took off the screen are the tab's ([`StandIn`], kept per tab
//! by the hub — `hub::stand_in`): a page is built for the tab in front
//! and taken down behind it (`Main.qml`), and rows that are still gone
//! are not back because the thing drawing them went away. What does end
//! that is the session being let go, which the owner is told about
//! ([`StandIn::session_gone`]) — a different thing from the operation
//! ending, because what a session numbered goes with it. What is waited
//! for on behalf of one page's own boxes is that page's ([`Press`],
//! [`StashOut`], kept on the tab model the boxes belong to): a page that
//! went away took the text and the pane with it, and there is nothing
//! left to empty or to walk anybody off.

mod branch_delete_out;
#[cfg(test)]
mod branch_delete_out_tests;
mod diff_reread;
#[cfg(test)]
mod diff_reread_tests;
mod press;
#[cfg(test)]
mod press_tests;
mod push_out;
#[cfg(test)]
mod push_out_tests;
mod stand_in;
#[cfg(test)]
mod stand_in_tests;
mod stash_out;
#[cfg(test)]
mod stash_out_tests;

pub use branch_delete_out::BranchDeleteOut;
pub use diff_reread::DiffReread;
pub use press::Press;
pub use push_out::PushOut;
pub use stand_in::{Row, Rows, StandIn};
pub use stash_out::StashOut;
