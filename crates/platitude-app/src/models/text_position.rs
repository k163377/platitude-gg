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

use std::ops::Range;

/// A byte of the text a row copies. Ordered as the text is read: by row,
/// then by byte within it (the derive follows the fields' order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
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

    /// This position cut to a line `len` bytes long: a byte past its end is
    /// the end.
    pub(super) fn cut_to(self, len: usize) -> Self {
        Self {
            byte: self.byte.min(len),
            ..self
        }
    }

    /// The same text's position once `gone` rows before it have been taken
    /// away — none where its own row was among them.
    pub(super) fn shifted_up(self, gone: usize) -> Option<Self> {
        Some(Self {
            row: self.row.checked_sub(gone)?,
            ..self
        })
    }

    /// What row `row`, a line `len` bytes long, holds of the text running
    /// from this position to `last` (not before it in reading order): its
    /// bytes, each end cut to the line — empty where the row only meets an
    /// end. None for a row outside the run.
    pub(super) fn bytes_until(self, last: Self, row: usize, len: usize) -> Option<Range<usize>> {
        if row < self.row || row > last.row {
            return None;
        }
        let start = if row == self.row {
            self.cut_to(len).byte
        } else {
            0
        };
        let end = if row == last.row {
            last.cut_to(len).byte
        } else {
            len
        };
        Some(start..end)
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

    fn at(row: usize, byte: usize) -> SourcePosition {
        SourcePosition { row, byte }
    }

    #[test]
    fn positions_are_read_row_first() {
        assert!(
            at(1, 9) < at(2, 0),
            "an earlier row comes first, whatever the byte"
        );
        assert!(at(2, 3) < at(2, 4));
        assert_eq!(at(2, 4).min(at(1, 9)), at(1, 9));
    }

    #[test]
    fn a_position_cut_to_its_line_stops_at_the_end() {
        assert_eq!(at(2, 9).cut_to(4), at(2, 4), "past the end is the end");
        assert_eq!(at(2, 3).cut_to(4), at(2, 3), "inside it stays");
    }

    #[test]
    fn rows_taken_from_above_move_a_position_up() {
        assert_eq!(at(5, 3).shifted_up(2), Some(at(3, 3)));
        assert_eq!(
            at(2, 3).shifted_up(2),
            Some(at(0, 3)),
            "down to the first row"
        );
        assert_eq!(at(1, 3).shifted_up(2), None, "its own row went");
    }

    #[test]
    fn a_row_holds_the_bytes_between_the_ends_it_carries() {
        let (first, last) = (at(1, 3), at(3, 2));
        assert_eq!(first.bytes_until(last, 1, 10), Some(3..10), "the first row");
        assert_eq!(first.bytes_until(last, 2, 10), Some(0..10), "a row between");
        assert_eq!(first.bytes_until(last, 3, 10), Some(0..2), "the last row");
        assert_eq!(at(2, 3).bytes_until(at(2, 7), 2, 10), Some(3..7), "one row");
        assert_eq!(first.bytes_until(last, 0, 10), None, "before the run");
        assert_eq!(first.bytes_until(last, 4, 10), None, "after it");
        // An end past its line is cut to it; the first row then holds none.
        assert_eq!(first.bytes_until(last, 1, 2), Some(2..2));
        assert_eq!(at(2, 3).bytes_until(at(2, 99), 2, 5), Some(3..5));
    }
}
