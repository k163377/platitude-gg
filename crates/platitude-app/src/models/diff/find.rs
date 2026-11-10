//! The find bar's search over the rows on screen (`DiffFindBar`): what the
//! typed lines hit, the hit the bar stands on, and the marks each
//! row draws for them. The matching is core's (`platitude_core::textfind`);
//! this end maps a hit's lines onto rows and columns and spells its bytes
//! the way the row draws them (as the selection's wash does, `selection`).

use std::ops::Range;

use platitude_core::textfind::{self, Hit, Options, Query};

use super::*;

/// Where each line stands on screen, by core's line id: its row and its
/// column (0 the row's own line, `RIGHT` the new side of a split row). A
/// context line read on both sides of a split row stands in two places.
type Places = Vec<Vec<(usize, i32)>>;

/// Core's ids for the lines of the patches (`textfind::hits_in`): a line's
/// place counted through every hunk of every patch. A count of its own —
/// neither a row on screen nor a line's place in its hunk
/// ([`DiffLineLocation`]) — reached from a location here and from nowhere
/// else.
struct LineIds {
    /// The ids of each hunk's lines, by patch and by hunk in it.
    hunks: Vec<Vec<Range<usize>>>,
    /// How many lines the patches hold: one past the last id.
    count: usize,
}

impl LineIds {
    fn of(patches: &[FilePatch]) -> Self {
        let mut hunks = Vec::with_capacity(patches.len());
        let mut count = 0;
        for patch in patches {
            let mut ids = Vec::with_capacity(patch.hunks.len());
            for hunk in &patch.hunks {
                let first = count;
                count += hunk.lines.len();
                ids.push(first..count);
            }
            hunks.push(ids);
        }
        Self { hunks, count }
    }

    /// The id of the line `at` names — none where it names no line of the
    /// patches.
    fn id(&self, at: DiffLineLocation) -> Option<usize> {
        self.hunks.get(at.patch)?.get(at.hunk)?.clone().nth(at.line)
    }
}

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
            // the bar's frame says so and its tooltip says why
            // (`DiffFindBar`).
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

    /// Where each line of the patches stands: every row's two columns,
    /// filed under the id of the line each draws (`DiffLineItem::location`
    /// says which line, [`LineIds`] its id).
    fn places(&self) -> Places {
        let Some(patches) = self.shown.as_deref() else {
            return Places::new();
        };
        let ids = LineIds::of(patches);
        let mut places = vec![Vec::new(); ids.count];
        for (row, item) in self.lines.iter().enumerate() {
            for side in [0, RIGHT] {
                let id = item.location(side).and_then(|at| ids.id(at));
                if let Some(found) = id.and_then(|id| places.get_mut(id)) {
                    found.push((row, side));
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
        let mut all: Vec<[Vec<Range<usize>>; 2]> = vec![Default::default(); rows];
        let mut now: Vec<[Vec<Range<usize>>; 2]> = vec![Default::default(); rows];
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
                    all[row][col].push(span.bytes.clone());
                    if standing {
                        now[row][col].push(span.bytes.clone());
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

    /// One column's hits on one row, ranges of the source line's bytes,
    /// spelled the way the row draws the line.
    fn spelled_found(
        &self,
        side: i32,
        row: usize,
        all: &[Range<usize>],
        now: &[Range<usize>],
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

/// Byte ranges in line order, run together where they touch or overlap,
/// as `spelled_ranges` reads them. Both sides' hits can run into one
/// shared line (`old end\nke` and `new end\nkeep` both end on `keep`), and
/// the line wears the longer. An empty range moves no end, so it never
/// joins two that stand apart.
fn joined(ranges: &[Range<usize>]) -> Vec<Range<usize>> {
    let mut sorted = ranges.to_vec();
    sorted.sort_unstable_by_key(|range| (range.start, range.end));
    let mut out: Vec<Range<usize>> = Vec::with_capacity(sorted.len());
    for range in sorted {
        match out.last_mut() {
            Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
            _ => out.push(range),
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
