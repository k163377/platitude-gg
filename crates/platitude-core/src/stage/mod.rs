//! Staging: [`whole`] runs one command over a list of paths, [`partial`]
//! rebuilds a patch out of the diff a selection was made on, and
//! [`refusal`] is what the second one says when that file has moved.

mod partial;
mod refusal;
mod whole;

pub use self::partial::{apply_partial, discard_partial, hunk_count};
pub use self::whole::{
    DiscardSide, discard_chosen, discard_rows, discard_to_head, discard_working_tree,
    remove_untracked, stage_all, stage_paths, unstage_all, unstage_paths, with_old_names,
};
