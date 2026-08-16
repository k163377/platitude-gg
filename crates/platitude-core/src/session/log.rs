//! The streaming log -> graph pipeline: options, the direct and swap
//! passes, background refreshes, and chunked row/label delivery.

use super::build::{LogTotals, StreamItem, make_row, sift_batch, wip_row};
use super::*;

impl RepoSession {
    pub fn log_options(&self) -> LogOptions {
        *self.lock_log_options()
    }

    /// Toggles tags in the graph walk and restarts the stream.
    pub fn set_include_tags(self: &Arc<Self>, include_tags: bool) {
        {
            let mut options = self.lock_log_options();
            if options.include_tags == include_tags {
                return;
            }
            options.include_tags = include_tags;
        }
        self.restart_log();
    }

    /// Changes the graph window size (`None` = full history) and restarts.
    pub fn set_log_limit(self: &Arc<Self>, limit: Option<u32>) {
        {
            let mut options = self.lock_log_options();
            if options.limit == limit {
                return;
            }
            options.limit = limit;
        }
        self.restart_log();
    }

    fn lock_log_options(&self) -> std::sync::MutexGuard<'_, LogOptions> {
        match self.log_options.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        }
    }

    /// Restarts the log → graph stream (used by manual full refresh).
    ///
    /// With tags enabled this runs **two passes**: a fast tag-less pass
    /// paints immediately (tag tips make the walk's frontier setup cost
    /// seconds on tag-heavy repositories), then a tag-inclusive pass
    /// rebuilds in the background and atomically replaces the graph.
    pub fn restart_log(self: &Arc<Self>) {
        let Some(workdir) = self.workdir() else {
            return;
        };

        // Swapped here, on the caller's thread, not inside the spawned
        // task: call order is what decides which pass owns the graph, and
        // spawn order does not follow it (core.md).
        let run_cancel = self.root_cancel.child_token();
        if let Some(prev) = self
            .log_cancel
            .lock()
            .map(|mut g| g.replace(run_cancel.clone()))
            .unwrap_or_default()
        {
            prev.cancel();
        }

        let s = Arc::clone(self);
        let options = self.log_options();
        self.runtime.spawn(async move {
            if options.include_tags {
                let fast = LogOptions {
                    include_tags: false,
                    ..options
                };
                if s.run_direct_pass(&workdir, fast, &run_cancel).await.is_ok() {
                    s.run_swap_pass(&workdir, options, &run_cancel).await;
                }
            } else {
                let _completed = s.run_direct_pass(&workdir, options, &run_cancel).await;
            }
        });
    }

    /// Rebuilds the graph off-screen and swaps it in only when it differs
    /// from what the UI already shows (see [`RepoSession::run_swap_pass`]).
    ///
    /// Background triggers (auto fetch, a finished write, an external
    /// dirty/clean flip) go through here instead of [`RepoSession::restart_log`]:
    /// a reset-and-restream repaints the pane even when history did not
    /// move, which reads as idle flicker once a periodic fetch is on.
    pub fn refresh_log(self: &Arc<Self>) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let run_cancel = self.root_cancel.child_token();
        if let Some(prev) = self
            .log_cancel
            .lock()
            .map(|mut g| g.replace(run_cancel.clone()))
            .unwrap_or_default()
        {
            prev.cancel();
        }
        let s = Arc::clone(self);
        let options = self.log_options();
        self.runtime.spawn(async move {
            s.run_swap_pass(&workdir, options, &run_cancel).await;
        });
    }

    /// Streams one pass straight to the UI (chunked, resets the graph).
    /// Returns Err after reporting when the pass failed or was cancelled.
    async fn run_direct_pass(
        self: &Arc<Self>,
        workdir: &std::path::Path,
        options: LogOptions,
        cancel: &CancellationToken,
    ) -> Result<(), ()> {
        let generation = self.log_gen.fetch_add(1, Ordering::SeqCst) + 1;
        {
            // Reset graph state for the new stream and announce it under
            // one lock: everything that reads row numbers out of `shared`
            // takes the same lock and sends what it read before letting
            // go, so no message can describe a graph the consumer is not
            // on yet (see `apply_refs`).
            let mut shared = self.lock_shared();
            // Whoever asked last owns the graph, and asking is what
            // cancelled this token (both entry points swap it before
            // spawning). Resetting for a stream nobody wants any more
            // would blank the record of what is on screen and leave
            // every later chip diff numbered for a graph that was never
            // shown; the walk is cancelled and would deliver no rows to
            // put back.
            if cancel.is_cancelled() {
                return Err(());
            }
            shared.builder = GraphBuilder::new();
            shared.generation = generation;
            shared.applied.clear();
            shared.sent_rows.clear();
            // Nothing has answered for this graph yet — not even this
            // pass, which only learns its footer when the walk ends. A
            // rebuild landing in between must not read the last graph's
            // answer as this one's.
            shared.sent_footer = None;
            self.sink.event(SessionEvent::LogStarted { generation });
        }
        let started = Instant::now();
        match self.stream_log(workdir, generation, options, cancel).await {
            Ok(totals) => {
                let footer = Footer {
                    walked: totals.walked,
                    // Truncation is a property of the walk: the shown count
                    // drifts from it in both directions (the WIP row adds
                    // one, sifted stash parents subtract), so comparing it
                    // against --max-count would flag the wrong streams.
                    truncated: options.limit.is_some_and(|n| totals.walked >= n),
                };
                // Recorded and sent under one lock, like every other
                // message describing what is in `shared`: a rebuild taking
                // the lock next compares against this footer, and must not
                // find it before the consumer has been told.
                let mut shared = self.lock_shared();
                // Only the stream the consumer is on may answer for it.
                // A superseded one would leave its numbers behind as the
                // record of somebody else's graph (see `emit_rows`).
                if shared.generation == generation {
                    shared.sent_footer = Some(footer);
                }
                self.sink.event(SessionEvent::LogFinished {
                    generation,
                    total: totals.shown,
                    elapsed_ms: started.elapsed().as_millis() as u64,
                    walked: footer.walked,
                    truncated: footer.truncated,
                });
                Ok(())
            }
            Err(error) => {
                if !matches!(error, GitError::Cancelled { .. }) {
                    self.sink.event(SessionEvent::LogFailed {
                        generation,
                        error: error.to_string(),
                    });
                }
                Err(())
            }
        }
    }

    /// Builds a full pass off-screen, then swaps it in as one reset +
    /// one chunk (the UI drains all three events in a single slot call,
    /// so the replacement is flicker-free).
    async fn run_swap_pass(
        self: &Arc<Self>,
        workdir: &std::path::Path,
        options: LogOptions,
        cancel: &CancellationToken,
    ) {
        let generation = self.log_gen.fetch_add(1, Ordering::SeqCst) + 1;
        let started = Instant::now();
        let mut builder = GraphBuilder::new();
        let mut rows: Vec<LogRow> = Vec::new();

        let result = self
            .collect_log(workdir, options, cancel, &mut builder, &mut rows)
            .await;
        let walked = match result {
            Ok(walked) => walked,
            Err(error) => {
                // The fast pass is already on screen; report quietly.
                if !matches!(error, GitError::Cancelled { .. }) {
                    self.fail("log", error);
                }
                return;
            }
        };

        let total = rows.len() as u32;
        {
            let mut shared = self.lock_shared();
            // Superseded: someone asked for a graph after this pass was
            // started, and that ask cancelled this token. Read here
            // rather than the generation counter — that one is stamped
            // when a pass begins running, which is not the order the
            // asks came in.
            if cancel.is_cancelled() {
                return;
            }
            let elapsed_ms = started.elapsed().as_millis() as u64;
            let mut applied: HashMap<u32, Vec<RefLabel>> = HashMap::new();
            for row in &mut rows {
                if let Ok(oid) = Oid::from_hex_str(&row.oid_hex) {
                    let labels = shared.label_map.labels_of(&oid);
                    if !labels.is_empty() {
                        row.labels = labels.to_vec();
                        applied.insert(row.row, labels.to_vec());
                    }
                }
            }
            let footer = Footer {
                walked,
                // See run_direct_pass: the walk decides truncation, not
                // the shown row count.
                truncated: options.limit.is_some_and(|n| walked >= n),
            };
            // Both halves of what the last pass delivered. Rows alone
            // would call a widened window "the same picture" and leave the
            // truncation notice claiming history the user just asked to
            // see — a rebuild is the only thing that speaks when one
            // overtakes the stream the change asked for (core.md).
            let unchanged = shared.sent_footer == Some(footer)
                && shared.sent_rows.len() == rows.len()
                && shared
                    .sent_rows
                    .iter()
                    .zip(&rows)
                    .all(|(sent, fresh)| *sent == RowPrint::of(fresh));
            shared.builder = builder;
            shared.applied = applied;
            if unchanged {
                // The UI already shows exactly this: swapping would only
                // reset the view (scroll anchor, selection re-resolve) for
                // an identical picture. Background refreshes land here on
                // every quiet auto-fetch tick — same options over an
                // unmoved repository walk the same commits, so the footer
                // matches whenever the rows do.
                //
                // The generation stays behind with it. Nothing was sent,
                // so the graph on screen is still the one before this
                // pass — and this walk numbered its rows the same way, or
                // it would not have compared equal.
                tracing::debug!(generation, total, "graph rebuild unchanged; swap skipped");
                return;
            }
            shared.sent_rows = rows.iter().map(RowPrint::of).collect();
            shared.sent_footer = Some(footer);
            shared.generation = generation;
            // Still under the lock (see run_direct_pass): a refs read that
            // takes it next diffs chips against this graph, and its event
            // must not overtake the rows it numbers.
            self.sink.event(SessionEvent::LogReplaced {
                generation,
                rows,
                elapsed_ms,
                walked: footer.walked,
                truncated: footer.truncated,
            });
        }
    }

    /// Refreshes refs, status(+op state), stashes and worktrees
    /// concurrently. Cheap enough for window-focus and post-operation
    /// triggers.
    pub fn refresh_quick(self: &Arc<Self>) {
        self.refresh_refs();
        self.refresh_status();
        self.refresh_stashes();
        self.refresh_worktrees();
    }

    /// The periodic re-read that runs while the repository is on screen:
    /// refs and status only. Stashes and worktrees ride the focus and
    /// post-write refreshes instead — two more processes every tick to
    /// catch what a poll practically never sees move on its own.
    ///
    /// Skipped while a write runs (that repository is mid-operation, and
    /// the write refreshes when it lands) and while the previous poll is
    /// still going, so a slow repository polls less often instead of
    /// stacking reads up.
    pub fn refresh_poll(self: &Arc<Self>) {
        if self.write_busy.load(Ordering::SeqCst) {
            tracing::trace!("poll skipped: a write is running");
            return;
        }
        let Ok(permit) = Arc::clone(&self.poll_slot).try_acquire_owned() else {
            tracing::trace!("poll skipped: the previous one has not finished");
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let _permit = permit;
            // Both reads can call for a rebuild, but the graph is one
            // picture: an external commit moves a ref *and* cleans the
            // tree, and walking twice would throw one pass away.
            let (refs_moved, wip_flipped) = tokio::join!(s.publish_refs(), s.publish_status());
            if refs_moved || wip_flipped {
                s.refresh_log();
            }
        });
    }

    async fn stream_log(
        self: &Arc<Self>,
        workdir: &std::path::Path,
        generation: u64,
        options: LogOptions,
        cancel: &CancellationToken,
    ) -> Result<LogTotals, GitError> {
        // An unborn HEAD has nothing to log. The refs read already
        // answered this, and asking git again is two processes in front
        // of the first chunk on every rebuild (`known_head_tip`).
        let head_tip = match self.known_head_tip() {
            Some(tip) => tip,
            None => refs::head_state(&self.executor, workdir, cancel).await?.oid,
        };
        if head_tip.is_none() {
            return Ok(LogTotals::default());
        }

        // Stashes are part of the graph: their oids join the walk and the
        // synthetic index/untracked parents are sifted out below.
        let stash_refs: HashMap<Oid, String> = stash::load(&self.executor, workdir, cancel)
            .await
            .map(|list| list.into_iter().map(|s| (s.oid, s.name)).collect())
            .unwrap_or_default();

        let mut cmd = GitCommand::new()
            .cwd(workdir)
            .args(["log", "-z", "--date-order", LOG_FORMAT_ARG])
            .args(["HEAD", "--branches", "--remotes"])
            .no_timeout();
        if options.include_tags {
            cmd = cmd.arg("--tags");
        }
        if let Some(limit) = options.limit {
            cmd = cmd.arg(format!("--max-count={limit}"));
        }
        if !stash_refs.is_empty() {
            // A stash may vanish between the listing and the walk.
            cmd = cmd.arg("--ignore-missing");
            for oid in stash_refs.keys() {
                cmd = cmd.arg(oid.to_hex());
            }
        }

        let mut parser = LogParser::new();
        let mut pending: Vec<CommitMeta> = Vec::new();
        let mut stash_skip: std::collections::HashSet<Oid> = std::collections::HashSet::new();
        let mut first_sent = false;
        let mut parse_error: Option<String> = None;
        let mut totals = LogTotals::default();

        // Dirty working tree: prepend the synthetic WIP row so the current
        // chain owns lane 0 from the very first paint.
        if self.wip_dirty.load(Ordering::SeqCst)
            && let Some(head_oid) = head_tip
        {
            self.emit_wip_row(generation, &head_oid);
            totals.shown += 1;
        }

        let result = self
            .executor
            .run_streaming(cmd, cancel, &mut |bytes| {
                if parse_error.is_some() {
                    return;
                }
                if let Err(e) = parser.feed(bytes, &mut pending) {
                    parse_error = Some(e.to_string());
                    cancel.cancel();
                    return;
                }
                let threshold = if first_sent {
                    CHUNK_ROWS
                } else {
                    FIRST_CHUNK_ROWS
                };
                if pending.len() >= threshold {
                    let batch = std::mem::take(&mut pending);
                    totals.walked += batch.len() as u32;
                    let mut items = Vec::with_capacity(batch.len());
                    sift_batch(batch, &stash_refs, &mut stash_skip, &mut items);
                    totals.shown += items.len() as u32;
                    first_sent = true;
                    self.emit_rows(generation, &items, parser.pool());
                }
            })
            .await;

        match result {
            Ok(_) => {}
            Err(e) => {
                // A self-inflicted cancel means the parser hit a fatal error.
                if let Some(msg) = parse_error {
                    return Err(GitError::UnexpectedOutput {
                        command: "git log".to_string(),
                        message: msg,
                    });
                }
                return Err(e);
            }
        }
        if let Err(e) = parser.finish() {
            return Err(GitError::UnexpectedOutput {
                command: "git log".to_string(),
                message: e.to_string(),
            });
        }
        if !pending.is_empty() {
            let batch = std::mem::take(&mut pending);
            totals.walked += batch.len() as u32;
            let mut items = Vec::with_capacity(batch.len());
            sift_batch(batch, &stash_refs, &mut stash_skip, &mut items);
            totals.shown += items.len() as u32;
            self.emit_rows(generation, &items, parser.pool());
        }
        Ok(totals)
    }

    /// Buffered variant of [`RepoSession::stream_log`]: rows accumulate
    /// into the caller's builder/vec without touching shared state or the
    /// sink (used by every offscreen rebuild — the tag-inclusive swap
    /// pass and `refresh_log`'s background refreshes). Returns the number
    /// of commits the walk emitted (what `--max-count` limits — the shown
    /// row count is `out.len()`).
    async fn collect_log(
        self: &Arc<Self>,
        workdir: &std::path::Path,
        options: LogOptions,
        cancel: &CancellationToken,
        builder: &mut GraphBuilder,
        out: &mut Vec<LogRow>,
    ) -> Result<u32, GitError> {
        // An unborn HEAD has nothing to log (see stream_log).
        let head_tip = match self.known_head_tip() {
            Some(tip) => tip,
            None => refs::head_state(&self.executor, workdir, cancel).await?.oid,
        };
        if head_tip.is_none() {
            return Ok(0);
        }

        // Dirty working tree: prepend the synthetic WIP row (mirrors
        // stream_log).
        if self.wip_dirty.load(Ordering::SeqCst)
            && let Some(head_oid) = head_tip
        {
            out.push(wip_row(&head_oid, builder));
        }

        // Stashes join the walk here too (see stream_log).
        let stash_refs: HashMap<Oid, String> = stash::load(&self.executor, workdir, cancel)
            .await
            .map(|list| list.into_iter().map(|s| (s.oid, s.name)).collect())
            .unwrap_or_default();

        let mut cmd = GitCommand::new()
            .cwd(workdir)
            .args(["log", "-z", "--date-order", LOG_FORMAT_ARG])
            .args(["HEAD", "--branches", "--remotes"])
            .no_timeout();
        if options.include_tags {
            cmd = cmd.arg("--tags");
        }
        if let Some(limit) = options.limit {
            cmd = cmd.arg(format!("--max-count={limit}"));
        }
        if !stash_refs.is_empty() {
            cmd = cmd.arg("--ignore-missing");
            for oid in stash_refs.keys() {
                cmd = cmd.arg(oid.to_hex());
            }
        }

        let mut parser = LogParser::new();
        let mut pending: Vec<CommitMeta> = Vec::new();
        let mut stash_skip: std::collections::HashSet<Oid> = std::collections::HashSet::new();
        let mut parse_error: Option<String> = None;
        let mut walked: u32 = 0;

        let result = self
            .executor
            .run_streaming(cmd, cancel, &mut |bytes| {
                if parse_error.is_some() {
                    return;
                }
                if let Err(e) = parser.feed(bytes, &mut pending) {
                    parse_error = Some(e.to_string());
                    cancel.cancel();
                    return;
                }
                let batch = std::mem::take(&mut pending);
                walked += batch.len() as u32;
                let mut items = Vec::with_capacity(batch.len());
                sift_batch(batch, &stash_refs, &mut stash_skip, &mut items);
                for item in items {
                    let mut row =
                        make_row(&item.meta, parser.pool(), builder, item.stash_ref.is_some());
                    if let Some(r) = item.stash_ref {
                        row.stash_ref = r;
                    }
                    out.push(row);
                }
            })
            .await;
        match result {
            Ok(_) => {}
            Err(e) => {
                if let Some(msg) = parse_error {
                    return Err(GitError::UnexpectedOutput {
                        command: "git log".to_string(),
                        message: msg,
                    });
                }
                return Err(e);
            }
        }
        if let Err(e) = parser.finish() {
            return Err(GitError::UnexpectedOutput {
                command: "git log".to_string(),
                message: e.to_string(),
            });
        }
        let batch = std::mem::take(&mut pending);
        walked += batch.len() as u32;
        let mut items = Vec::with_capacity(batch.len());
        sift_batch(batch, &stash_refs, &mut stash_skip, &mut items);
        for item in items {
            let mut row = make_row(&item.meta, parser.pool(), builder, item.stash_ref.is_some());
            if let Some(r) = item.stash_ref {
                row.stash_ref = r;
            }
            out.push(row);
        }
        Ok(walked)
    }

    /// Sends the synthetic WIP row (dirty working tree) as its own chunk.
    fn emit_wip_row(&self, generation: u64, head: &Oid) {
        let mut guard = self.lock_shared();
        if guard.generation != generation {
            return; // this stream is not the graph on screen (see emit_rows)
        }
        let row = wip_row(head, &mut guard.builder);
        guard.sent_rows.push(RowPrint::of(&row));
        self.sink.event(SessionEvent::LogChunk {
            generation,
            rows: vec![row],
        });
    }

    /// Builds graph rows for a batch and sends them (holding the shared
    /// lock so generations cannot interleave).
    ///
    /// A stream may only add to the graph it reset: `generation` matching
    /// the installed one is what says these rows belong to what the
    /// consumer shows. The counter would answer a different question —
    /// it moves for passes that never reach anybody, and a stream still
    /// on screen would stop delivering halfway through.
    fn emit_rows(&self, generation: u64, batch: &[StreamItem], pool: &crate::model::StrPool) {
        let mut guard = self.lock_shared();
        if guard.generation != generation {
            return;
        }
        let shared = &mut *guard;
        let mut rows = Vec::with_capacity(batch.len());
        for item in batch {
            let mut row = make_row(
                &item.meta,
                pool,
                &mut shared.builder,
                item.stash_ref.is_some(),
            );
            if let Some(r) = &item.stash_ref {
                row.stash_ref = r.clone();
            }
            let labels = shared.label_map.labels_of(&item.meta.oid).to_vec();
            if !labels.is_empty() {
                row.labels = labels.clone();
                shared.applied.insert(row.row, labels);
            }
            rows.push(row);
        }
        shared.sent_rows.extend(rows.iter().map(RowPrint::of));
        self.sink.event(SessionEvent::LogChunk { generation, rows });
    }

    /// The snapshot to publish: the one already on screen when this read
    /// found it unchanged, so the sidebar can tell "the same" from "equal"
    /// by pointer and rebuild nothing for it.
    ///
    /// Still published either way. Withholding the event instead would
    /// save the same work, but a consumer that attached after the last one
    /// went out would then sit empty until something moved, and "nothing
    /// changed" is the state that lasts longest.
    pub(super) fn published_snapshot(&self) -> Option<Arc<RefsSnapshot>> {
        match self.last_snapshot.lock() {
            Ok(slot) => slot.as_ref().map(Arc::clone),
            Err(e) => e.into_inner().as_ref().map(Arc::clone),
        }
    }

    pub(super) fn share_snapshot(&self, fresh: RefsSnapshot) -> Arc<RefsSnapshot> {
        let mut slot = match self.last_snapshot.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        if let Some(previous) = slot.as_ref()
            && **previous == fresh
        {
            return Arc::clone(previous);
        }
        let shared = Arc::new(fresh);
        *slot = Some(Arc::clone(&shared));
        shared
    }

    /// Installs a new label map into the join and sends the rows whose
    /// chips changed.
    ///
    /// Read and send happen under one lock, and the row numbers travel
    /// with the graph they were read from: a log pass installs its own
    /// state and announces it under the same lock, so a diff can neither
    /// be invalidated between the two nor arrive ahead of the rows it
    /// numbers. The generation covers what the lock cannot — a superseded
    /// pass that installs late leaves `shared` describing a graph the
    /// consumer already dropped, and its row numbers point at other
    /// commits there.
    pub(super) fn apply_refs(&self, label_map: LabelIndex) {
        let mut shared = self.lock_shared();
        shared.label_map = label_map;

        let mut fresh: HashMap<u32, Vec<RefLabel>> = HashMap::new();
        for (oid, labels) in shared.label_map.commits() {
            if let Some(row) = shared.builder.row_of(&oid) {
                fresh.insert(row, labels.to_vec());
            }
        }
        let mut changed: Vec<(u32, Vec<RefLabel>)> = Vec::new();
        for (row, labels) in &fresh {
            if shared.applied.get(row) != Some(labels) {
                changed.push((*row, labels.clone()));
            }
        }
        for row in shared.applied.keys() {
            if !fresh.contains_key(row) {
                changed.push((*row, Vec::new()));
            }
        }
        shared.applied = fresh;
        changed.sort_by_key(|(row, _)| *row);
        // Mirror the chip change into the delivered-rows record, or the
        // next background rebuild would see a phantom difference and swap
        // an identical graph.
        for (row, labels) in &changed {
            if let Some(sent) = shared.sent_rows.get_mut(*row as usize) {
                sent.labels = RowPrint::labels_of(labels);
            }
        }
        if changed.is_empty() {
            return;
        }
        self.sink.event(SessionEvent::LabelsChanged {
            generation: shared.generation,
            rows: changed,
        });
    }
}
