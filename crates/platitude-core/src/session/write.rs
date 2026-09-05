//! The session write queue: one request at a time, in submission order,
//! each followed by the refreshes it invalidated.

use super::*;

/// Whether `op` replays history a commit at a time, rather than touching
/// the index once and coming back.
///
/// These are the writes that can stand for tens of seconds — a replay
/// costs about 11ms a commit (measured), so a range of a few hundred is
/// seconds and the graph's own window is more than twenty — and they are
/// the ones git leaves a standing operation on disk for while they run.
/// Both halves of that matter: the poll is let through under these so the
/// badge can count the steps out ([`RepoSession::refresh_poll`]), and the
/// screen holds its write doors down for as long as one is out.
///
/// **The one place the set is written.** The poll's gate and the screen's
/// lock ask the same question, and a second spelling of it would let the
/// two disagree about what is running.
#[must_use]
pub fn replays_history(op: &str) -> bool {
    matches!(
        op,
        "merge" | "rebase" | "squash" | "drop" | "reword" | "cherry-pick" | "revert" | "resolve"
    )
}

/// Whether `op` is paced by the far end of a network connection — the
/// fetches and pushes, whose one real way to hang is a remote that stopped
/// answering. These keep their time budget and die with the session.
///
/// Every other write is local: it costs what the repository's own size
/// makes it cost, and slowness is not a hang — so the queue waits it out
/// to the end, with no stock budget and no cancellation, even through a
/// close. A kill mid-write is the one way this queue can lose what the
/// user asked for (measured: a killed commit is simply gone), and a
/// user's own wedged hook is the user's to deal with — the screen shows
/// busy and the command log shows what is running.
///
/// **The one place the set is written**, for the reason
/// [`replays_history`] gives: the budget lane and the cancel lane ask the
/// same question, and a second spelling would let them disagree.
///
/// **The default an unlisted op gets is the unsupervised lane** — waited
/// out, uncancellable, holding the quit gate. An op whose task reaches
/// the network in *any* half must be named here or enqueue through
/// [`RepoSession::write_remote_paced`] (the compound deletes do — their
/// labels are local ops' names); nothing mechanical catches the
/// omission, so the choice is this comment's to demand.
#[must_use]
pub fn remote_paced(op: &str) -> bool {
    matches!(op, "push" | "fetch" | AUTO_FETCH_OP | OPEN_FETCH_OP)
}

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
        self.enqueue(op, remote_paced(op), after, task);
    }

    /// [`RepoSession::write`] with the network lane chosen by the caller —
    /// for the compound writes whose op label the classifier cannot see
    /// through ("delete here and on the remote too" runs a push under a
    /// local op's name). The far end paces them, so they keep their
    /// budget and die with the session like any push.
    pub(super) fn write_remote_paced<F, Fut>(
        self: &Arc<Self>,
        op: &'static str,
        after: AfterWrite,
        task: F,
    ) where
        F: FnOnce(GitExecutor, RepoInfo, CancellationToken) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), GitError>> + Send + 'static,
    {
        self.enqueue(op, true, after, task);
    }

    fn enqueue<F, Fut>(
        self: &Arc<Self>,
        op: &'static str,
        remote_paced: bool,
        after: AfterWrite,
        task: F,
    ) where
        F: FnOnce(GitExecutor, RepoInfo, CancellationToken) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), GitError>> + Send + 'static,
    {
        let request = WriteRequest {
            op,
            after,
            remote_paced,
            run: Box::new(move |exec, repo, cancel| Box::pin(task(exec, repo, cancel))),
        };
        // Counted before it is sent, so the count can never trail the
        // queue: the loop's decrement pairs with exactly one increment.
        if !remote_paced {
            self.local_writes.fetch_add(1, Ordering::SeqCst);
        }
        // Enqueueing is synchronous, so the queue order is the order the UI
        // asked in. Sending only fails once the session has shut down.
        if self.write_tx.send(request).is_err() {
            if !remote_paced {
                self.local_writes.fetch_sub(1, Ordering::SeqCst);
            }
            tracing::debug!(op, "write dropped: the session is closed");
        }
    }

    /// Runs queued writes one at a time, in submission order.
    ///
    /// A close reaches this loop between requests, never inside one: the
    /// write already running is waited out to its end where its lane says
    /// so (its token is not the session's), and the requests already
    /// queued behind it still run — the queue's promise is that the asked
    /// order lands, and the quit gate holds the window for this same tail
    /// (`local_writes`). What the close does stop is intake: nothing sent
    /// after it is accepted, and the network-paced requests in the tail
    /// die at once on the session's cancelled token. `biased`, so a close
    /// that has landed wins the race into drain mode deterministically.
    pub(super) async fn write_loop(
        self: Arc<Self>,
        mut queue: tokio::sync::mpsc::UnboundedReceiver<WriteRequest>,
    ) {
        loop {
            let request = tokio::select! {
                biased;
                _ = self.root_cancel.cancelled() => break,
                request = queue.recv() => match request {
                    Some(request) => request,
                    None => return,
                },
            };
            self.serve(request).await;
        }
        queue.close();
        while let Some(request) = queue.recv().await {
            self.serve(request).await;
        }
    }

    /// One request, served whole: the flags the poll reads set around it,
    /// the run, and the count the quit gate reads given back.
    async fn serve(self: &Arc<Self>, request: WriteRequest) {
        // Set around the whole request, refreshes included, so the
        // poll keeps out until the write's own refresh has landed —
        // except under the writes that replay, which the poll is
        // allowed through so the screen can count them out.
        let replays = replays_history(request.op);
        let local = !request.remote_paced;
        self.write_busy.store(true, Ordering::SeqCst);
        self.write_replays.store(replays, Ordering::SeqCst);
        self.run_write(request).await;
        self.write_replays.store(false, Ordering::SeqCst);
        self.write_busy.store(false, Ordering::SeqCst);
        if local {
            self.local_writes.fetch_sub(1, Ordering::SeqCst);
        }
    }

    async fn run_write(self: &Arc<Self>, request: WriteRequest) {
        let WriteRequest {
            op,
            after,
            remote_paced,
            run,
        } = request;
        let Some(info) = self.repo_info() else {
            tracing::debug!(op, "write dropped: no repository is open");
            return;
        };
        self.sink.event(SessionEvent::WriteStarted { op });
        // The request's lane decides both halves of its supervision at
        // once: a network-paced write keeps the stock budget and dies
        // with the session, a local one runs unbudgeted — on a token
        // nothing cancels — to completion, even through a close. The
        // fetches nobody asked for travel this queue too — the interval's
        // and the one an opening fires — on a handle of their own: one
        // that lands leaves no row, and one git said no to leaves its own
        // (`Kept::UnaskedUnlessItFails`). Neither raises the panel by
        // being a row — nobody asked for it — so an offline laptop still
        // gets the one telling its first failure is entitled to.
        let (exec, cancel) = if op == AUTO_FETCH_OP || op == OPEN_FETCH_OP {
            (self.exec_unasked_fetch.clone(), self.root_cancel.clone())
        } else if remote_paced {
            (self.exec_user.clone(), self.root_cancel.clone())
        } else {
            (
                self.exec_user.clone().without_stock_timeouts(),
                CancellationToken::new(),
            )
        };
        let result = run(exec, info, cancel).await;
        // The write has ended, and nothing that looked at the repository
        // before this moment may speak for it after: the reads below are
        // the ones that answer for what the write left, and a poll that
        // began under it lands stale (`Standing::fence`). The number the
        // fence answers travels with the write's answer, so a consumer
        // landing on "the repository as the write left it" arms on the
        // report the write is owed by name rather than by counting.
        let head_seq = self.standing.fence();
        let rebuild_graph = match result {
            Ok(()) => {
                self.sink.event(SessionEvent::WriteFinished {
                    op,
                    error: None,
                    report: None,
                    head_seq,
                });
                matches!(after, AfterWrite::Graph | AfterWrite::Refs)
            }
            // Only the network lane can land here — a local write's token
            // is nobody's to cancel — and it means the session is closing.
            // The event pair must still balance: the UI counts
            // Started/Finished to know whether a write is in flight, and
            // an unmatched start would pin that count for good.
            Err(error) if error.is_cancelled() => {
                self.sink.event(SessionEvent::WriteFinished {
                    op,
                    error: Some(error.to_string()),
                    report: None,
                    head_seq,
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
                    head_seq,
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
        //
        // A write that could not reach the index or the tree reads them the
        // other way round, in series: the refs first, and the tree only
        // where they moved, because that move is the only thing a status
        // could report differently afterwards (`AfterWrite::Refs`). The
        // series is the point — a join would run the read it is trying not
        // to spend.
        let tree_only = after == AfterWrite::Tree;
        let (wip_flipped, refs_moved) = match after {
            AfterWrite::Tree => (self.read_status().await, false),
            AfterWrite::Refs => {
                let refs_moved = self.read_refs().await;
                let wip_flipped = if refs_moved {
                    self.read_status().await
                } else {
                    false
                };
                (wip_flipped, refs_moved)
            }
            AfterWrite::Snapshots | AfterWrite::Graph | AfterWrite::Author => {
                tokio::join!(self.read_status(), self.read_refs())
            }
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

#[cfg(test)]
mod tests {
    use super::{AUTO_FETCH_OP, OPEN_FETCH_OP, remote_paced, replays_history};

    /// The set is the writes that hand a range to git one commit at a
    /// time. The three one-commit edits are in it because each is a
    /// rebase underneath (`sequencer::plan_edit` + `run_plan`), so a
    /// squash near the root replays everything above it — the same wait
    /// under a shorter name.
    #[test]
    fn the_writes_that_replay_are_the_ones_a_range_can_make_long() {
        for op in [
            "merge",
            "rebase",
            "squash",
            "drop",
            "reword",
            "cherry-pick",
            "revert",
            "resolve",
        ] {
            assert!(replays_history(op), "{op} hands a range to git");
        }
        // Everything that touches the index once and comes back: the poll
        // stays out under these, the way it always has.
        for op in [
            "commit",
            "stage",
            "unstage",
            "discard",
            "stash",
            "switch",
            "branch",
            "tag",
            "push",
            "fetch",
            "config",
            "mergetool",
            AUTO_FETCH_OP,
            OPEN_FETCH_OP,
        ] {
            assert!(!replays_history(op), "{op} is one pass, not a replay");
        }
    }

    /// The network lane is the fetches and pushes in their four
    /// spellings; every other op *label* defaults to the local lane,
    /// which the session waits out — no stock budget binds it and a
    /// close does not cancel it. The two compound deletes that push
    /// under the labels "branch" and "tag" override the default at
    /// their call sites (`write_remote_paced`), which this table cannot
    /// see — the label answers for the plain ops alone.
    #[test]
    fn the_remote_paced_writes_are_the_fetches_and_pushes() {
        for op in ["push", "fetch", AUTO_FETCH_OP, OPEN_FETCH_OP] {
            assert!(remote_paced(op), "{op} is paced by the far end");
        }
        for op in [
            "stage",
            "unstage",
            "discard",
            "commit",
            "checkout",
            "reset",
            "branch",
            "tag",
            "stash",
            "merge",
            "rebase",
            "squash",
            "drop",
            "reword",
            "cherry-pick",
            "revert",
            "resolve",
            "mergetool",
            "identity",
            "remote",
        ] {
            assert!(!remote_paced(op), "{op} is local and waited out");
        }
    }
}
