//! Diff (patch) parser: unified, and the **combined** form git prints for
//! a path with more than one side.
//!
//! Classifies and numbers every line so the UI interprets nothing.
//! Commands must run with `--no-ext-diff` and the default `a/` `b/`
//! prefixes (the diff runners pin these).
//!
//! # Combined diffs
//!
//! A conflicted path has no single old side, so `git diff` compares the
//! working tree against every stage at once and prints one marker column
//! per parent (`diff --cc` / `@@@ -1,5 -1,5 +1,9 @@@`). git 2.55, every
//! shape in the tests:
//!
//! - The combined form appears only where both stages exist (`UU` and
//!   `AA`). With one side gone git says only `* Unmerged path <path>`,
//!   which parses to a [`FilePatch`] carrying [`FilePatch::unmerged`] and
//!   nothing else.
//! - A column holds `-` where that parent has the line and the result does
//!   not, `+` where the result has it and that parent does not, and a
//!   space otherwise. A line is therefore in the result unless some column
//!   says `-`.
//! - As in git's own colouring, any `+` makes the line an addition, any
//!   `-` a deletion, all-spaces context.
//! - `\ No newline at end of file` is never printed in this form, and a
//!   binary one says `Binary files differ` without naming either side.
//! - `git apply` refuses a combined patch, so nothing rebuilt from these
//!   bytes can be staged piece by piece.

mod combined;
mod header;
mod parse;
mod unified;

#[cfg(test)]
mod testkit;

pub use combined::side_of_markers;
pub use parse::parse_patch;

/// Classification of one diff content line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Context,
    Addition,
    Deletion,
    /// `\ No newline at end of file`
    NoNewline,
}

impl DiffLineKind {
    /// Whether a line of this kind is on the side `new_side` names —
    /// the side whose line numbers a reading counts in. `NoNewline` is
    /// on neither: it is git talking.
    pub fn on_side(self, new_side: bool) -> bool {
        match self {
            DiffLineKind::Context => true,
            DiffLineKind::Addition => new_side,
            DiffLineKind::Deletion => !new_side,
            DiffLineKind::NoNewline => false,
        }
    }
}

/// One rendered line of a hunk (text without its marker char).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    /// Line number on the old side (context/deletion). In a combined diff
    /// this is the **first** parent's numbering, present only on the lines
    /// that parent has.
    pub old_no: Option<u32>,
    /// Line number on the new side (context/addition).
    pub new_no: Option<u32>,
    pub text: String,
    /// The marker column of every parent, as git printed them (`" +"`,
    /// `"++"`, `"- "`). Empty for a unified diff, whose single marker is
    /// already said by [`DiffLine::kind`].
    pub markers: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    /// Ranges of the parents after the first, in git's order. Empty for a
    /// unified diff, which has exactly one old side.
    pub extra_old: Vec<(u32, u32)>,
    /// Function-context heading after the closing `@@` (may be empty).
    pub heading: String,
    pub lines: Vec<DiffLine>,
}

/// One file's patch within the parsed output.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FilePatch {
    /// Path on the old side (`None` for added files).
    pub old_path: Option<String>,
    /// Path on the new side (`None` for deleted files).
    pub new_path: Option<String>,
    pub is_binary: bool,
    /// The patch compares the result against more than one parent, so its
    /// lines carry [`DiffLine::markers`] and none of it can be staged
    /// (see the module note).
    pub is_combined: bool,
    /// git named the path unmerged and printed no patch (one side does not
    /// exist); the entry carries the path and nothing else.
    pub unmerged: bool,
    /// Which commit this patch is of, as a line to put above it: filled
    /// only where several commits' patches of a file stand one after
    /// another (`DiffTarget::Choice`, デザイン規約 §複数のコミットを選ぶ),
    /// empty otherwise.
    pub from_commit: String,
    pub hunks: Vec<DiffHunk>,
}

impl FilePatch {
    /// Display path: new side, falling back to old (deleted files).
    pub fn path(&self) -> &str {
        self.new_path
            .as_deref()
            .or(self.old_path.as_deref())
            .unwrap_or("")
    }
}
