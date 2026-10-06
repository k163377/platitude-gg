//! Background graph refresh entry points and their completion boundary.

use super::log::PassReads;
use super::refresh::Listed;
use super::*;

/// What a background graph refresh established when it completed — the
/// completion boundary, awaited through [`RefreshTask::outcome`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshOutcome {
    /// Not started: the repository is not open.
    Unavailable,
    /// Not started: a write and its follow-up refreshes still own the
    /// repository.
    WriteBusy,
    /// Not started: the previous poll still owns the single-flight slot.
    Busy,
    /// Completed; the graph already showed its answer.
    Unchanged,
    /// Completed and installed a different graph.
    Changed,
    /// A newer graph request superseded this one, or the session closed.
    Cancelled,
    /// Reading the graph failed.
    Failed,
}

impl RefreshOutcome {
    /// Whether the graph on screen now answers for the repository.
    ///
    /// A pass another ask took over says nothing, so a caller that has to
    /// know follows it to that ask first ([`RepoSession::graph_answer`]);
    /// a `Cancelled` still reaching such a caller means the session is
    /// closing. Every other outcome left the picture as it was, which a
    /// settling write reports as [`super::FollowUp::Graph`].
    #[must_use]
    pub fn landed(self) -> bool {
        matches!(self, Self::Changed | Self::Unchanged)
    }
}

/// How many graph passes could still walk, and what each ask came to.
///
/// A pass registers on the thread about to spawn it and stays until its
/// task ends. Cancelling a pass is only a request: a displaced pass can
/// still start the walk it was on its way to, so a caller that took the
/// stream over waits on [`RepoSession::wait_for_graph_passes`] before
/// reading what the session did.
pub(super) struct GraphPasses {
    live: std::sync::atomic::AtomicUsize,
    changed: tokio::sync::watch::Sender<u64>,
    /// Numbers the asks in the order the stream changed hands
    /// ([`RepoSession::take_log_run`]), so "newer than mine" names exactly
    /// the passes that could answer in its place.
    asks: AtomicU64,
    /// The newest ask that answered *for the graph* — what a displaced
    /// pass's caller waits for ([`RepoSession::graph_answer`]).
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
    /// Numbers one ask. Taken with the token that takes the stream over
    /// ([`RepoSession::take_log_run`]), on the same thread, so the
    /// numbering is the order the asks were made.
    pub(super) fn ask(self: &Arc<Self>) -> GraphRun {
        let ask = self.asks.fetch_add(1, Ordering::SeqCst) + 1;
        GraphRun {
            passes: Arc::clone(self),
            ask,
            answered: false,
        }
    }

    /// Records what an ask came to. A taken-over pass (`Cancelled`)
    /// records nothing, so its caller goes on waiting for the ask that
    /// displaced it; of the rest the newest ask stands, whichever order
    /// the passes end in.
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
    /// Terminates because every ask answers: from its task, or from
    /// [`GraphRun`]'s drop where the task never got there; a chain of
    /// handovers ends at the ask nobody took over. `None` where the
    /// session closed first.
    ///
    /// `after` itself counts: a record under the caller's own number can
    /// only be the drop guard's, and waiting past it would wait for an ask
    /// nothing will make — with the whole write queue behind it.
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

    /// Registers one pass. Call before spawning it: registering from
    /// inside the task would let the boundary read "idle" for a pass not
    /// started yet (likewise [`RepoSession::take_log_run`]).
    pub(super) fn enter(self: &Arc<Self>) -> GraphPassHeld {
        self.live.fetch_add(1, Ordering::SeqCst);
        GraphPassHeld(Arc::clone(self))
    }

    /// Waits until no pass is registered — every one in flight when called
    /// included, and any registered meanwhile.
    async fn wait_idle(&self) {
        let mut changed = self.changed.subscribe();
        while self.live.load(Ordering::SeqCst) != 0 {
            if changed.changed().await.is_err() {
                return;
            }
        }
    }
}

/// What a pass carries while it could still walk. Dropped by the task,
/// so a pass that unwound or went down with the runtime still reports
/// stopping.
pub(super) struct GraphPassHeld(Arc<GraphPasses>);

impl Drop for GraphPassHeld {
    fn drop(&mut self) {
        self.0.live.fetch_sub(1, Ordering::SeqCst);
        self.0
            .changed
            .send_modify(|generation| *generation = generation.wrapping_add(1));
    }
}

/// One ask for the graph, from taking the stream over
/// ([`RepoSession::take_log_run`]) to the answer it leaves.
///
/// Answered by the task that runs the pass, or by this drop where the
/// task unwound or went down with the runtime — otherwise a waiter in
/// [`RepoSession::graph_answer`] waits forever. [`super::PassWatch`]
/// guards the same unwind for the screen; this one for the write queue.
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

    /// Called once, with the whole task's outcome: a restart runs two
    /// passes under one ask, and the last one answers.
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
    /// The ask behind this, which [`RepoSession::graph_answer`] follows
    /// when its pass was taken over. `None` where no pass started, and for
    /// the poll: a tick another ask took over is that ask's to answer.
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

    /// Waits for the operation to finish. A dropped runtime reads as
    /// `Cancelled`: no later answer can arrive.
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
    /// Another remote-tag read owns the single-flight slot, and this
    /// request was dropped. Unlike a [`super::ReadFlight`], no repeat is
    /// owed: no local operation moves what a remote carries under
    /// `refs/tags/` (a push is branches only), and both askers (an
    /// opening, installing the interval) ask the same question of the same
    /// remotes — the read in flight is the answer.
    ///
    /// Said once that read has let the slot go, so the caller knows its
    /// answer is published and its next ask is not turned away by the
    /// same read.
    Busy,
    /// The repository is not open any more.
    Unavailable,
    /// A remote that was asked did not answer, so what is held is not
    /// what the remotes carry. Only this says so: such a read publishes
    /// nothing (the same silence as finding nothing new), and a caller
    /// waiting on the badge would spend its whole budget where git cannot
    /// reach the remote. Not an error — a badge is not worth one.
    Unanswered,
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
    /// Waits until the refs and status reads in flight have left their
    /// single flight. A snapshot event is sent from inside the reader,
    /// before it requests the graph refresh the answer implies; wait here
    /// after seeing one when that matters.
    pub async fn wait_for_snapshot_reads(&self) {
        tokio::join!(self.refs_read.wait_idle(), self.status_read.wait_idle());
    }

    /// Waits until every graph pass started when this was called has
    /// stopped, the one this caller displaced included:
    /// [`RefreshTask::outcome`] covers only its own pass, and a cancelled
    /// pass stops only when it next looks ([`GraphPasses`]). Code that
    /// counts what the session did waits here too.
    pub async fn wait_for_graph_passes(&self) {
        self.graph_passes.wait_idle().await;
    }

    /// Rebuilds the graph off-screen and swaps it in only when it differs
    /// from what the UI already shows ([`RepoSession::run_swap_pass`]).
    /// Background triggers go through here: [`RepoSession::restart_log`]
    /// repaints even when history did not move, which flickers under a
    /// periodic fetch.
    pub fn refresh_log(self: &Arc<Self>) {
        drop(self.start_refresh_log(None));
    }

    /// [`Self::refresh_log`], walking with reads the caller already took
    /// ([`PassReads`]) — the stashes its listing read, so the walk reads
    /// none of its own.
    pub(super) fn refresh_log_with(self: &Arc<Self>, reads: Option<PassReads>) {
        drop(self.start_refresh_log(reads));
    }

    /// [`Self::refresh_log`], returning its completion boundary — for
    /// callers that must tell "still running" from "finished without an
    /// event".
    pub fn refresh_log_tracked(self: &Arc<Self>) -> RefreshTask {
        self.start_refresh_log(None)
    }

    fn start_refresh_log(self: &Arc<Self>, reads: Option<PassReads>) -> RefreshTask {
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
            let outcome = s.run_swap_pass(&workdir, options, &run_cancel, reads).await;
            run.answer(outcome);
            if finished.send(outcome).is_err() {
                tracing::trace!("refresh completion was not observed");
            }
        });
        task
    }

    /// Rebuilds the graph with the reads the caller already took and waits
    /// for its answer, following a takeover to the ask that took it over
    /// ([`Self::graph_answer`]) — what a write's settling waits on.
    pub(super) async fn settle_graph_with(
        self: &Arc<Self>,
        reads: Option<PassReads>,
    ) -> RefreshOutcome {
        let task = self.start_refresh_log(reads);
        self.graph_answer(task).await
    }

    /// The answer `task` is owed, which may be another ask's: where a
    /// newer ask took the pass over, that ask says whether the repository
    /// reached the screen. Taking the handover as the answer lets a write
    /// report itself settled while its graph is still being walked, or
    /// landed where the replacement then failed
    /// (`session_integration::operations` pins both).
    ///
    /// Ends at the ask nobody took over, or when the session closes
    /// ([`GraphPasses::landed_after`]).
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

    /// Refs and status read again, and the rebuild they ask for — the half
    /// of the page's paced read (`session::pacer`, which adds the worktree
    /// listing) that a write landing in this tree asks for too
    /// ([`Self::read_again`]). Stashes ride the focus and post-write
    /// refreshes.
    ///
    /// Skipped while the previous poll is still going (a slow repository
    /// polls less often) and while any write of this working tree runs
    /// ([`RepoSession::tree_write`]): a session reopened over a closed
    /// tab's write would otherwise publish the tree mid-write. The session
    /// finishing such a write calls this on the others.
    ///
    /// Not skipped under a write that replays
    /// ([`OperationKind::replays_history`]): those can stand for tens of
    /// seconds, and skipping would leave no badge, progress or graph for
    /// the whole rewrite. git counts the steps out on disk
    /// ([`crate::integrate::rebase_standing`]), the snapshot reads coalesce
    /// with the write's own refresh ([`super::ReadFlight`]), and a tick
    /// that cannot keep up just lands later.
    pub fn refresh_poll(self: &Arc<Self>) {
        drop(self.start_refresh_poll());
    }

    /// Reads the repository again because a write landed in it — from
    /// this session or another on the same working tree
    /// ([`RepoSession::tell_the_tree`]).
    ///
    /// Owed, unlike a clock tick: the read holding the slot began before
    /// the write, so dropping this would leave a stale repository on
    /// screen. Refused, it is noted and taken by that read as it hands the
    /// slot back.
    pub(super) fn read_again(self: &Arc<Self>) {
        self.read_owed.store(true, Ordering::SeqCst);
        drop(self.start_refresh_poll());
    }

    /// Takes the read owed, if one is — called wherever the reason it was
    /// refused has just gone: a poll handing the slot back, and this
    /// session's write ending (`write::serve`).
    ///
    /// Both, because either can be last: the poll's retry is refused too
    /// if a write of this session started meanwhile, and a write's landing
    /// tells only the *other* sessions.
    ///
    /// A closed session owes nothing: its parked write (the app's
    /// `Hub::park_writes_of`) ends after the close, and a read then is
    /// paid for a page that is gone (`write::run_write`).
    pub(super) fn take_the_read_owed(self: &Arc<Self>) {
        if self.read_owed.load(Ordering::SeqCst) && self.keeps_what_it_reads() {
            drop(self.start_refresh_poll());
        }
    }

    /// Runs one poll tick and returns a boundary that includes any graph
    /// rebuild the tick requested.
    pub fn refresh_poll_tracked(self: &Arc<Self>) -> RefreshTask {
        self.start_refresh_poll()
    }

    fn start_refresh_poll(self: &Arc<Self>) -> RefreshTask {
        let ticket = match self.begin_poll() {
            Ok(ticket) => ticket,
            Err(skipped) => return RefreshTask::ready(skipped),
        };
        let s = Arc::clone(self);
        // Nobody follows a tick that was taken over (`RefreshTask::ask`).
        let (finished, task) = RefreshTask::pending(None);
        self.runtime.spawn(async move {
            let nothing_listed = async { (StashRead::default(), Listed::default()) };
            let poll = s.poll_reads(ticket, nothing_listed).await;
            // The completion is also the single-flight ownership boundary:
            // a caller woken by it must be able to start the following poll.
            if finished.send(poll.outcome).is_err() {
                tracing::trace!("poll completion was not observed");
            }
            // Last, and after the slot is back: a read this session was
            // refused while this one held it is served here or nowhere.
            s.take_the_read_owed();
        });
        task
    }

    /// Takes the poll's single flight, or says why the poll is skipped.
    /// Taken on the caller's thread, so a tick that finds the last one
    /// out is dropped at once.
    pub(super) fn begin_poll(self: &Arc<Self>) -> Result<PollTicket, RefreshOutcome> {
        if self
            .tree_write()
            .is_some_and(|write| !write.kind.replays_history())
        {
            tracing::trace!("poll skipped: a write is running");
            return Err(RefreshOutcome::WriteBusy);
        }
        let Ok(permit) = Arc::clone(&self.poll_slot).try_acquire_owned() else {
            tracing::trace!("poll skipped: the previous one has not finished");
            return Err(RefreshOutcome::Busy);
        };
        // The reads below answer for anything that landed before this
        // moment; a write landing inside them sets this again
        // ([`Self::read_again`]).
        self.read_owed.store(false, Ordering::SeqCst);
        Ok(PollTicket {
            permit,
            held: self.graph_passes.enter(),
        })
    }

    /// The poll's reads, the listings the caller adds beside them, and the
    /// rebuild they ask for, in the caller's task — the paced read meters
    /// them there (`session::pacer`). Gives the single flight back before
    /// it answers; the read owed is the caller's to take after that
    /// ([`Self::take_the_read_owed`]).
    pub(super) async fn poll_reads(
        self: &Arc<Self>,
        ticket: PollTicket,
        listings: impl Future<Output = (StashRead, Listed)>,
    ) -> PollRead {
        let PollTicket { permit, held } = ticket;
        // Settled before either read starts: inside the join, the status
        // read could consume the line-ending staleness before the refs
        // read sets it, leaving a withdrawn notice standing. Idempotent —
        // the refs read then pays only a stat.
        self.forget_what_the_config_decides();
        // Every read can call for a rebuild, but one walk serves them all,
        // after all of them: an external commit moves a ref *and* cleans
        // the tree, and a copy taken or a stash dropped outside moves
        // neither — the listings are their only word.
        let (refs, tree, (stashes, listed)) =
            tokio::join!(self.read_refs(), self.read_status(), listings);
        let moved = refs == Reread::Moved || tree == Reread::Moved || stashes.moved;
        let outcome = if moved || listed.walk {
            self.walk_here(stashes).await
        } else {
            RefreshOutcome::Unchanged
        };
        drop(permit);
        PollRead {
            outcome,
            _held: held,
        }
    }

    /// A swap pass in the caller's task, as [`Self::refresh_log`] runs one
    /// in a task of its own — walking with the stashes a listing behind
    /// the same reason read, while they still stand
    /// ([`Self::pass_reads_listed`]).
    pub(super) async fn walk_here(self: &Arc<Self>, stashes: StashRead) -> RefreshOutcome {
        let Some(workdir) = self.workdir() else {
            return RefreshOutcome::Cancelled;
        };
        let (run_cancel, mut run) = self.take_log_run();
        let _held = self.graph_passes.enter();
        let reads = self.pass_reads_listed(&workdir, &run_cancel, stashes).await;
        let options = self.log_options();
        let outcome = self
            .run_swap_pass(&workdir, options, &run_cancel, reads)
            .await;
        run.answer(outcome);
        outcome
    }
}

/// The poll's single flight, held, and its place among the graph passes.
pub(super) struct PollTicket {
    permit: tokio::sync::OwnedSemaphorePermit,
    held: GraphPassHeld,
}

/// What a poll came to, and its place among the graph passes, held until
/// the caller drops it — after it has taken the read owed
/// ([`RepoSession::take_the_read_owed`]), whose poll enters before this
/// leaves, so a wait for the passes never sees idle between the two.
pub(super) struct PollRead {
    pub(super) outcome: RefreshOutcome,
    _held: GraphPassHeld,
}
