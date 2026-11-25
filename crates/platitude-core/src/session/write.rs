//! The session write queue: one request at a time, in submission order —
//! except an unasked fetch, which yields while it waits ([`Line::next`])
//! — and behind them the refreshes they invalidated.
//!
//! **Three boundaries, one id.** *Accepted* when [`RepoSession::write`]
//! hands its [`OperationId`] back (synchronously, so queue order is the
//! order the UI asked in; a closed session hands back nothing). git's
//! answer is [`SessionEvent::WriteFinished`], before anything is read
//! again. *Settled* is [`SessionEvent::WriteSettled`], once reads that
//! looked after the write have published what it invalidated (naming
//! those that did not land). Every event and every git command under the
//! request carries the id, so a consumer matches its own write by id, not
//! by arrival order (`crate::operation`).
//!
//! **A commit's status is told at its answer**, worked out and not read
//! (`session::foretold`): behind `WriteFinished`, ahead of every read. The
//! reads behind the write follow as behind any, and the walk among them
//! goes ahead of the status read ([`RepoSession::read_behind_told`]).
//!
//! **The reads are made once behind a burst.** A write waiting as the one in
//! front answers starts at once, where its kind joins a burst
//! ([`OperationKind::joins_a_burst`]) and the tree's next turn is its own
//! ([`Burst::takes`]): the writes of a burst answer one after another and
//! settle together, behind one pass of reads covering what each of them
//! invalidated ([`ReadsOwed`]). In front of any other write, and where
//! nothing waits, the reads are made first and waited for to the last —
//! as they are once a burst has gone unread for [`unread_ceiling`], whatever
//! waits. A read the queue begins it waits out: left behind, a walk the
//! next write's own took over would settle its write on a graph that never
//! landed, and a consumer told `WriteSettled` would still be owed a
//! listing.
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

use super::refresh::Listed;
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
    /// thrown away there, `RepoSession::restore_discard`). It takes its turn
    /// in each of their orders as well as its own, from the same instant: what
    /// was accepted there before it — a commit, a reset, a rebase — runs
    /// first, and nothing accepted there after it runs under it. A plain
    /// `git commit` there runs its `pre-commit` hook with the index unlocked
    /// and reads the index again after it, so work put in meanwhile would go
    /// into that commit (`work_brought_back_into_another_worktree_waits_for_its_commit`).
    /// The sessions open on those worktrees read them again once it lands.
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

    /// [`Self::write`] for one that reads work out of the worktree at
    /// `there` (as git spells its path): where a session of this process
    /// stands on that worktree, it takes its turn in that one's order as
    /// well ([`Self::write_into`]), so what was accepted there before it
    /// — a commit, a stash — is done before it reads. A tree nobody here
    /// writes to has no order to wait in.
    pub(super) fn write_reading<F, Fut>(
        self: &Arc<Self>,
        kind: OperationKind,
        after: AfterWrite,
        there: &str,
        task: F,
    ) -> Option<OperationId>
    where
        F: FnOnce(GitExecutor, RepoInfo, CancellationToken) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), GitError>> + Send + 'static,
    {
        let own = self.write_order();
        let orders: Vec<Arc<WriteOrder>> = write_order::on_workdir(there)
            .filter(|order| !own.iter().any(|held| Arc::ptr_eq(held, order)))
            .into_iter()
            .collect();
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
        // the queue in order — but for an unasked fetch, which takes no
        // place (`Line::next`) — and each request waits for its own
        // place, so a pair placed in the other order than queued leaves the
        // loop waiting, for good, on a place held by a request only it can
        // run.
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
            self.relay_screen_soon();
        }
        Some(operation.id)
    }

    /// The write this session's queue is serving right now, the reads
    /// behind its burst included — `None` between bursts. What a task under the
    /// queue reads to speak about itself (`note_landing`).
    pub(super) fn running_write(&self) -> Option<Operation> {
        *relock(&self.write_running)
    }

    /// The write being run in this worktree, by this session or
    /// another sharing it — what the poll's gate reads
    /// ([`RepoSession::refresh_poll`]). Gating on this session's write
    /// alone would let a reopened tab read between another session's
    /// rebase steps and offer to abort a rebase still being done.
    ///
    /// A push and the fetches take no place in the tree's order
    /// ([`OperationKind::writes_here`]), so this session's own write still
    /// says one is out.
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

    /// Runs queued writes one at a time, in submission order — except an
    /// unasked fetch, which yields to a write somebody asked for queued
    /// behind it ([`Line::next`]).
    ///
    /// A close stops intake only: the running write and the queued tail
    /// still run (the quit gate holds the window for them —
    /// `local_writes`), the network-paced ones dying at once on the
    /// cancelled token. `biased`, so a landed close enters drain mode
    /// deterministically.
    pub(super) async fn write_loop(
        self: Arc<Self>,
        queue: tokio::sync::mpsc::UnboundedReceiver<WriteRequest>,
    ) {
        let mut line = Line::new(queue);
        loop {
            let request = tokio::select! {
                biased;
                _ = self.root_cancel.cancelled() => break,
                request = line.next() => match request {
                    Some(request) => request,
                    None => return,
                },
            };
            self.serve(request, &mut line).await;
        }
        line.close();
        while let Some(request) = line.next().await {
            self.serve(request, &mut line).await;
        }
    }

    /// Serves `request` and the writes that join its burst behind it
    /// ([`Self::next_of_the_burst`]), then makes the reads they left, once
    /// ([`Self::settle`]).
    async fn serve(self: &Arc<Self>, request: WriteRequest, line: &mut Line) {
        let mut burst = Burst::default();
        let mut request = request;
        loop {
            // Held from the burst's first write to the last read behind it,
            // with no gap between two of its writes, so the poll keeps out
            // until the burst's own reads have landed — except under
            // replaying writes, which the poll reads through to count them
            // out (the gate reads the kind held here).
            *relock(&self.write_running) = Some(request.operation);
            self.run_write(request, &mut burst).await;
            let Some(next) = self.next_of_the_burst(&burst, line) else {
                break;
            };
            // The tree's turn goes to `next` here, unread: its place is the
            // one behind (`Burst::takes`).
            if let Some(released) = burst.give_back() {
                self.tell_the_others(released);
            }
            request = next;
        }
        self.settle(burst).await;
    }

    /// The write the queue starts without reading behind `burst`, where one
    /// is waiting that the burst takes ([`Burst::takes`]). One it does not take
    /// stays first in line.
    fn next_of_the_burst(&self, burst: &Burst, line: &mut Line) -> Option<WriteRequest> {
        // A closed session reads nothing behind its writes: each settles
        // as it answers, and the tree's next holder is told of each.
        if self.root_cancel.is_cancelled() {
            return None;
        }
        let next = line.asked_now()?;
        if burst.takes(&next) {
            return Some(next);
        }
        line.ahead = Some(next);
        None
    }

    /// Makes the reads `burst` left, once for all of its writes, and says each
    /// of them settled — every one behind reads that looked after it, with
    /// the reads that did not land named under each.
    async fn settle(self: &Arc<Self>, mut burst: Burst) {
        // A session closed mid-write reads nothing behind it: the reads
        // would publish into a page that has gone, through a retired sink
        // (`Hub::release_tab`). The next session holding the tree reads
        // what the write left (`session::write_order`).
        let failed = if self.root_cancel.is_cancelled() || burst.owed == ReadsOwed::default() {
            Vec::new()
        } else {
            let (failed, walked_from) = self.settle_after(&burst).await;
            // A landed delete whose walk did not land stands on in the
            // graph until one does (`session::leaving`).
            if burst.owed.walk && failed.contains(&FollowUp::Graph) {
                for write in &burst.writes {
                    self.leaving_unwalked(write.id, walked_from);
                }
            }
            failed
        };
        // The reads are made: one fenced off from here on reads again for
        // itself (`RepoSession::read_status_again`).
        self.unread_tree.store(false, Ordering::SeqCst);
        self.unread_refs.store(false, Ordering::SeqCst);
        for write in &burst.writes {
            self.sink.event(SessionEvent::WriteSettled {
                id: write.id,
                kind: write.kind,
                failed: failed.clone(),
            });
        }
        // The last write's places go back here; the tree is free from here
        // on.
        let released = burst.give_back();
        // Landed, a delete is drawn by the walk behind its burst by now;
        // refused, it was put back at the answer (`session::leaving`).
        for write in &burst.writes {
            self.leaving_done(write.id);
        }
        *relock(&self.write_running) = None;
        if let Some(released) = released {
            self.tell_the_others(released);
        }
        // Nobody tells this session, so it asks itself: a read refused
        // while the burst was out has nowhere else to be taken
        // ([`RepoSession::take_the_read_owed`]). Any lane — a push owes it
        // as a commit does.
        self.take_the_read_owed();
    }

    /// Tells the sessions that kept out of the trees a write was in, its
    /// places given back: this worktree's by kind — the quit gate let go of
    /// the write at git's answer (`git_answered`) —, and every session on
    /// the other worktrees it put work into, this one being on none.
    fn tell_the_others(self: &Arc<Self>, released: Released) {
        if released.operation.kind.writes_here() {
            self.tell_the_tree();
        }
        for order in released.into {
            for reader in order.others(self) {
                reader.read_again();
            }
        }
    }

    /// git has answered a write, or it never ran: the quit gate stops
    /// counting it (`local_writes`, by lane). The reads behind it are left
    /// out on purpose — a close stops them, so a quit has nothing of theirs
    /// to wait for, and a walk gone silent would hold the window for good.
    fn git_answered(&self, lane: Lane) {
        if lane == Lane::Local {
            self.local_writes.fetch_sub(1, Ordering::SeqCst);
        }
    }

    /// Runs one write to git's answer and puts it on `burst`, with what it
    /// left to be read and its places still held.
    async fn run_write(self: &Arc<Self>, request: WriteRequest, burst: &mut Burst) {
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
        // other worktree it puts work into. The places are held until the
        // burst is read behind, or the next write of the burst takes its turn.
        for place in place.iter().chain(&into) {
            place.granted(operation).await;
        }
        let held = Held {
            operation,
            place,
            into,
        };
        let Some(info) = self.repo_info() else {
            // The id went out at acceptance, so the boundaries are owed —
            // a caller holding it waits for them. Every entry point sits
            // behind an open repository, so reaching here is a bug.
            tracing::debug!(?operation, "write refused: no repository is open");
            self.git_answered(lane);
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
            burst.answered(held, None, false);
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
        self.git_answered(lane);
        // Only a network write is cancelled (a local token is nobody's to
        // cancel): the session is closing, and nothing is read behind it.
        let cancelled = result.as_ref().is_err_and(GitError::is_cancelled);
        if !cancelled {
            self.note_unread(after);
        }
        // From here only reads begun after the write speak for the
        // repository; a poll begun under it lands stale (`Standing::fence`).
        // The fence's numbers travel with the write's answer, for a
        // consumer landing on what the write left.
        let fence = self.standing.fence();
        // The boundaries balance whatever the answer — the UI counts
        // Started/Finished, and an unmatched start pins the count. The
        // typed report travels beside git's words: the screen makes a
        // report of the one and logs the other.
        let (error, report) = match &result {
            Ok(()) => (None, None),
            Err(error) if cancelled => (Some(error.to_string()), None),
            Err(error) => (Some(error.to_string()), error.report().cloned()),
        };
        self.sink.event(SessionEvent::WriteFinished {
            id,
            kind,
            error,
            report,
            head_seq: fence.head_seq,
            reads_from: fence.reads_from,
        });
        if cancelled {
            burst.answered(held, None, false);
            return;
        }
        let landed = result.is_ok();
        let walk = match result {
            // The rest walk where a read behind them moved (`settle_after`)
            // — or where they took rows off at the press: the reads behind
            // a removed worktree can find nothing moved, its holds having
            // gone at the press (`RepoSession::note_worktrees_going`), yet
            // the record still holds its commits (`Leaving::unwalked`).
            Ok(()) => {
                matches!(after, AfterWrite::Graph | AfterWrite::Name { .. }) || self.takes_rows(id)
            }
            Err(error) => {
                tracing::warn!(?operation, %error, "write failed");
                // A refused delete's commits come back with its rows, before
                // anything is read (`session::leaving`).
                self.leaving_refused(id);
                // A half-finished command still changed the repository
                // (conflicted merge, interrupted rebase, partial apply),
                // but it did not move history the way it meant to.
                false
            }
        };
        // Behind the answer, ahead of every read: a write that committed the
        // index tells the status it left (`session::foretold`).
        let moved_rows = ReadsOwed::of(after).tree == TreeRead::Always;
        let told = self.tell_what_it_left(id, landed, moved_rows, &fence);
        self.forget_what_it_reached(after);
        burst.answered(held, Some((after, walk)), told);
    }

    /// Says which reads the queue is sure to make behind a write that
    /// invalidated `after` (`RepoSession::unread_tree`) — ahead of the
    /// write's fence, which is what sends a read to ask.
    fn note_unread(&self, after: AfterWrite) {
        let left = ReadsOwed::of(after);
        if left.tree == TreeRead::Always {
            self.unread_tree.store(true, Ordering::SeqCst);
        }
        if left.refs {
            self.unread_refs.store(true, Ordering::SeqCst);
        }
    }

    /// Drops what is read once and kept where a write that invalidated
    /// `after` can have moved it — at its answer, not with the reads, which
    /// a burst makes later. A write reaching the tree or the history can
    /// bring a new `.gitattributes` or a remote; a config write the remotes
    /// (its file's stamp drops the rest — `forget_what_the_config_decides`).
    /// A fetch, a delete and a worktree taken away reach neither.
    fn forget_what_it_reached(&self, after: AfterWrite) {
        match after {
            AfterWrite::Tree | AfterWrite::Snapshots | AfterWrite::Graph => {
                self.forget_derived();
            }
            AfterWrite::Config => self.remotes.forget(),
            AfterWrite::Refs | AfterWrite::Worktrees | AfterWrite::Name { .. } => {}
        }
    }

    /// The reads behind the writes of a burst, waited for to the last — but a
    /// walk gone silent, which answers as not landed and walks on
    /// ([`RefreshOutcome::Silent`]). Answers those that did not land, for
    /// the caller to name under each write's id (the failures themselves
    /// are on the error surface under no id), and the number from which a
    /// walk asked for began after every write of the burst.
    async fn settle_after(self: &Arc<Self>, burst: &Burst) -> (Vec<FollowUp>, u64) {
        let owed = burst.owed;
        let Behind {
            tree,
            refs,
            stashes,
            listed,
            ahead,
        } = self.read_behind(burst).await;
        let mut failed = Vec::new();
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
        let (walk, walked_from) = match ahead {
            // The walk ahead answered for the refs, the listings and the
            // tree as it was told at the answer. What is left to walk for is
            // a tree the read found otherwise: the worktree row turned
            // against what was told. Of the two walks the later then says
            // whether the repository reached the screen —
            // `RepoSession::graph_answer`'s decision for a walk another
            // takes over, held to two walked one after the other. With no
            // second one the first's answer is the write's.
            Some(ahead) => {
                let again = tree == Reread::Moved;
                if !again && !ahead.landed {
                    note_failed(&mut failed, FollowUp::Graph);
                }
                (again, ahead.from)
            }
            None => {
                // A stash moved by a write — or outside, beside it — moves
                // no ref the refs read lists: the listing is its only word.
                let moved = tree == Reread::Moved || refs_moved || stashes.moved || listed.walk;
                let walk = self.walk_owed(owed, moved, &mut failed).await;
                // Every walk asked for from here on began after the writes
                // landed.
                (walk, self.graph_passes.next_ask())
            }
        };
        if walk {
            if !self.walk_behind(stashes).await {
                note_failed(&mut failed, FollowUp::Graph);
            }
        } else if listed.lay {
            self.relay_screen();
        }
        // Head reach is asked here because a stash push, pop or drop moves
        // no ref (so the refs read will not ask), yet changes whether
        // anything else holds the tip. An index-only write, a fetch and a
        // config write change none of it.
        if owed.reach && !refs_moved {
            self.settle_head_reach();
        }
        (failed, walked_from)
    }

    /// The snapshot reads a burst is owed ([`ReadsOwed`]), every one of them
    /// landed or failed before this answers.
    ///
    /// Every read that decides the walk comes before it — the tree, the
    /// refs and the listings each can move a row — so one walk answers for
    /// the burst, over the repository as its writes left it. A listing read
    /// after the walk would make two: one with the old holders (a removed
    /// worktree still in them), one on the listing's news.
    ///
    /// An index-only write reads the tree alone: the refs are where they
    /// were, and theirs is the longest read (`AfterWrite::Tree`).
    ///
    /// A write that cannot reach index or tree reads the refs first and the
    /// tree only where they moved, the one thing a status could then report
    /// differently (`AfterWrite::Refs`) — in series on purpose: a join
    /// would run the read the series saves.
    ///
    /// A delete moves a ref every time, so it is told at acceptance whether
    /// the tree is owed a read, and reads none otherwise
    /// (`AfterWrite::Name`).
    ///
    /// The listings are read with the rest, and the stashes they list go to
    /// the walk, which reads none of its own (`walk_inputs`). They are
    /// awaited: `WriteSettled` is only worth sending once all have
    /// published, and a listing still out when a later delete answers lets
    /// that one's refusal put the first one's row back on screen — the app
    /// stands rows in under one write at a time (`ops::StandIn`), which is
    /// why no delete joins a burst (`OperationKind::joins_a_burst`).
    async fn read_behind(self: &Arc<Self>, burst: &Burst) -> Behind {
        let owed = burst.owed;
        if burst.told_ahead && owed.walk && owed.tree == TreeRead::Always {
            return self.read_behind_told(owed).await;
        }
        let listings = self.read_listings_owed(owed.listings);
        let (tree, refs, (stashes, listed)) = match owed.tree {
            TreeRead::WhereRefsMoved => {
                let ((tree, refs), listings) = tokio::join!(self.read_refs_then_tree(), listings);
                (tree, refs, listings)
            }
            tree => tokio::join!(
                async {
                    if tree == TreeRead::Always {
                        self.read_status().await
                    } else {
                        Reread::Same
                    }
                },
                self.read_refs_owed(owed.refs),
                listings,
            ),
        };
        Behind {
            tree,
            refs,
            stashes,
            listed,
            ahead: None,
        }
    }

    /// [`Self::read_behind`] for a burst whose tree is told already — a
    /// commit's, worked out at its answer (`session::foretold`, whose
    /// decision this is the other half of): the history is walked as soon
    /// as the refs and the listings are in, beside the status read and not
    /// behind it, so the commit's row does not wait for the longest read.
    ///
    /// The tree's word is in for that walk: it lays the worktree row from
    /// what was told (`Standing::wip_dirty`). A status read that finds the
    /// row otherwise answers `Reread::Moved`, and the caller walks again.
    ///
    /// Not where the refs read failed: HEAD is then where the record had it
    /// before the commit (`known_head_tip`) — the status read is the only
    /// other one that reports it — and a walk ahead would hang the worktree
    /// row on the commit before, which a status finding the row as told
    /// walks nothing more for. The walk waits for that read instead, as
    /// behind any write (`ahead` left `None`): one walk, laid on the HEAD
    /// the status read reported, rather than a second one behind a wrong
    /// graph drawn first.
    async fn read_behind_told(self: &Arc<Self>, owed: ReadsOwed) -> Behind {
        let walked = async {
            let (refs, (stashes, listed)) = tokio::join!(
                self.read_refs_owed(owed.refs),
                self.read_listings_owed(owed.listings),
            );
            if refs == Reread::Failed {
                return (refs, stashes, listed, None);
            }
            // Before the walk is asked for: it began after the writes
            // landed, as every one asked for from here on.
            let from = self.graph_passes.next_ask();
            let landed = self.walk_behind(stashes.clone()).await;
            (refs, stashes, listed, Some(Ahead { from, landed }))
        };
        let (tree, (refs, stashes, listed, ahead)) = tokio::join!(self.read_status(), walked);
        Behind {
            tree,
            refs,
            stashes,
            listed,
            ahead,
        }
    }

    /// The refs read where a burst is owed it.
    async fn read_refs_owed(self: &Arc<Self>, owed: bool) -> Reread {
        if owed {
            self.read_refs().await
        } else {
            Reread::Same
        }
    }

    /// The listings a burst is owed, with the stashes they list.
    async fn read_listings_owed(self: &Arc<Self>, listings: Listings) -> (StashRead, Listed) {
        match listings {
            Listings::None => Default::default(),
            Listings::Worktrees => (StashRead::default(), self.read_worktrees().await),
            Listings::Both => self.read_listings().await,
        }
    }

    /// Whether the history is walked behind a burst: its writes ask for it
    /// whatever the reads found (`ReadsOwed::walk`), or a read behind it
    /// `moved`.
    ///
    /// A fetch that left the refs where they were has nothing new to walk —
    /// unless the graph is behind: a read beside the fetch may have seen
    /// its refs move first, and walks them itself (waited for, so the write
    /// settles behind it), or a walk failed earlier and `STALE GRAPH`
    /// stands (`a_fetch_that_brings_nothing_down_walks_a_graph_left_behind`).
    async fn walk_owed(&self, owed: ReadsOwed, moved: bool, failed: &mut Vec<FollowUp>) -> bool {
        let mut walk = owed.walk || moved;
        if owed.behind_a_fetch && !walk {
            self.wait_for_graph_passes().await;
            walk = self.graph_stale.load(Ordering::SeqCst);
            // A walk gone silent beside it is not waited out, nor walked
            // over — that would stop it while the streak lasts: the graph
            // has not landed.
            if !walk && self.graph_silent.load(Ordering::SeqCst) {
                note_failed(failed, FollowUp::Graph);
            }
        }
        walk
    }

    /// Walks the history behind a burst, with the stashes its listing read
    /// where they still stand, and answers whether the walk landed.
    ///
    /// Off-screen rebuild: the old graph stays until the new one swaps in,
    /// or nothing repaints if unchanged. Awaited, or a later write or test
    /// barrier could overtake and cancel the refresh it follows.
    async fn walk_behind(self: &Arc<Self>, stashes: StashRead) -> bool {
        let reads = match self.workdir() {
            Some(workdir) => {
                self.pass_reads_listed(&workdir, &self.root_cancel, stashes)
                    .await
            }
            None => None,
        };
        self.settle_graph_with(reads).await.landed()
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

/// How long the queue goes from write to write without reading behind
/// them: once the first write of a burst answered this long ago, the reads
/// are made before the next write starts (user decision — ask before
/// changing). Judged between writes only: the queue makes its reads once a
/// write has answered, never under one.
#[mry::mry]
pub fn unread_ceiling() -> std::time::Duration {
    std::time::Duration::from_secs(30)
}

/// Whether a write of `next` joins a burst whose first write answered at
/// `since` — starts behind it with nothing read in between: its kind joins
/// bursts ([`OperationKind::joins_a_burst`]) and this one is short of the
/// ceiling ([`unread_ceiling`]).
fn joins(next: OperationKind, since: tokio::time::Instant) -> bool {
    next.joins_a_burst() && since.elapsed() < unread_ceiling()
}

/// Whether the tree is read behind a burst ([`ReadsOwed`]), least first.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
enum TreeRead {
    #[default]
    Never,
    /// Behind the refs, and only where they moved (`AfterWrite::Refs`).
    WhereRefsMoved,
    Always,
}

/// Which listings are read behind a burst ([`ReadsOwed`]), least first. The
/// stashes are never read without the worktrees: every write that asks for
/// the one asks for both.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
enum Listings {
    #[default]
    None,
    Worktrees,
    Both,
}

/// What a burst of answered writes left to be read: each write's
/// [`AfterWrite`] folded into one, so one pass of reads answers for all of
/// them — the most any of them asks of each read. A burst of one reads what
/// its write's `AfterWrite` says and nothing more. The default is nothing:
/// a write cancelled with the session, behind which nothing is read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ReadsOwed {
    tree: TreeRead,
    refs: bool,
    listings: Listings,
    /// The history is walked whatever the reads find: a write moved it, or
    /// took rows off at its press.
    walk: bool,
    /// A fetch is among the writes: where nothing asks for a walk, a graph
    /// left behind is still caught up (`settle_after`).
    behind_a_fetch: bool,
    /// Whether anything else holds HEAD's tip is asked again.
    reach: bool,
}

impl ReadsOwed {
    /// What one write that invalidated `after` leaves to be read on its
    /// own, the walk aside.
    fn of(after: AfterWrite) -> Self {
        let mut owed = Self::default();
        owed.add(after, false);
        owed
    }

    /// Adds what a write that invalidated `after` left; `walk` is whether
    /// it asks for the history whatever the reads find.
    fn add(&mut self, after: AfterWrite, walk: bool) {
        let (tree, refs, listings) = match after {
            AfterWrite::Tree => (TreeRead::Always, false, Listings::None),
            AfterWrite::Name { status: false } => (TreeRead::Never, true, Listings::None),
            AfterWrite::Name { status: true } | AfterWrite::Config => {
                (TreeRead::Always, true, Listings::None)
            }
            AfterWrite::Refs => (TreeRead::WhereRefsMoved, true, Listings::None),
            AfterWrite::Worktrees => (TreeRead::WhereRefsMoved, true, Listings::Worktrees),
            AfterWrite::Snapshots | AfterWrite::Graph => (TreeRead::Always, true, Listings::Both),
        };
        self.tree = self.tree.max(tree);
        self.refs |= refs;
        self.listings = self.listings.max(listings);
        self.walk |= walk;
        self.behind_a_fetch |= after == AfterWrite::Refs;
        self.reach |= !matches!(
            after,
            AfterWrite::Tree | AfterWrite::Refs | AfterWrite::Config
        );
    }
}

/// What the snapshot reads behind a burst found
/// (`RepoSession::read_behind`).
struct Behind {
    tree: Reread,
    refs: Reread,
    stashes: StashRead,
    listed: Listed,
    /// The walk made ahead of the tree's read, where the tree was told at
    /// the answer (`RepoSession::read_behind_told`).
    ahead: Option<Ahead>,
}

/// A walk made behind a burst before its tree was read.
struct Ahead {
    /// The number from which a walk asked for began after the burst's
    /// writes — this one's among them.
    from: u64,
    landed: bool,
}

/// A write git has answered, its turn in the trees it wrote still held
/// (`WriteRequest::place`, `WriteRequest::into`).
struct Held {
    operation: Operation,
    place: Option<write_order::Place>,
    into: Vec<write_order::Place>,
}

/// A write whose places have gone back, landed or refused: what the
/// sessions that kept out of its trees are told by
/// (`RepoSession::tell_the_others`).
struct Released {
    operation: Operation,
    /// The orders of the other worktrees it put work into.
    into: Vec<Arc<WriteOrder>>,
}

/// The writes git has answered since the reads were last made, in the
/// order they answered — what one pass of reads settles.
#[derive(Default)]
struct Burst {
    writes: Vec<Operation>,
    owed: ReadsOwed,
    /// When the first of them answered: since when the screen has been
    /// behind the repository.
    since: Option<tokio::time::Instant>,
    /// The last of them, its places held until the burst is read behind or
    /// the next write of the burst takes its turn.
    last: Option<Held>,
    /// The status the last of them left was told at its answer, worked out
    /// and not read (`session::foretold`): the walk behind the burst need
    /// not wait for the tree's read (`RepoSession::read_behind_told`). The
    /// last one's only — a write answering behind it moves the tree again,
    /// and what that one left is the reads' to say.
    told_ahead: bool,
}

impl Burst {
    /// git answered a write: it joins the burst with what it left to be
    /// read — nothing where nothing is read behind it — and whether the
    /// status it left was `told` at the answer.
    fn answered(&mut self, held: Held, left: Option<(AfterWrite, bool)>, told: bool) {
        self.since.get_or_insert_with(tokio::time::Instant::now);
        self.writes.push(held.operation);
        if let Some((after, walk)) = left {
            self.owed.add(after, walk);
        }
        self.last = Some(held);
        self.told_ahead = told;
    }

    /// Whether the queue starts `next` without reading behind this burst:
    /// its kind joins within the ceiling ([`joins`]), and its turn comes
    /// the moment the last write's places go back — in this tree and in
    /// every other it puts work into. Behind a write of another session it
    /// would wait with this burst unread, and the reads made then would cross
    /// that session's write.
    fn takes(&self, next: &WriteRequest) -> bool {
        let Some(since) = self.since else {
            return false;
        };
        if !joins(next.operation.kind, since) {
            return false;
        }
        let held: Vec<&write_order::Place> = self
            .last
            .iter()
            .flat_map(|last| last.place.iter().chain(&last.into))
            .collect();
        next.place
            .iter()
            .chain(&next.into)
            .all(|place| place.next_behind(&held))
    }

    /// Gives the last write's places back, answering what to tell the
    /// sessions that kept out of its trees.
    fn give_back(&mut self) -> Option<Released> {
        let Held {
            operation,
            place,
            into,
        } = self.last.take()?;
        let orders = into.iter().map(write_order::Place::order).collect();
        drop(place);
        drop(into);
        Some(Released {
            operation,
            into: orders,
        })
    }
}

/// Names a read that did not land, once: the refs can be asked for twice
/// behind one burst (by a write and by the worktree listing's joins).
fn note_failed(failed: &mut Vec<FollowUp>, read: FollowUp) {
    if !failed.contains(&read) {
        failed.push(read);
    }
}

/// The requests waiting to be served: the queue, the unasked fetches passed
/// over in it, and the write taken off it that a burst did not go on to.
struct Line {
    queue: tokio::sync::mpsc::UnboundedReceiver<WriteRequest>,
    /// The unasked fetches held back, oldest first.
    held: std::collections::VecDeque<WriteRequest>,
    /// A write somebody asked for, taken off the queue to see whether the
    /// burst takes it ([`Self::asked_now`]) and left for the reads: the next
    /// to be served.
    ahead: Option<WriteRequest>,
}

impl Line {
    fn new(queue: tokio::sync::mpsc::UnboundedReceiver<WriteRequest>) -> Self {
        Self {
            queue,
            held: std::collections::VecDeque::new(),
            ahead: None,
        }
    }

    /// Stops intake; what is queued is still served.
    fn close(&mut self) {
        self.queue.close();
    }

    /// The next request to serve: the first write somebody asked for that
    /// is queued, else the oldest unasked fetch held back, else whatever
    /// comes next. `None` once the queue is closed and empty.
    ///
    /// An unasked fetch — fired by a chance tick, then a run ask that
    /// yields (`session::asks`) — is passed by later writes only while it
    /// waits: one that has started is not stopped. It takes no place in the
    /// worktree's order ([`OperationKind::writes_here`]), so serving it
    /// later reorders nothing the order guards; the writes somebody asked
    /// for keep theirs.
    ///
    /// A fetch that wakes an idle loop is held back too and the queue
    /// drained once more: a write queued right behind it, before this task
    /// runs again, still goes first.
    async fn next(&mut self) -> Option<WriteRequest> {
        loop {
            if let Some(request) = self.asked_now() {
                return Some(request);
            }
            if let Some(request) = self.held.pop_front() {
                return Some(request);
            }
            match self.queue.recv().await {
                Some(request) if !request.operation.kind.asked_for() => {
                    self.held.push_back(request);
                }
                other => return other,
            }
        }
    }

    /// The first write somebody asked for that is waiting right now, the
    /// unasked fetches in front of it held back — `None` where none is,
    /// without waiting for one.
    fn asked_now(&mut self) -> Option<WriteRequest> {
        if let Some(request) = self.ahead.take() {
            return Some(request);
        }
        loop {
            match self.queue.try_recv() {
                Ok(request) if !request.operation.kind.asked_for() => {
                    self.held.push_back(request);
                }
                Ok(request) => return Some(request),
                Err(_) => return None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn left(after: AfterWrite) -> ReadsOwed {
        ReadsOwed::of(after)
    }

    /// A burst of one reads exactly what its write's `AfterWrite` names.
    #[test]
    fn one_write_owes_the_reads_its_after_write_names() {
        let tree = left(AfterWrite::Tree);
        assert_eq!(
            (tree.tree, tree.refs, tree.listings, tree.reach),
            (TreeRead::Always, false, Listings::None, false),
            "an index-only write reads the tree alone"
        );
        let name = left(AfterWrite::Name { status: false });
        assert_eq!(
            (name.tree, name.refs, name.listings, name.reach),
            (TreeRead::Never, true, Listings::None, true)
        );
        for after in [AfterWrite::Name { status: true }, AfterWrite::Config] {
            let owed = left(after);
            assert_eq!(
                (owed.tree, owed.refs, owed.listings),
                (TreeRead::Always, true, Listings::None),
                "{after:?}"
            );
        }
        let fetch = left(AfterWrite::Refs);
        assert_eq!(
            (fetch.tree, fetch.refs, fetch.listings, fetch.behind_a_fetch),
            (TreeRead::WhereRefsMoved, true, Listings::None, true)
        );
        assert!(!fetch.reach && !left(AfterWrite::Config).reach);
        let worktrees = left(AfterWrite::Worktrees);
        assert_eq!(
            (worktrees.tree, worktrees.refs, worktrees.listings),
            (TreeRead::WhereRefsMoved, true, Listings::Worktrees)
        );
        for after in [AfterWrite::Snapshots, AfterWrite::Graph] {
            let owed = left(after);
            assert_eq!(
                (owed.tree, owed.refs, owed.listings, owed.reach),
                (TreeRead::Always, true, Listings::Both, true),
                "{after:?}"
            );
        }
        assert_ne!(
            left(AfterWrite::Tree),
            ReadsOwed::default(),
            "every write leaves something to read"
        );
    }

    /// A burst owes the most any of its writes asks of each read, in whatever
    /// order they answered.
    #[test]
    fn a_burst_owes_the_most_any_of_its_writes_asks() {
        let mut owed = ReadsOwed::default();
        owed.add(AfterWrite::Graph, true);
        owed.add(AfterWrite::Tree, false);
        let mut reversed = ReadsOwed::default();
        reversed.add(AfterWrite::Tree, false);
        reversed.add(AfterWrite::Graph, true);
        assert_eq!(owed, reversed);
        assert_eq!(
            (owed.tree, owed.refs, owed.listings, owed.walk, owed.reach),
            (TreeRead::Always, true, Listings::Both, true, true),
            "the stage behind a commit takes nothing off what the commit left"
        );

        let mut fetched = ReadsOwed::default();
        fetched.add(AfterWrite::Refs, false);
        fetched.add(AfterWrite::Tree, false);
        assert_eq!(
            (fetched.tree, fetched.refs, fetched.behind_a_fetch),
            (TreeRead::Always, true, true),
            "the stage behind a fetch has the tree read whether or not the refs moved"
        );
    }

    /// A request of `kind` that takes no place in any order, as the queue
    /// holds one.
    fn waiting(kind: OperationKind) -> WriteRequest {
        WriteRequest {
            operation: Operation::new(kind, AfterWrite::Tree),
            place: None,
            into: Vec::new(),
            run: Box::new(|_, _, _| Box::pin(async { Ok(()) })),
        }
    }

    /// The tree's next turn has to be the write's own in every order it has
    /// a place in — its own tree's and each it puts work into: behind
    /// another session's write it would wait with the burst unread.
    #[test]
    fn a_burst_takes_a_write_only_with_its_turn_next_in_every_order() {
        let here = write_order::of(Path::new("/write-burst-test/here/.git"));
        let there = write_order::of(Path::new("/write-burst-test/there/.git"));
        let (answered, _) = write_order::take_places(Some(&here), &[]);
        let mut burst = Burst::default();
        burst.answered(
            Held {
                operation: Operation::new(OperationKind::Commit, AfterWrite::Graph),
                place: answered,
                into: Vec::new(),
            },
            Some((AfterWrite::Graph, true)),
            false,
        );

        let (another_sessions, _) = write_order::take_places(Some(&there), &[]);
        let (place, into) = write_order::take_places(Some(&here), std::slice::from_ref(&there));
        let next = WriteRequest {
            place,
            into,
            ..waiting(OperationKind::Stage)
        };
        assert!(
            !burst.takes(&next),
            "a write of another session is in front of it in the tree it puts work into"
        );
        drop(another_sessions);
        assert!(
            burst.takes(&next),
            "next in both orders once that place went back"
        );
    }

    /// A write of `kind` git has answered, holding no place.
    fn answered(kind: OperationKind) -> Held {
        Held {
            operation: Operation::new(kind, AfterWrite::Tree),
            place: None,
            into: Vec::new(),
        }
    }

    /// The walk goes ahead of the tree's read only behind the write whose
    /// status was told: a write answering behind it has moved the tree
    /// again, and the burst waits for the read as any does.
    #[test]
    fn a_write_behind_the_one_told_ahead_has_the_burst_wait_for_the_tree() {
        let mut burst = Burst::default();
        burst.answered(
            answered(OperationKind::Commit),
            Some((AfterWrite::Graph, true)),
            true,
        );
        assert!(burst.told_ahead);
        burst.answered(
            answered(OperationKind::Stage),
            Some((AfterWrite::Tree, false)),
            false,
        );
        assert!(!burst.told_ahead);
    }

    /// On a paused clock, so both edges are judged: a burst takes a stage one
    /// instant short of the ceiling and none once it is reached — counted
    /// from its first write's answer, whatever answered in it since. Counted
    /// from the last, a steady row of presses would never be read behind.
    #[tokio::test(start_paused = true)]
    async fn a_burst_is_joined_until_the_ceiling_from_its_first_answer() {
        let an_instant = std::time::Duration::from_millis(1);
        let half = unread_ceiling() / 2;
        let mut burst = Burst::default();
        assert!(
            !burst.takes(&waiting(OperationKind::Stage)),
            "a burst nothing has answered in takes nothing"
        );
        burst.answered(
            answered(OperationKind::Commit),
            Some((AfterWrite::Graph, true)),
            false,
        );
        tokio::time::advance(half).await;
        burst.answered(
            answered(OperationKind::Stage),
            Some((AfterWrite::Tree, false)),
            false,
        );

        tokio::time::advance(unread_ceiling() - half - an_instant).await;
        assert!(
            burst.takes(&waiting(OperationKind::Stage)),
            "read behind before the ceiling was reached"
        );
        assert!(
            !burst.takes(&waiting(OperationKind::Commit)),
            "a kind that joins no burst is read in front of whatever the clock says"
        );
        tokio::time::advance(an_instant).await;
        assert!(
            !burst.takes(&waiting(OperationKind::Stage)),
            "joined a whole ceiling after the burst's first answer"
        );
    }
}
