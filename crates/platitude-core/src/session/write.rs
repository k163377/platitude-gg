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
        // The fetches nobody asked for travel this queue too — the
        // interval's and the one an opening fires — and stay out of the
        // command log unless background reads are on, so neither can make
        // an offline laptop raise the panel.
        let exec = if op == AUTO_FETCH_OP || op == OPEN_FETCH_OP {
            self.executor.clone()
        } else {
            self.exec_user.clone()
        };
        let result = run(exec, info, cancel).await;
        let rebuild_graph = match result {
            Ok(()) => {
                self.sink.event(SessionEvent::WriteFinished {
                    op,
                    error: None,
                    report: None,
                });
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
                    report: None,
                });
                return;
            }
            Err(error) => {
                tracing::warn!(op, %error, "write failed");
                // A write that did not happen and has something to say
                // for itself travels beside git's words: the screen makes
                // a report out of the one and keeps the other for the log.
                let report = error.report().cloned();
                self.sink.event(SessionEvent::WriteFinished {
                    op,
                    error: Some(error.to_string()),
                    report,
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
        self.forget_derived();

        // Settle the working tree and the refs before touching the graph:
        // the WIP row exists only while the tree is dirty and a write that
        // lands a commit moves a ref, so rebuilding first and then reacting
        // to either would walk the whole history twice for one write.
        //
        // A write that only moved the index reads the tree alone: the refs
        // are where they were, and asking again is the longest read in the
        // app on a repository with refs in it (`AfterWrite::Tree`).
        let tree_only = after == AfterWrite::Tree;
        let (wip_flipped, refs_moved) = if tree_only {
            (self.publish_status().await, false)
        } else {
            tokio::join!(self.publish_status(), self.publish_refs(false))
        };
        if rebuild_graph || wip_flipped || refs_moved {
            // Off-screen rebuild: the pane keeps showing the old graph
            // until the finished one swaps in (or nothing changed and
            // nothing repaints — the auto-fetch common case). The write
            // queue does not advance until this answer lands: otherwise a
            // later write or test barrier can overtake and cancel the very
            // refresh it is meant to follow.
            let _ = self.refresh_log_tracked().outcome().await;
        }
        // A stash push, pop or drop moves no ref, so the refs read has no
        // reason to ask again — and it is exactly what changes whether
        // something other than this branch still holds the tip.
        //
        // An index-only write changes none of the three: what is published
        // is a question about commits, a stash is made by a command that
        // says so, and a worktree is added or removed by another.
        if !tree_only {
            if !refs_moved {
                self.settle_head_reach();
            }
            self.refresh_stashes();
            self.refresh_worktrees();
        }
        if after == AfterWrite::Author {
            self.refresh_author();
        }
    }
}
