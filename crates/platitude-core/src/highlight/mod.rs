//! Syntax colours for the lines a diff shows.
//!
//! Two roads. A language with a grammar ([`grammar`]) has its file
//! parsed whole ([`tree`]) — no walk, no budget. Everything below
//! describes the regex-lexer fallback, for languages no grammar claims
//! and for a file that has outgrown its grammar ([`tree::reads`]).
//!
//! What a hunk's lines mean depends on everything above them, so the file
//! is walked from line 1 down to each hunk: this side's rows read straight
//! through that walk, the other side's changes through a fork taken where
//! the hunk begins and stepped over the shared context lines. The work is
//! bounded by one per-patch line budget (`patch::LEX_LINE_BUDGET`); rows
//! past it go out plain.
//!
//! Without the file each hunk starts cold, and its first lines can come
//! out coloured as the file never would: top-of-file rules differ.
//!
//! # Conflicts
//!
//! The two sides between git's markers are alternatives: read one after
//! the other, a `/*` left open in `ours` paints `theirs` and everything
//! below. So markers are read as structure — the state is saved where the
//! region opens, put back at each divide, and the file goes on where
//! `ours` left off (the side the branch being worked on stands on).
//! Markers match exactly (seven characters, then the end of the line or a
//! space and a label) and only inside a combined diff, the only form git
//! prints for a path it stopped on; anywhere else a row of `=` is text.

mod grammar;
mod patch;
mod theme;
mod tree;
mod walk;

#[cfg(test)]
mod testkit;

pub use patch::{colors, colors_cached, colors_quick, deep, knows};

/// Lexer states remembered at intervals down one file, so the next
/// reading of the same text starts near its hunks.
///
/// Handed back by [`colors_cached`]; worth keeping wherever the same file
/// is read again (every partial stage re-reads the unchanged worktree
/// file). A cache built over different text is not used — the source
/// hash rides inside.
pub struct LexCache {
    /// Hash of the source text the states were read down.
    source: u64,
    /// The syntax the states were walked with: a `ParseState` is only
    /// valid for its own grammar, and the same bytes can stand under two
    /// names (a rename git did not pair), so a cache whose syntax
    /// disagrees is cleared.
    syntax: String,
    /// `(line index, the lexer's place before reading that line)`, in
    /// ascending order. Grown only: entries past what this
    /// reading walks stay for the deeper hunk a later reading may have.
    states: Vec<(usize, walk::LineState)>,
}

impl LexCache {
    /// The deepest checkpoint at or below `target` that is ahead of
    /// `current` — where a walk standing at `current` may jump to.
    fn jump(&self, current: usize, target: usize) -> Option<&(usize, walk::LineState)> {
        self.states
            .iter()
            .rev()
            .find(|(at, _)| *at <= target && *at > current)
    }

    /// Remembers the lexer's place before reading line `at`, unless this
    /// line already has one.
    fn record(&mut self, at: usize, state: Option<walk::LineState>) {
        let Some(state) = state else {
            return;
        };
        match self.states.binary_search_by_key(&at, |(a, _)| *a) {
            Ok(_) => {}
            Err(i) => self.states.insert(i, (at, state)),
        }
    }
}

/// A colour as the theme gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    /// `0xRRGGBB` as this type — the one conversion both palettes use.
    pub(crate) const fn of(rgb: u32) -> Self {
        Self {
            r: ((rgb >> 16) & 0xff) as u8,
            g: ((rgb >> 8) & 0xff) as u8,
            b: (rgb & 0xff) as u8,
        }
    }
}

/// One run of a line the theme paints in a single colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    /// Length of the run in bytes of [`crate::parse::diff::DiffLine`]'s
    /// text, the string the row draws.
    pub len: usize,
    pub color: Rgb,
}

/// What was read off one line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineColors {
    /// The theme's runs, empty where the line has none.
    pub spans: Vec<Span>,
    /// One of git's conflict markers, which the pane draws as
    /// scaffolding. Read whether or not the language is known.
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
/// patch, which hunk, which line of it. A binary or unmerged patch, or an
/// unknown language outside a combined diff, has no hunks here;
/// [`Self::line`] answers nothing for them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffColors {
    pub(super) patches: Vec<PatchColors>,
}

impl DiffColors {
    /// What was read off one line — nothing, where nothing was. Borrowed:
    /// asked once per row.
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

    /// Whether nothing anywhere in this diff was read. A pane never needs
    /// it (rows colour one at a time from [`Self::line`]).
    pub fn is_empty(&self) -> bool {
        self.patches
            .iter()
            .flatten()
            .all(|hunk| hunk.iter().all(LineColors::said_nothing))
    }
}
