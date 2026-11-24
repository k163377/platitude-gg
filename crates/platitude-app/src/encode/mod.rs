//! Pure shaping helpers: core DTOs → what the QML layer reads, in the
//! shapes `wire` defines — nothing crosses the bridge as a string to be
//! taken apart.

mod checkout;
mod columns;
mod counts;
mod discards;
mod far;
mod graph;
mod identity;
mod labels;
mod landing;
mod markup;
mod mates;
mod menu;
mod metatype;
mod push;
mod remote;
mod routing;
mod rows;
mod select;
mod text_end;
mod tree;
mod wire;
mod words;

#[cfg(test)]
mod tests;

pub use checkout::SwitchMove;
pub use columns::{Candidates, widest_lines};
pub use counts::shown_count;
pub use discards::{DiscardRows, discard_key, discard_rows, look_word, shown_matches};
pub(crate) use far::{put_far, read_far};
pub use graph::{Lanes, avatar_code, conflict_side_colors, lanes_of, spell_lanes, tail_lanes};
pub use identity::Identity;
pub(crate) use labels::pr_set;
pub use labels::{Chips, chips_of, chips_shown, gone_keys, named_kind_word, ref_kind_word};
pub use landing::{Landed, Landing};
pub use markup::{
    Runs, plain_byte, plain_char, plain_ranges, source_byte, source_char, spelled_place,
    spelled_ranges,
};
pub use mates::{Mates, mates_of};
pub use menu::{CommitMenu, RefMenu, WorktreeMaking};
pub use push::PushRecord;
pub use routing::{KeyedFile, Routing, file_key, file_of_key};
pub use rows::{
    DiffRow, LineMarks, Marks, SplitRow, flatten_after, flatten_patches, is_combined, is_new_file,
    is_unmerged_only, mode_change, pair_rows, whole_only_why,
};
pub use select::{diff_key, hunk_selection, worktree_target};
pub use text_end::{Ended, TextEnd};
pub use tree::{Reading, Standing, TreeReading};
pub(crate) use wire::{
    Fields, Listed, One, Optional, Record, field, oid_arg, oid_text, optional_record,
    optional_text, pinned_oid,
};
// `EndingWords` is not re-exported: no caller names it, and in this private
// module a `pub use` nobody names is an unused import.
pub use words::{ending_words, human_size, rename_source};
