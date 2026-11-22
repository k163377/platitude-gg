//! A place in a pane's text, in the two units a press passes through: a
//! place in the string the row's layout was handed ([`LayoutPosition`]),
//! and the byte of the text a copy takes that it stands for
//! ([`SourcePosition`]). Which conversion turns one into the other is the
//! model's own (`encode::markup::source_byte` for the diff,
//! `encode::markup::plain_byte` per column for the log). A selection is
//! two of the latter ([`SelectionEnds`]) and the word a double-click took
//! ([`Selection`]), which a drag grows by the same rule in both
//! ([`Selection::dragged_to`]).
//!
//! Unsigned. The slots hand signed values, and `new` reads them once:
//! one that is negative makes no position. A position says nothing of
//! whether its row or offset exists — the model that reads one answers
//! for that.

use std::ops::{Range, RangeInclusive};

/// A byte of the text a row copies. Ordered as the text is read: by row,
/// then by byte within it (the derive follows the fields' order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
    fn cut_to(self, len: usize) -> Self {
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
}

/// The two ends of a selection as the hand made them: `anchor` where the
/// press went down (or the double-clicked word's far edge,
/// [`SelectionEnds::grown_by_word`]), `focus` where the drag has carried
/// the other end. Kept
/// in that order — a drag made upwards has its focus first in reading
/// order — and sorted only where reading order is asked for. Two ends on
/// one position are a selection holding nothing, not no selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SelectionEnds {
    pub anchor: SourcePosition,
    pub focus: SourcePosition,
}

impl SelectionEnds {
    /// The word over `bytes` of row `row` — what a double-click selects —
    /// anchored at its start.
    fn word(row: usize, bytes: Range<usize>) -> Self {
        Self {
            anchor: SourcePosition {
                row,
                byte: bytes.start,
            },
            focus: SourcePosition {
                row,
                byte: bytes.end,
            },
        }
    }

    /// The ends a drag reaching `position` gives the double-clicked word
    /// `self`, as an editor grows one: the word stays whole, and the far
    /// end snaps out to the edge of the word it lands in — that word's
    /// start going back, its end going on — or stands where it is, on a
    /// boundary between two. `across` answers the word `position` cuts in
    /// two on its row's line (`platitude_core::word::run_across`).
    fn grown_by_word(
        self,
        position: SourcePosition,
        across: impl FnOnce() -> Option<Range<usize>>,
    ) -> Self {
        let (first, last) = self.ordered().into_inner();
        let (anchor, byte) = if position >= last {
            (first, across().map_or(position.byte, |run| run.end))
        } else if position < first {
            (last, across().map_or(position.byte, |run| run.start))
        } else {
            return self;
        };
        Self {
            anchor,
            focus: SourcePosition { byte, ..position },
        }
    }

    /// The ends in reading order, first to last, and the positions between
    /// them. Not the bytes a copy takes, whose last end is not taken
    /// (`bytes_on`).
    pub(super) fn ordered(self) -> RangeInclusive<SourcePosition> {
        self.anchor.min(self.focus)..=self.anchor.max(self.focus)
    }

    /// What row `row`, a line `len` bytes long, holds of the text between
    /// the ends: its bytes, each end cut to the line — empty where the row
    /// only meets an end. None for a row outside them. Sorted before the
    /// cut, which takes the same bytes: the cut never reorders two bytes of
    /// one row, and across rows the row decides.
    pub(super) fn bytes_on(self, row: usize, len: usize) -> Option<Range<usize>> {
        let (first, last) = self.ordered().into_inner();
        if row < first.row || row > last.row {
            return None;
        }
        let start = if row == first.row {
            first.cut_to(len).byte
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

    /// The same text's ends once `gone` rows before them have been taken
    /// away — none where either end's row was among them.
    pub(super) fn shifted_up(self, gone: usize) -> Option<Self> {
        Some(Self {
            anchor: self.anchor.shifted_up(gone)?,
            focus: self.focus.shifted_up(gone)?,
        })
    }
}

/// A selection as the hand holds it: its two ends, and the word a
/// double-click took, which the drag behind it grows word by word. One
/// value, replaced whole — a word stands only beside the ends it gave or
/// the ends grown from it, and goes when they go. Which pane it is of, and
/// which column of it, is the model's to hold beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Selection {
    ends: SelectionEnds,
    /// None for a selection a plain press or a row taken whole started.
    clicked_word: Option<SelectionEnds>,
}

impl Selection {
    /// What a press lays: nothing yet, at `position`.
    pub(super) fn pressed(position: SourcePosition) -> Self {
        Self::over(SelectionEnds {
            anchor: position,
            focus: position,
        })
    }

    /// What a double-click takes: the word over `bytes` of row `row`, kept
    /// for the drag behind it to grow by.
    pub(super) fn word(row: usize, bytes: Range<usize>) -> Self {
        let word = SelectionEnds::word(row, bytes);
        Self {
            ends: word,
            clicked_word: Some(word),
        }
    }

    /// A stretch taken whole, with no word to grow by.
    pub(super) fn over(ends: SelectionEnds) -> Self {
        Self {
            ends,
            clicked_word: None,
        }
    }

    pub(super) fn ends(self) -> SelectionEnds {
        self.ends
    }

    /// The same ends with no word to grow by: a double-click that took
    /// none.
    pub(super) fn wordless(self) -> Self {
        Self::over(self.ends)
    }

    /// The selection a drag reaching `position` leaves: the far end follows
    /// the pointer place by place, or — behind a double-click — word by
    /// word (`SelectionEnds::grown_by_word`, which `across` answers for).
    pub(super) fn dragged_to(
        self,
        position: SourcePosition,
        across: impl FnOnce() -> Option<Range<usize>>,
    ) -> Self {
        let ends = match self.clicked_word {
            Some(clicked) => clicked.grown_by_word(position, across),
            None => SelectionEnds {
                focus: position,
                ..self.ends
            },
        };
        Self { ends, ..self }
    }

    /// The same selection once `gone` rows before it have been taken away —
    /// none where either end's row was among them. The word lies between
    /// the ends, so it moves with them.
    pub(super) fn shifted_up(self, gone: usize) -> Option<Self> {
        Some(Self {
            ends: self.ends.shifted_up(gone)?,
            clicked_word: match self.clicked_word {
                Some(word) => Some(word.shifted_up(gone)?),
                None => None,
            },
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

    fn ends(anchor: SourcePosition, focus: SourcePosition) -> SelectionEnds {
        SelectionEnds { anchor, focus }
    }

    #[test]
    fn the_ends_read_in_order_whichever_way_the_drag_went() {
        let (down, up) = (ends(at(1, 3), at(3, 2)), ends(at(3, 2), at(1, 3)));
        assert_eq!(down.ordered(), at(1, 3)..=at(3, 2));
        assert_eq!(up.ordered(), down.ordered(), "a drag made upwards");
        assert_eq!(ends(at(2, 4), at(2, 4)).ordered(), at(2, 4)..=at(2, 4));
    }

    #[test]
    fn a_row_holds_the_bytes_between_the_ends() {
        let run = ends(at(1, 3), at(3, 2));
        assert_eq!(run.bytes_on(1, 10), Some(3..10), "the first row");
        assert_eq!(run.bytes_on(2, 10), Some(0..10), "a row between");
        assert_eq!(run.bytes_on(3, 10), Some(0..2), "the last row");
        assert_eq!(
            ends(at(2, 3), at(2, 7)).bytes_on(2, 10),
            Some(3..7),
            "one row"
        );
        assert_eq!(run.bytes_on(0, 10), None, "before the run");
        assert_eq!(run.bytes_on(4, 10), None, "after it");
        assert_eq!(
            ends(at(3, 2), at(1, 3)).bytes_on(1, 10),
            Some(3..10),
            "a drag made upwards"
        );
        assert_eq!(
            ends(at(2, 4), at(2, 4)).bytes_on(2, 10),
            Some(4..4),
            "a press with no drag"
        );
    }

    #[test]
    fn an_end_past_its_line_is_cut_to_it() {
        // The first row then holds none.
        assert_eq!(ends(at(1, 3), at(3, 2)).bytes_on(1, 2), Some(2..2));
        assert_eq!(ends(at(2, 3), at(2, 99)).bytes_on(2, 5), Some(3..5));
        assert_eq!(ends(at(2, 99), at(2, 3)).bytes_on(2, 5), Some(3..5));
    }

    #[test]
    fn a_word_grows_out_to_the_edges_of_the_words_a_drag_reaches() {
        let word = SelectionEnds::word(2, 4..8);
        assert_eq!(word, ends(at(2, 4), at(2, 8)), "anchored at its start");
        // Into a word on a later row, which runs 3..9 there.
        assert_eq!(
            word.grown_by_word(at(3, 5), || Some(3..9)),
            ends(at(2, 4), at(3, 9)),
            "on to that word's end"
        );
        // Back into one before it, running 0..3.
        assert_eq!(
            word.grown_by_word(at(2, 1), || Some(0..3)),
            ends(at(2, 8), at(2, 0)),
            "back to its start, the word kept whole"
        );
        assert_eq!(
            word.grown_by_word(at(2, 10), || None),
            ends(at(2, 4), at(2, 10)),
            "a boundary between two words is where it stands"
        );
        assert_eq!(
            word.grown_by_word(at(2, 6), || panic!("not asked inside the word")),
            word,
            "inside the word itself"
        );
    }

    #[test]
    fn a_drag_goes_place_by_place_until_a_word_is_taken() {
        let pressed = Selection::pressed(at(2, 4));
        assert_eq!(pressed.ends(), ends(at(2, 4), at(2, 4)), "nothing yet");
        assert_eq!(
            pressed
                .dragged_to(at(3, 5), || panic!("no word to grow by"))
                .ends(),
            ends(at(2, 4), at(3, 5))
        );

        let word = Selection::word(2, 4..8);
        let grown = word.dragged_to(at(3, 5), || Some(3..9));
        assert_eq!(grown.ends(), ends(at(2, 4), at(3, 9)), "to that word's end");
        assert_eq!(
            grown.dragged_to(at(2, 6), || None),
            word,
            "back inside the word it grew from, which it still holds"
        );
        assert_eq!(
            grown
                .wordless()
                .dragged_to(at(3, 5), || panic!("the word was let go"))
                .ends(),
            ends(at(2, 4), at(3, 5)),
            "let go of, the ends stay and the drag is plain"
        );
    }

    #[test]
    fn a_selection_moves_up_with_its_word_or_goes_whole() {
        let grown = Selection::word(2, 4..8).dragged_to(at(3, 5), || Some(3..9));
        assert_eq!(
            grown.shifted_up(2),
            Some(Selection::word(0, 4..8).dragged_to(at(1, 5), || Some(3..9)))
        );
        assert_eq!(
            grown.shifted_up(3),
            None,
            "the word's row went, an end's too"
        );
    }

    #[test]
    fn rows_taken_from_above_move_both_ends_or_neither() {
        let run = ends(at(4, 1), at(2, 5));
        assert_eq!(run.shifted_up(2), Some(ends(at(2, 1), at(0, 5))));
        assert_eq!(run.shifted_up(3), None, "the focus's row went");
        assert_eq!(ends(at(2, 5), at(4, 1)).shifted_up(3), None, "the anchor's");
    }
}
