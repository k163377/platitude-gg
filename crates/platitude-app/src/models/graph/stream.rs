//! The walk's answers as they arrive — one function per kind of message
//! the feed carries — and what the footer is left saying.
//!
//! Every arm but `Head` and `Stale` reads `generation` first and drops
//! what an older stream is still saying: a chunk of a graph this model no
//! longer shows is rows of other commits.

use super::*;

impl GraphModel {
    pub(super) fn take_feed(&mut self) {
        let Some(feed) = self.feed.clone() else {
            return;
        };
        // Whether anything the bindings read moved: an unchanged HEAD
        // report (one follows every write) must not wake them.
        let mut changed = false;
        for msg in feed.drain() {
            match msg {
                // No generation check: HEAD is the repository's, not a
                // stream's.
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
                GraphMsg::Relaid {
                    generation,
                    from,
                    rows,
                    walked,
                    truncated,
                } => self.relay_walk(generation, from, &rows, walked, truncated),
                GraphMsg::Failed {
                    generation,
                    message,
                } => self.fail_walk(generation, message),
                // No generation check: the session says this of whatever
                // graph is standing (`log::tell_graph_stale`).
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
            // The walk's marks (`marks.rs`), in two lines: the spans grow
            // with the rows and the parent ids with the parenthood, and a
            // merge-heavy history moves only the second.
            crate::harness::memprobe::note_bytes(
                "graph-marks",
                self.tab_id,
                self.marks.capacity() * size_of::<RowMark>(),
                self.marks.len(),
            );
            crate::harness::memprobe::note_bytes(
                "graph-parents",
                self.tab_id,
                self.parent_oids.capacity() * size_of::<Oid>(),
                self.parent_oids.len(),
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
            self.set_aside.clear();
            self.reset_count += 1;
            self.loading = true;
            self.row_total = 0;
            self.walked_total = 0;
            // A walk that fails before its first chunk leaves these
            // standing; the last pass's answer would claim agreement with
            // a status never walked from.
            self.wip_row = false;
            self.carried_top = false;
            // The query stands; its answers went with the rows, and the
            // chunks re-count them.
            self.match_count = 0;
            self.first_matched = false;
            self.tail_matched = false;
            self.max_lanes = 1;
            self.first_chunk_ms = -1;
            self.total_ms = -1;
            self.truncated = false;
            // A restart is not the graph the press was made in; it
            // settles a footer of its own.
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
        self.settle_ends();
        self.row_total = self.rows.len() as i32;
        debug_assert_eq!(self.marks.len(), self.rows.len());
    }

    fn take_labels(
        &mut self,
        generation: u64,
        rows: Vec<(u32, Vec<platitude_core::session::RefLabel>)>,
    ) {
        if generation != self.generation {
            return;
        }
        let pr = crate::encode::pr_set();
        for (row, labels) in rows {
            let idx = row as usize;
            if let Some(existing) = self.rows.get(idx) {
                let mut updated = existing.clone();
                updated.labels = crate::encode::chips_of(&labels, pr);
                // Chip names are searched, so the chips can change a
                // row's match.
                if let Some(query) = &self.query {
                    updated.matched = Self::hits(query, &updated);
                    self.match_count += i32::from(updated.matched) - i32::from(existing.matched);
                }
                self.set(idx, updated);
            }
        }
        self.settle_ends();
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

    pub(super) fn replace_walk(
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
        // Before the splice, so each row is notified once, already
        // marked.
        self.mark_incoming(&mut items);
        self.match_count = items.iter().filter(|i| i.matched).count() as i32;
        self.first_matched = items.first().is_some_and(|i| i.matched);
        self.tail_matched = items.last().is_some_and(|i| i.matched);
        self.max_lanes = rows
            .iter()
            .map(|row| i32::from(row.width))
            .max()
            .unwrap_or(1)
            .max(1);
        // Written before the splice churns `rows` into the same shape.
        self.replace_marks(rows);
        self.drawn_again_from_set_aside();
        // A pass landed, as in `finish_walk`.
        self.pinned_oid = None;
        self.splice_notified(items);
        debug_assert_eq!(self.marks.len(), self.rows.len());
        let loaded = self.rows.len() as i32;
        self.settle_footer(loaded, elapsed_ms, walked, truncated);
        // Only a replacement zeroes these: one message has no first chunk
        // to time, and it answers whatever failed last.
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

    /// A stream could not draw this graph, so the rows that arrived are
    /// not all of them. `message` is git's words, or empty for a walk that
    /// ended with no answer at all (`session::pass_watch::PassWatch`) —
    /// `failed` without `error`.
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
    /// only the length delta inserts or removes rows. The view keeps
    /// its scroll position and a list with rows in it throughout.
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
                // SAFETY: as in `models::notify` — the registry's pointer
                // for this value's attached QObject, which outlives the
                // slot (main thread), and only ever shared.
                unsafe { &*proxy }.base_begin_remove_rows(
                    &mut *self,
                    &qtbridge::qtbridge_type_lib::QModelIndex::default(),
                    new_len as i32,
                    old_len as i32 - 1,
                );
                self.rows.truncate(new_len);
                // SAFETY: see above.
                unsafe { &*proxy }.base_end_remove_rows(&mut *self);
            } else {
                self.rows.truncate(new_len);
            }
        }

        self.notify_runs(ranges);

        if !extra.is_empty() {
            self.extend_notified(extra);
        }
    }

    /// What the footer says once a walk has stopped: the counts, the time,
    /// and — only where the walk was cut short — the lanes running off the
    /// last row. The generation check stays with the caller, and so does
    /// what only a replacement resets.
    pub(super) fn settle_footer(
        &mut self,
        row_total: i32,
        elapsed_ms: u64,
        walked: u32,
        truncated: bool,
    ) {
        self.loading = false;
        // Any settled walk answers a press that was out: the one it asked
        // for, or the pass that overtook it on the widened window.
        self.growing = false;
        self.total_ms = elapsed_ms as i32;
        self.row_total = row_total;
        self.walked_total = walked as i32;
        self.truncated = truncated;
        self.finish_count += 1;
        // Read off the rows: only they say which of the overtaking passes
        // is on screen. Not by the zero id alone — a copy's row carries it
        // too (`GraphModel::carried_row_of`), and a pass that put one first
        // would read as up to date while one read behind.
        self.carried_top = self.carried.contains_key(&0);
        self.wip_row = !self.carried_top
            && self
                .rows
                .first()
                .is_some_and(|row| platitude_core::oid::Oid::hex_is_zero(&row.oid_hex));
        // Asked here, so a window the settings widen moves the step.
        // Written only where the session answered: a walk that settled
        // just before its tab closed drains with no session, and a 0 would
        // offer `Load 0 more commits`.
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
            crate::encode::Lanes::default()
        };
    }
}

#[cfg(test)]
mod tests {
    use super::super::item::GraphRowItem;
    use super::super::{CarriedRow, GraphModel};

    /// A row wearing git's all-zero id, as every working copy's
    /// uncommitted row does.
    fn zero_row() -> GraphRowItem {
        GraphRowItem {
            oid_hex: "0".repeat(40),
            ..GraphRowItem::default()
        }
    }

    fn commit_row(oid_hex: &str) -> GraphRowItem {
        GraphRowItem {
            oid_hex: oid_hex.into(),
            ..GraphRowItem::default()
        }
    }

    /// Settles one pass over `rows`; `copies` are the indexes
    /// `extend_marks` would file as a neighbour's.
    fn pass(model: &mut GraphModel, rows: Vec<GraphRowItem>, copies: &[usize]) {
        let total = rows.len();
        model.clear_marks();
        model.rows = rows;
        for at in copies {
            model.carried.insert(*at, CarriedRow::default());
        }
        model.settle_footer(
            i32::try_from(total).unwrap_or_default(),
            0,
            u32::try_from(total).unwrap_or_default(),
            false,
        );
    }

    /// A landing on this window's uncommitted work waits for the pass that
    /// carries its row. A pass that beat the first status carries the
    /// neighbours' zero-id rows and not ours; read by the id alone, it
    /// would let the landings (`RepoPage.trySelectDefault`,
    /// `RepoPage.tryPendingWipSelect`) take a copy's row.
    #[test]
    fn a_pass_that_beat_the_status_carries_no_row_this_window_can_land_on() {
        let mut model = GraphModel::default();

        // The walk, ahead of this window's first status: the copies have
        // rows and we have none.
        pass(
            &mut model,
            vec![zero_row(), zero_row(), commit_row("a1")],
            &[0, 1],
        );
        assert!(!model.wip_row, "a neighbour copy's row stood in for ours");
        assert!(model.carried_top, "whose the leading row was went unsaid");

        // The status lands and asks for the walk again. Ours is prepended,
        // and the copies' rows are still there under it.
        pass(
            &mut model,
            vec![zero_row(), zero_row(), zero_row(), commit_row("a1")],
            &[1, 2],
        );
        assert!(model.wip_row, "the pass that carries our row was refused");
        assert!(!model.carried_top, "ours was filed as a neighbour's");

        // And a tree gone clean takes ours away while theirs stand.
        pass(&mut model, vec![zero_row(), commit_row("a1")], &[0]);
        assert!(!model.wip_row, "ours was reported over a clean tree");
    }
}
