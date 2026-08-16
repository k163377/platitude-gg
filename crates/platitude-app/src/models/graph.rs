use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use platitude_core::find::{Query, Row};
use platitude_core::session::LogRow;
use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, qobject};

use crate::encode::{co_author_pairs, encode_geometry, encode_labels, label_names};
use crate::hub::{Feed, GraphMsg};

use super::{impl_extend_notified, qml_register};

// ---------------------------------------------------------------------------
// GraphModel: the commit graph rows
// ---------------------------------------------------------------------------

// Kept lean: one instance per commit in the window. The short sha is
// derived in QML from `oid_hex` (mechanical substring); `avatar` is a
// packed local identicon code (see encode::avatar_code). PartialEq
// feeds the in-place replacement: unchanged rows emit no dataChanged.
//
// **Fifteen fields is the ceiling** — `#[derive(QModelItem)]` refuses a
// sixteenth. Anything the rows need that QML never reads belongs on the
// way in rather than here: the lane count each row needs is taken off
// the `LogRow` while the item is built (`max_lanes`).
#[derive(QModelItem, Default, Clone, PartialEq)]
pub struct GraphRowItem {
    oid_hex: String,
    author: String,
    /// The address the picture is filed under — already folded and read
    /// through mailmap by the log parser. Kept on the row so re-reading
    /// the assignments needs no second pass over git.
    author_email: String,
    atime: i64,
    subject: String,
    node_lane: i32,
    node_color: i32,
    avatar: i32,
    /// A `file:` URL when this author has a picture, empty otherwise —
    /// resolved here rather than in QML so a delegate coming back from
    /// the reuse pool has the answer already in its row.
    avatar_url: String,
    /// Packed `Co-authored-by` records (see `encode::encode_co_authors`) —
    /// the first draws the badge on the node, all of them are named in
    /// the row's hover.
    co_authors: String,
    /// Message body without the co-author trailers — the row's hover.
    body: String,
    geometry: String,
    labels: String,
    /// `stash@{n}` when the row is a stash; empty otherwise.
    stash_ref: String,
    /// The find bar's line is somewhere in this row. False for every row
    /// while nothing is being searched for — the delegate dims off the
    /// pane's own "there is a search on", so an all-false model with no
    /// query dims nothing.
    matched: bool,
}

impl platitude_core::mem::Footprint for GraphRowItem {
    fn heap_bytes(&self) -> usize {
        self.oid_hex.heap_bytes()
            + self.author.heap_bytes()
            + self.author_email.heap_bytes()
            + self.subject.heap_bytes()
            + self.avatar_url.heap_bytes()
            + self.co_authors.heap_bytes()
            + self.body.heap_bytes()
            + self.geometry.heap_bytes()
            + self.labels.heap_bytes()
            + self.stash_ref.heap_bytes()
    }
}

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

    /// Whether the query — if there is one — is somewhere in this row.
    ///
    /// The three packed fields are unpacked through the readers that sit
    /// beside their encoders, so the search sees names and addresses and
    /// never the flags, separators or identicon codes they are packed
    /// with.
    fn hits(query: &Query, item: &GraphRowItem) -> bool {
        let mut people: Vec<&str> = vec![item.author.as_str()];
        let mut addresses: Vec<&str> = vec![item.author_email.as_str()];
        if !item.co_authors.is_empty() {
            for (name, address) in co_author_pairs(&item.co_authors) {
                people.push(name);
                addresses.push(address);
            }
        }
        let mut tokens: Vec<&str> = label_names(&item.labels).collect();
        if !item.stash_ref.is_empty() {
            tokens.push(item.stash_ref.as_str());
        }
        // `body` is not passed: the description is not searched (see
        // `platitude-core::find`). The row still carries it for the hover
        // card.
        query.matches(&Row {
            oid_hex: &item.oid_hex,
            subject: &item.subject,
            people: &people,
            addresses: &addresses,
            tokens: &tokens,
        })
    }

    /// Sets `matched` on the rows already in the model and counts them.
    ///
    /// Written in place rather than through [`Self::splice_notified`]:
    /// that one takes a whole new `Vec`, and cloning every row's strings
    /// on every keystroke is exactly the work this search exists to
    /// avoid. Only the runs that actually changed are notified.
    #[expect(unsafe_code)]
    fn remark_notified(&mut self) {
        let mut ranges: Vec<(usize, usize)> = Vec::new();
        let mut count = 0;
        for i in 0..self.rows.len() {
            let now = match &self.query {
                Some(q) => Self::hits(q, &self.rows[i]),
                None => false,
            };
            if now {
                count += 1;
            }
            if self.rows[i].matched != now {
                self.rows[i].matched = now;
                match ranges.last_mut() {
                    Some((_, last)) if *last + 1 == i => *last = i,
                    _ => ranges.push((i, i)),
                }
            }
        }
        self.match_count = count;
        self.settle_first();
        if ranges.is_empty() {
            return;
        }
        if let Some(proxy) = self.try_get_rust_proxy_ptr() {
            for (first, last) in ranges {
                // SAFETY: same pattern as splice_notified above — the
                // proxy stays valid while the QObject side is attached,
                // and we are on the Qt main thread inside a slot.
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
    }

    /// Marks rows on their way in, before anyone sees them.
    ///
    /// A row arriving under a standing query has to arrive already lit:
    /// marking it afterwards would notify a change on a row nobody has
    /// drawn yet, and a chunk streaming in mid-search would flicker dark
    /// for a frame.
    fn mark_incoming(&self, items: &mut [GraphRowItem]) {
        let Some(query) = &self.query else {
            return;
        };
        for item in items {
            item.matched = Self::hits(query, item);
        }
    }

    /// Re-reads whether the newest row answers the query. Called wherever
    /// the rows or their marks move.
    fn settle_first(&mut self) {
        self.first_matched = self.rows.first().is_some_and(|r| r.matched);
    }

    /// Rows answering the query, as their indices in order.
    fn match_rows(&self) -> impl DoubleEndedIterator<Item = i32> {
        self.rows
            .iter()
            .enumerate()
            .filter(|(_, r)| r.matched)
            .map(|(i, _)| i as i32)
    }
}

#[qobject(Base = QListModel, ConvertToCamelCase, NoQmlElement)]
impl GraphModel {
    qproperty!("loading", Member = loading, Notify = stats_changed);
    qproperty!("rowTotal", Member = row_total, Notify = stats_changed);
    qproperty!("walkedTotal", Member = walked_total, Notify = stats_changed);
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
    // How many loaded rows the find bar's line is in. A property rather
    // than the return of the slot that sets the query: a background
    // refresh re-marks the rows without anybody typing, and a binding is
    // the only thing that hears about that (app-ui.md §QML バインディング
    // はプロパティにしか反応しない). Doc comments do not go on
    // `qproperty!` — the macro rejects attributes.
    qproperty!("matchCount", Member = match_count, Notify = stats_changed);
    // Whether a search is on, and whether the newest row answers it. Both
    // properties for the same reason `matchCount` is.
    qproperty!("searching", Member = searching, Notify = stats_changed);
    qproperty!(
        "firstMatched",
        Member = first_matched,
        Notify = stats_changed
    );
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
        let invoker = self.get_qml_method_invoker();
        self.feed = crate::hub::attach_feed(tab_id, |f| &f.graph, invoker);
    }

    #[qslot]
    #[expect(clippy::too_many_lines)]
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
                        self.walked_total = 0;
                        // The query stands — a restart is the same
                        // history read again — but its answers went with
                        // the rows, and the chunks re-count them.
                        self.match_count = 0;
                        self.first_matched = false;
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
                    let avatars = crate::hub::AvatarUrls::current();
                    let mut items: Vec<GraphRowItem> =
                        rows.iter().map(|row| to_row_item(row, &avatars)).collect();
                    for row in &rows {
                        self.max_lanes = self.max_lanes.max(i32::from(row.width));
                    }
                    self.mark_incoming(&mut items);
                    self.match_count += items.iter().filter(|i| i.matched).count() as i32;
                    self.extend_notified(items);
                    self.settle_first();
                    self.row_total = self.rows.len() as i32;
                }
                GraphMsg::Labels { generation, rows } => {
                    if generation != self.generation {
                        // Row numbers of a graph this model no longer
                        // shows: the chips belong to other commits here.
                        continue;
                    }
                    for (row, labels) in rows {
                        let idx = row as usize;
                        if let Some(existing) = self.rows.get(idx) {
                            let mut updated = existing.clone();
                            updated.labels = encode_labels(&labels);
                            // The names on the row are searched, so the
                            // second pass that puts the chips on can turn
                            // a row's light on or off.
                            if let Some(query) = &self.query {
                                updated.matched = Self::hits(query, &updated);
                                self.match_count +=
                                    i32::from(updated.matched) - i32::from(existing.matched);
                            }
                            self.set(idx, updated);
                        }
                    }
                    self.settle_first();
                }
                GraphMsg::Finished {
                    generation,
                    total,
                    elapsed_ms,
                    walked,
                    truncated,
                } => {
                    if generation == self.generation {
                        self.loading = false;
                        self.total_ms = elapsed_ms as i32;
                        self.row_total = total as i32;
                        self.walked_total = walked as i32;
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
                        self.rows.shrink_to_fit();
                        tracing::info!(total, elapsed_ms, truncated, "graph stream finished");
                    }
                }
                GraphMsg::Replaced {
                    generation,
                    rows,
                    elapsed_ms,
                    walked,
                    truncated,
                } => {
                    if generation <= self.generation {
                        continue; // superseded by a newer stream
                    }
                    self.generation = generation;
                    let avatars = crate::hub::AvatarUrls::current();
                    let mut items: Vec<GraphRowItem> =
                        rows.iter().map(|row| to_row_item(row, &avatars)).collect();
                    // Before the splice, so a rebuild under a standing
                    // query notifies each row once — with its light
                    // already right — instead of twice.
                    self.mark_incoming(&mut items);
                    self.match_count = items.iter().filter(|i| i.matched).count() as i32;
                    self.first_matched = items.first().is_some_and(|i| i.matched);
                    self.max_lanes = rows
                        .iter()
                        .map(|row| i32::from(row.width))
                        .max()
                        .unwrap_or(1)
                        .max(1);
                    self.splice_notified(items);
                    self.loading = false;
                    self.first_chunk_ms = 0;
                    self.total_ms = elapsed_ms as i32;
                    self.row_total = self.rows.len() as i32;
                    self.walked_total = walked as i32;
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
        if crate::memprobe::enabled() {
            crate::memprobe::note("graph-rows", self.tab_id, &self.rows);
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

    /// The lane colour of the row a ref sits on, as an index into the
    /// graph palette; -1 when no row on screen carries that name.
    ///
    /// What it is for: a conflicted file's diff paints each side in the
    /// colour its branch already has in the graph, so the pane borrows an
    /// answer rather than inventing a second one. -1 is a real answer —
    /// the walk is a window (`--max-count`), and a branch outside it has
    /// no colour to borrow.
    #[qslot]
    fn color_of_ref(&self, name: String) -> i32 {
        if name.is_empty() {
            return -1;
        }
        self.rows
            .iter()
            .find(|r| crate::encode::label_names(&r.labels).any(|n| n == name))
            .map_or(-1, |r| r.node_color)
    }

    /// The colour each side of a conflict is drawn in — the graph's answer
    /// where it has one, a stable one off the name where it does not, and
    /// never the same on both sides
    /// (`encode::conflict_side_colors` decides; these two only pick a half
    /// out of its answer, since a slot cannot hand back a pair).
    #[qslot]
    fn conflict_color_ours(&self, ours: String, theirs: String) -> i32 {
        self.conflict_colors(&ours, &theirs).0
    }

    #[qslot]
    fn conflict_color_theirs(&self, ours: String, theirs: String) -> i32 {
        self.conflict_colors(&ours, &theirs).1
    }

    /// Re-reads the assigned pictures onto the rows already loaded.
    ///
    /// Assigning one is not a thing git knows about, so nothing about the
    /// repository changed and re-walking the history to find that out
    /// would be the most expensive way to move a handful of pixels. The
    /// splice only notifies the rows whose author actually got one.
    #[qslot]
    fn refresh_avatars(&mut self) {
        let avatars = crate::hub::AvatarUrls::current();
        let rows: Vec<GraphRowItem> = self
            .rows
            .iter()
            .map(|row| GraphRowItem {
                avatar_url: avatars.url_of(&row.author_email),
                ..row.clone()
            })
            .collect();
        self.splice_notified(rows);
    }

    /// How many loaded rows carry a picture. Automation only: QML cannot
    /// walk this model's rows, so counting them there would be counting
    /// nothing (measured — a helper doing exactly that reported zero
    /// while the faces were on screen).
    #[qslot]
    fn avatar_row_count(&self) -> i32 {
        self.rows
            .iter()
            .filter(|row| !row.avatar_url.is_empty())
            .count() as i32
    }

    /// The authors of the loaded rows, one per address, packed as
    /// `name\u{1f}email` and joined by `\u{1e}`.
    ///
    /// What the settings card offers instead of asking somebody to type an
    /// address: the people whose commits are on screen are the people
    /// whose faces are worth setting. Read when the card opens, off rows
    /// already in memory — no git runs for it.
    #[qslot]
    fn author_choices(&self) -> String {
        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
        let mut out: Vec<String> = Vec::new();
        for row in &self.rows {
            if row.author_email.is_empty() || !seen.insert(&row.author_email) {
                continue;
            }
            out.push(format!("{}\u{1f}{}", row.author, row.author_email));
        }
        out.sort();
        out.join("\u{1e}")
    }

    /// Puts the find bar's line to the rows, lighting the ones it is in.
    ///
    /// An empty line — or one that is only whitespace — is not a search
    /// (`find::Query::new`): the marks come off and `matchCount` goes to
    /// zero, which is what the bar reads as "nothing is being looked
    /// for".
    #[qslot]
    fn set_find(&mut self, text: String) {
        let next = Query::new(&text);
        if next == self.query {
            return;
        }
        self.query = next;
        self.searching = self.query.is_some();
        self.remark_notified();
        self.stats_changed();
    }

    /// Row of the first match at or after `from`, wrapping to the first
    /// match of all when there is none below; -1 when nothing matches.
    /// Where an incremental search lands.
    #[qslot]
    fn match_from(&self, from: i32) -> i32 {
        self.match_rows()
            .find(|row| *row >= from)
            .or_else(|| self.match_rows().next())
            .unwrap_or(-1)
    }

    /// Row of the next match after `row`, wrapping past the end.
    #[qslot]
    fn match_after(&self, row: i32) -> i32 {
        self.match_rows()
            .find(|r| *r > row)
            .or_else(|| self.match_rows().next())
            .unwrap_or(-1)
    }

    /// Row of the previous match before `row`, wrapping past the start.
    #[qslot]
    fn match_before(&self, row: i32) -> i32 {
        self.match_rows()
            .rfind(|r| *r < row)
            .or_else(|| self.match_rows().next_back())
            .unwrap_or(-1)
    }

    /// Which match this row is, counting from 1; 0 when it is not one.
    /// The left half of the bar's count.
    #[qslot]
    fn match_ordinal(&self, row: i32) -> i32 {
        self.match_rows()
            .position(|r| r == row)
            .map_or(0, |i| i as i32 + 1)
    }

    /// Full commit id at a row (selection, its recovery after a rewrite,
    /// and the smoke hooks).
    #[qslot]
    fn oid_at(&self, row: i32) -> String {
        usize::try_from(row)
            .ok()
            .and_then(|i| self.rows.get(i))
            .map(|r| r.oid_hex.clone())
            .unwrap_or_default()
    }
}
impl GraphModel {
    /// Shared by the two slots above, so the pair is decided once.
    fn conflict_colors(&self, ours: &str, theirs: &str) -> (i32, i32) {
        crate::encode::conflict_side_colors(
            (self.color_of_ref(ours.to_string()), ours),
            (self.color_of_ref(theirs.to_string()), theirs),
        )
    }
}

qml_register!(GraphModel, "GraphModel", singleton = false);

fn to_row_item(row: &LogRow, avatars: &crate::hub::AvatarUrls) -> GraphRowItem {
    GraphRowItem {
        oid_hex: row.oid_hex.clone(),
        author: row.author.clone(),
        author_email: row.author_email.clone(),
        atime: row.time,
        subject: row.subject.clone(),
        node_lane: i32::from(row.node_lane),
        node_color: i32::from(row.node_color),
        avatar: crate::encode::avatar_code(&row.author),
        avatar_url: avatars.url_of(&row.author_email),
        co_authors: crate::encode::encode_co_authors(&row.co_authors),
        body: row.body.clone(),
        geometry: encode_geometry(&row.segments),
        labels: encode_labels(&row.labels),
        stash_ref: row.stash_ref.clone(),
        // Set by the marking pass that runs before anyone sees the row
        // (`mark_incoming`), so a chunk arriving under a standing query
        // arrives already lit.
        matched: false,
    }
}
