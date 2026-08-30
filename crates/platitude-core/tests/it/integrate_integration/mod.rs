//! Merge / rebase / cherry-pick / revert, the conflict flow they share, and
//! interactive rebase driven by the todo-editor helper.
//!
//! What they run git with lives in [`crate::support::integrate`]; a helper
//! used by one module only stays in that module.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
// An inner attribute here covers the child modules too.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

mod drop;
mod edits;
mod merge;
mod mergetool;
mod pick_revert;
mod plan;
mod publish;
mod rebase;
mod sides;
mod todo;
