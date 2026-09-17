//! Line-ending notices, read from the bytes git printed.
//!
//! Nothing in this module converts anything: what lands in the index is
//! git's decision (`core.autocrlf`, `core.eol`, `.gitattributes`), and the
//! notices only say what that decision did.
//!
//! **The one setting this app writes lives here too** ([`setting`]), and it
//! belongs to the settings screen alone — a reader who goes there went to
//! change it. The notices only report: an offer to fix itself,
//! attached to a warning, is where every other GUI's line-ending
//! accident starts (デザイン規約 §改行コードの警告).
//!
//! Four cases produce a notice (デザイン規約 §改行コードの警告):
//!
//! | | case | decided by |
//! |---|---|---|
//! | (a) | an existing file's endings flip | the patch bytes — exact |
//! | (b) | the change makes a file mixed, or more mixed | the patch bytes — exact |
//! | (c) | a new file does not match its neighbours | a sample — an estimate |
//! | (d) | a file that had no ending gains its first | a sample — an estimate |
//!
//! (a) and (b) are settled here by [`read`]. (c) and (d) need to know what
//! the files around this one look like, which the patch cannot say; [`read`]
//! reports the shape it found and the caller pairs it with a baseline.
//!
//! # Why the patch bytes and not `ls-files --eol`
//!
//! `ls-files --eol` answers "what does this file look like now", which is
//! not the question. A file that was already mixed before anyone touched it
//! is not this change's fault and gets no notice; the same file gets one the
//! moment the change adds a line that disagrees with it. Only the diff knows
//! which lines are new.
//!
//! # What git hands over (measured, git 2.55)
//!
//! - The terminator is part of the content line: a CRLF line prints as
//!   `+one\r\n`, so the `\r` sits immediately before the `\n`.
//! - A whole-file flip prints every line as `-` then `+` with no context —
//!   which is what makes "the endings changed" distinguishable from "some
//!   lines changed".
//! - `\ No newline at end of file` follows the line it is about and belongs
//!   to whichever side that line was on. It appears for the old side, the
//!   new side, or both.
//! - Under `core.autocrlf=true` a worktree turned to CRLF produces an
//!   **empty** diff (git compares in index space) while status still calls
//!   the file modified, and an untracked CRLF file renders through
//!   `--no-index` with its CRs already normalised away. So on that setting
//!   these cases mostly cannot arise — correctly, because git is converting
//!   and there is nothing to warn about. The exception is an index blob that
//!   already holds CRs, where git converts nothing and the CRs are real data.

mod attrs;
mod notice;
mod read;
mod sample;
mod scan;
pub mod setting;
mod worktree;

#[cfg(test)]
mod read_tests;

pub use attrs::{Ruling, normalises, ruling, rulings, rulings_given};
pub use notice::{Notice, settle};
pub use read::{Sighting, read, read_one};
pub use sample::{baseline, cache_key};
pub use worktree::{Shape, untracked_shapes, worktree_shapes};

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
    /// Display spelling. **Plain text** — chips are lowercase
    /// monospace, which an all-caps abbreviation does not fit
    /// (デザイン規約 §git 用語のコード表記).
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
    /// History diffs show only these: (c) and (d) would need the files
    /// **around** the changed one as they stood at that commit, which is a
    /// different tree from the one on disk.
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
