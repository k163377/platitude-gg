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

use super::*;

/// One row's share of the selection before it is spelled out — so the rows
/// a drag sweeps on every move compare without building strings.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Wash {
    /// Not in the selection, or in it but not a line the plain copy takes.
    None,
    /// In it from end to end.
    Whole,
    /// In it between these two byte offsets of the row's source line.
    Part(usize, usize),
}

/// The rows a selection covers, first and last inclusive.
type Span = (usize, usize);

/// The new side of a split row.
const RIGHT: i32 = 1;

impl DiffModel {
    /// The source line (the file's own bytes) column `side` of row `row`
    /// draws.
    pub(super) fn source_line(&self, side: i32, row: usize) -> Option<&str> {
        let item = self.lines.get(row)?;
        let patch = usize::try_from(item.patch).ok()?;
        let hunk = usize::try_from(item.hunk).ok()?;
        let line = if side == RIGHT {
            item.pair_line
        } else {
            item.line
        };
        let line = usize::try_from(line).ok()?;
        let patch = self.shown.as_ref()?.get(patch)?;
        Some(&patch.hunks.get(hunk)?.lines.get(line)?.text)
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

    pub(super) fn byte_at(&self, side: i32, row: i32, at: i32) -> i32 {
        let at = usize::try_from(at).unwrap_or(0);
        let byte = usize::try_from(row)
            .ok()
            .and_then(|row| self.source_line(side, row))
            .map_or(0, |text| source_byte(text, at));
        i32::try_from(byte).unwrap_or(0)
    }

    /// The two ends in reading order, or nothing where no selection
    /// stands — the one place that sorts them.
    fn taken(&self) -> Option<(usize, usize, usize, usize)> {
        if !self.sel_active {
            return None;
        }
        let from = (
            usize::try_from(self.sel_from_row).ok()?,
            usize::try_from(self.sel_from_at).unwrap_or(0),
        );
        let to = (
            usize::try_from(self.sel_to_row).ok()?,
            usize::try_from(self.sel_to_at).unwrap_or(0),
        );
        let (first, last) = if from <= to { (from, to) } else { (to, from) };
        Some((first.0, first.1, last.0, last.1))
    }

    /// The rows the selection covers, clamped to the rows that exist.
    fn span(&self) -> Option<Span> {
        let (first, _, last, _) = self.taken()?;
        let end = self.lines.len().checked_sub(1)?;
        (first <= end).then(|| (first, last.min(end)))
    }

    pub(super) fn holds(&self, side: i32, row: i32, at: i32) -> bool {
        let Some((first_row, first_at, last_row, last_at)) = self.taken() else {
            return false;
        };
        if side != self.sel_side {
            return false;
        }
        let (Ok(row), Ok(at)) = (usize::try_from(row), usize::try_from(at)) else {
            return false;
        };
        (first_row, first_at) <= (row, at) && (row, at) <= (last_row, last_at)
    }

    /// The end a drag carried (`sel_to_*` — a right-click's whole row ends
    /// at the line's end), in the places its row spells: where a text box's
    /// caret would stand, which the menu key opens under
    /// (デザイン規約 §メニュー のキーボード).
    pub(super) fn moving_end(&self) -> Ended {
        if !self.sel_active {
            return Ended::none();
        }
        let Ok(row) = usize::try_from(self.sel_to_row) else {
            return Ended::none();
        };
        let byte = usize::try_from(self.sel_to_at).unwrap_or(0);
        let place = self
            .source_line(self.sel_side, row)
            .map_or(0, |text| spelled_place(text, byte));
        Ended::some(TextEnd {
            side: self.sel_side,
            row: self.sel_to_row,
            place: i32::try_from(place).unwrap_or(i32::MAX),
        })
    }

    pub(super) fn start_select(&mut self, side: i32, row: i32, at: i32) -> bool {
        self.move_ends(side, row, at, row, at, true)
    }

    /// The far end follows the pointer; the pressed end and its column
    /// stay — `side` is read only where no selection stands to extend.
    pub(super) fn drag_select(&mut self, side: i32, row: i32, at: i32) -> bool {
        if !self.sel_active {
            return self.start_select(side, row, at);
        }
        let (side, from_row, from_at) = (self.sel_side, self.sel_from_row, self.sel_from_at);
        self.move_ends(side, from_row, from_at, row, at, true)
    }

    pub(super) fn select_whole_row(&mut self, side: i32, row: i32) -> bool {
        let end = usize::try_from(row)
            .ok()
            .and_then(|row| self.source_line(side, row))
            .map_or(0, str::len);
        self.move_ends(side, row, 0, row, i32::try_from(end).unwrap_or(0), true)
    }

    /// Nothing is selected any more (a plain click, or a press on nothing).
    pub(super) fn drop_selection(&mut self) -> bool {
        self.move_ends(0, 0, 0, 0, 0, false)
    }

    /// The same while the rows themselves go: no wash is written back.
    pub(super) fn forget_selection(&mut self) {
        self.sel_from_row = 0;
        self.sel_from_at = 0;
        self.sel_to_row = 0;
        self.sel_to_at = 0;
        self.sel_side = 0;
        self.sel_active = false;
        self.sel_has_new = false;
        self.sel_removed = 0;
    }

    /// Moves the two ends and brings the rows and the counts along. Returns
    /// whether anything moved — the slots fire `changed` only then, so a
    /// drag that stays on one character costs no signal.
    fn move_ends(
        &mut self,
        side: i32,
        from_row: i32,
        from_at: i32,
        to_row: i32,
        to_at: i32,
        active: bool,
    ) -> bool {
        let before = self.span();
        let was_active = self.sel_active;
        let was = (
            self.sel_side,
            self.sel_from_row,
            self.sel_from_at,
            self.sel_to_row,
            self.sel_to_at,
        );
        self.sel_from_row = from_row;
        self.sel_from_at = from_at;
        self.sel_to_row = to_row;
        self.sel_to_at = to_at;
        self.sel_side = side;
        self.sel_active = active;
        let after = self.span();
        if before == after
            && was_active == active
            && was == (side, from_row, from_at, to_row, to_at)
        {
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
    fn lay_wash(&mut self, before: Option<Span>, after: Option<Span>, whole: bool) {
        let mut ranges = match (before, after) {
            (None, None) => return,
            (None, Some(span)) | (Some(span), None) => vec![span],
            (Some(a), Some(b)) if whole => vec![a, b],
            (Some(a), Some(b)) => vec![(a.0.min(b.0), a.0.max(b.0)), (a.1.min(b.1), a.1.max(b.1))],
        };
        ranges.sort_unstable();
        let mut runs = Vec::new();
        // The ranges overlap where the edges met or crossed; each row is
        // written once.
        let mut done: Option<usize> = None;
        for (first, last) in ranges {
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
            Wash::Part(first, last) => Optional::new(
                self.source_line(side, row)
                    .map(|text| Washed::runs(spelled_ranges(text, &[(first, last - first)]))),
            ),
        }
    }

    /// What this side of this row gives the plain `Copy` — and wears.
    fn wash_of(&self, side: i32, row: usize) -> Wash {
        if side != self.sel_side || !self.takes(side, row) {
            return Wash::None;
        }
        let Some((first_row, first_at, last_row, last_at)) = self.taken() else {
            return Wash::None;
        };
        if row < first_row || row > last_row {
            return Wash::None;
        }
        let Some(text) = self.source_line(side, row) else {
            return Wash::None;
        };
        let first = if row == first_row {
            first_at.min(text.len())
        } else {
            0
        };
        let last = if row == last_row {
            last_at.min(text.len())
        } else {
            text.len()
        };
        if last <= first {
            // An end row the selection only touches the edge of gives the
            // copy an empty line, and has nothing to wash.
            return if row == first_row && row == last_row {
                Wash::None
            } else {
                Wash::Part(first, first)
            };
        }
        if first == 0 && last == text.len() {
            Wash::Whole
        } else {
            Wash::Part(first, last)
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
        if let Some((first, last)) = self.span() {
            for row in first..=last {
                match self.wash_of(self.sel_side, row) {
                    Wash::None => {}
                    Wash::Whole => {
                        takes += 1;
                        inked = true;
                    }
                    Wash::Part(a, b) => {
                        takes += 1;
                        inked |= b > a;
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
        if let Some((first, last)) = self.span() {
            for row in first..=last {
                let (from, to) = match self.wash_of(self.sel_side, row) {
                    Wash::None => continue,
                    Wash::Whole => (0, usize::MAX),
                    Wash::Part(from, to) => (from, to),
                };
                let Some(text) = self.source_line(self.sel_side, row) else {
                    continue;
                };
                if !first_line {
                    out.push('\n');
                }
                first_line = false;
                out.push_str(text.get(from..to.min(text.len())).unwrap_or_default());
            }
        }
        out
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
        if let Some((first, last)) = self.span() {
            for row in first..=last {
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
