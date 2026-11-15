//! `cargo xtask wrap` — the prose of this tree's markdown wrapped at one
//! width, and nothing else about it touched.
//!
//! A paragraph's lines are joined and broken again at 120 columns. A line
//! end only moves where what the reader sees stays as it was: CSS draws a
//! line end between two Japanese characters as nothing and any other as a
//! space (`width`), judging by the characters drawn either side of it
//! (`inline`). What is not a paragraph — code, tables, headings, HTML,
//! block quotes, front matter — is read as such (`blocks`) and left byte
//! for byte.

mod blocks;
mod cli;
mod document;
mod fill;
mod inline;
mod width;

pub(crate) use cli::{COMMANDS, note, run};

/// One paragraph of a document, as lines.
struct Paragraph {
    /// The first line's index.
    start: usize,
    /// One past the last line's index.
    end: usize,
    /// Where the text begins on the first line, which is also the column
    /// every other line of it is indented to: what comes before it is the
    /// indent and the markers of the list items it sits in.
    lead: usize,
}

impl Paragraph {
    const EMPTY: Self = Self {
        start: 0,
        end: 0,
        lead: 0,
    };
}
