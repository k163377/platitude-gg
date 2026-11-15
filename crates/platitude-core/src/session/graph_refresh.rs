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
    /// git has said nothing for the walk's silence budget
    /// (`session::walk::walk_silence`) — this pass's walk, or the one it
    /// waits behind: answered as not landed, the walk left running — it
    /// lands on screen if git answers after all.
    Silent,
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
/// task ends, or until it is answered `Silent` ([`PassAnswer`]): a walk
/// git says nothing in is waited on by nobody. Cancelling a pass is only a
/// request: a displaced pass can still start the walk it was on its way
/// to, so a caller that took the stream over waits on
/// [`RepoSession::wait_for_graph_passes`] before reading what the session
/// did.
pub(super) struct GraphPasses {
    live: std::sync::atomic::AtomicUsize,
    changed: tokio::sync::watch::Sender<u64>,
    /// Numbers the asks in the order the stream changed hands
    /// ([`RepoSession::take_pass`]), so "newer than mine" names exactly
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
    /// ([`RepoSession::take_pass`]), on the same thread, so the
    /// numbering is the order the asks were made.
    pub(super) fn ask(self: &Arc<Self>) -> GraphRun {
        let ask = self.asks.fetch_add(1, Ordering::SeqCst) + 1;
        GraphRun {
            passes: Arc::clone(self),
            ask,
            answered: false,
        }
    }

    /// The number the next ask will be known by.
    pub(super) fn next_ask(&self) -> u64 {
        self.asks.load(Ordering::SeqCst) + 1
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
    /// started yet (likewise [`RepoSession::take_pass`]).
    pub(super) fn enter(self: &Arc<Self>) -> GraphPassHeld {
        self.live.fetch_add(1, Ordering::SeqCst);
        GraphPassHeld(Arc::clone(self))
    }

    fn is_idle(&self) -> bool {
        self.live.load(Ordering::SeqCst) == 0
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

/// What a pass carries while it could still walk. Dropped with the
/// pass's answer, which its task holds — so a pass that unwound or went
/// down with the runtime still reports stopping — or by a `Silent` answer
/// ([`PassAnswer`]).
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
/// ([`RepoSession::take_pass`]) to the answer it leaves.
///
/// Answered through the pass's answer ([`PassAnswer`]) — by the task that
/// runs the pass, or as its walk goes silent — or by this drop where the
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

    /// Called once ([`PassAnswer::answer`]): with the whole task's outcome
    /// — a restart runs two passes under one ask, and the last one answers
    /// — or at the first walk to go silent.
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

/// One pass's place, taken on the caller's thread before the pass is
/// spawned ([`RepoSession::take_pass`]): its token, its turn, and where its
/// answer goes.
pub(super) struct PassStart {
    pub(super) token: CancellationToken,
    pub(super) lap: Lap,
    pub(super) answer: Arc<PassAnswer>,
}

/// Where a pass's answer goes, owed once: at the pass's end, or as soon as
/// its walk goes silent ([`RepoSession::walk_went_silent`]) or it is left
/// behind one that has (`asks::Lap::start`) — whoever waits on the pass
/// moves on then, and the walk does not stop. Holds what the answer lets
/// go: the ask's record ([`GraphRun`]) and the caller's boundary
/// ([`RefreshTask`]); and the pass's place among the passes still walking
/// ([`GraphPassHeld`]), which only a `Silent` answer gives up before the
/// task does — after its turn (`asks::Running`), as the task drops them.
pub(super) struct PassAnswer {
    /// The pass's number in the takeover (`asks::Takeover`).
    pub(super) number: u64,
    /// The ask's number ([`GraphPasses::ask`]), which a landing walk names
    /// to the deletes it answers for (`RepoSession::leaving_walked`).
    pub(super) ask: u64,
    owed: Mutex<Option<Owed>>,
    held: Mutex<Option<GraphPassHeld>>,
}

struct Owed {
    run: GraphRun,
    finished: Option<tokio::sync::oneshot::Sender<RefreshOutcome>>,
}

impl PassAnswer {
    /// The answer one pass owes, and the boundary its caller waits on.
    pub(super) fn owed(
        run: GraphRun,
        number: u64,
        held: GraphPassHeld,
    ) -> (Arc<Self>, RefreshTask) {
        let ask = run.ask();
        let (finished, task) = RefreshTask::pending(Some(ask));
        let answer = Self {
            number,
            ask,
            owed: Mutex::new(Some(Owed {
                run,
                finished: Some(finished),
            })),
            held: Mutex::new(Some(held)),
        };
        (Arc::new(answer), task)
    }

    /// Answers for the pass, if nothing has yet; the first answer stands.
    pub(super) fn answer(&self, outcome: RefreshOutcome) {
        let Some(mut owed) = relock(&self.owed).take() else {
            return;
        };
        owed.run.answer(outcome);
        if let Some(finished) = owed.finished.take()
            && finished.send(outcome).is_err()
        {
            tracing::trace!("refresh completion was not observed");
        }
        // After the answer is out, so a wait for the passes that ends here
        // finds it recorded.
        if outcome == RefreshOutcome::Silent {
            drop(relock(&self.held).take());
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
    /// owed: a write that moves what a remote carries under `refs/tags/`
    /// reads it again itself (`session::ops_remote_tags`), and both askers (an
    /// opening, installing the interval) ask the same question of the same
    /// remotes — the read in flight is the answer. Whatever both missed
    /// comes with the next auto fetch tick: the poll reads no remote tags.
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
    /// The boundary sampled by `PageSettled`'s readings term and session integration tests: snapshot and carried
    /// reads have ended, paced worktrees owe no due read, chosen starts have ended, and the consumer's generation
    /// matches the latest graph. The consumer separately awaits its stream's Finished (`GraphModel.loading`).
    /// Future polls and due worktrees on an inactive page owe no new read; starts already chosen still count. A
    /// pass answered `Silent` counts as ended ([`GraphPasses`]): the page settles while its walk runs on.
    pub fn graph_settled_at(&self, generation: u64) -> bool {
        self.refs_read.is_idle()
            && self.status_read.is_idle()
            && self.stash_read.is_idle()
            && self.worktrees_read.is_idle()
            && !self.carried_pass.busy()
            && self.pacing.readings_settled()
            && self.graph_passes.is_idle()
            && self.log_gen.load(Ordering::SeqCst) == generation
    }

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
    /// counts what the session did waits here too. A pass answered
    /// `Silent` counts as stopped from then ([`PassAnswer`]): it may still
    /// walk and land later, if git answers.
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
        let (pass, task) = self.take_pass(false);
        let s = Arc::clone(self);
        let options = self.log_options();
        self.runtime.spawn(async move {
            let PassStart { token, lap, answer } = pass;
            let outcome = match lap.start(|| answer.answer(RefreshOutcome::Silent)).await {
                Some(_lap) => {
                    s.run_swap_pass(&workdir, options, &token, reads, &answer)
                        .await
                }
                None => RefreshOutcome::Cancelled,
            };
            answer.answer(outcome);
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
    /// reached the screen (user decision — ask before changing). Taking the
    /// handover as the answer lets a write report itself settled while its
    /// graph is still being walked, or landed where the replacement then
    /// failed (`session_integration::operations` pins both). Answering
    /// `Cancelled` as "did not land" would report a healthy takeover (a
    /// Reload) as a failure.
    ///
    /// The price: a write's settling stretches by somebody else's walk.
    /// Its `WriteFinished` has gone out already, so the screen answers as
    /// fast; what waits is the queue's next request.
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
    /// listing). Stashes ride the focus and post-write refreshes, and a
    /// write landing in this tree from another session asks for both
    /// listings too ([`Self::read_again`]).
    ///
    /// Skipped while the previous poll is still going (a slow repository
    /// polls less often) and while any write of this worktree runs
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
    /// this session or another on the same worktree
    /// ([`RepoSession::tell_the_tree`]).
    ///
    /// Owed, unlike a clock tick: the read holding the slot began before
    /// the write, so dropping this would leave a stale repository on
    /// screen. Refused, it is noted and taken by that read as it hands the
    /// slot back.
    ///
    /// With the stash and worktree listings: the write may be a stash drop
    /// or a worktree removal a closed tab let run on, which moves no ref —
    /// and nothing else this session reads would say so before the
    /// page's own paced read comes round.
    pub(super) fn read_again(self: &Arc<Self>) {
        self.poll_gate.owe();
        drop(self.start_refresh_poll());
    }

    /// Takes the read owed, if one is — called wherever the reason it was
    /// refused has just gone: a poll handing the slot back, and this
    /// session's burst of writes ending (`write::settle`).
    ///
    /// Both, because either can be last: the poll's retry is refused too
    /// if a write of this session started meanwhile, and a write's landing
    /// tells only the *other* sessions.
    ///
    /// A closed session owes nothing: its parked write (the app's
    /// `Hub::park_writes_of`) ends after the close, and a read then is
    /// paid for a page that is gone (`write::settle`).
    pub(super) fn take_the_read_owed(self: &Arc<Self>) {
        if self.poll_gate.owed() && self.keeps_what_it_reads() {
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
            let owed = ticket.owed;
            let listings = async {
                if owed {
                    s.read_listings().await
                } else {
                    (StashRead::default(), Listed::default())
                }
            };
            let poll = s.poll_reads(ticket, listings).await;
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
        let Some(turn) = self.poll_gate.try_turn() else {
            tracing::trace!("poll skipped: the previous one has not finished");
            return Err(RefreshOutcome::Busy);
        };
        // The reads below answer for anything that landed before this
        // moment; a write landing inside them owes the next poll
        // ([`Self::read_again`]).
        let owed = self.poll_gate.take_owed();
        Ok(PollTicket {
            turn,
            held: self.graph_passes.enter(),
            owed,
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
        let PollTicket { turn, held, .. } = ticket;
        // Settled before either read starts: inside the join, the status
        // read could consume the line-ending staleness before the refs
        // read sets it, leaving a withdrawn notice standing. Idempotent —
        // the refs read then pays only a stat.
        self.forget_what_the_config_decides();
        // Every read can call for a rebuild, but one walk serves them all,
        // after all of them: an external commit moves a ref *and* cleans
        // the tree, and a worktree taken or a stash dropped outside moves
        // neither — the listings are their only word.
        let (refs, tree, (stashes, listed)) =
            tokio::join!(self.read_refs(), self.read_status(), listings);
        let moved = refs == Reread::Moved || tree == Reread::Moved || stashes.moved;
        let outcome = if moved || listed.walk {
            self.walk_here(stashes).await
        } else {
            RefreshOutcome::Unchanged
        };
        // Given back without taking what is owed: the caller takes it
        // through `take_the_read_owed`, which keeps out of a running write.
        drop(turn);
        PollRead {
            outcome,
            _held: held,
        }
    }

    /// A swap pass the caller waits on, as [`Self::refresh_log`] runs one
    /// nobody waits on — walking with the stashes a listing behind the same
    /// reason read, while they still stand at the pass's turn
    /// ([`Self::pass_reads_listed`]). Its own answer, a takeover answering
    /// `Cancelled`. The pass runs in a task of its own so that a walk gone
    /// silent answers the caller and walks on ([`PassAnswer`]): the poll
    /// waiting here lets its single flight go. The task is metered as the
    /// caller's work (`session::pacer`); a walk gone silent is weighed as
    /// far as it has lived at the silence, before the caller weighs it
    /// (`process::meter`'s `charge_so_far`).
    pub(super) async fn walk_here(self: &Arc<Self>, stashes: StashRead) -> RefreshOutcome {
        let Some(workdir) = self.workdir() else {
            return RefreshOutcome::Cancelled;
        };
        let (pass, task) = self.take_pass(false);
        let s = Arc::clone(self);
        let options = self.log_options();
        self.runtime.spawn(crate::process::carried(async move {
            let PassStart { token, lap, answer } = pass;
            let outcome = match lap.start(|| answer.answer(RefreshOutcome::Silent)).await {
                Some(_lap) => {
                    let reads = s.pass_reads_listed(&workdir, &token, stashes).await;
                    s.run_swap_pass(&workdir, options, &token, reads, &answer)
                        .await
                }
                None => RefreshOutcome::Cancelled,
            };
            answer.answer(outcome);
        }));
        task.outcome().await
    }
}

/// The poll's single flight, held, and its place among the graph passes.
pub(super) struct PollTicket {
    turn: super::asks::Turn,
    held: GraphPassHeld,
    /// Whether a write landing in this tree from another session is
    /// answered by this poll ([`RepoSession::read_again`]): it then reads
    /// the listings too, whoever started it.
    pub(super) owed: bool,
}

/// What a poll came to, and its place among the graph passes, held until
/// the caller drops it — after it has taken the read owed
/// ([`RepoSession::take_the_read_owed`]), whose poll enters before this
/// leaves, so a wait for the passes never sees idle between the two.
pub(super) struct PollRead {
    pub(super) outcome: RefreshOutcome,
    _held: GraphPassHeld,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pass answered `Silent` leaves the passes still walking at once —
    /// a fetch's settling waits them out (`RepoSession::wait_for_graph_passes`),
    /// and must not wait on a walk git says nothing in. Any other answer
    /// leaves them only as the answer goes with the pass's task; neither
    /// leaves twice.
    #[test]
    fn only_a_silent_answer_lets_the_pass_go_before_its_task() {
        let passes = Arc::new(GraphPasses::default());
        let (answer, _task) = PassAnswer::owed(passes.ask(), 1, passes.enter());
        answer.answer(RefreshOutcome::Changed);
        assert!(!passes.is_idle(), "its task still holds its place");
        drop(answer);
        assert!(passes.is_idle());

        let (answer, _task) = PassAnswer::owed(passes.ask(), 2, passes.enter());
        answer.answer(RefreshOutcome::Silent);
        assert!(passes.is_idle(), "nobody waits on a silent walk");
        drop(answer);
        assert!(passes.is_idle(), "and its task's end leaves nothing more");
    }
}
