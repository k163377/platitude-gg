//! Pure encoding helpers: core DTOs → compact strings the QML layer decodes
//! mechanically (draw tokens / chip records).

mod columns;
mod graph;
mod labels;
mod markup;
mod rows;
mod select;
mod words;

pub use columns::widest_columns;
pub use graph::{avatar_code, conflict_side_colors, encode_geometry, tail_lanes};
pub(crate) use labels::fake_pr_set;
pub use labels::{
    co_author_pairs, encode_co_authors, encode_labels, gone_keys, label_key, label_kind_word,
    label_name_of, label_names, labels_head, labels_shown,
};
pub use rows::{DiffRow, flatten_patches, is_combined, is_new_file, is_unmerged_only};
pub use select::{diff_key, hunk_selection, worktree_target};
// `EndingWords` itself is not re-exported: every caller takes it off
// `ending_words` by inference, and a name nobody writes is an unused
// import here (`encode` is a private module, so a `pub use` in it is not
// a public re-export).
pub use words::{ending_words, human_size, image_data_url, rename_source};

/// Record separator for the packed lists QML unpacks itself: label chips
/// and co-authors. Neither a refname, a person's name nor an address can
/// hold it.
pub const RECORD_SEP: char = '\u{1f}';

/// Field separator inside one record — a chip's name from the remotes it
/// was read off, a co-author's name from their address. Present only when
/// there is a second field, and like [`RECORD_SEP`] it cannot occur in any
/// of those.
pub const FIELD_SEP: char = '\u{1e}';
