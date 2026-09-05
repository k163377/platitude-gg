//! What the walk knew about each drawn row that no delegate draws: the
//! marks it left, the parenthood the lanes only picture, and the index
//! a row is found by.
//!
//! **Beside the rows rather than on them.** `GraphRowItem` is at the
//! fifteen fields `#[derive(QModelItem)]` allows, and nothing here is a
//! role — it is asked for when a menu opens, the way the stash selector
//! is (`publishedAt` / `rebaseRewritesPublished` / `reaches`). Kept in
//! step with the rows at the three places they move: cleared in
//! `reset_unnotified`, extended in `take_chunk`, rebuilt whole in
//! `replace_walk`.

use platitude_core::publish::WalkedRow;

use super::*;

/// One row's share of that: its id, whether a remote already has it, and
/// where its parents end in [`GraphModel::parent_oids`].
#[derive(Clone, Copy)]
pub(super) struct RowMark {
    oid: Oid,
    published: bool,
    /// One past this row's last parent; the row before says where they
    /// start, and the first row starts at zero. **A span rather than a
    /// list per row** — the window is two allocations this way and two
    /// thousand the other, and this is the part of the graph's cost that
    /// grows with what somebody asks to see.
    parents_end: u32,
}

impl GraphModel {
    pub(super) fn clear_marks(&mut self) {
        self.marks.clear();
        self.parent_oids.clear();
        self.index.clear();
    }

    /// Takes the marks off a chunk of walked rows, in the rows' order.
    pub(super) fn extend_marks(&mut self, rows: &[LogRow]) {
        self.marks.reserve(rows.len());
        self.index.reserve(rows.len());
        for row in rows {
            // `oid_hex` is `Oid::to_hex` on the way in, so this reads
            // back. A row that somehow arrived with an id that does not
            // is answered as the nowhere id the WIP row carries: no range
            // ends on it and nothing names it as a parent.
            let oid = Oid::from_hex_str(&row.oid_hex).unwrap_or_else(|_| Oid::zero_unsized());
            self.parent_oids.extend(row.parents.iter().copied());
            self.index.push((oid, self.marks.len() as u32));
            self.marks.push(RowMark {
                oid,
                published: row.published,
                parents_end: self.parent_oids.len() as u32,
            });
        }
        // Sorted once per chunk rather than kept in order per row: the
        // walk lands in a handful of chunks, and sorting a window is
        // nothing beside turning its rows into items. The stable sort,
        // because it finds the runs already in order — everything before
        // this chunk — and merges the chunk in, rather than sorting the
        // whole window again per chunk.
        self.index.sort();
    }

    /// The whole window at once, for a pass that replaced it.
    pub(super) fn replace_marks(&mut self, rows: &[LogRow]) {
        self.clear_marks();
        self.extend_marks(rows);
    }

    pub(super) fn published_row(&self, row: usize) -> bool {
        self.marks.get(row).is_some_and(|mark| mark.published)
    }

    fn parents_of(&self, row: usize) -> &[Oid] {
        let Some(mark) = self.marks.get(row) else {
            return &[];
        };
        let start = row
            .checked_sub(1)
            .and_then(|before| self.marks.get(before))
            .map_or(0, |before| before.parents_end as usize);
        self.parent_oids
            .get(start..mark.parents_end as usize)
            .unwrap_or_default()
    }

    /// The drawn rows as the range questions read them, in walk order.
    pub(super) fn walked_rows(&self) -> impl Iterator<Item = WalkedRow<'_>> {
        self.marks.iter().enumerate().map(|(row, mark)| WalkedRow {
            oid: mark.oid,
            parents: self.parents_of(row),
            published: mark.published,
        })
    }

    /// Which row an id is drawn on; `None` when none is.
    pub(super) fn row_at(&self, oid: &Oid) -> Option<usize> {
        self.index
            .binary_search_by(|(drawn, _)| drawn.cmp(oid))
            .ok()
            .and_then(|at| self.index.get(at))
            .map(|(_, row)| *row as usize)
    }

    /// The same for an id given as hex — `None` for a string that is no
    /// id at all, which no row can be drawn for.
    pub(super) fn row_of_hex(&self, oid_hex: &str) -> Option<usize> {
        let oid = Oid::from_hex_str(oid_hex).ok()?;
        self.row_at(&oid)
    }

    /// Whether `from` reaches `to` off the drawn rows, or `None` where the
    /// window cannot say (`publish::reaches`). Both ends have to be drawn
    /// for the rows to say anything, and the index answers that before
    /// the rows are walked — the branch older than the window, which is
    /// the one git is then asked about, costs no walk.
    pub(super) fn reaches_between(&self, from: Oid, to: Oid) -> Option<bool> {
        if from != to && (self.row_at(&from).is_none() || self.row_at(&to).is_none()) {
            return None;
        }
        platitude_core::publish::reaches(self.walked_rows(), from, to)
    }
}
