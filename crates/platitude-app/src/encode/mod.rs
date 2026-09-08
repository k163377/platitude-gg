//! Pure encoding helpers: core DTOs → compact strings the QML layer decodes
//! mechanically (draw tokens / chip records).

mod columns;
mod graph;
mod labels;
mod markup;
mod rows;
mod select;
mod words;

#[cfg(test)]
mod rows_tests;

pub use columns::{any_wide, has_wide, widest_lines};
pub use graph::{avatar_code, conflict_side_colors, encode_geometry, tail_lanes};
pub(crate) use labels::pr_set;
pub use labels::{
    co_author_pairs, encode_co_authors, encode_labels, gone_keys, label_key, label_kind_word,
    label_name_of, label_names, labels_shown,
};
pub use markup::{display_ranges, hit_byte};
pub use rows::{DiffRow, flatten_patches, is_combined, is_new_file, is_unmerged_only};
pub use select::{diff_key, hunk_selection, worktree_target};
// `EndingWords` itself is not re-exported: every caller takes it off
// `ending_words` by inference, and a name nobody writes is an unused
// import here (`encode` is a private module, so a `pub use` in it is not
// a public re-export).
pub use words::{ending_words, human_size, rename_source};

/// Record separator for the packed lists QML unpacks itself: label chips
/// and co-authors. A refname cannot hold it (git refuses control
/// characters), and a person's name or address carrying this unprintable
/// would at worst split its own record — no real one does.
pub const RECORD_SEP: char = '\u{1f}';

/// Field separator inside one record — a chip's name from the remotes it
/// was read off, a co-author's name from their address. Present only when
/// there is a second field, and like [`RECORD_SEP`] it cannot occur in any
/// of those.
pub const FIELD_SEP: char = '\u{1e}';

/// The outer pair, for a list whose rows carry packed fields of their own:
/// the two above are spoken for *inside* such a row, a commit's co-author
/// cell being a `\u{1f}`/`\u{1e}` packing already
/// ([`labels::encode_co_authors`]). Picked for the same reason those two
/// were — no name, address, path or line of a message holds one.
pub const ROW_SEP: char = '\u{1d}';
pub const CELL_SEP: char = '\u{1c}';
