//! A place in a pane's text, in the two units a press passes through: a
//! place in the string the row's layout was handed ([`LayoutPosition`]),
//! and the byte of the text a copy takes that it stands for
//! ([`SourcePosition`]). Which conversion turns one into the other is the
//! model's own (`encode::markup::source_byte` for the diff,
//! `encode::markup::plain_byte` per column for the log).
//!
//! Unsigned. The slots hand signed values, and `new` reads them once:
//! one that is negative makes no position. A position says nothing of
//! whether its row or offset exists — the model that reads one answers
//! for that.

/// A byte of the text a row copies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct SourcePosition {
    /// The model's row on screen, not a line number of the file.
    pub row: usize,
    /// A UTF-8 byte offset into the text that row copies.
    pub byte: usize,
}

impl SourcePosition {
    /// None where either is negative.
    pub(super) fn new(row: i32, byte: i32) -> Option<Self> {
        Some(Self {
            row: usize::try_from(row).ok()?,
            byte: usize::try_from(byte).ok()?,
        })
    }
}

/// A place in the string a row lays out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LayoutPosition {
    /// The model's row on screen, not a line number of the file.
    pub row: usize,
    /// A UTF-16 code unit offset into the string handed to the layout — not
    /// pixels, not characters, not a byte of the source.
    pub utf16: usize,
}

impl LayoutPosition {
    /// None where either is negative.
    pub(super) fn new(row: i32, utf16: i32) -> Option<Self> {
        Some(Self {
            row: usize::try_from(row).ok()?,
            utf16: usize::try_from(utf16).ok()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_negative_row_or_offset_makes_no_position() {
        assert_eq!(SourcePosition::new(-1, 0), None, "a row before the rows");
        assert_eq!(SourcePosition::new(0, -1), None, "a byte before the line");
        assert_eq!(
            SourcePosition::new(2, 3),
            Some(SourcePosition { row: 2, byte: 3 })
        );
        assert_eq!(LayoutPosition::new(-1, 0), None, "a row before the rows");
        assert_eq!(LayoutPosition::new(0, -1), None, "a place before the line");
        assert_eq!(
            LayoutPosition::new(2, 3),
            Some(LayoutPosition { row: 2, utf16: 3 })
        );
    }
}
