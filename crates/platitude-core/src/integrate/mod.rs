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

pub use merge::{MergeOptions, merge};
pub use pick::{cherry_pick, revert};
pub use rebase::{RebaseOptions, RebaseOutcome, rebase};
pub use resolve::{Continuation, InProgress, resolve, resolve_current};

pub(crate) use rebase::{rebase_command, refusal_or};
