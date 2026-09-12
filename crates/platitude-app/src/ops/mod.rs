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
//! Held per tab by the hub rather than by the page that draws it
//! (`hub::stand_in`), so the state of a write outlives whichever QML
//! component happens to be showing it: a page is built for the tab in
//! front and taken down behind it (`Main.qml`), and an operation that is
//! still out is not over because the thing drawing it went away. What does
//! end it is the session being let go, which is a decision the owner is
//! told about (`StandIn::session_gone`) — and a different one from the
//! operation itself ending, because what a session numbered goes with it.

mod stand_in;
#[cfg(test)]
mod stand_in_tests;

pub use stand_in::{Row, Rows, StandIn};
