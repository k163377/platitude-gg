//! The walk's answers as they arrive: one function per kind of message
//! the feed carries, what each does to the rows already held, and what
//! the footer under them is left saying.
//!
//! Every arm reads `generation` first and drops what an older stream is
//! still saying — a replacement supersedes anything before it, and a
//! chunk of a graph this model no longer shows is rows of other commits.

use super::*;

impl GraphModel {
    pub(super) fn take_feed(&mut self) {
        let Some(feed) = self.feed.clone() else {
            return;
        };
        // Whether anything the bindings read moved. A report of HEAD that
        // found it where it was — the first read after every write sends
        // one — moves nothing here, and waking every binding on the
        // footer for it is a sweep per write for no change.
        let mut changed = false;
        for msg in feed.drain() {
            match msg {
                // Read as sent, with no generation to check: where HEAD
                // stands is the repository's, not one stream's, and a
                // stream starting over does not move it.
                GraphMsg::Head(head) => {
                    let head = Oid::from_hex_str(&head.oid_hex).ok();
                    changed |= self.head_oid != head;
                    self.head_oid = head;
                    continue;
                }
                GraphMsg::Started { generation } => self.start_walk(generation),
                GraphMsg::Chunk { generation, rows } => self.take_chunk(generation, &rows),
                GraphMsg::Labels { generation, rows } => self.take_labels(generation, rows),
                GraphMsg::Finished {
                    generation,
                    total,
                    elapsed_ms,
                    walked,
                    truncated,
                } => self.finish_walk(generation, total, elapsed_ms, walked, truncated),
                GraphMsg::Replaced {
                    generation,
                    rows,
                    elapsed_ms,
                    walked,
                    truncated,
                } => self.replace_walk(generation, &rows, elapsed_ms, walked, truncated),
                GraphMsg::Failed {
                    generation,
                    message,
                } => self.fail_walk(generation, message),
                // Read as sent, with no generation to check: the session
                // says this of whatever graph is standing, and every
                // stream that starts, lands or finds nothing to change
                // takes it back from the same place (`log::tell_graph_stale`).
                GraphMsg::Stale { stale } => self.stale = stale,
            }
            changed = true;
        }
        changed |= self.settle_head();
        if crate::harness::memprobe::enabled() {
            crate::harness::memprobe::note("graph-rows", self.tab_id, &self.rows);
            crate::harness::memprobe::note_bytes(
                "graph-index",
                self.tab_id,
                self.index.capacity() * size_of::<(Oid, u32)>(),
                self.index.len(),
            );
        }
        if changed {
            self.stats_changed();
        }
    }

    fn start_walk(&mut self, generation: u64) {
        if generation > self.generation {
            self.generation = generation;
            self.reset();
            self.reset_count += 1;
            self.loading = true;
            self.row_total = 0;
            self.walked_total = 0;
            // The column is empty, so it holds no working-tree row either:
            // a walk that fails before its first chunk leaves this
            // standing, and the last pass's answer there would say the
            // graph agrees with a status it was never walked from.
            self.wip_row = false;
            // The query stands — a restart is the same history read
            // again — but its answers went with the rows, and the chunks
            // re-count them.
            self.match_count = 0;
            self.first_matched = false;
            self.max_lanes = 1;
            self.first_chunk_ms = -1;
            self.total_ms = -1;
            self.truncated = false;
            // A stream starting over is a different graph from the one
            // the press was made in, and it will settle a footer of its
            // own.
            self.growing = false;
            self.error = String::new();
            self.failed = false;
            self.started_at = Some(Instant::now());
        }
    }

    fn take_chunk(&mut self, generation: u64, rows: &[LogRow]) {
        if generation != self.generation {
            return;
        }
        if self.first_chunk_ms < 0
            && let Some(t0) = self.started_at
        {
            self.first_chunk_ms = t0.elapsed().as_millis() as i32;
            tracing::info!(first_chunk_ms = self.first_chunk_ms, "graph first chunk");
        }
        let avatars = crate::hub::AvatarUrls::current();
        let pr = crate::encode::pr_set();
        let mut items: Vec<GraphRowItem> = rows
            .iter()
            .map(|row| to_row_item(row, &avatars, pr))
            .collect();
        for row in rows {
            self.max_lanes = self.max_lanes.max(i32::from(row.width));
        }
        self.mark_incoming(&mut items);
        self.match_count += items.iter().filter(|i| i.matched).count() as i32;
        self.extend_marks(rows);
        self.extend_notified(items);
        self.settle_first();
        self.row_total = self.rows.len() as i32;
        debug_assert_eq!(self.marks.len(), self.rows.len());
    }

    fn take_labels(
        &mut self,
        generation: u64,
        rows: Vec<(u32, Vec<platitude_core::session::RefLabel>)>,
    ) {
        if generation != self.generation {
            // Row numbers of a graph this model no longer shows: the
            // chips belong to other commits here.
            return;
        }
        let pr = crate::encode::pr_set();
        for (row, labels) in rows {
            let idx = row as usize;
            if let Some(existing) = self.rows.get(idx) {
                let mut updated = existing.clone();
                updated.labels = encode_labels(&labels, pr);
                // The names on the row are searched, so the second pass
                // that puts the chips on can turn a row's light on or
                // off.
                if let Some(query) = &self.query {
                    updated.matched = Self::hits(query, &updated);
                    self.match_count += i32::from(updated.matched) - i32::from(existing.matched);
                }
                self.set(idx, updated);
            }
        }
        self.settle_first();
    }

    fn finish_walk(
        &mut self,
        generation: u64,
        total: u32,
        elapsed_ms: u64,
        walked: u32,
        truncated: bool,
    ) {
        if generation == self.generation {
            self.settle_footer(total as i32, elapsed_ms, walked, truncated);
            self.rows.shrink_to_fit();
            self.marks.shrink_to_fit();
            self.parent_oids.shrink_to_fit();
            self.index.shrink_to_fit();
            // A pass landed: the stand-in follows HEAD from here, drawn
            // or not (`settle_head`).
            self.pinned_oid = None;
            tracing::info!(total, elapsed_ms, truncated, "graph stream finished");
        }
    }

    fn replace_walk(
        &mut self,
        generation: u64,
        rows: &[LogRow],
        elapsed_ms: u64,
        walked: u32,
        truncated: bool,
    ) {
        if generation <= self.generation {
            return; // superseded by a newer stream
        }
        self.generation = generation;
        let avatars = crate::hub::AvatarUrls::current();
        let pr = crate::encode::pr_set();
        let mut items: Vec<GraphRowItem> = rows
            .iter()
            .map(|row| to_row_item(row, &avatars, pr))
            .collect();
        // Before the splice, so a rebuild under a standing query notifies
        // each row once — with its light already right — instead of
        // twice.
        self.mark_incoming(&mut items);
        self.match_count = items.iter().filter(|i| i.matched).count() as i32;
        self.first_matched = items.first().is_some_and(|i| i.matched);
        self.max_lanes = rows
            .iter()
            .map(|row| i32::from(row.width))
            .max()
            .unwrap_or(1)
            .max(1);
        // The whole graph, so the marks are the whole graph's too —
        // written before the splice churns `rows` into the same shape.
        self.replace_marks(rows);
        // A pass landed: the stand-in follows HEAD from here, drawn or
        // not (`settle_head`).
        self.pinned_oid = None;
        self.splice_notified(items);
        debug_assert_eq!(self.marks.len(), self.rows.len());
        let loaded = self.rows.len() as i32;
        self.settle_footer(loaded, elapsed_ms, walked, truncated);
        // Only a replacement zeroes these: it is one message rather than
        // a stream, so there was no first chunk to time, and it is the
        // answer to whatever failed last.
        self.first_chunk_ms = 0;
        self.error = String::new();
        self.failed = false;
        tracing::info!(
            total = self.row_total,
            elapsed_ms,
            truncated,
            "graph replaced in place"
        );
    }

    /// A stream said it could not draw this graph, so the rows that did
    /// arrive are not all of them. `message` is whoever's words there are
    /// — git's, where git is what failed — and empty for the one that has
    /// none: a walk that ended without an answer at all
    /// (`session::pass_watch::PassWatch`). The screen tells those two
    /// apart by `failed` standing without an `error` beside it.
    fn fail_walk(&mut self, generation: u64, message: String) {
        if generation == self.generation {
            self.loading = false;
            self.growing = false;
            self.error = message;
            self.failed = true;
        }
    }

    /// Replaces the whole list in place: unchanged rows stay untouched,
    /// contiguous runs of changed rows emit one ranged dataChanged, and
    /// only the length delta inserts or removes rows. No model reset —
    /// the view keeps its scroll position and never shows an empty list.
    #[expect(unsafe_code)]
    pub(super) fn splice_notified(&mut self, new_rows: Vec<GraphRowItem>) {
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
                push_run(&mut ranges, i);
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

        self.notify_runs(ranges);

        if !extra.is_empty() {
            self.extend_notified(extra);
        }
    }

    /// What the footer under the graph says once a walk has stopped: the
    /// counts, the time it took, and — only where the walk was cut short —
    /// the lanes still running off the bottom of the last row.
    ///
    /// Whether this stream is the one being listened to stays with the
    /// caller: a chunked walk answers only for its own generation, and a
    /// replacement supersedes anything older. So does what only a
    /// replacement resets.
    pub(super) fn settle_footer(
        &mut self,
        row_total: i32,
        elapsed_ms: u64,
        walked: u32,
        truncated: bool,
    ) {
        self.loading = false;
        // Whatever this walk was, it is the answer to any press that was
        // out: the wider one it asked for landing, or the pass that
        // overtook it having walked the window it widened.
        self.growing = false;
        self.total_ms = elapsed_ms as i32;
        self.row_total = row_total;
        self.walked_total = walked as i32;
        self.truncated = truncated;
        self.finish_count += 1;
        // What this pass walked with, read off the rows it left rather
        // than held from the ask: a pass is cancelled and replaced by the
        // one that overtook it, and only the rows say which of them is on
        // screen.
        self.wip_row = self
            .rows
            .first()
            .is_some_and(|row| platitude_core::oid::Oid::hex_is_zero(&row.oid_hex));
        // Only the cut needs the step, and only a settled walk knows
        // there was one — asked for here rather than held from the
        // opening, so a window the settings widen moves the step with it.
        //
        // **Written only where the session answered.** There being no
        // session is not an answer of zero: this drains from a queued
        // call, so a walk that settled just before its tab closed lands
        // here with nothing to ask, and a 0 written then is a footer
        // offering `Load 0 more commits` for a press that would load the
        // step the session still has.
        if truncated
            && let Some(step) = crate::hub::from_session(self.tab_id, |s| s.log_window_step())
        {
            self.window_step = i32::try_from(step).unwrap_or(i32::MAX);
        }
        self.tail_geometry = if truncated {
            self.rows
                .last()
                .map(|r| crate::encode::tail_lanes(&r.geometry))
                .unwrap_or_default()
        } else {
            String::new()
        };
    }
}
