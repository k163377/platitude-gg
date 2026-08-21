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
mod item;
mod qobject;
mod stream;

// The siblings reach these through `use super::*`, which sees what this
// module can see — so the row item stays inside `graph` and nothing
// leaves it but the model itself (`models::mod`).
use item::{GraphRowItem, to_row_item};

#[derive(Default)]
pub struct GraphModel {
    rows: Vec<GraphRowItem>,
    generation: u64,
    loading: bool,
    row_total: i32,
    /// Commits the walk emitted — the truncation footer's number. Equals
    /// the window limit whenever `truncated`, where `row_total` drifts
    /// off it (the WIP row adds one, sifted stash parents subtract).
    walked_total: i32,
    max_lanes: i32,
    first_chunk_ms: i32,
    total_ms: i32,
    truncated: bool,
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
    error: String,
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
