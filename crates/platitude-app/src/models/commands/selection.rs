//! The reader's own selection of the log's text: where it runs, what each
//! row draws of it, and what it hands the clipboard
//! (デザイン規約 §git が言ったことを読む場所).
//!
//! **A row is one line, and the line is its columns joined by tabs**
//!: `12:03:17\tgit switch -- 3.2\t29 ms`. A tab
//! is where the eye sees a column break and a space is where it sees one
//! word after another, so a log pasted into anything that reads tabs comes
//! out in the three columns it was read in.
//!
//! **The panel draws those columns at three places of its own** — the
//! clock at the near edge, the command after it, the outcome against the
//! far one — so a byte of the line is not `x / charW` from the row's
//! start. This is where that is answered, and it is answered the same way
//! twice: `encode::markup::hit_byte` walks a column to find the byte a
//! press is on, and `encode::markup::display_ranges` walks the same column
//! to say where a byte range is drawn. The pane hands over a column and an
//! x inside it; everything else is read off the rows.
//!
//! Two things follow and are worth saying once:
//!
//!  - **the tabs are the gaps.** One tab character stands for the whole
//!    run of empty pixels between two columns, so a wash that covers it
//!    covers the gap — which is what a selection reads like in a terminal,
//!    and what was asked for.
//!  - **git's own words come with a command line taken whole.** There is
//!    no way on screen to point at part of the block under a failure (it
//!    wraps, and the rows are commands rather than lines), so pointing at
//!    the row is pointing at all of it — the same bargain the diff makes
//!    with its removed lines (§diff の中身をコピーする).

use super::*;

/// `HH:mm:ss`, which is the only shape the clock column is ever written
/// in — so the byte the tab after it sits on is this, on every row.
const CLOCK_LEN: usize = 8;

/// The columns a press can land in, as the pane names them. The odd
/// numbers are the two gaps: they hold one tab each, and the pane knows
/// only how far across the empty pixels the press was (0..1).
pub(super) const AT_CLOCK: i32 = 0;
pub(super) const AT_GAP_CMD: i32 = 1;
pub(super) const AT_CMD: i32 = 2;
pub(super) const AT_GAP_OUT: i32 = 3;
pub(super) const AT_OUT: i32 = 4;

/// One row's line, taken apart into the three columns it is drawn as and
/// the byte each of them starts at.
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
            // Two words about how it went, and a space between them
            // because that is one thing being said and not two columns:
            // `exit 128 12 ms`. Either half can be missing -- a command
            // that went through says only how long it took, and one still
            // running says neither.
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

    /// The three columns with the byte each starts at, in reading order.
    fn columns(&self) -> [(usize, &str); 3] {
        [
            (0, self.clock.as_str()),
            (self.at_cmd(), self.cmd.as_str()),
            (self.at_out(), self.out.as_str()),
        ]
    }
}

/// The clock a row is stamped with, in the reader's own time of day.
///
/// The offset comes from the display side (`Date.getTimezoneOffset()`, in
/// minutes to add to local to reach UTC) because nothing else here knows
/// it: `std::time` deals in UTC alone, and a crate that knows zones is a
/// dependency this does not need. It is asked for again every time the
/// panel is shown, so a session carried across a change of offset stamps
/// the rows that arrive afterwards with the new one.
pub(super) fn clock_of(at_ms: i64, zone_minutes: i32) -> String {
    let local = at_ms - i64::from(zone_minutes) * 60_000;
    let day = local.div_euclid(1000).rem_euclid(86_400);
    format!("{:02}:{:02}:{:02}", day / 3600, (day % 3600) / 60, day % 60)
}

impl CommandsModel {
    pub(super) fn line_at(&self, row: usize) -> Option<Line> {
        self.rows.get(row).map(Line::of)
    }

    /// Which byte of a row's line a press landed on, given the column it
    /// landed in and how far along that column it was. A gap holds one
    /// tab, and which side of it the press was on is all there is to say
    /// about it (`x` is the fraction across the empty pixels there).
    pub(super) fn hit(&self, row: i32, at: i32, x: f64, char_w: f64, wide_delta: f64) -> i32 {
        let Some(line) = usize::try_from(row).ok().and_then(|row| self.line_at(row)) else {
            return 0;
        };
        let byte = match at {
            AT_CLOCK => hit_byte(&line.clock, x, char_w, wide_delta),
            AT_GAP_CMD if x < 0.5 => CLOCK_LEN,
            AT_GAP_CMD => line.at_cmd(),
            AT_CMD => line.at_cmd() + hit_byte(&line.cmd, x, char_w, wide_delta),
            AT_GAP_OUT if x < 0.5 => line.cmd_end(),
            AT_GAP_OUT => line.at_out(),
            AT_OUT => line.at_out() + hit_byte(&line.out, x, char_w, wide_delta),
            // Below the line: the block of words under a failure. It is
            // taken whole or not at all, so a press in it is the end of
            // the line it belongs to.
            _ => line.len(),
        };
        i32::try_from(byte).unwrap_or(0)
    }

    // ---- where the selection runs ------------------------------------

    /// The two ends in reading order, or nothing while none stands. The
    /// pair is kept in the order the hand made it -- the press first --
    /// so a drag upwards is as ordinary as one downwards, and this is the
    /// one place that sorts it.
    ///
    /// **Both ends are cut to the line they are on.** A hand that has run
    /// off the end of a row names a byte past it, and every reader of the
    /// pair below indexes a line with it: uncut, `to` addresses nothing
    /// and the whole selection reads as empty (measured — a drag to
    /// the end of the last row copied nothing at all).
    fn taken(&self) -> Option<(usize, usize, usize, usize)> {
        if !self.sel_active {
            return None;
        }
        let end = |row: usize, at: i32| {
            let at = usize::try_from(at).unwrap_or(0);
            self.line_at(row).map_or(0, |line| at.min(line.len()))
        };
        let from_row = usize::try_from(self.sel_from_row).ok()?;
        let to_row = usize::try_from(self.sel_to_row).ok()?;
        let from = (from_row, end(from_row, self.sel_from_at));
        let to = (to_row, end(to_row, self.sel_to_at));
        let (first, last) = if from <= to { (from, to) } else { (to, from) };
        Some((first.0, first.1, last.0, last.1))
    }

    /// The rows the selection touches, so that only they are re-spelled.
    fn span(&self) -> Option<(usize, usize)> {
        self.taken().map(|(first, _, last, _)| (first, last))
    }

    pub(super) fn start_select(&mut self, row: i32, at: i32) -> bool {
        let was = self.span();
        self.sel_active = true;
        self.sel_from_row = row;
        self.sel_from_at = at;
        self.sel_to_row = row;
        self.sel_to_at = at;
        self.respell(was)
    }

    pub(super) fn drag_select(&mut self, row: i32, at: i32) -> bool {
        if !self.sel_active || (self.sel_to_row == row && self.sel_to_at == at) {
            return false;
        }
        let was = self.span();
        self.sel_to_row = row;
        self.sel_to_at = at;
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
        self.sel_from_row = -1;
        self.sel_from_at = 0;
        self.sel_to_row = -1;
        self.sel_to_at = 0;
    }

    /// What the oldest rows falling off the end does to the ends: they are
    /// row numbers, and the rows under them have moved. A selection whose
    /// first end went over the edge is not the reader's selection any
    /// more, so it goes rather than sliding onto rows nobody picked.
    pub(super) fn shift_selection(&mut self, gone: i32) {
        if !self.sel_active {
            return;
        }
        if self.sel_from_row < gone || self.sel_to_row < gone {
            self.forget_selection();
            return;
        }
        self.sel_from_row -= gone;
        self.sel_to_row -= gone;
    }

    // ---- what each row draws of it -----------------------------------

    /// Re-spells every row that could have changed: the ones the selection
    /// covers now, and the ones it covered before. Answers whether any of
    /// them actually did.
    ///
    /// The rows are written first and the view told afterwards, in runs —
    /// a drag that sweeps a hundred rows is one notification rather than a
    /// hundred (`models::notify`).
    fn respell(&mut self, was: Option<(usize, usize)>) -> bool {
        let now = self.span();
        let (first, last) = match (was, now) {
            (None, None) => return false,
            (Some(span), None) | (None, Some(span)) => span,
            (Some(a), Some(b)) => (a.0.min(b.0), a.1.max(b.1)),
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
    /// Reached from `drain` as well: a row that has just finished draws a
    /// different line, having grown words about how it went that it did
    /// not have while it ran.
    pub(super) fn respell_row(&mut self, row: usize) -> bool {
        let spelled = self.spell(row);
        let Some(item) = self.rows.get_mut(row) else {
            return false;
        };
        if item.sel == spelled {
            return false;
        }
        item.sel = spelled;
        true
    }

    /// One row's share, as the delegate reads it:
    /// `<clock run>|<command run>|<outcome run>|<whole>`, each run in the
    /// `col:wides:width:wides` the wash is placed by
    /// (`encode::display_ranges`) and empty where that column holds none
    /// of the selection. `<whole>` is `1` where the line is taken end to
    /// end, which is what brings git's own words with it.
    fn spell(&self, row: usize) -> String {
        let Some((first, first_at, last, last_at)) = self.taken() else {
            return String::new();
        };
        if row < first || row > last {
            return String::new();
        }
        let Some(line) = self.line_at(row) else {
            return String::new();
        };
        let from = if row == first { first_at } else { 0 };
        let to = if row == last { last_at } else { line.len() };
        if from >= to {
            return String::new();
        }
        let mut out = String::new();
        for (base, text) in line.columns() {
            let start = from.max(base);
            let end = to.min(base + text.len());
            if start < end {
                out.push_str(&display_ranges(text, &[(start - base, end - start)]));
            }
            out.push('|');
        }
        out.push(if from == 0 && to == line.len() {
            '1'
        } else {
            '0'
        });
        out
    }

    // ---- what it hands the clipboard ---------------------------------

    /// The selected text, cut at its two ends. Rows are lines; a line
    /// taken whole brings the block of words under it, indented by the tab
    /// that puts it under the command column on screen.
    pub(super) fn copied(&self) -> String {
        let Some((first, first_at, last, last_at)) = self.taken() else {
            return String::new();
        };
        let mut out = String::new();
        for row in first..=last {
            let (Some(line), Some(item)) = (self.line_at(row), self.rows.get(row)) else {
                continue;
            };
            let text = line.text();
            let from = if row == first { first_at } else { 0 };
            let to = if row == last { last_at } else { text.len() };
            if from >= to {
                continue;
            }
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(text.get(from..to).unwrap_or_default());
            if from != 0 || to != text.len() || item.state != "failed" {
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
