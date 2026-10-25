//! The reader's own selection of the diff's text: where it runs, what
//! each row draws of it, and what it hands the clipboard
//! (デザイン規約 §diff の中身をコピーする). It lives here because only
//! `shown` holds the file's own bytes (rules-refs/app-ui.md
//! 「本文の選択とコピー」).
//!
//! **Column**: `0` is the rows' own lines (the only one as one column, the
//! old side when split), `1` the new side of a split row; a drag is of one
//! column (デザイン規約 §diff を 2 列で読む). Its wash goes on `sel` /
//! `pair_sel`.
//!
//! **Place**: a position in the line as the row spells it, asked of the
//! row's own layout (`LineRuler`) and turned into a byte and back here
//! (`encode::markup::source_byte` / `spelled_ranges`) — no font measuring,
//! no column counting.
//!
//! **The copy comes from the file's bytes**: a coloured row's `text` has
//! its tabs spelled out as `&nbsp;` (`encode::markup`), so a copy off the
//! screen would paste spaces.

use std::ops::{Range, RangeInclusive};

use super::*;

/// One row's share of the selection before it is spelled out — so the rows
/// a drag sweeps on every move compare without building strings.
#[derive(Clone, PartialEq, Eq)]
enum Wash {
    /// Not in the selection, or in it but not a line the plain copy takes.
    None,
    /// In it from end to end.
    Whole,
    /// In it over these bytes of the row's source line — empty for an end
    /// row the selection only touches the edge of.
    Part(Range<usize>),
}

/// The new side of a split row (the find bar's marks read it too, `find`).
pub(super) const RIGHT: i32 = 1;

impl DiffModel {
    /// The source line (the file's own bytes) column `side` of row `row`
    /// draws.
    pub(super) fn source_line(&self, side: i32, row: usize) -> Option<&str> {
        let item = self.lines.get(row)?;
        let line = if side == RIGHT {
            item.pair_line
        } else {
            item.line
        };
        // The row carries its place as QML's signed roles, -1 for none.
        let location = DiffLineLocation {
            patch: usize::try_from(item.patch).ok()?,
            hunk: usize::try_from(item.hunk).ok()?,
            line: usize::try_from(line).ok()?,
        };
        Some(&location.line_in(self.shown.as_ref()?)?.text)
    }

    /// The kind of the line one side of a row holds — `""` for nothing
    /// there.
    fn kind_of(&self, side: i32, row: usize) -> &str {
        let Some(item) = self.lines.get(row) else {
            return "";
        };
        if side == RIGHT {
            &item.pair_kind
        } else {
            &item.kind
        }
    }

    /// Whether the plain `Copy` takes this side of this row: the lines of
    /// that column's file — ctx + add as one column or on the new side,
    /// ctx + del on the old side. Headings and git's `\ No newline` note
    /// are not the file.
    fn takes(&self, side: i32, row: usize) -> bool {
        let kind = self.kind_of(side, row);
        let own = if self.split && side != RIGHT {
            "del"
        } else {
            "add"
        };
        kind == "ctx" || kind == own
    }

    pub(super) fn byte_at(&self, side: i32, position: LayoutPosition) -> i32 {
        let byte = self
            .source_line(side, position.row)
            .map_or(0, |text| source_byte(text, position.utf16));
        i32::try_from(byte).unwrap_or(0)
    }

    /// The two ends in reading order, or nothing where no selection
    /// stands — the one place that sorts them.
    fn taken(&self) -> Option<(SourcePosition, SourcePosition)> {
        if !self.sel_active {
            return None;
        }
        let (anchor, focus) = (self.sel_anchor, self.sel_focus);
        Some((anchor.min(focus), anchor.max(focus)))
    }

    /// The rows the selection covers, clamped to the rows that exist.
    fn span(&self) -> Option<RangeInclusive<usize>> {
        let (first, last) = self.taken()?;
        let end = self.lines.len().checked_sub(1)?;
        (first.row <= end).then(|| first.row..=last.row.min(end))
    }

    pub(super) fn holds(&self, side: i32, position: SourcePosition) -> bool {
        let Some((first, last)) = self.taken() else {
            return false;
        };
        side == self.sel_side && (first..=last).contains(&position)
    }

    /// The end a drag carried (`sel_focus` — a right-click's whole row ends
    /// at the line's end), in the places its row spells: where a text box's
    /// caret would stand, which the menu key opens under
    /// (デザイン規約 §メニュー のキーボード).
    pub(super) fn moving_end(&self) -> Ended {
        if !self.sel_active {
            return Ended::none();
        }
        let focus = self.sel_focus;
        let place = self
            .source_line(self.sel_side, focus.row)
            .map_or(0, |text| spelled_place(text, focus.byte));
        Ended::some(TextEnd {
            side: self.sel_side,
            row: i32::try_from(focus.row).unwrap_or(i32::MAX),
            place: i32::try_from(place).unwrap_or(i32::MAX),
        })
    }

    pub(super) fn start_select(&mut self, side: i32, position: SourcePosition) -> bool {
        self.move_ends(side, position, position, true)
    }

    /// The far end follows the pointer; the pressed end and its column
    /// stay — `side` is read only where no selection stands to extend.
    pub(super) fn drag_select(&mut self, side: i32, position: SourcePosition) -> bool {
        if !self.sel_active {
            return self.start_select(side, position);
        }
        self.move_ends(self.sel_side, self.sel_anchor, position, true)
    }

    pub(super) fn select_whole_row(&mut self, side: i32, row: usize) -> bool {
        let end = self.source_line(side, row).map_or(0, str::len);
        let head = SourcePosition { row, byte: 0 };
        let tail = SourcePosition { row, byte: end };
        self.move_ends(side, head, tail, true)
    }

    /// Nothing is selected any more (a plain click, or a press on nothing).
    pub(super) fn drop_selection(&mut self) -> bool {
        let none = SourcePosition::default();
        self.move_ends(0, none, none, false)
    }

    /// The same while the rows themselves go: no wash is written back.
    pub(super) fn forget_selection(&mut self) {
        self.sel_anchor = SourcePosition::default();
        self.sel_focus = SourcePosition::default();
        self.sel_side = 0;
        self.sel_active = false;
        self.sel_has_new = false;
        self.sel_removed = 0;
    }

    /// Moves the two ends — `anchor` the pressed one, `focus` the moving
    /// one, kept in that order (`taken` sorts them) — and brings the rows
    /// and the counts along. Returns whether anything moved — the slots
    /// fire `changed` only then, so a drag that stays on one character
    /// costs no signal.
    fn move_ends(
        &mut self,
        side: i32,
        anchor: SourcePosition,
        focus: SourcePosition,
        active: bool,
    ) -> bool {
        let before = self.span();
        let was_active = self.sel_active;
        let was = (self.sel_side, self.sel_anchor, self.sel_focus);
        self.sel_anchor = anchor;
        self.sel_focus = focus;
        self.sel_side = side;
        self.sel_active = active;
        let after = self.span();
        if before == after && was_active == active && was == (side, anchor, focus) {
            return false;
        }
        self.lay_wash(before, after, was.0 != side);
        self.settle_counts();
        true
    }

    /// Rewrites the wash where it can have changed and notifies exactly
    /// those rows. With both spans standing only the moving edges are
    /// revisited — a row inside both is whole in each — unless the column
    /// changed (`whole`), when every row of either span moves side.
    fn lay_wash(
        &mut self,
        before: Option<RangeInclusive<usize>>,
        after: Option<RangeInclusive<usize>>,
        whole: bool,
    ) {
        let mut ranges = match (before, after) {
            (None, None) => return,
            (None, Some(span)) | (Some(span), None) => vec![span],
            (Some(a), Some(b)) if whole => vec![a, b],
            (Some(a), Some(b)) => {
                let (a_first, a_last) = a.into_inner();
                let (b_first, b_last) = b.into_inner();
                vec![
                    a_first.min(b_first)..=a_first.max(b_first),
                    a_last.min(b_last)..=a_last.max(b_last),
                ]
            }
        };
        // Walked by first row: what one range wrote is skipped in the next.
        ranges.sort_unstable_by_key(|range| *range.start());
        let mut runs = Vec::new();
        // The ranges overlap where the edges met or crossed; each row is
        // written once.
        let mut done: Option<usize> = None;
        for (first, last) in ranges.into_iter().map(RangeInclusive::into_inner) {
            let first = done.map_or(first, |last_done| first.max(last_done + 1));
            for row in first..=last {
                if row >= self.lines.len() {
                    break;
                }
                if self.settle_row(row) {
                    push_run(&mut runs, row);
                }
                done = Some(row);
            }
        }
        self.notify_runs(runs);
    }

    /// Writes one row's two washes; says whether either changed.
    fn settle_row(&mut self, row: usize) -> bool {
        let own = self.spelled(0, row);
        let pair = self.spelled(RIGHT, row);
        let Some(item) = self.lines.get_mut(row) else {
            return false;
        };
        if *item.sel == *own && *item.pair_sel == *pair {
            return false;
        }
        item.sel = own;
        item.pair_sel = pair;
        true
    }

    /// One side's wash, the shape the row draws it in.
    fn spelled(&self, side: i32, row: usize) -> Optional<Washed> {
        match self.wash_of(side, row) {
            Wash::None => Optional::none(),
            Wash::Whole => Optional::some(Washed::whole()),
            Wash::Part(bytes) => Optional::new(
                self.source_line(side, row)
                    .map(|text| Washed::runs(spelled_ranges(text, &[bytes]))),
            ),
        }
    }

    /// What this side of this row gives the plain `Copy` — and wears.
    fn wash_of(&self, side: i32, row: usize) -> Wash {
        if side != self.sel_side || !self.takes(side, row) {
            return Wash::None;
        }
        let Some((first, last)) = self.taken() else {
            return Wash::None;
        };
        let Some(text) = self.source_line(side, row) else {
            return Wash::None;
        };
        let Some(bytes) = first.bytes_until(last, row, text.len()) else {
            return Wash::None;
        };
        if bytes.is_empty() {
            // An end row the selection only touches the edge of gives the
            // copy an empty line, and has nothing to wash.
            return if first.row == last.row {
                Wash::None
            } else {
                Wash::Part(bytes)
            };
        }
        if bytes == (0..text.len()) {
            Wash::Whole
        } else {
            Wash::Part(bytes)
        }
    }

    /// Whether `Copy removed lines` has anything to offer: not in a split
    /// diff's old column, where the plain `Copy` already takes them.
    fn removed_offered(&self) -> bool {
        !(self.split && self.sel_side != RIGHT)
    }

    /// Recounts what the selection holds, so the menu can be assembled
    /// from properties alone.
    fn settle_counts(&mut self) {
        let mut takes = 0usize;
        let mut inked = false;
        let mut removed = 0usize;
        if let Some(rows) = self.span() {
            for row in rows {
                match self.wash_of(self.sel_side, row) {
                    Wash::None => {}
                    Wash::Whole => {
                        takes += 1;
                        inked = true;
                    }
                    Wash::Part(bytes) => {
                        takes += 1;
                        inked |= !bytes.is_empty();
                    }
                }
                // Removed lines are always in column 0, even on a row
                // whose right side is washed.
                if self.removed_offered() && self.kind_of(0, row) == "del" {
                    removed += 1;
                }
            }
        }
        // One empty line would copy ""; two copy a newline, which can be
        // meant.
        self.sel_has_new = inked || takes > 1;
        self.sel_removed = i32::try_from(removed).unwrap_or(i32::MAX);
    }

    pub(super) fn copied_new(&self) -> String {
        let mut out = String::new();
        let mut first_line = true;
        if let Some(rows) = self.span() {
            for row in rows {
                let Some(text) = self.source_line(self.sel_side, row) else {
                    continue;
                };
                let bytes = match self.wash_of(self.sel_side, row) {
                    Wash::None => continue,
                    Wash::Whole => 0..text.len(),
                    Wash::Part(bytes) => bytes,
                };
                if !first_line {
                    out.push('\n');
                }
                first_line = false;
                out.push_str(text.get(bytes).unwrap_or_default());
            }
        }
        out
    }

    /// Which file's lines the selection is of and the first and last of
    /// them, for a link to those lines; `None` where it holds no line. The
    /// lines the plain `Copy` takes where it takes any — the new file's,
    /// or the old one's in a split diff's left column — else the removed
    /// lines it reaches over, which are the old file's. An end row the
    /// selection only touches the edge of holds no line of it.
    pub(super) fn line_span(&self) -> Option<LineSpan> {
        let rows = self.span()?;
        let old_column = self.split && self.sel_side != RIGHT;
        let mut taken = (i32::MAX, 0);
        let mut removed = (i32::MAX, 0);
        for row in rows {
            let Some(item) = self.lines.get(row) else {
                continue;
            };
            if self.wash_of(self.sel_side, row) != Wash::None && !self.edge_only(row) {
                let no = if old_column { item.old_no } else { item.new_no };
                if no > 0 {
                    taken = (taken.0.min(no), taken.1.max(no));
                }
            }
            if self.kind_of(0, row) == "del" && item.old_no > 0 {
                removed = (removed.0.min(item.old_no), removed.1.max(item.old_no));
            }
        }
        let (newer, (first, last)) = if taken.1 > 0 {
            (!old_column, taken)
        } else if removed.1 > 0 {
            (false, removed)
        } else {
            return None;
        };
        Some(LineSpan { newer, first, last })
    }

    /// Whether the selection only touches the edge of this end row — ends
    /// at the head of its last row, or starts at the end of a first row
    /// with text: the copy takes the line break there, and none of the line.
    fn edge_only(&self, row: usize) -> bool {
        let Some((first, last)) = self.taken() else {
            return false;
        };
        let len = self.source_line(self.sel_side, row).map_or(0, str::len);
        (row == last.row && last.byte == 0) || (row == first.row && len > 0 && first.byte >= len)
    }

    /// The removed lines the selection reaches over, whole — they carry no
    /// wash to say which part was meant (デザイン規約 §diff の中身をコピーする).
    /// Nothing where the plain `Copy` already takes them
    /// (`removed_offered`).
    pub(super) fn copied_removed(&self) -> String {
        let mut out = String::new();
        let mut first_line = true;
        if !self.removed_offered() {
            return out;
        }
        if let Some(rows) = self.span() {
            for row in rows {
                if self.kind_of(0, row) != "del" {
                    continue;
                }
                let Some(text) = self.source_line(0, row) else {
                    continue;
                };
                if !first_line {
                    out.push('\n');
                }
                first_line = false;
                out.push_str(text);
            }
        }
        out
    }
}
