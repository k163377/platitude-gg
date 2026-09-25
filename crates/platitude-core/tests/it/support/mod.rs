//! Integration-test support: the repositories a test builds, what it runs
//! git with, the sessions it opens, and the waits that judge one finished.

// `allow-*-in-tests` only covers `#[test]` fns; this covers every module below.
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

// Only the names the suite reaches as `crate::support::<name>`; the rest go
// through their module (`wait::QUIET_BUDGET`), since an unused re-export warns.
pub use exec::{Ends, Said};
pub use graph::{replay_graph, replay_rows};
pub use repo::{TestRepo, barrier_filter, barrier_hook, info};
pub use wait::Patience;
