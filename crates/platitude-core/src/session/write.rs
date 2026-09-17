//! The session write queue: one request at a time, in submission order,
//! each followed by the refreshes it invalidated.
//!
//! **Three boundaries, one id.** A request is *accepted* the moment
//! [`RepoSession::write`] hands its [`OperationId`] back — synchronously,
//! so the queue order is the order the UI asked in; a closed session
//! hands back nothing. git's own
//! answer is [`SessionEvent::WriteFinished`], sent the moment the command
//! ends and before anything it invalidated has been read again. The reads
//! that follow — the tree, the refs, the graph where either moved, the
//! stash and worktree listings and whatever those ask for, the author
//! configuration after an identity write — are *settled* when
//! [`SessionEvent::WriteSettled`] goes out, naming the reads that did not
//! land, and only then does the queue take the next request. Every one of
//! those events carries the id the acceptance returned, as does every git
//! command spawned under the request, so a consumer waiting on its own
//! write matches the id and infers nothing from the order answers arrive
//! in (`crate::operation`).
//!
//! **One queue per session, one order per working tree.** A session
//! closed mid-write keeps that write running, and a tab reopened over it
//! opens a second session on the same index. Acceptance therefore takes
//! the write a place in the tree's order as well as an id
//! (`session::write_order`), and the queue runs a request only once that
//! place comes up — so what the tree does is the order the asks were made
//! in, across sessions and not only within one:
//!
//! * a write accepted by the closed session runs before anything the new
//!   one accepts, however long it has to be waited out;
//! * a write accepted by the new session runs after every one already
//!   accepted, and no local write of either is ever lost or killed;
//! * the reads a poll takes stay out of the tree for the whole of any
//!   write on it, this session's or another's
//!   ([`RepoSession::tree_write`]), and are taken again the moment that
//!   write lands ([`RepoSession::tell_the_tree`]) — as a read the
//!   session owes, so one asked for while it was already reading, or
//!   while a write of its own was out, is served when that read or
//!   that write ends;
//! * **the reads an opening makes go straight out** — a tab has to
//!   paint, and holding them would leave it on "loading" for as long as
//!   somebody else's rebase takes. They can therefore read a tree
//!   mid-write; what makes that safe is the line above, since the new
//!   session is on the tree's list from the moment it opens
//!   (`join_write_order`, before the `Opened` event). So the last
//!   reading any session holds is always one taken after the last write
//!   on that tree;
//! * the reads *behind* a write are the writing session's own, and a
//!   closed session makes none — its page is gone, and the session that
//!   holds the tree next is the one that reads what it left.

use super::*;

impl RepoSession {
    // --- writes ---------------------------------------------------------

    /// Accepts one write for the queue: what it is, what it invalidates,
    /// and the task that runs it. Answers with the id every event about
    /// the write will carry — the acceptance boundary — or with `None`
    /// where the queue took nothing: the session is closed, its loop has
    /// ended, and an id handed out now would name a write nothing will
    /// ever answer.
    ///
    /// The lane is the kind's ([`OperationKind::lane`]); nothing here
    /// decides it.
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
        // The place in the working tree's order is taken here, inside the
        // call that hands the id back, so two sessions on one tree are
        // ordered by when each accepted: a tab opened over a write still
        // running cannot step in front of it, and neither can the writes
        // it goes on to accept (`session::write_order`). What takes one
        // is what changes this copy ([`OperationKind::writes_here`]) —
        // the question the lane does not answer, because a composite
        // delete is paced by the network and still takes a ref away here
        // before it gets there.
        //
        // **Numbered, placed and queued under one lock.** The loop serves
        // this queue in order and every request waits for its own place,
        // so a pair accepted the other way round would leave the loop
        // holding a place behind a request in its own queue — waiting on
        // the write it is itself the only one that can run, for good. Two
        // callers asking at once is all that would take. The id is taken
        // here for the same reason: numbered outside, two requests
        // racing can be numbered the other way round from the order the
        // queue took them, and a log read across two sessions on one
        // tree would say the wrong one went first. **Ids answer to
        // equality alone** ([`OperationId`]), and every reader here
        // matches its own.
        let (operation, local, sent) = {
            let _accepting = relock(&self.accepting);
            let operation = Operation::new(kind, after);
            let local = operation.lane == Lane::Local;
            let place = kind
                .writes_here()
                .then(|| self.write_order())
                .flatten()
                .map(|order| order.take_place());
            // Counted before it is sent, so the count can never trail the
            // queue: the loop's decrement pairs with exactly one increment.
            if local {
                self.local_writes.fetch_add(1, Ordering::SeqCst);
            }
            // Enqueueing is synchronous, so the queue order is the order
            // the UI asked in. Sending only fails once the loop has ended.
            let sent = self.write_tx.send(WriteRequest {
                operation,
                place,
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
            tracing::debug!(?operation, "write not accepted: the session is closed");
            return None;
        }
        Some(operation.id)
    }

    /// The write **this session's** queue is serving right now, refreshes
    /// included — `None` between requests. What a task running under the
    /// queue reads to speak about itself (`note_landing`).
    pub(super) fn running_write(&self) -> Option<Operation> {
        *relock(&self.write_running)
    }

    /// The write being run in this **working tree**, by this session or
    /// by another sharing it — what the poll's gate reads
    /// ([`RepoSession::refresh_poll`]).
    ///
    /// A tab reopened over a close's tail is a different session looking
    /// at the same index, and a status read taken between a rebase's
    /// steps says the tree is mid-rebase — a banner offering to abort
    /// something that is still being done. The tree's own answer is the
    /// one to gate on (`session::write_order`).
    ///
    /// The network lanes take no place in that order, so this session's
    /// own write is still what says a push or a fetch is out.
    pub(super) fn tree_write(&self) -> Option<Operation> {
        match self.write_order() {
            Some(order) => order.running().or_else(|| self.running_write()),
            None => self.running_write(),
        }
    }

    /// Tells every other session on this working tree to read it again:
    /// a write it did not make has just landed there, and its own polls
    /// have been keeping out of the tree for as long as that write held
    /// the front ([`Self::tree_write`]).
    ///
    /// Called once the place is back — a session told to look while the
    /// tree still says it is being written skips the tick it was woken
    /// for.
    ///
    /// What is asked for is a read the session **owes**
    /// ([`RepoSession::read_again`]): the read it is already making
    /// began before this write, so a refusal that went nowhere would
    /// leave it showing a repository that is gone.
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
    /// A close reaches this loop between requests: the
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

    /// One request, served whole: the write held up for the poll to read
    /// around it, the run, and the count the quit gate reads given back.
    async fn serve(self: &Arc<Self>, request: WriteRequest) {
        let operation = request.operation;
        // Held around the whole request, refreshes included, so the poll
        // keeps out until the write's own refresh has landed — except
        // under the writes that replay, which the poll is allowed
        // through so the screen can count them out; the gate reads the
        // kind off what is held here.
        *relock(&self.write_running) = Some(operation);
        // The request carries its place in the working tree's order and
        // gives it back by ending, so the tree is free from here on.
        self.run_write(request).await;
        *relock(&self.write_running) = None;
        // Two questions, two answers: the quit gate counts the writes a
        // close waits out (the lane), and the tree is told about the
        // writes that changed it (the kind).
        if operation.lane == Lane::Local {
            self.local_writes.fetch_sub(1, Ordering::SeqCst);
        }
        if operation.kind.writes_here() {
            self.tell_the_tree();
        }
        // And this session is told by nobody, so it asks itself: a read it
        // was refused while this write was out has no other place left to
        // be taken from ([`RepoSession::take_the_read_owed`]). Whatever
        // the lane — a push owes the same read a commit does.
        self.take_the_read_owed();
    }

    async fn run_write(self: &Arc<Self>, request: WriteRequest) {
        let WriteRequest {
            operation,
            place,
            run,
        } = request;
        let Operation {
            id,
            kind,
            lane,
            after,
        } = operation;
        // The working tree's turn, before anything is said about the
        // write: `WriteStarted` means git is running it, and a write held
        // behind one another session asked for first is not running yet
        // (`session::write_order`). The place is held to the end of this
        // function — the reads behind the write included, which is the
        // same tail this session's own poll is kept out of.
        if let Some(place) = &place {
            place.granted(operation).await;
        }
        let Some(info) = self.repo_info() else {
            // The id went out at acceptance, so the boundaries are owed:
            // a caller holding it waits for them, and nothing else would
            // ever come. Reported as the refusal it is — the entry points
            // all sit behind an open repository today, so this says
            // something has gone wrong.
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
        // The write has ended, and what speaks for the repository from
        // here is a read begun after it: the reads below are the ones
        // that answer for what the write left, and a poll that began
        // under it lands stale (`Standing::fence`). The number the fence
        // answers travels with the write's answer, so a consumer landing
        // on "the repository as the write left it" arms on the report
        // the write is owed by name.
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
                matches!(after, AfterWrite::Graph | AfterWrite::Refs)
            }
            // Only the network lane can land here — a local write's token
            // is nobody's to cancel — and it means the session is closing.
            // The boundaries must still balance: the UI counts
            // Started/Finished to know whether a write is in flight, and
            // an unmatched start would pin that count for good. Nothing
            // is read behind a cancelled write, so nothing failed to.
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
                // A write that did not happen and has something to say
                // for itself travels beside git's words: the screen makes
                // a report out of the one and keeps the other for the log.
                let report = error.report().cloned();
                self.sink.event(SessionEvent::WriteFinished {
                    id,
                    kind,
                    error: Some(error.to_string()),
                    report,
                    head_seq: fence.head_seq,
                    reads_from: fence.reads_from,
                });
                // A half-finished command still changed the repository
                // (conflicted merge, interrupted rebase, partial apply),
                // but it did not move history the way it meant to.
                false
            }
        };

        // A session closed mid-write reads nothing behind it. The reads
        // are the expensive half — on a repository the size of the
        // budget's, the refs and the graph rebuild — and they would
        // publish into a page that has gone, through a sink that has been
        // retired (`Hub::release_tab`). What the write left is read by
        // whichever session holds the tree next, which is queued behind
        // this very place (`session::write_order`).
        let failed = if self.root_cancel.is_cancelled() {
            Vec::new()
        } else {
            self.settle_after(operation, rebuild_graph).await
        };
        self.sink
            .event(SessionEvent::WriteSettled { id, kind, failed });
    }

    /// The reads behind a write that has answered — the ones that put
    /// what it left on screen — waited for to the last, and answered
    /// with the ones that did not land. What did not land is collected as
    /// it goes and named under the write's id by the caller: the failures
    /// themselves are on the error surface already, under no id at all.
    async fn settle_after(
        self: &Arc<Self>,
        operation: Operation,
        rebuild_graph: bool,
    ) -> Vec<FollowUp> {
        let after = operation.after;
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
        // series is the point — a join would run the read the series
        // saves.
        let mut failed = Vec::new();
        let tree_only = after == AfterWrite::Tree;
        let (tree, refs) = match after {
            AfterWrite::Tree => (self.read_status().await, Reread::Same),
            AfterWrite::Refs => {
                let refs = self.read_refs().await;
                let tree = if refs == Reread::Moved {
                    self.read_status().await
                } else {
                    Reread::Same
                };
                (tree, refs)
            }
            AfterWrite::Snapshots | AfterWrite::Graph | AfterWrite::Author => {
                tokio::join!(self.read_status(), self.read_refs())
            }
        };
        if tree == Reread::Failed {
            note_failed(&mut failed, FollowUp::Status);
        }
        if refs == Reread::Failed {
            note_failed(&mut failed, FollowUp::Refs);
        }
        let refs_moved = refs == Reread::Moved;
        if rebuild_graph || tree == Reread::Moved || refs_moved {
            // Off-screen rebuild: the pane keeps showing the old graph
            // until the finished one swaps in (or nothing changed and
            // nothing repaints — the auto-fetch common case). The write
            // queue waits for this answer: otherwise a later write or
            // test barrier can overtake and cancel the very refresh it
            // is meant to follow.
            if !self.settle_graph().await.landed() {
                note_failed(&mut failed, FollowUp::Graph);
            }
        }
        // A stash push, pop or drop moves no ref, so the refs read has no
        // reason to ask again — and it is exactly what changes whether
        // something other than this branch still holds the tip.
        //
        // An index-only write changes none of the three: what is published
        // is a question about commits, a stash is made by a command that
        // says so, and a worktree is added or removed by another.
        //
        // The listings are read beside each other and waited for, the
        // reads a listing asks for included: the settled boundary below is
        // only worth sending once they have all published, and a listing
        // still in flight when the next write answers is what let a
        // refusal put a dropped stash back on screen (the application
        // stands rows in until a listing proves them gone).
        let mut listings = Vec::new();
        if !tree_only {
            if !refs_moved {
                self.settle_head_reach();
            }
            listings.extend(self.refresh_stashes().map(|task| (FollowUp::Stashes, task)));
            listings.extend(
                self.refresh_worktrees()
                    .map(|task| (FollowUp::Worktrees, task)),
            );
        }
        if after == AfterWrite::Author {
            listings.extend(self.refresh_author().map(|task| (FollowUp::Author, task)));
        }
        for (listing, task) in listings {
            match task.await {
                Ok(more) => {
                    for read in more {
                        note_failed(&mut failed, read);
                    }
                }
                // The task unwound: nothing it was to publish did.
                Err(error) => {
                    tracing::warn!(?operation, %error, "a listing read after the write did not finish");
                    note_failed(&mut failed, listing);
                }
            }
        }
        failed
    }
}

/// Names a read that did not land, once.
///
/// One read can be asked for twice behind a single write — the graph by
/// the write itself and again by what the worktree listing found — and
/// a consumer reading the list as "which of my reads are missing" would
/// otherwise see the same one twice.
fn note_failed(failed: &mut Vec<FollowUp>, read: FollowUp) {
    if !failed.contains(&read) {
        failed.push(read);
    }
}
