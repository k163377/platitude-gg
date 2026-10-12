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

pub use merge::{MergeOptions, merge, refused_over_work, stopped_message};
pub use pick::{cherry_pick, revert};
pub use rebase::{
    RebaseOptions, RebaseOutcome, RebaseStop, rebase, rebase_progress, rebase_standing,
};
pub use resolve::{Continuation, InProgress, resolve, resolve_current};

pub(crate) use rebase::{UNTRACKED_IN_THE_WAY, landed, rebase_command, turned_down};

/// What git says when work in the working tree stands in the way of what
/// it would write — tracked changes or untracked files, under a merge, a
/// pull's merge, a checkout or a stash's apply (`LC_ALL=C`; unpack-trees'
/// and merge-ort's own words since long before the minimum).
pub(crate) const OVERWRITES_WORK: &str = "would be overwritten";

/// [`OVERWRITES_WORK`] as the words that make an exit 1 a refusal rather
/// than an answer (`GitCommand::except_when_it_says`): the row then keeps
/// git's list of the files in the way.
pub(crate) const REFUSED_OVER_WORK: &[&str] = &[OVERWRITES_WORK];

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
