//! The reader's own selection of the log's text: where it runs, what each
//! row draws of it, and what it hands the clipboard
//! (デザイン規約 §git が言ったことを読む場所).
//!
//! A row is one line: its columns joined by tabs
//! (`12:03:17\tgit switch -- 3.2\t29 ms`), so a paste keeps the columns.
//! The panel draws each column at a place of its own, so a byte is found
//! per column: the pane lays the column ([`CommandsModel::column`]) on the
//! row's ruler (`LineRuler`) and brings back a place, and
//! `encode::markup::plain_byte` / `plain_ranges` turn places into bytes and
//! back. Nothing here measures a font.
//!
//! - One tab stands for the whole gap between two columns, so a wash that
//!   covers it covers the gap.
//! - git's own words come only with a line taken whole: the block under a
//!   failure cannot be pointed into (デザイン規約 §diff の中身をコピーする).

use std::ops::RangeInclusive;

use super::*;

/// `HH:mm:ss` — the clock column's only shape, so the first tab is at
/// this byte on every row.
const CLOCK_LEN: usize = 8;

/// The columns a press can land in, as the pane names them. The odd
/// numbers are the two gaps: one tab each, so their only places are its
/// two ends.
pub(super) const AT_CLOCK: i32 = 0;
pub(super) const AT_GAP_CMD: i32 = 1;
pub(super) const AT_CMD: i32 = 2;
pub(super) const AT_GAP_OUT: i32 = 3;
pub(super) const AT_OUT: i32 = 4;

/// One row's line, as the three columns it is drawn as.
pub(super) struct Line {
    pub(super) clock: String,
    pub(super) cmd: String,
    pub(super) out: String,
}

impl Line {
    fn of(item: &CommandItem) -> Self {
        Self {
            clock: item.clock.clone(),
            cmd: format!("git {}", item.args),
            // Result and duration are one column, space-joined
            // (`exit 128 12 ms`); either can be missing.
            out: match (item.result.as_str(), item.duration.as_str()) {
                ("", duration) => duration.to_string(),
                (result, "") => result.to_string(),
                (result, duration) => format!("{result} {duration}"),
            },
        }
    }

    fn at_cmd(&self) -> usize {
        self.clock.len() + 1
    }
    fn cmd_end(&self) -> usize {
        self.at_cmd() + self.cmd.len()
    }
    fn at_out(&self) -> usize {
        self.cmd_end() + 1
    }
    pub(super) fn len(&self) -> usize {
        if self.out.is_empty() {
            self.cmd_end()
        } else {
            self.at_out() + self.out.len()
        }
    }

    /// The whole of it, which is what a copy takes and what the byte
    /// offsets above index into.
    pub(super) fn text(&self) -> String {
        let mut out = String::with_capacity(self.len());
        out.push_str(&self.clock);
        out.push('\t');
        out.push_str(&self.cmd);
        if !self.out.is_empty() {
            out.push('\t');
            out.push_str(&self.out);
        }
        out
    }

    fn columns(&self) -> [(usize, &str); 3] {
        [
            (0, self.clock.as_str()),
            (self.at_cmd(), self.cmd.as_str()),
            (self.at_out(), self.out.as_str()),
        ]
    }
}

/// The clock a row is stamped with, in the reader's own time of day. The
/// offset comes from the display side (`Date.getTimezoneOffset()`):
/// `std::time` knows only UTC, and a zone crate is a dependency this does
/// not need.
pub(super) fn clock_of(at_ms: i64, zone_minutes: i32) -> String {
    let local = at_ms - i64::from(zone_minutes) * 60_000;
    let day = local.div_euclid(1000).rem_euclid(86_400);
    format!("{:02}:{:02}:{:02}", day / 3600, (day % 3600) / 60, day % 60)
}

impl CommandsModel {
    pub(super) fn line_at(&self, row: usize) -> Option<Line> {
        self.rows.get(row).map(Line::of)
    }

    /// The text of one of the three columns a row draws, for the pane's
    /// ruler (`CommandsTextSelect` / `CommandRowDelegate`). A gap has
    /// nothing to lay out, so it answers empty.
    pub(super) fn column(&self, row: i32, at: i32) -> String {
        let Some(line) = usize::try_from(row).ok().and_then(|row| self.line_at(row)) else {
            return String::new();
        };
        match at {
            AT_CLOCK => line.clock,
            AT_CMD => line.cmd,
            AT_OUT => line.out,
            _ => String::new(),
        }
    }

    /// Which byte of a row's line a press landed on, given the column and
    /// the place in that column's string. A gap's two places are the two
    /// sides of its tab.
    pub(super) fn hit(&self, column: i32, position: LayoutPosition) -> i32 {
        let Some(line) = self.line_at(position.row) else {
            return 0;
        };
        let place = position.utf16;
        let byte = match column {
            AT_CLOCK => plain_byte(&line.clock, place),
            AT_GAP_CMD if place == 0 => CLOCK_LEN,
            AT_GAP_CMD => line.at_cmd(),
            AT_CMD => line.at_cmd() + plain_byte(&line.cmd, place),
            AT_GAP_OUT if place == 0 => line.cmd_end(),
            AT_GAP_OUT => line.at_out(),
            AT_OUT => line.at_out() + plain_byte(&line.out, place),
            // Below the line: the words under a failure are taken whole or
            // not at all, so a press there is the line's end.
            _ => line.len(),
        };
        i32::try_from(byte).unwrap_or(0)
    }

    // ---- where the selection runs ------------------------------------

    /// The two ends in reading order, or nothing while none stands — the
    /// one place that sorts the pair, which is kept press first.
    ///
    /// The ends are kept as pressed: on a row whose command is still
    /// running, the outcome column names a byte past the line (the tab
    /// before it is not there yet). Each row's bytes are cut to its line
    /// where they are read (`SourcePosition::bytes_until`); sorting before
    /// that cut takes the same bytes — the cut never reorders two bytes of
    /// one row, and across rows the row decides.
    fn taken(&self) -> Option<(SourcePosition, SourcePosition)> {
        if !self.sel_active {
            return None;
        }
        let (anchor, focus) = (self.sel_anchor, self.sel_focus);
        Some((anchor.min(focus), anchor.max(focus)))
    }

    /// The rows the selection touches, so that only they are re-spelled.
    fn span(&self) -> Option<RangeInclusive<usize>> {
        self.taken().map(|(first, last)| first.row..=last.row)
    }

    pub(super) fn start_select(&mut self, position: SourcePosition) -> bool {
        let was = self.span();
        self.sel_active = true;
        self.sel_anchor = position;
        self.sel_focus = position;
        self.respell(was)
    }

    pub(super) fn drag_select(&mut self, position: SourcePosition) -> bool {
        if !self.sel_active || self.sel_focus == position {
            return false;
        }
        let was = self.span();
        self.sel_focus = position;
        self.respell(was)
    }

    pub(super) fn drop_selection(&mut self) -> bool {
        if !self.sel_active {
            return false;
        }
        let was = self.span();
        self.forget_selection();
        self.respell(was)
    }

    pub(super) fn forget_selection(&mut self) {
        self.sel_active = false;
        self.sel_anchor = SourcePosition::default();
        self.sel_focus = SourcePosition::default();
    }

    /// Moves the ends (row numbers) up past the `gone` oldest rows that fell
    /// off; a selection with an end over the edge is dropped.
    pub(super) fn shift_selection(&mut self, gone: usize) {
        if !self.sel_active {
            return;
        }
        match (
            self.sel_anchor.shifted_up(gone),
            self.sel_focus.shifted_up(gone),
        ) {
            (Some(anchor), Some(focus)) => {
                self.sel_anchor = anchor;
                self.sel_focus = focus;
            }
            _ => self.forget_selection(),
        }
    }

    // ---- what each row draws of it -----------------------------------

    /// Re-spells the rows the selection covers now or covered before, and
    /// answers whether any changed.
    fn respell(&mut self, was: Option<RangeInclusive<usize>>) -> bool {
        let now = self.span();
        let (first, last) = match (was, now) {
            (None, None) => return false,
            (Some(span), None) | (None, Some(span)) => span.into_inner(),
            (Some(a), Some(b)) => (*a.start().min(b.start()), *a.end().max(b.end())),
        };
        let mut runs = Vec::new();
        for row in first..=last.min(self.rows.len().saturating_sub(1)) {
            if self.respell_row(row) {
                push_run(&mut runs, row);
            }
        }
        let touched = !runs.is_empty();
        self.notify_runs(runs);
        touched
    }

    /// Writes one row's share of the selection into its `sel` role, and
    /// answers whether that changed anything — the caller tells the view.
    pub(super) fn respell_row(&mut self, row: usize) -> bool {
        let spelled = self.spell(row);
        let Some(item) = self.rows.get_mut(row) else {
            return false;
        };
        if *item.sel == *spelled {
            return false;
        }
        item.sel = spelled;
        true
    }

    /// One row's share, as the delegate reads it (`CommandWash`).
    fn spell(&self, row: usize) -> Optional<CommandWash> {
        let Some((first, last)) = self.taken() else {
            return Optional::none();
        };
        // A re-spell walks rows the selection has left: they are answered
        // before their line is built (`line_at` allocates).
        if row < first.row || row > last.row {
            return Optional::none();
        }
        let Some(line) = self.line_at(row) else {
            return Optional::none();
        };
        let Some(bytes) = first
            .bytes_until(last, row, line.len())
            .filter(|bytes| !bytes.is_empty())
        else {
            return Optional::none();
        };
        let mut columns: Vec<Runs> = Vec::with_capacity(3);
        for (base, text) in line.columns() {
            let start = bytes.start.max(base);
            let end = bytes.end.min(base + text.len());
            columns.push(if start < end {
                let in_column = start - base..end - base;
                plain_ranges(text, &[in_column])
            } else {
                Runs::default()
            });
        }
        let mut columns = columns.into_iter();
        Optional::some(CommandWash {
            clock: columns.next().unwrap_or_default(),
            command: columns.next().unwrap_or_default(),
            outcome: columns.next().unwrap_or_default(),
            whole: bytes == (0..line.len()),
        })
    }

    // ---- what it hands the clipboard ---------------------------------

    /// The selected text, cut at its two ends. A failed line taken whole
    /// brings git's words under it, each indented by a tab so it pastes
    /// under the command column.
    pub(super) fn copied(&self) -> String {
        let Some((first, last)) = self.taken() else {
            return String::new();
        };
        let mut out = String::new();
        for row in first.row..=last.row {
            let (Some(line), Some(item)) = (self.line_at(row), self.rows.get(row)) else {
                continue;
            };
            let text = line.text();
            let Some(bytes) = first.bytes_until(last, row, text.len()) else {
                continue;
            };
            if bytes.is_empty() {
                continue;
            }
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(text.get(bytes.clone()).unwrap_or_default());
            if bytes != (0..text.len()) || item.status != "failed" {
                continue;
            }
            for said in item.output.lines() {
                out.push('\n');
                out.push('\t');
                out.push_str(said);
            }
        }
        out
    }
}
