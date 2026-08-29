//! Staging: whole files, and hunk/line subsets via rebuilt partial patches.
//!
//! The two halves ask different questions of git and refuse for different
//! reasons, so they are kept apart: [`whole`] runs one command over a list
//! of paths, [`partial`] rebuilds a patch out of the diff a selection was
//! made on, and [`refusal`] is what the second one says when the file it
//! was made on has moved.

mod partial;
mod refusal;
mod whole;

pub use self::partial::{apply_partial, discard_partial, hunk_count};
pub use self::whole::{
    DiscardSide, discard_chosen, discard_to_head, discard_worktree, remove_untracked, stage_all,
    stage_paths, unstage_all, unstage_paths,
};
