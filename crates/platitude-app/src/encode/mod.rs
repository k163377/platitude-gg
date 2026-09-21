//! Pure shaping helpers: core DTOs → what the QML layer reads. A record
//! goes over as a JS object with named fields, a list of them as a JS
//! array, and a plain list of names as a JS array of strings
//! (`wire`); nothing crosses the bridge as a string to be taken apart.

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

pub use columns::{Candidates, widest_lines};
pub use graph::{Lanes, avatar_code, conflict_side_colors, lanes_of, spell_lanes, tail_lanes};
pub(crate) use labels::pr_set;
pub use labels::{Chips, chips_of, chips_shown, gone_keys, ref_kind_word};
pub use landing::{Landed, Landing};
pub use markup::{Runs, plain_byte, plain_ranges, source_byte, spelled_ranges};
pub use mates::{Mates, mates_of};
pub use rows::{
    DiffRow, LineMarks, Marks, SplitRow, flatten_patches, is_combined, is_new_file,
    is_unmerged_only, pair_rows,
};
pub use select::{diff_key, hunk_selection, worktree_target};
pub(crate) use wire::{Fields, Listed, One, Optional, Record, field};
// `EndingWords` itself is not re-exported: every caller takes it off
// `ending_words` by inference, and a name nobody writes is an unused
// import here (`encode` is a private module, so a `pub use` in it is not
// a public re-export).
pub use words::{ending_words, human_size, rename_source};
