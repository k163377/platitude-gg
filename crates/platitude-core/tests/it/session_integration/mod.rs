//! End-to-end RepoSession tests on real temp repositories.
//!
//! What the tests open and watch it with lives in [`crate::support::session`];
//! a helper used by one module only stays in that module.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
// An inner attribute here covers the child modules too.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

mod auto_fetch;
mod carry_move;
mod carry_rewrite;
mod command_log;
mod details;
mod details_order;
mod helper_binary;
mod pass_watch;
mod query;
mod rebuild;
mod standing_op;
mod stream;
mod window;
mod write_queue;
