//! The find bar's search over the rows on screen (デザイン規約 §diff の中を探す):
//! what the typed lines hit, the hit the bar stands on, and the marks each
//! row draws for them. The matching is core's (`platitude_core::textfind`);
//! this end maps a hit's lines onto rows and columns and spells its bytes
//! the way the row draws them (as the selection's wash does, `selection`).

use platitude_core::textfind::{self, Hit, Options, Query};

use super::selection::RIGHT;
use super::*;

/// Where each line stands on screen, by core's line id: its row and its
/// column (0 the row's own line, `RIGHT` the new side of a split row). A
/// context line read on both sides of a split row stands in two places.
type Places = Vec<Vec<(usize, i32)>>;

impl DiffModel {
    /// A new query, or none (empty text), searched at once over the rows
    /// that stand; the place is given up and asked for again (`find_from_row`).
    pub(super) fn set_find_query(&mut self, text: String, options: Options) {
        self.find_text = text;
        self.find_options = options;
        self.find_at = 0;
        self.refind();
    }

    /// The standing query searched again over the rows that stand now — on
    /// the rows being laid out again (another file, one column or two) as
    /// on a new query. A place that no longer exists is given up.
    pub(super) fn refind(&mut self) {
        self.find_hits.clear();
        self.find_error.clear();
        self.find_searching = false;
        match Query::new(&self.find_text, self.find_options) {
            Ok(None) => {}
            Ok(Some(query)) => {
                self.find_searching = true;
                if let Some(patches) = self.shown.as_deref() {
                    self.find_hits = textfind::hits_in(&query, patches);
                }
            }
            // A pattern that cannot be read is refused: no line stands,
            // the bar's frame says so and its tooltip says why (デザイン規約
            // §diff の中を探す).
            Err(error) => self.find_error = error.to_string(),
        }
        self.find_count = i32::try_from(self.find_hits.len()).unwrap_or(i32::MAX);
        if self.find_at > self.find_count {
            self.find_at = 0;
        }
        let places = self.places();
        self.lay_found(&places);
    }

    /// Stands on the first hit at or after `row`, round to the first past
    /// the end (the search starts from the top of what is on screen,
    /// デザイン規約 §コミットを探す). Says that hit's row, -1 for none.
    pub(super) fn find_from_row(&mut self, row: i32) -> i32 {
        let places = self.places();
        if self.find_hits.is_empty() {
            self.find_at = 0;
            self.lay_found(&places);
            return -1;
        }
        let from = usize::try_from(row).unwrap_or(0);
        let ordinal = self
            .find_hits
            .iter()
            .position(|hit| first_row(hit, &places).is_some_and(|r| r >= from))
            .unwrap_or(0);
        self.stand_on(ordinal + 1, &places)
    }

    /// One hit on (`1`) or back (`-1`), round at either end; the row it
    /// starts on, -1 for none.
    pub(super) fn find_step(&mut self, delta: i32) -> i32 {
        if self.find_count == 0 {
            return -1;
        }
        let at = if self.find_at == 0 {
            if delta > 0 { 1 } else { self.find_count }
        } else {
            (self.find_at - 1 + delta).rem_euclid(self.find_count) + 1
        };
        let places = self.places();
        self.stand_on(usize::try_from(at).unwrap_or(1), &places)
    }

    fn stand_on(&mut self, ordinal: usize, places: &Places) -> i32 {
        self.find_at = i32::try_from(ordinal).unwrap_or(0);
        self.lay_found(places);
        self.find_row
    }

    /// Where each line of the patches stands (`textfind::hits_in` counts
    /// the lines through every hunk of every patch, and so does this).
    fn places(&self) -> Places {
        let Some(patches) = self.shown.as_deref() else {
            return Places::new();
        };
        let mut base = Vec::with_capacity(patches.len());
        let mut count = 0;
        for patch in patches {
            let mut hunks = Vec::with_capacity(patch.hunks.len());
            for hunk in &patch.hunks {
                hunks.push(count);
                count += hunk.lines.len();
            }
            base.push(hunks);
        }
        let mut places = vec![Vec::new(); count];
        for (row, item) in self.lines.iter().enumerate() {
            let hunk_base = usize::try_from(item.patch)
                .ok()
                .and_then(|p| base.get(p))
                .and_then(|hunks| usize::try_from(item.hunk).ok().and_then(|h| hunks.get(h)));
            let Some(&hunk_base) = hunk_base else {
                continue;
            };
            for (line, col) in [(item.line, 0), (item.pair_line, RIGHT)] {
                if let Ok(line) = usize::try_from(line)
                    && let Some(found) = places.get_mut(hunk_base + line)
                {
                    found.push((row, col));
                }
            }
        }
        places
    }

    /// Writes every row's marks — every hit's, and the one the bar stands
    /// on — and tells the rows that changed; notes where that one stands
    /// (`find_row`).
    fn lay_found(&mut self, places: &Places) {
        let rows = self.lines.len();
        let mut all: Vec<[Vec<(usize, usize)>; 2]> = vec![Default::default(); rows];
        let mut now: Vec<[Vec<(usize, usize)>; 2]> = vec![Default::default(); rows];
        let current = usize::try_from(self.find_at).unwrap_or(0);
        self.find_row = -1;
        for (ordinal, hit) in self.find_hits.iter().enumerate() {
            let standing = ordinal + 1 == current;
            for span in &hit.spans {
                let Some(found) = places.get(span.id) else {
                    continue;
                };
                for &(row, side) in found {
                    let col = usize::from(side == RIGHT);
                    all[row][col].push((span.start, span.end));
                    if standing {
                        now[row][col].push((span.start, span.end));
                        if self.find_row < 0 {
                            self.find_row = i32::try_from(row).unwrap_or(-1);
                        }
                    }
                }
            }
        }
        let mut runs = Vec::new();
        for row in 0..rows {
            let own = self.spelled_found(0, row, &all[row][0], &now[row][0]);
            let pair = self.spelled_found(RIGHT, row, &all[row][1], &now[row][1]);
            let item = &mut self.lines[row];
            let same = item.marks.own.found == own.0
                && item.marks.own.found_now == own.1
                && item
                    .marks
                    .pair
                    .as_ref()
                    .is_none_or(|right| right.found == pair.0 && right.found_now == pair.1);
            if same {
                continue;
            }
            let mut marks = (*item.marks).clone();
            marks.own.found = own.0;
            marks.own.found_now = own.1;
            if let Some(right) = marks.pair.as_mut() {
                right.found = pair.0;
                right.found_now = pair.1;
            }
            item.marks = One::new(marks);
            push_run(&mut runs, row);
        }
        self.notify_runs(runs);
    }

    /// One column's hits on one row, `(start, end)` in the source line's
    /// bytes, spelled the way the row draws the line.
    fn spelled_found(
        &self,
        side: i32,
        row: usize,
        all: &[(usize, usize)],
        now: &[(usize, usize)],
    ) -> (Runs, Runs) {
        match self.source_line(side, row) {
            Some(text) if !all.is_empty() => (
                spelled_ranges(text, &joined(all)),
                spelled_ranges(text, &joined(now)),
            ),
            _ => (Runs::default(), Runs::default()),
        }
    }
}

/// Byte ranges `(start, end)` in line order, run together where they
/// touch or overlap, as the `(start, len)` `spelled_ranges` reads one at
/// a time. Both sides' hits can run into one shared line (`old end\nke`
/// and `new end\nkeep` both end on `keep`), and the line wears the longer.
fn joined(ranges: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut sorted = ranges.to_vec();
    sorted.sort_unstable();
    let mut out: Vec<(usize, usize)> = Vec::with_capacity(sorted.len());
    for (start, end) in sorted {
        match out.last_mut() {
            Some(last) if start <= last.0 + last.1 => last.1 = last.1.max(end - last.0),
            _ => out.push((start, end - start)),
        }
    }
    out
}

/// The first row a hit stands on: its first line's place.
fn first_row(hit: &Hit, places: &Places) -> Option<usize> {
    hit.spans
        .first()
        .and_then(|span| places.get(span.id))
        .and_then(|found| found.iter().map(|&(row, _)| row).min())
}
