//! The session write queue: one request at a time, in submission order,
//! each followed by the refreshes it invalidated.
//!
//! **Three boundaries, one id.** *Accepted* when [`RepoSession::write`]
//! hands its [`OperationId`] back (synchronously, so queue order is the
//! order the UI asked in; a closed session hands back nothing). git's
//! answer is [`SessionEvent::WriteFinished`], before anything is read
//! again. *Settled* is [`SessionEvent::WriteSettled`], once the reads it
//! invalidated have published (naming those that did not land); only
//! then does the queue take the next request. Every event and every git
//! command under the request carries the id, so a consumer matches its
//! own write by id, not by arrival order (`crate::operation`).
//!
//! **One queue per session, one order per worktree.** A session
//! closed mid-write keeps that write running, and a tab reopened over it
//! is a second session on the same index, so acceptance takes a place in
//! the tree's order as well as an id (`session::write_order`):
//!
//! * a write the closed session accepted runs before anything the new
//!   one accepts, and no local write of either is lost or killed;
//! * a poll's reads stay out of the tree for the whole of any write on
//!   it ([`RepoSession::tree_write`]) and are taken again, as a read
//!   owed, the moment it lands ([`RepoSession::tell_the_tree`]);
//! * **an opening's reads go straight out** (a tab has to paint), so they
//!   can read a tree mid-write; the line above makes that safe, since the
//!   new session is on the tree's list from before `Opened`
//!   (`join_write_order`) — the last reading any session holds is taken
//!   after the last write on that tree;
//! * the reads behind a write are the writing session's own, and a closed
//!   session makes none — the next holder of the tree reads what it left.

use super::*;

impl RepoSession {
    /// Accepts one write for the queue. Answers with the id every event
    /// about the write will carry, or `None` once the session's loop has
    /// ended (an id then would name a write nothing answers).
    pub(super) fn write<F, Fut>(
        self: &Arc<Self>,
        kind: OperationKind,
        after: AfterWrite,
        task: F,
    ) -> Option<OperationId>
    where
        F: FnOnce(GitExecutor, RepoInfo, CancellationToken) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), GitError>> + Send + 'static,
    {
        self.write_taking(kind, after, &[], task)
    }

    /// [`Self::write`] for a delete: what only `names` hold leaves the
    /// graph now, and comes back if git refuses (`session::leaving`).
    pub(super) fn write_taking<F, Fut>(
        self: &Arc<Self>,
        kind: OperationKind,
        after: AfterWrite,
        names: &[LeavingRef<'_>],
        task: F,
    ) -> Option<OperationId>
    where
        F: FnOnce(GitExecutor, RepoInfo, CancellationToken) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), GitError>> + Send + 'static,
    {
        self.accept(kind, after, names, Vec::new(), task)
    }

    /// [`Self::write`] for one that also puts work into other worktrees —
    /// those whose git directories are `into` (a restore of work
    /// thrown away there, 破棄記録仕様.md §4). It takes its turn in each of
    /// their orders as well as its own, from the same instant: what was
    /// accepted there before it — a commit, a reset, a rebase — runs first,
    /// and nothing accepted there after it runs under it. The sessions open
    /// on those worktrees read them again once it lands.
    pub(super) fn write_into<F, Fut>(
        self: &Arc<Self>,
        kind: OperationKind,
        after: AfterWrite,
        into: &[PathBuf],
        task: F,
    ) -> Option<OperationId>
    where
        F: FnOnce(GitExecutor, RepoInfo, CancellationToken) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), GitError>> + Send + 'static,
    {
        let own = self.write_order();
        let mut orders: Vec<Arc<WriteOrder>> = Vec::new();
        for git_dir in into {
            let order = write_order::of(git_dir);
            let known = own
                .iter()
                .chain(&orders)
                .any(|held| Arc::ptr_eq(held, &order));
            if !known {
                orders.push(order);
            }
        }
        self.accept(kind, after, &[], orders, task)
    }

    fn accept<F, Fut>(
        self: &Arc<Self>,
        kind: OperationKind,
        after: AfterWrite,
        names: &[LeavingRef<'_>],
        into: Vec<Arc<WriteOrder>>,
        task: F,
    ) -> Option<OperationId>
    where
        F: FnOnce(GitExecutor, RepoInfo, CancellationToken) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), GitError>> + Send + 'static,
    {
        let leaving = if names.is_empty() {
            None
        } else {
            self.resolve_leaving(names)
        };
        let stands_in = leaving.is_some();
        // The place in the tree's order is taken inside the call that
        // hands the id back, so a tab opened over a running write cannot
        // step in front of it (`session::write_order`). Taken by what
        // changes this worktree ([`OperationKind::writes_here`]), not by lane:
        // a composite delete is network-paced yet takes a ref away here.
        //
        // **Numbered, placed and queued under one lock.** The loop serves
        // the queue in order and each request waits for its own place, so
        // a pair placed in the other order than queued leaves the loop
        // waiting, for good, on a place held by a request only it can run.
        // Numbered outside, two racing requests could also be logged in the
        // wrong order across sessions. Ids answer to equality alone
        // ([`OperationId`]).
        let (operation, local, sent) = {
            let _accepting = relock(&self.accepting);
            let operation = Operation::new(kind, after);
            let local = operation.lane == Lane::Local;
            let own = kind.writes_here().then(|| self.write_order()).flatten();
            let (place, into) = write_order::take_places(own.as_ref(), &into);
            // Counted before it is sent, so the count can never trail the
            // queue: the loop's decrement pairs with exactly one increment.
            if local {
                self.local_writes.fetch_add(1, Ordering::SeqCst);
            }
            // Written down before it is sent, so no answer can come for a
            // delete that is not.
            if let Some(leaving) = leaving {
                self.leave(operation.id, leaving);
            }
            // Fails only once the loop has ended.
            let sent = self.write_tx.send(WriteRequest {
                operation,
                place,
                into,
                run: Box::new(move |exec, repo, cancel| Box::pin(task(exec, repo, cancel))),
            });
            (operation, local, sent)
        };
        // A refused request is dropped here with its place, which is how
        // a tree stops waiting for a write nobody will run.
        if sent.is_err() {
            if local {
                self.local_writes.fetch_sub(1, Ordering::SeqCst);
            }
            self.leaving_done(operation.id);
            tracing::debug!(?operation, "write not accepted: the session is closed");
            return None;
        }
        if stands_in {
            // Off the caller's thread: laying out is a pass over the window.
            let s = Arc::clone(self);
            self.runtime.spawn(async move { s.relay_leaving() });
        }
        Some(operation.id)
    }

    /// The write this session's queue is serving right now, refreshes
    /// included — `None` between requests. What a task under the queue
    /// reads to speak about itself (`note_landing`).
    pub(super) fn running_write(&self) -> Option<Operation> {
        *relock(&self.write_running)
    }

    /// The write being run in this worktree, by this session or
    /// another sharing it — what the poll's gate reads
    /// ([`RepoSession::refresh_poll`]). Gating on this session's write
    /// alone would let a reopened tab read between another session's
    /// rebase steps and offer to abort a rebase still being done.
    ///
    /// Network lanes take no place in the tree's order, so this session's
    /// own write still says a push or a fetch is out.
    pub(super) fn tree_write(&self) -> Option<Operation> {
        match self.write_order() {
            Some(order) => order.running().or_else(|| self.running_write()),
            None => self.running_write(),
        }
    }

    /// Tells every other session on this worktree to read it again;
    /// their polls kept out while this write held the front
    /// ([`Self::tree_write`]). Called once the place is back — told
    /// earlier, a session skips the tick as still being written.
    ///
    /// The read is one owed ([`RepoSession::read_again`]): a read already
    /// in flight began before this write, and a refused wake would leave it
    /// showing a repository that is gone.
    fn tell_the_tree(self: &Arc<Self>) {
        let Some(order) = self.write_order() else {
            return;
        };
        for reader in order.others(self) {
            reader.read_again();
        }
    }

    /// Runs queued writes one at a time, in submission order.
    ///
    /// A close stops intake only: the running write and the queued tail
    /// still run (the quit gate holds the window for them —
    /// `local_writes`), the network-paced ones dying at once on the
    /// cancelled token. `biased`, so a landed close enters drain mode
    /// deterministically.
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

    async fn serve(self: &Arc<Self>, request: WriteRequest) {
        let operation = request.operation;
        // Held around the whole request, refreshes included, so the poll
        // keeps out until the write's own refresh has landed — except
        // under replaying writes, which the poll reads through to count
        // them out (the gate reads the kind held here).
        *relock(&self.write_running) = Some(operation);
        let into: Vec<Arc<WriteOrder>> =
            request.into.iter().map(write_order::Place::order).collect();
        // The request's place goes back as it ends; the tree is free from
        // here on.
        self.run_write(request).await;
        // Landed, a delete is drawn by the walk behind it by now; refused,
        // it was put back at the answer (`session::leaving`).
        self.leaving_done(operation.id);
        *relock(&self.write_running) = None;
        // The quit gate counts by lane; the tree is told by kind.
        if operation.lane == Lane::Local {
            self.local_writes.fetch_sub(1, Ordering::SeqCst);
        }
        if operation.kind.writes_here() {
            self.tell_the_tree();
        }
        // The other worktrees it put work into: every session on them reads
        // them again, this one being on none.
        for order in into {
            for reader in order.others(self) {
                reader.read_again();
            }
        }
        // Nobody tells this session, so it asks itself: a read refused
        // while this write was out has nowhere else to be taken
        // ([`RepoSession::take_the_read_owed`]). Any lane — a push owes it
        // as a commit does.
        self.take_the_read_owed();
    }

    async fn run_write(self: &Arc<Self>, request: WriteRequest) {
        let WriteRequest {
            operation,
            place,
            into,
            run,
        } = request;
        let Operation {
            id,
            kind,
            lane,
            after,
        } = operation;
        // The tree's turn comes before `WriteStarted`, which means git is
        // running it (`session::write_order`), and so does the turn in each
        // other worktree it puts work into. The places are held to the end of
        // this function, the reads behind the write included.
        for place in place.iter().chain(&into) {
            place.granted(operation).await;
        }
        let Some(info) = self.repo_info() else {
            // The id went out at acceptance, so the boundaries are owed —
            // a caller holding it waits for them. Every entry point sits
            // behind an open repository, so reaching here is a bug.
            tracing::debug!(?operation, "write refused: no repository is open");
            let fence = self.standing.fence();
            self.sink.event(SessionEvent::WriteStarted { id, kind });
            self.sink.event(SessionEvent::WriteFinished {
                id,
                kind,
                error: Some("no repository is open".to_string()),
                report: None,
                head_seq: fence.head_seq,
                reads_from: fence.reads_from,
            });
            self.sink.event(SessionEvent::WriteSettled {
                id,
                kind,
                failed: Vec::new(),
            });
            return;
        };
        self.sink.event(SessionEvent::WriteStarted { id, kind });
        // The lane decides the supervision (`Lane`): a local write runs
        // unbudgeted on a token nothing cancels, even through a close; the
        // others keep the stock budget and die with the session. Unasked
        // fetches run on their own handle (`Kept::UnaskedUnlessItFails`)
        // and never raise the panel by being a row.
        let (exec, cancel) = match lane {
            Lane::UnaskedFetch => (self.exec_unasked_fetch.clone(), self.root_cancel.clone()),
            Lane::Remote => (self.exec_user.clone(), self.root_cancel.clone()),
            Lane::Local => (
                self.exec_user.clone().without_stock_timeouts(),
                CancellationToken::new(),
            ),
        };
        // Every command the task spawns names the write it runs under, so
        // a compound write is several rows under one id in the log.
        let result = run(exec.under(id), info, cancel).await;
        // From here only reads begun after the write speak for the
        // repository; a poll begun under it lands stale (`Standing::fence`).
        // The fence's numbers travel with the write's answer, for a
        // consumer landing on what the write left.
        let fence = self.standing.fence();
        let rebuild_graph = match result {
            Ok(()) => {
                self.sink.event(SessionEvent::WriteFinished {
                    id,
                    kind,
                    error: None,
                    report: None,
                    head_seq: fence.head_seq,
                    reads_from: fence.reads_from,
                });
                // The rest walk where a read behind them moved
                // (`settle_after`).
                matches!(after, AfterWrite::Graph | AfterWrite::Name { .. })
            }
            // Only a network write lands here (a local token is nobody's
            // to cancel): the session is closing. The boundaries must
            // still balance — the UI counts Started/Finished, and an
            // unmatched start pins the count. Nothing is read behind it.
            Err(error) if error.is_cancelled() => {
                self.sink.event(SessionEvent::WriteFinished {
                    id,
                    kind,
                    error: Some(error.to_string()),
                    report: None,
                    head_seq: fence.head_seq,
                    reads_from: fence.reads_from,
                });
                self.sink.event(SessionEvent::WriteSettled {
                    id,
                    kind,
                    failed: Vec::new(),
                });
                return;
            }
            Err(error) => {
                tracing::warn!(?operation, %error, "write failed");
                // The typed report travels beside git's words: the screen
                // makes a report of the one and logs the other.
                let report = error.report().cloned();
                self.sink.event(SessionEvent::WriteFinished {
                    id,
                    kind,
                    error: Some(error.to_string()),
                    report,
                    head_seq: fence.head_seq,
                    reads_from: fence.reads_from,
                });
                // A refused delete's commits come back with its rows, before
                // anything is read (`session::leaving`).
                self.leaving_refused(id);
                // A half-finished command still changed the repository
                // (conflicted merge, interrupted rebase, partial apply),
                // but it did not move history the way it meant to.
                false
            }
        };

        // A session closed mid-write reads nothing behind it: the reads
        // would publish into a page that has gone, through a retired sink
        // (`Hub::release_tab`). The next session holding the tree reads
        // what the write left (`session::write_order`).
        let failed = if self.root_cancel.is_cancelled() {
            Vec::new()
        } else {
            self.settle_after(operation, rebuild_graph).await
        };
        self.sink
            .event(SessionEvent::WriteSettled { id, kind, failed });
    }

    /// The reads behind a write that has answered, waited for to the
    /// last. Answers those that did not land, for the caller to name under
    /// the write's id (the failures themselves are on the error surface
    /// under no id).
    async fn settle_after(
        self: &Arc<Self>,
        operation: Operation,
        rebuild_graph: bool,
    ) -> Vec<FollowUp> {
        let after = operation.after;
        // What is read once and kept, dropped where the write can have
        // moved it: a write reaching the tree or the history can bring a
        // new `.gitattributes` or a remote; a config write the remotes
        // (its file's stamp drops the rest — `forget_what_the_config_decides`).
        // A fetch, a delete and a worktree taken away reach neither.
        match after {
            AfterWrite::Tree | AfterWrite::Snapshots | AfterWrite::Graph | AfterWrite::Author => {
                self.forget_derived();
            }
            AfterWrite::Config => self.remotes.forget(),
            AfterWrite::Refs | AfterWrite::Worktrees | AfterWrite::Name { .. } => {}
        }

        // Every read that decides the walk comes before it — the tree, the
        // refs and the listings each can move a row — so one walk answers
        // for the write, over the repository as the write left it.
        //
        // An index-only write reads the tree alone: the refs are where they
        // were, and theirs is the longest read (`AfterWrite::Tree`).
        //
        // A write that cannot reach index or tree reads the refs first and
        // the tree only where they moved, the one thing a status could then
        // report differently (`AfterWrite::Refs`) — in series on purpose: a
        // join would run the read the series saves.
        //
        // A delete moves a ref every time, so it is told at acceptance
        // whether the tree is owed a read, and reads none otherwise
        // (`AfterWrite::Name`).
        //
        // The listings are read with the rest, and the stashes they list go
        // to the walk, which reads none of its own (`walk_inputs`). They are
        // awaited: `WriteSettled` is only worth sending once all have
        // published, and a listing still in flight when the next write
        // answers can let a refusal put a dropped stash back on screen (the
        // app stands rows in until a listing proves them gone).
        let mut failed = Vec::new();
        let (tree, refs, (stashes, listed)) = match after {
            AfterWrite::Tree => (self.read_status().await, Reread::Same, Default::default()),
            AfterWrite::Name { status: false } => {
                (Reread::Same, self.read_refs().await, Default::default())
            }
            AfterWrite::Name { status: true } | AfterWrite::Config => {
                let (tree, refs) = tokio::join!(self.read_status(), self.read_refs());
                (tree, refs, Default::default())
            }
            AfterWrite::Refs => {
                let (tree, refs) = self.read_refs_then_tree().await;
                (tree, refs, Default::default())
            }
            AfterWrite::Worktrees => {
                let ((tree, refs), listed) =
                    tokio::join!(self.read_refs_then_tree(), self.read_worktrees());
                (tree, refs, (StashRead::default(), listed))
            }
            AfterWrite::Snapshots | AfterWrite::Graph | AfterWrite::Author => {
                tokio::join!(self.read_status(), self.read_refs(), self.read_listings())
            }
        };
        if tree == Reread::Failed {
            note_failed(&mut failed, FollowUp::Status);
        }
        if refs == Reread::Failed {
            note_failed(&mut failed, FollowUp::Refs);
        }
        for read in listed.failed {
            note_failed(&mut failed, read);
        }
        let refs_moved = refs == Reread::Moved;
        // A stash moved by the write — or outside, beside it — moves no ref
        // the refs read lists: the listing is its only word.
        let moved = tree == Reread::Moved || refs_moved || stashes.moved;
        let mut walk = rebuild_graph || moved || listed.walk;
        // A fetch that left the refs where they were has nothing new to
        // walk — unless the graph is behind: a read beside the fetch may
        // have seen its refs move first, and walks them itself (waited for,
        // so the write settles behind it), or a walk failed earlier.
        if after == AfterWrite::Refs && !walk {
            self.wait_for_graph_passes().await;
            walk = self.graph_stale.load(Ordering::SeqCst);
        }
        if walk {
            let reads = match self.workdir() {
                Some(workdir) => {
                    self.pass_reads_listed(&workdir, &self.root_cancel, stashes)
                        .await
                }
                None => None,
            };
            // Off-screen rebuild: the old graph stays until the new one
            // swaps in, or nothing repaints if unchanged. Awaited, or a
            // later write or test barrier could overtake and cancel the
            // refresh it follows.
            if !self.settle_graph_with(reads).await.landed() {
                note_failed(&mut failed, FollowUp::Graph);
            }
        }
        // Head reach is asked here because a stash push, pop or drop moves
        // no ref (so the refs read will not ask), yet changes whether
        // anything else holds the tip. An index-only write, a fetch and a
        // config write change none of it.
        let reaches = !matches!(
            after,
            AfterWrite::Tree | AfterWrite::Refs | AfterWrite::Config
        );
        if reaches && !refs_moved {
            self.settle_head_reach();
        }
        if after == AfterWrite::Author
            && let Some(task) = self.refresh_author()
        {
            match task.await {
                Ok(more) => {
                    for read in more {
                        note_failed(&mut failed, read);
                    }
                }
                // The task unwound: nothing it was to publish did.
                Err(error) => {
                    tracing::warn!(?operation, %error, "the author read after the write did not finish");
                    note_failed(&mut failed, FollowUp::Author);
                }
            }
        }
        // A landed delete whose walk did not land stands on in the graph
        // until one does (`session::leaving`).
        if rebuild_graph && failed.contains(&FollowUp::Graph) {
            self.leaving_unwalked(operation.id);
        }
        failed
    }

    /// The refs, and the tree only where they moved — the series a write
    /// that cannot reach the index or the tree reads (`AfterWrite::Refs`).
    async fn read_refs_then_tree(self: &Arc<Self>) -> (Reread, Reread) {
        let refs = self.read_refs().await;
        let tree = if refs == Reread::Moved {
            self.read_status().await
        } else {
            Reread::Same
        };
        (tree, refs)
    }
}

/// Names a read that did not land, once: the refs can be asked for twice
/// behind one write (by the write and by the worktree listing's joins).
fn note_failed(failed: &mut Vec<FollowUp>, read: FollowUp) {
    if !failed.contains(&read) {
        failed.push(read);
    }
}
