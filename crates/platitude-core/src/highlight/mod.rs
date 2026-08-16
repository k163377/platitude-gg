//! Syntax colours for the lines a diff shows.
//!
//! A patch is a few lines out of the middle of a file, and what those
//! lines mean depends on everything above them. So the file itself is
//! walked from line 1 down to each hunk and the reading is *forked*
//! there: the hunk's rows are coloured from where the file stands, and
//! the file's own walk carries on to the next hunk. A block comment that
//! opened fifty lines up, a `class` two hundred lines up — both are in
//! the state by the time the hunk begins.
//!
//! Without the file (it could not be read, it is too big, the language is
//! unknown) each hunk starts clean instead. That is not merely less
//! context: the top-of-file rules are *different* rules, so the hunk's
//! first line comes out coloured in a way the file itself never would
//! (2026-08-13 実測: QML's `readonly property` reads as storage keywords
//! at file scope and as plain identifiers inside an `Item {}`, so the
//! first line of every hunk disagreed with the rest of it).
//!
//! # Conflicts
//!
//! The working tree of a conflicted file has git's markers in it, and the
//! two sides between them are **alternatives rather than a sequence**:
//! read one straight after the other, a lexer carries whatever `ours`
//! left open — a `/*`, a `"""` — into `theirs`, and everything below is
//! painted as the inside of something that is not there. So the markers
//! are read as structure. The state is saved where the region opens, put
//! back at each divide, and what the file goes on in is where **ours**
//! left off: that is the side the branch being worked on is standing on,
//! and where the two disagree nothing further down is certain anyway.
//!
//! Marker lines are matched exactly — seven characters, then the end of
//! the line or a space and a label — and **only inside a combined diff**,
//! which is the only form git prints for a path it stopped on. A row of
//! `=` under a heading in Markdown, or a page of documentation quoting a
//! marker, is text like any other.

mod patch;
mod theme;
mod walk;

#[cfg(test)]
mod testkit;

pub use patch::colors;
pub use theme::knows;

/// A colour as the theme gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// One run of a line the theme paints in a single colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    /// Length of the run in **bytes** of [`crate::parse::diff::DiffLine`]'s
    /// text — the same string the row will draw, so the runs can be laid
    /// over it without measuring anything twice.
    pub len: usize,
    pub color: Rgb,
}

/// What was read off one line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineColors {
    /// The theme's runs, empty where the line has none.
    pub spans: Vec<Span>,
    /// One of git's conflict markers — the fence around a region rather
    /// than anything the file says, so the pane draws it as scaffolding.
    /// Read whether or not the language is one the set knows: a conflict
    /// in a plain text file has the same fences.
    pub fence: bool,
}

impl LineColors {
    pub(super) fn nothing() -> Self {
        Self::default()
    }

    pub(super) fn said_nothing(&self) -> bool {
        self.spans.is_empty() && !self.fence
    }
}

type HunkColors = Vec<LineColors>;
type PatchColors = Vec<HunkColors>;

/// Colours for one diff, addressed the way its rows already are: which
/// patch, which hunk, which line of it.
///
/// Every patch and every line gets an entry even where there is nothing
/// to say — a binary file, a language nothing was found for — so the
/// shape always matches the diff it was made from and a caller can walk
/// the two together without counting.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffColors {
    pub(super) patches: Vec<PatchColors>,
}

impl DiffColors {
    /// What was read off one line — nothing, where nothing was. Borrowed
    /// rather than handed over: the caller asks once per row of a diff
    /// that can be tens of thousands of rows long.
    pub fn line(&self, patch: usize, hunk: usize, line: usize) -> &LineColors {
        static NOTHING: LineColors = LineColors {
            spans: Vec::new(),
            fence: false,
        };
        self.patches
            .get(patch)
            .and_then(|p| p.get(hunk))
            .and_then(|h| h.get(line))
            .unwrap_or(&NOTHING)
    }

    /// Whether nothing in this diff was read — the pane then draws its
    /// rows uncoloured.
    pub fn is_empty(&self) -> bool {
        self.patches
            .iter()
            .flatten()
            .all(|hunk| hunk.iter().all(LineColors::said_nothing))
    }
}
