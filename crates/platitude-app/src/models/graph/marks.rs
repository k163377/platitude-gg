//! What the walk knew about each drawn row that no delegate draws: the
//! marks it left, the parenthood the lanes only picture, and the index
//! a row is found by.
//!
//! Beside the rows: `GraphRowItem` is at the fifteen fields
//! `#[derive(QModelItem)]` allows, and none of this is a role — menus ask
//! for it as they open (`publishedAt` / `rebaseRewritesPublished` /
//! `branchDeleteMerged`). Kept in step with the rows at the four places
//! they move: cleared in `reset_unnotified`, extended in `take_chunk`,
//! rebuilt whole in `replace_walk` and `relay_walk`.

use platitude_core::publish::WalkedRow;

use super::*;

/// One row's share: its id, whether a remote already has it, and where
/// its parents end in [`GraphModel::parent_oids`].
#[derive(Clone, Copy)]
pub(super) struct RowMark {
    oid: Oid,
    published: bool,
    /// One past this row's last parent; the row before says where they
    /// start (zero for the first). A span, not a list per row: two
    /// allocations for the whole window instead of one per row.
    parents_end: u32,
}

impl RowMark {
    pub(super) fn oid(&self) -> Oid {
        self.oid
    }
}

impl GraphModel {
    pub(super) fn clear_marks(&mut self) {
        self.marks.clear();
        self.parent_oids.clear();
        self.index.clear();
        self.carried.clear();
        self.carried_revision = self.carried_revision.wrapping_add(1);
    }

    /// Takes the marks off a chunk of walked rows, in the rows' order.
    pub(super) fn extend_marks(&mut self, rows: &[LogRow]) {
        self.marks.reserve(rows.len());
        self.index.reserve(rows.len());
        for row in rows {
            // `oid_hex` came from `Oid::to_hex`; one that does not parse
            // becomes the WIP row's zero id, which no range ends on and no
            // row names as a parent.
            let oid = Oid::from_hex_str(&row.oid_hex).unwrap_or_else(|_| Oid::zero_unsized());
            self.push_mark(oid, row.published, &row.parents, row.carried.as_ref());
        }
        // Once per chunk (a handful per walk). The stable sort: it finds
        // everything before this chunk already in order and merges the
        // chunk in.
        self.index.sort();
    }

    /// One row's marks, after the ones already kept — a walked row's, or a
    /// row a relaying keeps (`relay.rs`). The caller sorts the index.
    pub(super) fn push_mark(
        &mut self,
        oid: Oid,
        published: bool,
        parents: &[Oid],
        carried: Option<&platitude_core::session::Carried>,
    ) {
        // A tally is a count of files; one past `i32` is not a working
        // tree anybody has.
        let count = |n: usize| i32::try_from(n).unwrap_or(i32::MAX);
        // Filed by the index the delegate asks with. All six written even
        // when zero: none at all is what says a row is not another worktree's
        // (`carried_tally`).
        if let Some(carried) = carried {
            let k = &carried.kinds;
            self.carried.insert(
                self.marks.len(),
                super::CarriedRow {
                    name: carried.name.to_string(),
                    path: carried.path.clone(),
                    tally: Optional::some(Tally {
                        added: count(k.added),
                        modified: count(k.modified),
                        deleted: count(k.deleted),
                        renamed: count(k.renamed),
                        copied: count(k.copied),
                        conflicted: count(k.conflicted),
                    }),
                },
            );
            self.carried_revision = self.carried_revision.wrapping_add(1);
        }
        self.parent_oids.extend(parents.iter().copied());
        self.index.push((oid, self.marks.len() as u32));
        self.marks.push(RowMark {
            oid,
            published,
            parents_end: self.parent_oids.len() as u32,
        });
    }

    /// The whole window at once, for a pass that replaced it.
    pub(super) fn replace_marks(&mut self, rows: &[LogRow]) {
        self.clear_marks();
        self.extend_marks(rows);
    }

    pub(super) fn published_row(&self, row: usize) -> bool {
        self.marks.get(row).is_some_and(|mark| mark.published)
    }

    pub(super) fn parents_of(&self, row: usize) -> &[Oid] {
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

    /// The same for an id given as hex; `None` for a string that is no id.
    pub(super) fn row_of_hex(&self, oid_hex: &str) -> Option<usize> {
        let oid = Oid::from_hex_str(oid_hex).ok()?;
        self.row_at(&oid)
    }

    /// Whether `from` reaches `to` off the drawn rows, or `None` where the
    /// window cannot say. Both ends must be drawn; the index checks that
    /// first, so an end older than the window (git's to answer then) costs
    /// no walk.
    pub(super) fn reaches_between(&self, from: Oid, to: Oid) -> Option<bool> {
        if from != to && (self.row_at(&from).is_none() || self.row_at(&to).is_none()) {
            return None;
        }
        platitude_core::publish::reaches(self.walked_rows(), from, to)
    }
}
