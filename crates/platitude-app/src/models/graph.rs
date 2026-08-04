use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use platitude_core::session::LogRow;
use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, qobject};

use crate::encode::{encode_geometry, encode_labels};
use crate::hub::{Feed, GraphMsg, Hub};

use super::{impl_extend_notified, qml_register};

// ---------------------------------------------------------------------------
// GraphModel: the commit graph rows
// ---------------------------------------------------------------------------

// Kept lean: one instance per commit in the window. The short sha is
// derived in QML from `oid_hex` (mechanical substring); `avatar` is a
// packed local identicon code (see encode::avatar_code). PartialEq
// feeds the in-place replacement: unchanged rows emit no dataChanged.
#[derive(QModelItem, Default, Clone, PartialEq)]
pub struct GraphRowItem {
    oid_hex: String,
    author: String,
    atime: i64,
    subject: String,
    node_lane: i32,
    node_color: i32,
    row_width: i32,
    avatar: i32,
    geometry: String,
    labels: String,
    /// `stash@{n}` when the row is a stash; empty otherwise.
    stash_ref: String,
}

#[derive(Default)]
pub struct GraphModel {
    rows: Vec<GraphRowItem>,
    generation: u64,
    loading: bool,
    row_total: i32,
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
    /// Lanes running off the end of the window (`lane.color;...`), drawn
    /// by the truncation footer.
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

impl GraphModel {
    /// Replaces the whole list in place: unchanged rows stay untouched,
    /// contiguous runs of changed rows emit one ranged dataChanged, and
    /// only the length delta inserts or removes rows. No model reset —
    /// the view keeps its scroll position and never shows an empty list.
    #[expect(unsafe_code)]
    fn splice_notified(&mut self, new_rows: Vec<GraphRowItem>) {
        let old_len = self.rows.len();
        let new_len = new_rows.len();
        let common = old_len.min(new_len);
        let mut head = new_rows;
        let extra = head.split_off(common);

        // In-place writes first (no Qt runs between here and the
        // notifications below — everything happens inside one slot).
        let mut ranges: Vec<(usize, usize)> = Vec::new();
        for (i, item) in head.into_iter().enumerate() {
            if self.rows[i] != item {
                self.rows[i] = item;
                match ranges.last_mut() {
                    Some((_, last)) if *last + 1 == i => *last = i,
                    _ => ranges.push((i, i)),
                }
            }
        }

        if old_len > new_len {
            if let Some(proxy) = self.try_get_rust_proxy_ptr() {
                // SAFETY: same pattern as QListModelBase::remove — the
                // proxy pointer stays valid while the QObject side is
                // attached, and we are on the Qt main thread in a slot.
                unsafe { &mut *proxy }.base_begin_remove_rows(
                    &mut *self,
                    &qtbridge::qtbridge_type_lib::QModelIndex::default(),
                    new_len as i32,
                    old_len as i32 - 1,
                );
                self.rows.truncate(new_len);
                // SAFETY: see above.
                unsafe { &mut *proxy }.base_end_remove_rows(&mut *self);
            } else {
                self.rows.truncate(new_len);
            }
        }

        if let Some(proxy) = self.try_get_rust_proxy_ptr() {
            for (first, last) in ranges {
                // SAFETY: see above; base_index only builds an index.
                let top_left = unsafe { &*proxy }.base_index(
                    &*self,
                    first as i32,
                    0,
                    &qtbridge::qtbridge_type_lib::QModelIndex::default(),
                );
                // SAFETY: see above; base_index only builds an index.
                let bottom_right = unsafe { &*proxy }.base_index(
                    &*self,
                    last as i32,
                    0,
                    &qtbridge::qtbridge_type_lib::QModelIndex::default(),
                );
                // SAFETY: see above.
                unsafe { &mut *proxy }.base_data_changed(&mut *self, &top_left, &bottom_right);
            }
        }

        if !extra.is_empty() {
            self.extend_notified(extra);
        }
    }
}

#[qobject(Base = QListModel, ConvertToCamelCase, NoQmlElement)]
impl GraphModel {
    qproperty!("loading", Member = loading, Notify = stats_changed);
    qproperty!("rowTotal", Member = row_total, Notify = stats_changed);
    qproperty!("maxLanes", Member = max_lanes, Notify = stats_changed);
    qproperty!(
        "firstChunkMs",
        Member = first_chunk_ms,
        Notify = stats_changed
    );
    qproperty!("totalMs", Member = total_ms, Notify = stats_changed);
    qproperty!("truncated", Member = truncated, Notify = stats_changed);
    qproperty!("finishCount", Member = finish_count, Notify = stats_changed);
    qproperty!("resetCount", Member = reset_count, Notify = stats_changed);
    qproperty!(
        "tailGeometry",
        Member = tail_geometry,
        Notify = stats_changed
    );
    qproperty!("error", Member = error, Notify = stats_changed);

    #[qsignal]
    fn stats_changed(&mut self);

    #[qslot]
    fn attach(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
        if let Some(Some(feeds)) = Hub::with(|hub| hub.feeds(tab_id)) {
            let feed = Arc::clone(&feeds.graph);
            feed.attach(self.get_qml_method_invoker());
            self.feed = Some(feed);
        }
    }

    #[qslot]
    fn drain(&mut self) {
        let Some(feed) = self.feed.clone() else {
            return;
        };
        for msg in feed.drain() {
            match msg {
                GraphMsg::Started { generation } => {
                    if generation > self.generation {
                        self.generation = generation;
                        self.reset();
                        self.reset_count += 1;
                        self.loading = true;
                        self.row_total = 0;
                        self.max_lanes = 1;
                        self.first_chunk_ms = -1;
                        self.total_ms = -1;
                        self.truncated = false;
                        self.error = String::new();
                        self.started_at = Some(Instant::now());
                    }
                }
                GraphMsg::Chunk { generation, rows } => {
                    if generation != self.generation {
                        continue;
                    }
                    if self.first_chunk_ms < 0
                        && let Some(t0) = self.started_at
                    {
                        self.first_chunk_ms = t0.elapsed().as_millis() as i32;
                        tracing::info!(first_chunk_ms = self.first_chunk_ms, "graph first chunk");
                    }
                    let items: Vec<GraphRowItem> = rows.iter().map(to_row_item).collect();
                    for item in &items {
                        self.max_lanes = self.max_lanes.max(item.row_width);
                    }
                    self.extend_notified(items);
                    self.row_total = self.rows.len() as i32;
                }
                GraphMsg::Labels { rows } => {
                    for (row, labels) in rows {
                        let idx = row as usize;
                        if let Some(existing) = self.rows.get(idx) {
                            let mut updated = existing.clone();
                            updated.labels = encode_labels(&labels);
                            self.set(idx, updated);
                        }
                    }
                }
                GraphMsg::Finished {
                    generation,
                    total,
                    elapsed_ms,
                    truncated,
                } => {
                    if generation == self.generation {
                        self.loading = false;
                        self.total_ms = elapsed_ms as i32;
                        self.row_total = total as i32;
                        self.truncated = truncated;
                        self.finish_count += 1;
                        self.tail_geometry = if truncated {
                            self.rows
                                .last()
                                .map(|r| crate::encode::tail_lanes(&r.geometry))
                                .unwrap_or_default()
                        } else {
                            String::new()
                        };
                        // Release Vec growth slack after the stream ends.
                        self.rows.shrink_to_fit();
                        tracing::info!(total, elapsed_ms, truncated, "graph stream finished");
                    }
                }
                GraphMsg::Replaced {
                    generation,
                    rows,
                    elapsed_ms,
                    truncated,
                } => {
                    if generation <= self.generation {
                        continue; // superseded by a newer stream
                    }
                    self.generation = generation;
                    let items: Vec<GraphRowItem> = rows.iter().map(to_row_item).collect();
                    self.max_lanes = items
                        .iter()
                        .map(|item| item.row_width)
                        .max()
                        .unwrap_or(1)
                        .max(1);
                    self.splice_notified(items);
                    self.loading = false;
                    self.first_chunk_ms = 0;
                    self.total_ms = elapsed_ms as i32;
                    self.row_total = self.rows.len() as i32;
                    self.truncated = truncated;
                    self.finish_count += 1;
                    self.tail_geometry = if truncated {
                        self.rows
                            .last()
                            .map(|r| crate::encode::tail_lanes(&r.geometry))
                            .unwrap_or_default()
                    } else {
                        String::new()
                    };
                    self.error = String::new();
                    tracing::info!(
                        total = self.row_total,
                        elapsed_ms,
                        truncated,
                        "graph replaced in place"
                    );
                }
                GraphMsg::Failed {
                    generation,
                    message,
                } => {
                    if generation == self.generation {
                        self.loading = false;
                        self.error = message;
                    }
                }
            }
        }
        self.stats_changed();
    }

    /// Row index of a commit (sidebar jump); -1 when absent.
    #[qslot]
    fn row_of(&self, oid_hex: String) -> i32 {
        self.rows
            .iter()
            .position(|r| r.oid_hex == oid_hex)
            .map_or(-1, |i| i as i32)
    }

    /// Reflog selector when the commit is a stash row (empty otherwise).
    #[qslot]
    fn stash_ref_of(&self, oid_hex: String) -> String {
        self.rows
            .iter()
            .find(|r| r.oid_hex == oid_hex)
            .map(|r| r.stash_ref.clone())
            .unwrap_or_default()
    }

    /// Full commit id at a row (selection / keyboard navigation).
    #[qslot]
    fn oid_at(&self, row: i32) -> String {
        usize::try_from(row)
            .ok()
            .and_then(|i| self.rows.get(i))
            .map(|r| r.oid_hex.clone())
            .unwrap_or_default()
    }
}
qml_register!(GraphModel, "GraphModel", singleton = false);

fn to_row_item(row: &LogRow) -> GraphRowItem {
    GraphRowItem {
        oid_hex: row.oid_hex.clone(),
        author: row.author.clone(),
        atime: row.time,
        subject: row.subject.clone(),
        node_lane: i32::from(row.node_lane),
        node_color: i32::from(row.node_color),
        row_width: i32::from(row.width),
        avatar: crate::encode::avatar_code(&row.author),
        geometry: encode_geometry(&row.segments),
        labels: encode_labels(&row.labels),
        stash_ref: row.stash_ref.clone(),
    }
}
