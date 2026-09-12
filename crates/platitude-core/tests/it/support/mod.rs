//! Integration-test support: the repositories a test builds, what it runs
//! git with, the sessions it opens, and the waits that judge one finished.

// Test-only helper: panicking on setup failure is the desired behavior, but
// the `allow-*-in-tests` clippy options only cover `#[test]` functions.
// Applies to every module below it.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic, dead_code)]

pub mod busy;
pub mod exec;
pub mod graph;
pub mod integrate;
pub mod remote;
pub mod repo;
pub mod session;
pub mod stage;
pub mod wait;

// `crate::support::<name>` is where the suite has always reached for these.
// Everything else a module holds is reached through it (`wait::QUIET_BUDGET`)
// — re-exporting a name nothing calls for is an unused import, not a path
// anybody is keeping.
pub use exec::{Ends, Said};
pub use graph::{replay_graph, replay_rows};
pub use repo::{TestRepo, barrier_hook, info};
pub use wait::Patience;
