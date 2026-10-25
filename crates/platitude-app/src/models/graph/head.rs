//! Where the worktree stands in the loaded rows, and what that row
//! draws — for the stand-in (`GraphHeadPin`) shown while the row is
//! scrolled off. Properties, because the rows and their chips land in
//! separate passes (app-ui.md「QML バインディングはプロパティにしか反応しない」).

use super::*;

fn take<T: PartialEq + Clone>(slot: &mut T, value: &T, moved: &mut bool) {
    if slot != value {
        *slot = value.clone();
        *moved = true;
    }
}

impl GraphModel {
    /// Re-reads which loaded row the worktree stands on and copies off
    /// it what the stand-in draws; answers whether any of that moved.
    /// Called after every drain and re-marking — the rows, their chips, the
    /// HEAD report and the find line each move the answer on their own.
    /// Unmoved fields are not rewritten, so the steady case allocates
    /// nothing.
    ///
    /// Found by the id the session reported (`GraphMsg::Head`), not by the
    /// chips: those land a pass after the row, and a HEAD that moved under
    /// a standing graph leads the pin without waiting for the rebuild.
    ///
    /// A HEAD reported ahead of the rows that draw it (a terminal commit
    /// polls in before its rebuild) keeps the pin on the last row that was
    /// HEAD's until a pass lands (`pinned_oid`, cleared by `finish_walk` /
    /// `replace_walk`), rather than blinking out for the rebuild.
    ///
    /// -1 where HEAD is outside the window or not yet reported.
    pub(super) fn settle_head(&mut self) -> bool {
        let found = self.head_oid.and_then(|oid| self.row_at(&oid));
        if found.is_some() {
            self.pinned_oid = self.head_oid;
        }
        let found = found.or_else(|| self.pinned_oid.and_then(|oid| self.row_at(&oid)));
        let mut moved = false;
        let head_row = found.map_or(-1, |i| i as i32);
        if self.head_row != head_row {
            self.head_row = head_row;
            moved = true;
        }
        // Outside the window the stand-in reads the empty answers.
        let none = GraphRowItem::default();
        let row = found.and_then(|i| self.rows.get(i)).unwrap_or(&none);
        take(&mut self.head_row_oid, &row.oid_hex, &mut moved);
        take(&mut self.head_labels, &row.labels, &mut moved);
        take(&mut self.head_subject, &row.subject, &mut moved);
        take(&mut self.head_avatar_url, &row.avatar_url, &mut moved);
        take(&mut self.head_geometry, &row.geometry, &mut moved);
        take(&mut self.head_matched, &row.matched, &mut moved);
        let (color, lane, avatar) = (row.node_color, row.node_lane, row.avatar);
        if (self.head_color, self.head_lane, self.head_avatar) != (color, lane, avatar) {
            (self.head_color, self.head_lane, self.head_avatar) = (color, lane, avatar);
            moved = true;
        }
        moved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The stand-in names the commit it stands for: another commit taking
    /// HEAD's row in place (an amend, a reset) moves the answer though the
    /// row is the same, so what hung off the old commit's chip can go
    /// (`GraphHeadPin`).
    #[test]
    fn the_stand_in_names_the_commit_on_heads_row() {
        let first = "a1".repeat(20);
        let second = "b2".repeat(20);
        let oid = Oid::from_hex_str(&first).expect("an id");
        let mut model = GraphModel {
            rows: vec![GraphRowItem {
                oid_hex: first.clone(),
                ..GraphRowItem::default()
            }],
            index: vec![(oid, 0)],
            head_oid: Some(oid),
            ..GraphModel::default()
        };
        assert!(model.settle_head(), "the first read moves it");
        assert_eq!(model.head_row, 0);
        assert_eq!(model.head_row_oid, first);
        assert!(!model.settle_head(), "read again, nothing moved");

        let amended = Oid::from_hex_str(&second).expect("an id");
        model.rows[0].oid_hex = second.clone();
        model.index = vec![(amended, 0)];
        model.head_oid = Some(amended);
        assert!(
            model.settle_head(),
            "another commit on the same row moves it"
        );
        assert_eq!(model.head_row, 0, "the row is the same");
        assert_eq!(model.head_row_oid, second);

        model.head_oid = None;
        model.pinned_oid = None;
        assert!(model.settle_head());
        assert_eq!(model.head_row, -1);
        assert_eq!(
            model.head_row_oid, "",
            "outside the window it stands for nothing"
        );
    }
}
