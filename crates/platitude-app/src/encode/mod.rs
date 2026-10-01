//! Pure shaping helpers: core DTOs → what the QML layer reads, in the
//! shapes `wire` defines — nothing crosses the bridge as a string to be
//! taken apart.

mod columns;
mod graph;
mod labels;
mod landing;
mod markup;
mod mates;
mod rows;
mod select;
mod wire;
mod words;

#[cfg(test)]
mod rows_tests;
#[cfg(test)]
mod wire_qml;

pub use columns::{Candidates, widest_lines};
pub use graph::{Lanes, avatar_code, conflict_side_colors, lanes_of, spell_lanes, tail_lanes};
pub(crate) use labels::pr_set;
pub use labels::{Chips, chips_of, chips_shown, gone_keys, menu_kind_word, ref_kind_word};
pub use landing::{Landed, Landing};
pub use markup::{Runs, plain_byte, plain_ranges, source_byte, spelled_ranges};
pub use mates::{Mates, mates_of};
pub use rows::{
    DiffRow, LineMarks, Marks, SplitRow, flatten_patches, is_combined, is_new_file,
    is_unmerged_only, pair_rows,
};
pub use select::{diff_key, hunk_selection, worktree_target};
pub(crate) use wire::{Fields, Listed, One, Optional, Record, field};
// `EndingWords` is not re-exported: no caller names it, and in this private
// module a `pub use` nobody names is an unused import.
pub use words::{ending_words, human_size, rename_source};
