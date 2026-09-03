//! What the walk knew about each drawn row that no delegate draws: the
//! marks it left, and the parenthood the lanes only picture.
//!
//! **Beside the rows rather than on them.** `GraphRowItem` is at the
//! fifteen fields `#[derive(QModelItem)]` allows, and nothing here is a
//! role — it is asked for when a menu opens, the way the stash selector
//! is (`publishedAt` / `rebaseRewritesPublished`). Kept in step with the
//! rows at the three places they move: cleared in `reset_unnotified`,
//! extended in `take_chunk`, rebuilt whole in `replace_walk`.

use platitude_core::Oid;
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
    }

    /// Takes the marks off a chunk of walked rows, in the rows' order.
    pub(super) fn extend_marks(&mut self, rows: &[LogRow]) {
        self.marks.reserve(rows.len());
        for row in rows {
            self.parent_oids.extend(row.parents.iter().copied());
            self.marks.push(RowMark {
                // `oid_hex` is `Oid::to_hex` on the way in, so this reads
                // back. A row that somehow arrived with an id that does
                // not is answered as the nowhere id the WIP row carries:
                // no range ends on it and nothing names it as a parent.
                oid: Oid::from_hex_str(&row.oid_hex).unwrap_or_else(|_| Oid::zero_unsized()),
                published: row.published,
                parents_end: self.parent_oids.len() as u32,
            });
        }
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

    /// The drawn rows as the range question reads them, in walk order.
    pub(super) fn walked_rows(&self) -> impl Iterator<Item = WalkedRow<'_>> {
        self.marks.iter().enumerate().map(|(row, mark)| WalkedRow {
            oid: mark.oid,
            parents: self.parents_of(row),
            published: mark.published,
        })
    }

    /// Where the working tree stands, as an id; `None` while HEAD is
    /// outside the window and no row has it (`head::settle_head`).
    pub(super) fn head_oid(&self) -> Option<Oid> {
        usize::try_from(self.head_row)
            .ok()
            .and_then(|row| self.marks.get(row))
            .map(|mark| mark.oid)
    }

    /// Which row an id is drawn on; `None` when none is.
    pub(super) fn row_at(&self, oid: &Oid) -> Option<usize> {
        self.marks.iter().position(|mark| mark.oid == *oid)
    }
}
