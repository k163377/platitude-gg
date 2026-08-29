//! GraphModel: the commit graph rows.

use std::sync::Arc;
use std::time::Instant;

use platitude_core::find::{Query, Row};
use platitude_core::session::LogRow;
use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, qobject};

use crate::encode::{co_author_pairs, encode_geometry, encode_labels, label_names};
use crate::hub::{Feed, GraphMsg};

use super::{impl_extend_notified, impl_notify_runs, push_run, qml_register};

mod find;
mod head;
mod item;
mod qobject;
mod stream;

use item::{GraphRowItem, to_row_item};

#[derive(Default)]
pub struct GraphModel {
    rows: Vec<GraphRowItem>,
    generation: u64,
    loading: bool,
    /// Chip records to leave undrawn — see the property's own note in
    /// `qobject.rs`. Nothing here reads it: the rows are handed out by
    /// reference (`QListModel::get`), so the leaving-out happens where
    /// the chips are drawn.
    gone_chips: String,
    row_total: i32,
    /// Commits the walk emitted — the truncation footer's number. Equals
    /// the window limit whenever `truncated`, where `row_total` drifts
    /// off it (the WIP row adds one, sifted stash parents subtract).
    walked_total: i32,
    max_lanes: i32,
    first_chunk_ms: i32,
    total_ms: i32,
    truncated: bool,
    /// How many commits one press of the tail would add, as the session
    /// has it (`RepoSession::log_window_step`). Read from there when a
    /// walk settles rather than held as a number of our own: the window
    /// the step is a quarter of is the session's to say, and the footer
    /// names the step in the words it offers.
    window_step: i32,
    /// A press of the tail is out and the wider walk has not landed yet.
    /// The footer wears the wait where the words are, like every other
    /// press that goes to git (デザイン規約 §進行中・長押しの定数).
    growing: bool,
    /// Completed stream passes (direct + replacements + reloads). QML
    /// watches this edge to re-resolve the selection by oid.
    finish_count: i32,
    /// Model resets (streaming restarts only — in-place replacements do
    /// not reset). QML re-anchors the viewport only when this moves,
    /// because only a reset zeroes the scroll position.
    reset_count: i32,
    /// What the find bar is looking for, and how many rows answer it.
    /// Held here because the rows are: every pass that rebuilds them has
    /// to re-mark them, or a background refresh would quietly put the
    /// light out while the bar still says how many are lit.
    query: Option<Query>,
    match_count: i32,
    /// Whether anything is being looked for at all — which is not the
    /// same as anything being found. What dims the rows: with a query
    /// and no answers, every row is "not one of them".
    searching: bool,
    /// Whether the newest row is one of the answers. The graph steps down
    /// from under the card for it (規約 §コミットを探す), and since the
    /// working-tree row can never match, this is also the answer to "is
    /// the tree clean".
    first_matched: bool,
    /// Lanes running off the end of the window (`t<lane>.<color>;...`,
    /// uppercase for a dashed leash — `encode::tail_lanes`), drawn by the
    /// truncation footer.
    tail_geometry: String,
    /// The loaded row the working tree stands on, and what a stand-in for
    /// it draws — the chip records it carries and the subject it says.
    /// -1 and two empty strings while HEAD is outside the window, which
    /// is a row nothing can lead to (`head::settle_head`).
    head_row: i32,
    head_labels: String,
    head_subject: String,
    head_color: i32,
    head_lane: i32,
    head_avatar: i32,
    head_avatar_url: String,
    head_geometry: String,
    error: String,
    /// A pass reported that it could not draw the graph. **Not the same as
    /// `error` being set**: a walk that ended without an answer at all —
    /// the task fell over — has nobody's words to show, and the screen
    /// says that in its own (`Words.graphStopped`, app-ui.md「Rust に文言を
    /// 置かない」).
    failed: bool,
    started_at: Option<Instant>,
    feed: Option<Arc<Feed<GraphMsg>>>,
    tab_id: i32,
}

impl QListModel for GraphModel {
    type Item = GraphRowItem;

    fn len(&self) -> usize {
        self.rows.len()
    }
    fn get(&self, index: usize) -> Option<&GraphRowItem> {
        self.rows.get(index)
    }
    fn set_unnotified(&mut self, index: usize, value: GraphRowItem) -> bool {
        match self.rows.get_mut(index) {
            Some(slot) => {
                *slot = value;
                true
            }
            None => false,
        }
    }
    fn reset_unnotified(&mut self) {
        self.rows.clear();
    }
}

impl_extend_notified!(GraphModel, rows, GraphRowItem);
impl_notify_runs!(GraphModel);

qml_register!(GraphModel, "GraphModel", singleton = false);
