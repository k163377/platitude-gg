//! Syntax colours for the lines a diff shows.
//!
//! Two roads. A language with a grammar ([`grammar`]) has its file
//! parsed **whole** ([`tree`]) — no walk, no budget, correct from the
//! first paint, an order of magnitude faster than the lexer (measured).
//! Everything below describes the second road: the regex-lexer fallback
//! for languages no grammar claims.
//!
//! A patch is a few lines out of the middle of a file, and what those
//! lines mean depends on everything above them. So the file itself is
//! walked from line 1 down to each hunk: this side's rows — the context
//! lines and its own changes — read straight through that walk, and the
//! other side's changes read through a *fork* taken where the hunk
//! begins, stepped over the shared context lines so they stay in place.
//! A block comment that opened fifty lines up, a `class` two hundred
//! lines up — both are in the state by the time the hunk begins.
//!
//! What bounds the work is a per-patch line budget
//! (`patch::LEX_LINE_BUDGET`): everything the lexer reads — the walk
//! down to each hunk, both sides of every row — spends from one pool,
//! and rows past it go out plain. That is the whole of what a diff may
//! cost, whatever its shape (a 20,000-line rewrite measured 1.2s
//! unbounded).
//!
//! Without the file (it could not be read, it is too big, the language is
//! unknown) each hunk starts clean instead. That is not merely less
//! context: the top-of-file rules are *different* rules, so the hunk's
//! first line comes out coloured in a way the file itself never would
//! (measured, QML's `readonly property` reads as storage keywords
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

mod grammar;
mod patch;
mod theme;
mod tree;
mod walk;

#[cfg(test)]
mod testkit;

pub use patch::{colors, colors_cached, colors_quick, deep, knows};

/// Lexer states remembered at intervals down one file, so the next
/// reading of the same text starts near its hunks instead of at line 1.
///
/// Handed back by [`colors_cached`], and worth keeping wherever the same
/// file will be read again — which is every partial stage: staging moves
/// the index, not the worktree file the colours are read against, so the
/// re-read that follows every hunk staged walks text this cache has
/// already walked. It is honest about staleness on its own: the source
/// text's hash rides inside, and a cache built over different text is
/// simply not used.
pub struct LexCache {
    /// Hash of the source text the states were read down.
    source: u64,
    /// The name of the syntax the states were walked with. Text alone
    /// is not enough of a key: a `ParseState` is only valid for its own
    /// grammar, and the same bytes can stand under two names — a rename
    /// git did not pair, a file vendored twice — so a cache whose
    /// syntax disagrees is cleared rather than resumed.
    syntax: String,
    /// `(line index, the lexer's place before reading that line)`, in
    /// ascending order. Grown, never rewritten: entries past what this
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
        // Ascending order holds by construction: a walk only ever moves
        // forward, and re-walked ground already has its entries.
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
    /// `0xRRGGBB` as this type — the one conversion both palettes use,
    /// so a colour written as one number cannot drift from its bytes.
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

    /// Whether nothing anywhere in this diff was read. A question about
    /// the whole diff, which is not how a pane draws one: rows are
    /// coloured one at a time from [`Self::line`]'s spans, so nobody has
    /// to ask this to know what to do with a row. It answers for a
    /// colouring as a whole instead — tests, and any caller outside this
    /// crate wanting to know whether the set found anything at all.
    pub fn is_empty(&self) -> bool {
        self.patches
            .iter()
            .flatten()
            .all(|hunk| hunk.iter().all(LineColors::said_nothing))
    }
}
