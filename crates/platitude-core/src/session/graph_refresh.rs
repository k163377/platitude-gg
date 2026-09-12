//! Background graph refresh entry points and their completion boundary.

use super::*;

/// What a background graph refresh established when it completed.
///
/// This is a completion boundary, not a timing estimate. Consumers that
/// need to act after a refresh can wait for [`RefreshTask::outcome`]
/// instead of guessing from a period of silence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshOutcome {
    /// The operation was not started because the repository is not open.
    Unavailable,
    /// The operation was not started because a write and its follow-up
    /// refreshes still own the repository.
    WriteBusy,
    /// The operation was not started because the previous poll still owns
    /// the single-flight slot.
    Busy,
    /// The operation completed and the graph already showed its answer.
    Unchanged,
    /// The operation completed and installed a different graph.
    Changed,
    /// A newer graph request superseded this one, or the session closed.
    Cancelled,
    /// Reading the graph failed.
    Failed,
}

impl RefreshOutcome {
    /// Whether the graph on screen now answers for the repository.
    ///
    /// **A handover is not an answer.** A pass another ask took over
    /// says nothing about the repository — the ask that took it over is
    /// the one that will — so a caller that has to know waits for that
    /// one first ([`RepoSession::graph_answer`]). By the time a
    /// `Cancelled` reaches such a caller it means nothing ever will: the
    /// session is closing. Every other outcome here leaves the picture
    /// as it was, which is what a write settling behind the rebuild
    /// reports under its own id ([`super::FollowUp::Graph`]).
    #[must_use]
    pub fn landed(self) -> bool {
        matches!(self, Self::Changed | Self::Unchanged)
    }
}

/// How many graph passes could still walk.
///
/// A pass registers here on the thread that is about to spawn it and
/// stays registered until its task has ended. **Cancelling a pass is an
/// ask, not an end**: the one a later request displaced is still on a
/// worker somewhere, and until it comes back it can start the walk it
/// was already on its way to. A caller that took the stream over and
/// then reads what the session did is reading a count the displaced pass
/// has not finished adding to — [`RepoSession::wait_for_graph_passes`]
/// is the boundary that closes it.
///
/// It also numbers the asks and keeps what each came to, which is the
/// other half of the same question: a pass that was taken over leaves
/// its caller waiting on whoever took it over, and that is the ask this
/// records ([`GraphRun`]).
pub(super) struct GraphPasses {
    live: std::sync::atomic::AtomicUsize,
    changed: tokio::sync::watch::Sender<u64>,
    /// Numbers the asks for the graph, in the order the stream changed
    /// hands ([`RepoSession::take_log_run`]): an ask that took a pass
    /// over is numbered after the pass it displaced, so "newer than
    /// mine" names exactly the passes that could answer in its place.
    asks: AtomicU64,
    /// The newest ask that answered *for the graph* rather than being
    /// taken over in its turn. What a displaced pass's caller waits for
    /// ([`RepoSession::graph_answer`]).
    landed: tokio::sync::watch::Sender<Option<(u64, RefreshOutcome)>>,
}

impl Default for GraphPasses {
    fn default() -> Self {
        let (changed, _) = tokio::sync::watch::channel(0);
        let (landed, _) = tokio::sync::watch::channel(None);
        Self {
            live: std::sync::atomic::AtomicUsize::new(0),
            changed,
            asks: AtomicU64::new(0),
            landed,
        }
    }
}

impl GraphPasses {
    /// Numbers one ask for the graph. Taken with the token that takes
    /// the stream over ([`RepoSession::take_log_run`]) and on the same
    /// thread, so the numbering is the order the asks were made.
    pub(super) fn ask(self: &Arc<Self>) -> GraphRun {
        let ask = self.asks.fetch_add(1, Ordering::SeqCst) + 1;
        GraphRun {
            passes: Arc::clone(self),
            ask,
            answered: false,
        }
    }

    /// Records what an ask came to.
    ///
    /// A pass that was taken over is not an answer for the graph — the
    /// ask that took it over is — so those are not recorded at all, and
    /// a caller waiting behind one goes on waiting for the ask that
    /// displaced it. Of the rest the newest stands: two passes can end
    /// in either order, and an older one's answer must not overwrite a
    /// newer one's.
    fn answered(&self, ask: u64, outcome: RefreshOutcome) {
        if outcome == RefreshOutcome::Cancelled {
            return;
        }
        self.landed.send_if_modified(|held| match held {
            Some((seen, _)) if *seen >= ask => false,
            _ => {
                *held = Some((ask, outcome));
                true
            }
        });
    }

    /// Waits for an ask at or after `after` to answer for the graph —
    /// the answer a caller whose own pass was taken over is owed.
    ///
    /// **This terminates because every ask answers.** Each one is
    /// numbered before its pass is spawned and reports its outcome from
    /// the task, or from [`GraphRun`]'s drop where the task never got
    /// there; a chain of handovers ends at the ask nobody took over.
    /// `None` where the session closed first — every pass under its
    /// token is cancelled and nothing new will be asked for, so there is
    /// no answer left to wait for.
    ///
    /// **`after` itself counts**, and only the drop guard can put it
    /// there: a pass that was taken over records nothing, so a record
    /// under the caller's own number means that pass ended without
    /// answering. Waiting past it would be waiting for a newer ask that
    /// nothing is going to make — and the caller is a write's settling,
    /// so the whole queue would wait with it.
    async fn landed_after(
        &self,
        after: u64,
        closing: &CancellationToken,
    ) -> Option<RefreshOutcome> {
        let mut changed = self.landed.subscribe();
        loop {
            let seen = *changed.borrow_and_update();
            if let Some((ask, outcome)) = seen
                && ask >= after
            {
                return Some(outcome);
            }
            tokio::select! {
                biased;
                () = closing.cancelled() => return None,
                answered = changed.changed() => answered.ok()?,
            }
        }
    }

    /// Registers one pass. **Called before the pass is spawned**, never
    /// from inside the spawned task: a task is not scheduled in the order
    /// the asks came in, and one that registered itself would leave the
    /// boundary reading "idle" for the pass that has not started yet
    /// (see [`RepoSession::take_log_run`], which is taken the same way
    /// and for the same reason).
    pub(super) fn enter(self: &Arc<Self>) -> GraphPassHeld {
        self.live.fetch_add(1, Ordering::SeqCst);
        GraphPassHeld(Arc::clone(self))
    }

    /// Waits until every pass registered when this was called has
    /// stopped. A later one may register again; this closes the work
    /// already in flight rather than reserving silence.
    async fn wait_idle(&self) {
        let mut changed = self.changed.subscribe();
        while self.live.load(Ordering::SeqCst) != 0 {
            if changed.changed().await.is_err() {
                return;
            }
        }
    }
}

/// What a pass carries for as long as it could still walk.
///
/// Dropped by the task itself, so a pass that unwound or went down with
/// the runtime still reports that it has stopped rather than leaving the
/// boundary waiting on it forever.
pub(super) struct GraphPassHeld(Arc<GraphPasses>);

impl Drop for GraphPassHeld {
    fn drop(&mut self) {
        self.0.live.fetch_sub(1, Ordering::SeqCst);
        self.0
            .changed
            .send_modify(|generation| *generation = generation.wrapping_add(1));
    }
}

/// One ask for the graph, from the moment it takes the stream over
/// ([`RepoSession::take_log_run`]) to the answer it leaves behind.
///
/// **Answered by the task that runs the pass, and by this drop where
/// the task never got there** — it unwound, or went down with the
/// runtime. Somebody may be waiting behind this ask for it to say what
/// became of the graph ([`RepoSession::graph_answer`]), and an ask that
/// left no answer would leave them waiting for a pass nothing will ever
/// run. [`super::PassWatch`] is the other guard on the same unwind and
/// speaks to the screen; this one speaks to the queue behind it.
pub(super) struct GraphRun {
    passes: Arc<GraphPasses>,
    ask: u64,
    answered: bool,
}

impl GraphRun {
    /// The number this ask is known by, for the boundary a caller
    /// follows ([`RefreshTask`]).
    pub(super) fn ask(&self) -> u64 {
        self.ask
    }

    /// What this ask came to. Called once, with the outcome the whole
    /// task settled on: a restart runs two passes under one ask, and it
    /// is the last of them that answers for the graph.
    pub(super) fn answer(&mut self, outcome: RefreshOutcome) {
        self.answered = true;
        self.passes.answered(self.ask, outcome);
    }
}

impl Drop for GraphRun {
    fn drop(&mut self) {
        if !self.answered {
            self.passes.answered(self.ask, RefreshOutcome::Failed);
        }
    }
}

/// Completion of one explicitly tracked background refresh.
pub struct RefreshTask {
    /// The ask this is the boundary of, where one was started — what
    /// [`RepoSession::graph_answer`] follows when the pass behind it was
    /// taken over. `None` for an answer that started no pass at all, and
    /// for the poll, whose boundary nobody follows: a tick another ask
    /// took over is that ask's to answer, not the tick's.
    ask: Option<u64>,
    answer: tokio::sync::oneshot::Receiver<RefreshOutcome>,
}

impl RefreshTask {
    fn pending(ask: Option<u64>) -> (tokio::sync::oneshot::Sender<RefreshOutcome>, Self) {
        let (send, answer) = tokio::sync::oneshot::channel();
        (send, Self { ask, answer })
    }

    fn ready(outcome: RefreshOutcome) -> Self {
        let (send, task) = Self::pending(None);
        if send.send(outcome).is_err() {
            tracing::trace!("refresh completion was not observed");
        }
        task
    }

    /// Waits for the operation itself to finish. A dropped runtime is the
    /// same observable result as cancellation: no later answer from this
    /// operation can arrive.
    pub async fn outcome(self) -> RefreshOutcome {
        self.answer.await.unwrap_or(RefreshOutcome::Cancelled)
    }
}

/// What one explicitly tracked remote-tag catch-up established.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteTagRefreshOutcome {
    /// Automatic fetching is off, so an unasked network read is forbidden.
    Disabled,
    /// Tags are outside the graph, so their badges are not worth a read.
    Hidden,
    /// Another remote-tag read already owns the single-flight slot, and
    /// this request was dropped rather than booked behind it.
    ///
    /// A [`super::ReadFlight`] runs a repeat because its second caller can
    /// know something the read in flight does not — a write that landed
    /// after it started.
    /// Nothing here does: no local operation moves what a remote carries
    /// under `refs/tags/` (a push is branches only), and the two places
    /// that ask (an opening, and installing the interval) ask the same
    /// question of the same remotes. A request arriving while the interval
    /// was off cannot see this either, since the permission it brings is
    /// what lets a read run at all. So the read that made this `Busy` is
    /// the answer, and it publishes one.
    ///
    /// **Said once that read has let the slot go.** The ask is dropped;
    /// its completion is not: it is booked behind the read in flight, so
    /// a caller waiting on it knows the answer it was told to read has
    /// been published, and an ask it makes next is not turned away by
    /// the same read.
    Busy,
    /// The repository is not open any more.
    Unavailable,
    /// The remotes answered with the readings already held.
    Unchanged,
    /// The remote-tag index moved and a refs repaint was requested.
    Changed,
    /// The session closed before the read could answer.
    Cancelled,
}

/// Completion boundary for one remote-tag catch-up request.
pub struct RemoteTagRefreshTask(tokio::sync::oneshot::Receiver<RemoteTagRefreshOutcome>);

impl RemoteTagRefreshTask {
    pub(super) fn pending() -> (tokio::sync::oneshot::Sender<RemoteTagRefreshOutcome>, Self) {
        let (send, receive) = tokio::sync::oneshot::channel();
        (send, Self(receive))
    }

    pub(super) fn ready(outcome: RemoteTagRefreshOutcome) -> Self {
        let (send, task) = Self::pending();
        if send.send(outcome).is_err() {
            tracing::trace!("remote-tag refresh completion was not observed");
        }
        task
    }

    /// Waits for the request itself to finish or decline to run.
    pub async fn outcome(self) -> RemoteTagRefreshOutcome {
        self.0.await.unwrap_or(RemoteTagRefreshOutcome::Cancelled)
    }
}

impl RepoSession {
    /// Waits until the refs and status readers already in flight have left
    /// their single flight.
    ///
    /// A snapshot event is delivered from inside the reader, before it can
    /// request the graph refresh that answer implies. Coordinating code uses
    /// this boundary after observing those events when that distinction
    /// matters.
    pub async fn wait_for_snapshot_reads(&self) {
        tokio::join!(self.refs_read.wait_idle(), self.status_read.wait_idle());
    }

    /// Waits until every graph pass that had been started when this was
    /// called has stopped — the one this caller displaced included.
    ///
    /// [`RefreshTask::outcome`] answers for the pass it was taken from
    /// and for no other. Asking for a rebuild also **cancels** whichever
    /// pass held the stream, and that pass stops when it next looks
    /// rather than when the ask was made (see [`GraphPasses`]).
    /// Coordinating code that counts what the session did takes this
    /// boundary as well.
    pub async fn wait_for_graph_passes(&self) {
        self.graph_passes.wait_idle().await;
    }

    /// Rebuilds the graph off-screen and swaps it in only when it differs
    /// from what the UI already shows (see [`RepoSession::run_swap_pass`]).
    ///
    /// Background triggers (auto fetch, a finished write, an external
    /// dirty/clean flip) go through here instead of [`RepoSession::restart_log`]:
    /// a reset-and-restream repaints the pane even when history did not
    /// move, which reads as idle flicker once a periodic fetch is on.
    pub fn refresh_log(self: &Arc<Self>) {
        drop(self.start_refresh_log());
    }

    /// Starts the same background rebuild as [`Self::refresh_log`] and
    /// returns its causal completion boundary.
    ///
    /// The ordinary UI path is fire-and-forget. Tests and coordinating
    /// callers use this form when the distinction between "still running"
    /// and "finished without an event" matters.
    pub fn refresh_log_tracked(self: &Arc<Self>) -> RefreshTask {
        self.start_refresh_log()
    }

    fn start_refresh_log(self: &Arc<Self>) -> RefreshTask {
        let Some(workdir) = self.workdir() else {
            return RefreshTask::ready(RefreshOutcome::Unavailable);
        };
        let (run_cancel, mut run) = self.take_log_run();
        let held = self.graph_passes.enter();
        let s = Arc::clone(self);
        let options = self.log_options();
        let (finished, task) = RefreshTask::pending(Some(run.ask()));
        self.runtime.spawn(async move {
            let _held = held;
            let outcome = s.run_swap_pass(&workdir, options, &run_cancel).await;
            run.answer(outcome);
            if finished.send(outcome).is_err() {
                tracing::trace!("refresh completion was not observed");
            }
        });
        task
    }

    /// Rebuilds the graph and waits for the answer the rebuild is owed,
    /// following a pass another ask took over to the ask that took it
    /// over ([`Self::graph_answer`]) — what a write's settling waits on.
    pub(super) async fn settle_graph(self: &Arc<Self>) -> RefreshOutcome {
        let task = self.refresh_log_tracked();
        self.graph_answer(task).await
    }

    /// The answer `task` is owed, which is not always its own.
    ///
    /// **A pass taken over answers for nothing.** Something asked for a
    /// newer graph while this one was walking, and that ask is the one
    /// that says whether the repository reached the screen — so a caller
    /// that has to know waits for it rather than reading the handover as
    /// an answer. Read as one, a write reports itself settled while the
    /// graph it asked for is still being walked, and calls the rebuild
    /// landed even where the ask that replaced it then failed (measured
    /// — `session_integration::operations`).
    ///
    /// The wait ends at the ask nobody took over, or at a session
    /// closing under all of them ([`GraphPasses::landed_after`]).
    pub(super) async fn graph_answer(&self, task: RefreshTask) -> RefreshOutcome {
        let ask = task.ask;
        let outcome = task.outcome().await;
        if outcome != RefreshOutcome::Cancelled {
            return outcome;
        }
        // Nothing was started, so nothing took it over either.
        let Some(ask) = ask else {
            return outcome;
        };
        self.graph_passes
            .landed_after(ask, &self.root_cancel)
            .await
            .unwrap_or(RefreshOutcome::Cancelled)
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
    ///
    /// **Any write of that working tree, not only this session's**
    /// ([`RepoSession::tree_write`]). A tab closed mid-write keeps the
    /// write running and a tab reopened over it is a new session on the
    /// same index, which would otherwise read the tree between a
    /// rebase's steps and publish it as where the repository stands. The
    /// session that finishes such a write calls this on the others, so a
    /// tick skipped for somebody else's write is taken the moment it
    /// lands rather than at the next tick of the clock.
    ///
    /// **Except under a write that replays**
    /// ([`OperationKind::replays_history`]).
    /// Those stand for as long as the range is deep — seconds, and past
    /// twenty on a window's worth of commits — and skipping the poll
    /// through all of it leaves the window saying nothing at all: no
    /// badge, no progress, no graph, for the whole of a rewrite the
    /// reader asked for. git counts the steps out on disk as it goes
    /// (`rebase-merge/msgnum`, read by [`crate::integrate::rebase_standing`]),
    /// so the tick that runs under one has an answer to publish. The
    /// snapshot reads coalesce with the write's own refresh
    /// ([`super::ReadFlight`]), and a tick that cannot keep up with the
    /// replay simply lands later — the picture is allowed to fall behind,
    /// but not to stop.
    pub fn refresh_poll(self: &Arc<Self>) {
        drop(self.start_refresh_poll());
    }

    /// Reads the repository again because a write landed in it — from
    /// this session or from another on the same working tree
    /// ([`RepoSession::tell_the_tree`]).
    ///
    /// **Owed, not offered.** A clock tick the single-flight slot turns
    /// away is no loss: another comes. This one is news, and the read
    /// holding the slot began before the write, so dropping it would
    /// leave the session showing a repository that no longer exists
    /// until something else happened to ask. Refused, it is written down
    /// and taken by the reader in front as it hands the slot back.
    pub(super) fn read_again(self: &Arc<Self>) {
        self.read_owed.store(true, Ordering::SeqCst);
        drop(self.start_refresh_poll());
    }

    /// Takes the read owed, if one is — called where a poll hands the
    /// slot back, which is the moment a refused one can be served.
    fn take_the_read_owed(self: &Arc<Self>) {
        if self.read_owed.load(Ordering::SeqCst) {
            drop(self.start_refresh_poll());
        }
    }

    /// Runs one poll tick and returns a boundary that includes any graph
    /// rebuild the tick requested.
    pub fn refresh_poll_tracked(self: &Arc<Self>) -> RefreshTask {
        self.start_refresh_poll()
    }

    fn start_refresh_poll(self: &Arc<Self>) -> RefreshTask {
        if self
            .tree_write()
            .is_some_and(|write| !write.kind.replays_history())
        {
            tracing::trace!("poll skipped: a write is running");
            return RefreshTask::ready(RefreshOutcome::WriteBusy);
        }
        let Ok(permit) = Arc::clone(&self.poll_slot).try_acquire_owned() else {
            tracing::trace!("poll skipped: the previous one has not finished");
            return RefreshTask::ready(RefreshOutcome::Busy);
        };
        // The reads below start from here, so they answer for anything
        // that landed before this moment — a write landing inside them
        // owes another, and sets this again ([`Self::read_again`]).
        self.read_owed.store(false, Ordering::SeqCst);
        let held = self.graph_passes.enter();
        let s = Arc::clone(self);
        // Nobody follows a tick that was taken over: a newer ask for the
        // graph is that ask's to answer, and the tick has no caller
        // waiting on the repository having reached the screen.
        let (finished, task) = RefreshTask::pending(None);
        self.runtime.spawn(async move {
            let _held = held;
            // The config stamp is settled before either read starts. The
            // refs half invalidates what the config decides on its way in,
            // and the status half consumes the line-ending staleness that
            // leaves behind — ordered by completion inside the join, the
            // status read can spend the mark before the refs read has set
            // it, and a withdrawn notice then stands until something else
            // happens to ask. Idempotent, so the refs read finding the
            // stamp already current costs a stat and nothing else.
            s.forget_what_the_config_decides();
            // Both reads can call for a rebuild, but the graph is one
            // picture: an external commit moves a ref *and* cleans the
            // tree, and walking twice would throw one pass away.
            let (refs, tree) = tokio::join!(s.read_refs(), s.read_status());
            let outcome = if refs == Reread::Moved || tree == Reread::Moved {
                let Some(workdir) = s.workdir() else {
                    drop(permit);
                    if finished.send(RefreshOutcome::Cancelled).is_err() {
                        tracing::trace!("poll completion was not observed");
                    }
                    s.take_the_read_owed();
                    return;
                };
                let (run_cancel, mut run) = s.take_log_run();
                let options = s.log_options();
                let outcome = s.run_swap_pass(&workdir, options, &run_cancel).await;
                run.answer(outcome);
                outcome
            } else {
                RefreshOutcome::Unchanged
            };
            // The completion is also the single-flight ownership boundary:
            // a caller woken by it must be able to start the following poll.
            drop(permit);
            if finished.send(outcome).is_err() {
                tracing::trace!("poll completion was not observed");
            }
            // Last, and after the slot is back: a read this session was
            // refused while this one held it is served here or nowhere.
            s.take_the_read_owed();
        });
        task
    }
}
