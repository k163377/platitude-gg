//! The application's own half of a write: what the screen holds from the
//! press until the answer, and which answer puts it down.
//!
//! Plain Rust — no `qtbridge`, session or repository — so the transitions
//! run under a bare `cargo test`.
//!
//! A write is named by the id the queue accepted it under
//! (`platitude_core::OperationId` as the bridge carries it: never zero, and
//! `None` where the queue accepted nothing). The id is the whole of the
//! match — whatever else answered is somebody else's.
//!
//! Where an owner is held follows what it holds. The rows a delete took off
//! the screen are the tab's ([`StandIn`], kept per tab by the hub —
//! `hub::stand_in`), because a page is taken down behind the tab in front
//! (`Main.qml`) while the rows stay gone. They end with the session
//! ([`StandIn::session_gone`]), not the operation, since what a session
//! numbered goes with it. What one page's own boxes wait on is that page's
//! ([`Press`], [`TreeOut`], kept on the tab model the boxes belong to): a
//! page that went away has nothing left to empty or walk anybody off.
//!
//! Bringing one more operation here takes three steps that only work
//! together: the slot that asks for the write records what is pending
//! itself, under the id the queue accepted (no announcing signal such as
//! `deleting(kind, id)`); the answer is handed out by that id from
//! `drain::settle_write` (no guessing from the order of `writeSeq`); and
//! the owner is held where what it holds lives (above). `Press` is the
//! base: an owner that remembers more holds one and adds its own part
//! (`TreeOut`, `BranchDeleteOut`); `StandIn` and `DiffReread` are other
//! kinds. QML is left with handing out the picture and saying that an edge
//! came (`listingDrawn`, `takeStashLanding`, `takeCommitLanding`,
//! `takeDiffRead`).

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
mod tree_out;
#[cfg(test)]
mod tree_out_tests;

pub use branch_delete_out::BranchDeleteOut;
pub use diff_reread::DiffReread;
pub use press::Press;
pub use push_out::PushOut;
pub use stand_in::{Row, Rows, StandIn};
pub use tree_out::TreeOut;
