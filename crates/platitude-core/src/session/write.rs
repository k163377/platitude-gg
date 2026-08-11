//! The session write queue: one request at a time, in submission order,
//! each followed by the refreshes it invalidated.

use super::*;

impl RepoSession {
    // --- writes ---------------------------------------------------------

    /// Runs one write command under the session write lock, reports its
    /// lifecycle and refreshes afterwards.
    ///
    /// The lock is released before refreshing so a queued write is not held
    /// up by snapshot reads.
    pub(super) fn write<F, Fut>(self: &Arc<Self>, op: &'static str, after: AfterWrite, task: F)
    where
        F: FnOnce(GitExecutor, RepoInfo, CancellationToken) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), GitError>> + Send + 'static,
    {
        let request = WriteRequest {
            op,
            after,
            run: Box::new(move |exec, repo, cancel| Box::pin(task(exec, repo, cancel))),
        };
        // Enqueueing is synchronous, so the queue order is the order the UI
        // asked in. Sending only fails once the session has shut down.
        if self.write_tx.send(request).is_err() {
            tracing::debug!(op, "write dropped: the session is closed");
        }
    }

    /// Runs queued writes one at a time, in submission order.
    pub(super) async fn write_loop(
        self: Arc<Self>,
        mut queue: tokio::sync::mpsc::UnboundedReceiver<WriteRequest>,
    ) {
        loop {
            let request = tokio::select! {
                _ = self.root_cancel.cancelled() => return,
                request = queue.recv() => match request {
                    Some(request) => request,
                    None => return,
                },
            };
            // Set around the whole request, refreshes included, so the
            // poll keeps out until the write's own refresh has landed.
            self.write_busy.store(true, Ordering::SeqCst);
            self.run_write(request).await;
            self.write_busy.store(false, Ordering::SeqCst);
        }
    }

    async fn run_write(self: &Arc<Self>, request: WriteRequest) {
        let WriteRequest { op, after, run } = request;
        let Some(info) = self.repo_info() else {
            tracing::debug!(op, "write dropped: no repository is open");
            return;
        };
        let cancel = self.root_cancel.clone();
        self.sink.event(SessionEvent::WriteStarted { op });
        // Auto fetch travels this queue too, but nobody asked for it: it
        // stays out of the command log unless background reads are on,
        // and so cannot make an offline laptop raise the panel every
        // interval.
        let exec = if op == AUTO_FETCH_OP {
            self.executor.clone()
        } else {
            self.exec_user.clone()
        };
        let result = run(exec, info, cancel).await;
        let rebuild_graph = match result {
            Ok(()) => {
                self.sink
                    .event(SessionEvent::WriteFinished { op, error: None });
                after == AfterWrite::Graph
            }
            // Cancelled means the session is closing, but the event pair
            // must still balance: the UI counts Started/Finished to know
            // whether a write is in flight, and an unmatched start would
            // pin that count for good.
            Err(error) if error.is_cancelled() => {
                self.sink.event(SessionEvent::WriteFinished {
                    op,
                    error: Some(error.to_string()),
                });
                return;
            }
            Err(error) => {
                tracing::warn!(op, %error, "write failed");
                self.sink.event(SessionEvent::WriteFinished {
                    op,
                    error: Some(error.to_string()),
                });
                // A half-finished command still changed the repository
                // (conflicted merge, interrupted rebase, partial apply),
                // but it did not move history the way it meant to.
                false
            }
        };

        // Anything that touched the repository can have brought a new
        // `.gitattributes` or changed what a directory's files look like,
        // and a stale house style is worse than asking again.
        self.forget_eol_baselines();

        // Settle the working tree and the refs before touching the graph:
        // the WIP row exists only while the tree is dirty and a write that
        // lands a commit moves a ref, so rebuilding first and then reacting
        // to either would walk the whole history twice for one write.
        let (wip_flipped, refs_moved) = tokio::join!(self.publish_status(), self.publish_refs());
        if rebuild_graph || wip_flipped || refs_moved {
            // Off-screen rebuild: the pane keeps showing the old graph
            // until the finished one swaps in (or nothing changed and
            // nothing repaints — the auto-fetch common case).
            self.refresh_log();
        }
        // A stash push, pop or drop moves no ref, so the refs read has no
        // reason to ask again — and it is exactly what changes whether
        // something other than this branch still holds the tip.
        if !refs_moved {
            self.settle_head_reach();
        }
        self.refresh_stashes();
        self.refresh_worktrees();
        if after == AfterWrite::Author {
            self.refresh_author();
        }
    }
}
