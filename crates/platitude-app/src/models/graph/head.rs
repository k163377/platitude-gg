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
    /// lane and colour, its face, and the lanes it is on. Called wherever
    /// the rows or their chips move: the walk streams the rows first and
    /// their names after, so the row is found before it can say its own
    /// name.
    ///
    /// -1 is a real answer: the walk is a window, and a HEAD outside it
    /// has no row for the stand-in to lead to.
    pub(super) fn settle_head(&mut self) {
        let found = self
            .rows
            .iter()
            .position(|r| crate::encode::labels_head(&r.labels));
        self.head_row = found.map_or(-1, |i| i as i32);
        let row = found.and_then(|i| self.rows.get(i));
        self.head_labels = row.map(|r| r.labels.clone()).unwrap_or_default();
        self.head_subject = row.map(|r| r.subject.clone()).unwrap_or_default();
        self.head_color = row.map_or(0, |r| r.node_color);
        self.head_lane = row.map_or(0, |r| r.node_lane);
        self.head_avatar = row.map_or(0, |r| r.avatar);
        self.head_avatar_url = row.map(|r| r.avatar_url.clone()).unwrap_or_default();
        self.head_geometry = row.map(|r| r.geometry.clone()).unwrap_or_default();
    }
}
