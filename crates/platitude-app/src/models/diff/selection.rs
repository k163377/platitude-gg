//! The reader's own selection of the diff's text: where it runs, what
//! each row draws of it, and what it hands the clipboard
//! (デザイン規約 §diff の中身をコピーする).
//!
//! **It lives here** for one reason: which bytes of a line the
//! clipboard should get is a question about the file's own text — and
//! the file's own text is here, on `shown`. The pane hands over a
//! column, a row and a place along it; everything after that is read
//! off the patches.
//!
//! **A column.** Read as one column, the diff has one: `0`, the rows'
//! own lines. Read side by side it has two, and a drag is of one of
//! them — the old side (`0`) or the new (`1`) — since what it selects is
//! that side's file (デザイン規約 §diff を 2 列で読む). Which column a
//! row's wash goes on follows: `sel` for `0`, `pair_sel` for `1`.
//!
//! **A place.** Where a line's characters are drawn is known only to the
//! layout that drew them, so the pane asks the row itself (`LineRuler`)
//! and brings back a place in the line as the row spells it; this side
//! turns that into a byte and back (`encode::markup::source_byte` /
//! `spelled_ranges`). Nothing here measures a font, and no count of
//! columns stands in for one.
//!
//! Two more things follow and are worth saying once:
//!
//!  - **the copy comes from the file's bytes.** A coloured row's `text`
//!    is markup whose spaces are `&nbsp;` and whose tabs have already been
//!    spelled out as the columns they reach (`encode::markup`), so a copy
//!    taken off the screen would paste indentation made of spaces and
//!    break the Makefile it came from.
//!  - **the wash is the answer.** A row is washed exactly where the plain
//!    `Copy` takes it, so the lines of the other side and the hunk
//!    headings inside a selection carry none — and what the reader can
//!    see they are getting is what they get.

use super::*;

/// What one row's share of the selection looks like, before it is spelled
/// out. Kept apart from the spelling so that the rows a drag sweeps over
/// — which is every row between the two ends, on every mouse move — can be
/// found unchanged without building a string to compare.
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
    /// The source line one side of row `row` draws — the file's own
    /// bytes. `side` is the column: the rows' own lines, or the new side
    /// of a split row.
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
    /// the file that column is. Read as one column, that is the new
    /// side — the unchanged and added lines; read side by side, the old
    /// column is the old file (unchanged and removed) and the new column
    /// the new one. A hunk heading and git's own `\ No newline` note are
    /// not the file talking at all.
    fn takes(&self, side: i32, row: usize) -> bool {
        let kind = self.kind_of(side, row);
        let own = if self.split && side != RIGHT {
            "del"
        } else {
            "add"
        };
        kind == "ctx" || kind == own
    }

    /// Which byte of a row's line the place `at` of it stands on. The pane
    /// asks the row's own layout which place the pointer is over
    /// (`LineRuler`) and brings that here; nothing about pixels
    /// crosses this line.
    pub(super) fn byte_at(&self, side: i32, row: i32, at: i32) -> i32 {
        let at = usize::try_from(at).unwrap_or(0);
        let byte = usize::try_from(row)
            .ok()
            .and_then(|row| self.source_line(side, row))
            .map_or(0, |text| source_byte(text, at));
        i32::try_from(byte).unwrap_or(0)
    }

    /// The two ends in reading order, or nothing where no selection
    /// stands. The pair is stored in the order the hand made it — the
    /// press first, the pointer second — so a drag upwards is as ordinary
    /// as one downwards, and this is the one place that sorts it.
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

    /// Whether a point of the text is inside the selection — what a
    /// right-click asks before deciding whether to take the row it landed
    /// on instead (デザイン規約 §diff の中身をコピーする). A point in the
    /// other column is outside it whatever its row.
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

    /// A press: the selection starts here and is empty until the hand
    /// moves.
    pub(super) fn start_select(&mut self, side: i32, row: i32, at: i32) -> bool {
        self.move_ends(side, row, at, row, at, true)
    }

    /// The hand has moved: the far end follows it and the near one stays
    /// where it was pressed. A drag is of one column — the press's — so
    /// `side` is read only where no selection stands to extend.
    pub(super) fn drag_select(&mut self, side: i32, row: i32, at: i32) -> bool {
        if !self.sel_active {
            return self.start_select(side, row, at);
        }
        let (side, from_row, from_at) = (self.sel_side, self.sel_from_row, self.sel_from_at);
        self.move_ends(side, from_row, from_at, row, at, true)
    }

    /// One whole row becomes the selection — a right-click that landed
    /// outside whatever was selected, which is the rule the file list
    /// already reads by (デザイン規約 §バケツごとの一覧: メニューは常に
    /// 光っている行に効く).
    pub(super) fn select_whole_row(&mut self, side: i32, row: i32) -> bool {
        let end = usize::try_from(row)
            .ok()
            .and_then(|row| self.source_line(side, row))
            .map_or(0, str::len);
        self.move_ends(side, row, 0, row, i32::try_from(end).unwrap_or(0), true)
    }

    /// Nothing is selected any more: a plain click, or a pane the reader
    /// pressed in with nothing under the press.
    pub(super) fn drop_selection(&mut self) -> bool {
        self.move_ends(0, 0, 0, 0, 0, false)
    }

    /// The same, for the moment the rows themselves are going: no wash is
    /// written back, because there is nothing left to write it on.
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

    /// Moves the two ends and brings the rows and the counts along. Says
    /// whether anything moved: the drag calls this on every mouse move,
    /// and the three published counts ride the model's one signal
    /// (`changed`), which the slot above only fires when there is
    /// something to fire it about.
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
        // A selection that changed column takes every row it stood on
        // with it, not only the ends: the rows inside both spans are the
        // ones whose wash moves from one side to the other.
        self.lay_wash(before, after, was.0 != side);
        self.settle_counts();
        true
    }

    /// Rewrites the wash wherever it can have changed, and tells the view
    /// about exactly those rows.
    ///
    /// **Only the ends are revisited when both spans stand.** A row inside
    /// both the old selection and the new one is washed end to end in each,
    /// so a drag across six thousand rows walks only the two moving
    /// edges — unless the column changed (`whole`), which every row in
    /// either span has to hear about.
    fn lay_wash(&mut self, before: Option<Span>, after: Option<Span>, whole: bool) {
        let mut ranges = match (before, after) {
            (None, None) => return,
            (None, Some(span)) | (Some(span), None) => vec![span],
            (Some(a), Some(b)) if whole => vec![a, b],
            (Some(a), Some(b)) => vec![(a.0.min(b.0), a.0.max(b.0)), (a.1.min(b.1), a.1.max(b.1))],
        };
        ranges.sort_unstable();
        let mut runs = Vec::new();
        // Where the two edges have met — a short selection, or one whose
        // ends moved past each other — the ranges overlap; a row is
        // written once either way.
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

    /// Writes one row's two washes, and says whether either changed. The
    /// shape is worked out first so that the two spellings that need no
    /// string — nothing, and the whole line — cost no allocation on the
    /// rows a drag sweeps past.
    fn settle_row(&mut self, row: usize) -> bool {
        let own = self.spelled(0, row);
        let pair = self.spelled(RIGHT, row);
        let Some(item) = self.lines.get_mut(row) else {
            return false;
        };
        if item.sel == own && item.pair_sel == pair {
            return false;
        }
        item.sel = own;
        item.pair_sel = pair;
        true
    }

    /// One side's wash, spelled the way the row draws it.
    fn spelled(&self, side: i32, row: usize) -> String {
        match self.wash_of(side, row) {
            Wash::None => String::new(),
            Wash::Whole => String::from("*"),
            Wash::Part(first, last) => self
                .source_line(side, row)
                .map(|text| spelled_ranges(text, &[(first, last - first)]))
                .unwrap_or_default(),
        }
    }

    /// What this side of this row gives the plain `Copy`, which is also
    /// what it wears. The other column wears nothing: a selection is of
    /// one.
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

    /// Whether the removed lines the selection reaches over are ones the
    /// menu's second word has to offer: only where the plain `Copy` is
    /// not already taking them — which it is, in the old column of a
    /// split diff.
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
                // The removed lines are in the rows' own column either way
                // round: as one column they are rows of their own, side by
                // side they stand on the left of a row whose right may
                // well be washed.
                if self.removed_offered() && self.kind_of(0, row) == "del" {
                    removed += 1;
                }
            }
        }
        // One line with nothing in it is a selection that would put an
        // empty string on the clipboard; two are a newline, which is
        // something a reader can have meant.
        self.sel_has_new = inked || takes > 1;
        self.sel_removed = i32::try_from(removed).unwrap_or(i32::MAX);
    }

    /// The selection's own column: the lines the plain `Copy` takes, cut
    /// at the two ends, in the file's own bytes.
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

    /// The removed lines the selection reaches over, whole.
    ///
    /// Whole: they carry no wash, so there is nothing on screen that
    /// could say which part of one was meant — the same answer GitKraken
    /// gives (デザイン規約 §diff の中身をコピーする). Nothing where the
    /// plain `Copy` is already taking them (`removed_offered`).
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
