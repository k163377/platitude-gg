//! Merge, rebase, cherry-pick and revert, plus the continue / abort / skip
//! routing they share.
//!
//! `--no-edit` is passed wherever git accepts it, though `GIT_EDITOR=true`
//! already prevents a hang: it keeps the intent visible in the logged
//! command line.

mod merge;
mod pick;
mod rebase;
mod resolve;

use crate::opstate;

pub use merge::{MergeOptions, merge, stopped_message};
pub use pick::{cherry_pick, revert};
pub use rebase::{
    RebaseOptions, RebaseOutcome, RebaseStop, rebase, rebase_progress, rebase_standing,
};
pub use resolve::{Continuation, InProgress, resolve, resolve_current};

pub(crate) use rebase::{landed, rebase_command};

/// Where an operation that can stop part-way came to rest. A conflict stop
/// is where an ordinary merge or pick ends up, so it must not read as a
/// failure; no exit code tells the two apart, so each command asks the
/// repository whether its operation is left standing.
///
/// `rebase` answers [`RebaseOutcome`] first (it also has the refusal a
/// stash gets past) and becomes one of these after the carry
/// (`session::build::rewrite_carrying`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Landing {
    /// git took the operation to the end: a commit, a fast-forward, a
    /// branch that was already in, or a sequence walked to its last step.
    Done,
    /// git stopped and left the operation standing. Nothing failed; the way
    /// on is the exit card (デザイン規約 §進行中の操作から出る).
    Stopped,
}
