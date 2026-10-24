//! A place in a pane's text, in the two units a press passes through: a
//! place in the string the row's layout was handed ([`LayoutPosition`]),
//! and the byte of the text a copy takes that it stands for
//! ([`SourcePosition`]). Which conversion turns one into the other is the
//! model's own (`encode::markup::source_byte` for the diff,
//! `encode::markup::plain_byte` per column for the log).
//!
//! Signed, as the slots and the selection's ends are — the log keeps `-1`
//! for no row. Neither says the row or the offset exists; whoever reads
//! one answers for what it does with one that does not.

/// A byte of the text a row copies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SourcePosition {
    /// The model's row on screen, not a line number of the file.
    pub row: i32,
    /// A UTF-8 byte offset into the text that row copies.
    pub byte: i32,
}

/// A place in the string a row lays out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LayoutPosition {
    /// The model's row on screen, not a line number of the file.
    pub row: i32,
    /// A UTF-16 code unit offset into the string handed to the layout — not
    /// pixels, not characters, not a byte of the source.
    pub utf16: i32,
}
