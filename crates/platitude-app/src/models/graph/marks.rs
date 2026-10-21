//! What the walk knew about each drawn row that no delegate draws: the
//! marks it left, the parenthood the lanes only picture, and the index
//! a row is found by.
//!
//! Beside the rows: `GraphRowItem` is at the fifteen fields
//! `#[derive(QModelItem)]` allows, and none of this is a role — menus ask
//! for it as they open (`publishedAt` / `rebaseRewritesPublished` /
//! `branchDeleteMerged`), and a choice of commits for the lanes between
//! its rows (`chosenRows`). Kept in step with the rows at the four places
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
                    home: carried.main,
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

    /// Whether drawn row `child` names drawn row `parent` among its parents —
    /// the two ends of one of the graph's edges, with no node on it between.
    pub(super) fn is_parent_of(&self, parent: usize, child: usize) -> bool {
        self.marks
            .get(parent)
            .is_some_and(|mark| self.parents_of(child).contains(&mark.oid))
    }

    /// The lanes a list of drawn rows (walk order) draws between neighbours,
    /// as `(above, below)` per row (`ChosenRow::lane_above`): "" at the
    /// list's two ends, "solid" from a row down to its parent, as the graph
    /// draws that edge, and "dashed" otherwise — commits between the two,
    /// a row from another line, or a stash's leash to its base, which the
    /// graph dashes too.
    pub(super) fn chosen_lanes(&self, rows: &[usize]) -> Vec<(&'static str, &'static str)> {
        let links: Vec<&'static str> = rows
            .windows(2)
            .map(|pair| {
                let stash = self
                    .rows
                    .get(pair[0])
                    .is_some_and(|row| !row.stash_ref.is_empty());
                if self.is_parent_of(pair[1], pair[0]) && !stash {
                    "solid"
                } else {
                    "dashed"
                }
            })
            .collect();
        (0..rows.len())
            .map(|k| {
                let above = k.checked_sub(1).and_then(|up| links.get(up)).copied();
                (above.unwrap_or(""), links.get(k).copied().unwrap_or(""))
            })
            .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(n: u8) -> Oid {
        Oid::from_hex_str(&format!("{n:02x}").repeat(20)).unwrap()
    }

    /// 3 ─ 2 ─ 1, with 4 forked off 1 and drawn between them.
    #[test]
    fn a_row_is_joined_to_the_parents_it_names_and_to_nothing_else() {
        let mut model = GraphModel::default();
        model.push_mark(oid(3), false, &[oid(2)], None);
        model.push_mark(oid(4), false, &[oid(1)], None);
        model.push_mark(oid(2), false, &[oid(1)], None);
        model.push_mark(oid(1), false, &[], None);

        assert!(model.is_parent_of(2, 0), "2 is 3's parent");
        assert!(model.is_parent_of(3, 2), "1 is 2's parent");
        assert!(!model.is_parent_of(2, 1), "4 does not come from 2");
        assert!(!model.is_parent_of(0, 2), "a child is no parent");
        assert!(
            !model.is_parent_of(9, 0),
            "a row past the window is nobody's parent"
        );
    }

    /// The same four rows, listed as a choice would list them.
    #[test]
    fn a_choice_draws_solid_lanes_to_parents_and_dashed_ones_elsewhere() {
        let mut model = GraphModel::default();
        model.push_mark(oid(3), false, &[oid(2)], None);
        model.push_mark(oid(4), false, &[oid(1)], None);
        model.push_mark(oid(2), false, &[oid(1)], None);
        model.push_mark(oid(1), false, &[], None);

        assert_eq!(
            model.chosen_lanes(&[0, 2, 3]),
            [("", "solid"), ("solid", "solid"), ("solid", "")],
            "a straight run is solid all the way down"
        );
        assert_eq!(
            model.chosen_lanes(&[0, 1, 3]),
            [("", "dashed"), ("dashed", "solid"), ("solid", "")],
            "a row from another line is dashed to from above"
        );
        assert_eq!(
            model.chosen_lanes(&[0, 3]),
            [("", "dashed"), ("dashed", "")],
            "a commit between two chosen ones dashes the lane"
        );
        assert_eq!(model.chosen_lanes(&[2]), [("", "")], "one row has no lane");
    }

    /// A stash's leash to its base is dashed in the graph, so it is here.
    #[test]
    fn a_stash_is_dashed_to_its_base() {
        let mut model = GraphModel::default();
        model.push_mark(oid(5), false, &[oid(1)], None);
        model.push_mark(oid(1), false, &[], None);
        model.rows = vec![
            GraphRowItem {
                stash_ref: "stash@{0}".into(),
                ..GraphRowItem::default()
            },
            GraphRowItem::default(),
        ];

        assert!(
            model.is_parent_of(1, 0),
            "the walk keeps the base as the stash's parent"
        );
        assert_eq!(
            model.chosen_lanes(&[0, 1]),
            [("", "dashed"), ("dashed", "")]
        );
    }
}
