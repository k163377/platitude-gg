//! Where the working tree stands in the loaded rows, and what that row
//! would show if it were on screen.
//!
//! The pane draws a stand-in for it while the row itself is scrolled off
//! (`GraphHeadPin`), so the answer has to be a property rather than a
//! slot: the rows arrive in chunks and the chips a second pass later,
//! and only a property tells a binding about either (app-ui.md
//! 「QML バインディングはプロパティにしか反応しない」).

use super::*;

impl GraphModel {
    /// Re-reads which loaded row the working tree stands on, and takes
    /// off it everything the stand-in draws — its chips, its subject, its
    /// lane and colour, its face, and the lanes it is on. Called at the
    /// end of every drain: the rows, their chips and the report of where
    /// HEAD is all move the answer, and each lands on its own.
    ///
    /// **The row is found by the id the session reported**
    /// (`GraphMsg::Head`), through the index — not by the chips, which
    /// arrive a pass after the rows and are one refs read's picture. So
    /// the row is found the moment it is drawn, before it can say its
    /// own name, and a HEAD that moved under a standing graph leads the
    /// pin to the new row without waiting for the rebuild.
    ///
    /// A field that has not moved is not rewritten, so the steady case
    /// allocates nothing. Answers whether anything the stand-in draws
    /// moved.
    ///
    /// **A HEAD reported ahead of the rows that draw it keeps the stand-in
    /// where it stood.** A commit made in a terminal is reported by the
    /// poll seconds before the rebuild that draws its row lands, and a
    /// pin blinking out for the length of the rebuild says nothing a
    /// reader can use; the last row that was HEAD's stays pinned until a
    /// pass lands (`pinned_oid`, cleared by `finish_walk` / `replace_walk`).
    ///
    /// -1 is a real answer: the walk is a window, and a HEAD outside it
    /// — or one not yet reported — has no row for the stand-in to lead
    /// to.
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
        let row = found.and_then(|i| self.rows.get(i));
        let mut take_str = |slot: &mut String, value: &str| {
            if slot != value {
                value.clone_into(slot);
                moved = true;
            }
        };
        take_str(
            &mut self.head_labels,
            row.map(|r| r.labels.as_str()).unwrap_or_default(),
        );
        take_str(
            &mut self.head_subject,
            row.map(|r| r.subject.as_str()).unwrap_or_default(),
        );
        take_str(
            &mut self.head_avatar_url,
            row.map(|r| r.avatar_url.as_str()).unwrap_or_default(),
        );
        take_str(
            &mut self.head_geometry,
            row.map(|r| r.geometry.as_str()).unwrap_or_default(),
        );
        let (color, lane, avatar) =
            row.map_or((0, 0, 0), |r| (r.node_color, r.node_lane, r.avatar));
        if (self.head_color, self.head_lane, self.head_avatar) != (color, lane, avatar) {
            (self.head_color, self.head_lane, self.head_avatar) = (color, lane, avatar);
            moved = true;
        }
        moved
    }
}
