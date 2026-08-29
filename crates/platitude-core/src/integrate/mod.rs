//! Merge, rebase, cherry-pick and revert, plus the continue / abort / skip
//! routing they share.
//!
//! All four can stop halfway and leave the repository mid-operation; that is
//! normal, not an error. The caller reports git's message and re-reads
//! [`crate::opstate`] to find out where things stand.
//!
//! `--no-edit` is passed wherever git accepts it. The process layer already
//! pins `GIT_EDITOR=true` so nothing can hang on an editor, but being
//! explicit keeps the intent visible in the command line we log.

mod merge;
mod pick;
mod rebase;
mod resolve;

use crate::opstate;

pub use merge::{MergeOptions, merge, stopped_message};
pub use pick::{cherry_pick, revert};
pub use rebase::{RebaseOptions, RebaseOutcome, rebase};
pub use resolve::{Continuation, InProgress, resolve, resolve_current};

pub(crate) use rebase::{landed, rebase_command};

/// Where an operation that can stop part-way came to rest.
///
/// Reading the two apart is the whole point: an operation that stops on a
/// conflict is where merging a branch that moved on — or copying a commit
/// onto one — normally ends up, and calling that a failure puts a red line
/// over an ordinary afternoon.
///
/// **No exit code tells them apart**, so each command below asks the
/// repository instead: the operation left standing is the stop, and
/// anything else that exited non-zero is the failure it looks like.
///
/// `rebase` answers [`RebaseOutcome`] first, because it has a third
/// thing to say — the refusal a stash gets past. It becomes one of these
/// once the carry is behind it (`session::build::rewrite_carrying`), and
/// a stopped rebase lands exactly where the other three do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Landing {
    /// git took the operation to the end: a commit, a fast-forward, a
    /// branch that was already in, or a sequence walked to its last step.
    Done,
    /// git stopped and left the operation standing — its marker, the
    /// markers in the tree, the conflicted rows. Nothing failed; the way
    /// on is the exit card (デザイン規約 §進行中の操作から出る).
    Stopped,
}
