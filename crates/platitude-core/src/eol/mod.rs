//! Line-ending notices, read from the bytes git printed.
//!
//! Nothing here converts anything: what lands in the index is git's
//! decision (`core.autocrlf`, `core.eol`, `.gitattributes`), and the notices
//! only say what it did. [`setting`] is the settings screen's alone; the
//! notices never offer a fix (デザイン規約 §改行コードの警告, which also
//! defines cases (a)–(d), see [`Reading`]).
//!
//! (a) and (b) are exact and settled here by [`read`]. (c) and (d) need to
//! know what the files around this one look like, which the patch cannot
//! say; [`read`] reports the shape and the caller pairs it with a baseline.
//!
//! # Why the patch bytes and not `ls-files --eol`
//!
//! A file that was already mixed before this change gets no notice; it
//! gets one the moment the change adds a line that disagrees with it. Only
//! the diff knows which lines are new.
//!
//! # What git hands over
//!
//! - The terminator is part of the content line: a CRLF line prints as
//!   `+one\r\n`.
//! - A whole-file flip prints every line as `-` then `+` with no context,
//!   which is what tells it apart from "some lines changed".
//! - `\ No newline at end of file` follows the line it is about and belongs
//!   to that line's side — old, new, or both.
//! - Under `core.autocrlf=true` a worktree turned to CRLF diffs empty (git
//!   compares in index space) and an untracked CRLF file renders through
//!   `--no-index` already normalised, so these cases mostly cannot arise —
//!   correctly. The exception is an index blob that already holds CRs:
//!   git converts nothing and the CRs are real data.

mod attrs;
mod notice;
mod read;
mod sample;
mod scan;
pub mod setting;
mod working_tree;

#[cfg(test)]
mod read_tests;

pub use attrs::{Ruling, normalises, ruling, rulings, rulings_given};
pub use notice::{Notice, settle};
pub use read::{Sighting, read, read_one};
pub use sample::{baseline, cache_key};
pub use working_tree::{Shape, untracked_shapes, working_tree_shapes};

/// A line terminator this module can name.
///
/// Old-Mac CR-only files are not a case: a file with no `\n` in it reads as
/// one line with no terminator, which is also how git counts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eol {
    Lf,
    Crlf,
}

impl Eol {
    /// Display spelling, as plain text — not a chip
    /// (デザイン規約 §改行コードの警告).
    pub fn as_str(self) -> &'static str {
        match self {
            Eol::Lf => "LF",
            Eol::Crlf => "CRLF",
        }
    }
}

/// What one file's patch says. (c) and (d) still need a baseline before
/// anything is shown; the rest are final.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    /// Nothing to say about this file.
    Quiet,
    /// (a) every line of an existing file changed its ending.
    Flipped { from: Eol, to: Eol },
    /// (b) the change adds lines that disagree with the file they land in.
    Mixed { lines: u32, added: Eol, file: Eol },
    /// (c) a file that did not exist before, whose endings are uniform.
    NewFile { eol: Eol },
    /// (d) a file that held no line ending at all now has one.
    FirstEnding { eol: Eol },
}

impl Reading {
    /// Whether this reading was decided by the patch bytes alone.
    ///
    /// History diffs show only these: (c) and (d) would need the neighbours
    /// as they stood at that commit, not the tree on disk.
    pub fn is_exact(self) -> bool {
        matches!(self, Reading::Flipped { .. } | Reading::Mixed { .. })
    }
}

/// Where the baseline's sample files came from, so the notice names only
/// the range it actually speaks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// Every sample shares this extension and sits in the same directory.
    Here(String),
    /// Every sample shares this extension.
    Ext(String),
    /// The samples have nothing in common but the repository.
    Repo,
}

/// What the files around a path look like. An estimate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Baseline {
    pub eol: Eol,
    pub scope: Scope,
}
