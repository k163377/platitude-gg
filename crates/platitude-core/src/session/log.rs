//! The log -> graph pipeline: options, the direct and swap passes, and
//! the background refreshes. What one pass reads is [`super::walk`].

use super::*;

impl RepoSession {
    pub fn log_options(&self) -> LogOptions {
        *self.lock_log_options()
    }

    /// Whether the graph is drawing tags — the TAGS band's eye, and what
    /// every row's chips are cut against (`LabelIndex::labels_of`).
    ///
    /// Read from the session and **not from the pass's own `LogOptions`**:
    /// the first of a restart's two passes runs with the tags taken out of
    /// the walk to get a picture up (see `restart_log`), and a chip cut to
    /// match that would take every tag off the screen for the length of
    /// that pass and then put it back.
    pub(super) fn tags_shown(&self) -> bool {
        self.lock_log_options().include_tags
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

        let run_cancel = self.take_log_token();
        let held = self.graph_passes.enter();

        let s = Arc::clone(self);
        let options = self.log_options();
        self.runtime.spawn(async move {
            let _held = held;
            if options.include_tags {
                let fast = LogOptions {
                    include_tags: false,
                    ..options
                };
                if s.run_direct_pass(&workdir, fast, &run_cancel).await.is_ok() {
                    let _outcome = s.run_swap_pass(&workdir, options, &run_cancel).await;
                }
            } else {
                let _completed = s.run_direct_pass(&workdir, options, &run_cancel).await;
            }
        });
    }

    /// Takes the log stream over: a fresh token for this pass, with the
    /// one it displaces cancelled.
    ///
    /// **Called on the caller's thread, before the pass is spawned**,
    /// never from inside the spawned task: call order is what decides
    /// which pass owns the graph, and spawn order does not follow it
    /// (core.md).
    pub(super) fn take_log_token(&self) -> CancellationToken {
        let run_cancel = self.root_cancel.child_token();
        if let Some(prev) = self
            .log_cancel
            .lock()
            .map(|mut g| g.replace(run_cancel.clone()))
            .unwrap_or_default()
        {
            prev.cancel();
        }
        run_cancel
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
    pub(super) async fn run_swap_pass(
        self: &Arc<Self>,
        workdir: &std::path::Path,
        options: LogOptions,
        cancel: &CancellationToken,
    ) -> RefreshOutcome {
        // Superseded before it began: somebody took the stream over
        // between this pass being asked for and its first read. The check
        // further down stops a finished pass from installing a graph
        // nobody wants; this one stops it from being walked at all, and
        // the walk is the most expensive read in the app. **The opening
        // is where that lands**: its tag-inclusive pass waits out the
        // tag-less one that paints, so anything asking for a rebuild in
        // between (a write, a poll tick, a test taking its baseline) used
        // to leave a whole history walk running for a graph that had
        // already been replaced.
        if cancel.is_cancelled() {
            return RefreshOutcome::Cancelled;
        }
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
                if matches!(error, GitError::Cancelled { .. }) {
                    return RefreshOutcome::Cancelled;
                }
                self.fail("log", error);
                return RefreshOutcome::Failed;
            }
        };

        let total = rows.len() as u32;
        let tags = self.tags_shown();
        {
            let mut shared = self.lock_shared();
            // Superseded: someone asked for a graph after this pass was
            // started, and that ask cancelled this token. Read here
            // rather than the generation counter — that one is stamped
            // when a pass begins running, which is not the order the
            // asks came in.
            if cancel.is_cancelled() {
                return RefreshOutcome::Cancelled;
            }
            let elapsed_ms = started.elapsed().as_millis() as u64;
            let mut applied: HashMap<u32, Vec<RefLabel>> = HashMap::new();
            for row in &mut rows {
                if let Ok(oid) = Oid::from_hex_str(&row.oid_hex) {
                    let labels = shared.label_map.labels_of(&oid, tags);
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
                return RefreshOutcome::Unchanged;
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
        RefreshOutcome::Changed
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
}
