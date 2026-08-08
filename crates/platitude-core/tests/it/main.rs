//! All real-git integration tests, deliberately in one binary: `cargo test`
//! runs test binaries one after another, so separate files would serialize
//! on the wall clock and each would pay its own link. One binary shares a
//! single parallel test-thread pool across the whole suite.
//!
//! Run a subset by name (names are prefixed with the module):
//! `cargo test -p platitude-core --test it session_integration`

mod support;

mod commit_branch_integration;
mod details_diff;
mod edge_strings;
mod exec;
mod identity_integration;
mod integrate_integration;
mod logparse;
mod preview_integration;
mod reachable_integration;
mod refs_integration;
mod remote_stash_integration;
mod remote_tags_integration;
mod rename_integration;
mod session_integration;
mod stage_integration;
mod worktree_state;
