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
pub(super) struct GraphPasses {
    live: std::sync::atomic::AtomicUsize,
    changed: tokio::sync::watch::Sender<u64>,
}

impl Default for GraphPasses {
    fn default() -> Self {
        let (changed, _) = tokio::sync::watch::channel(0);
        Self {
            live: std::sync::atomic::AtomicUsize::new(0),
            changed,
        }
    }
}

impl GraphPasses {
    /// Registers one pass. **Called before the pass is spawned**, never
    /// from inside the spawned task: a task is not scheduled in the order
    /// the asks came in, and one that registered itself would leave the
    /// boundary reading "idle" for the pass that has not started yet
    /// (see [`RepoSession::take_log_token`], which is taken the same way
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

/// Completion of one explicitly tracked background refresh.
pub struct RefreshTask(tokio::sync::oneshot::Receiver<RefreshOutcome>);

impl RefreshTask {
    fn pending() -> (tokio::sync::oneshot::Sender<RefreshOutcome>, Self) {
        let (send, receive) = tokio::sync::oneshot::channel();
        (send, Self(receive))
    }

    fn ready(outcome: RefreshOutcome) -> Self {
        let (send, task) = Self::pending();
        if send.send(outcome).is_err() {
            tracing::trace!("refresh completion was not observed");
        }
        task
    }

    /// Waits for the operation itself to finish. A dropped runtime is the
    /// same observable result as cancellation: no later answer from this
    /// operation can arrive.
    pub async fn outcome(self) -> RefreshOutcome {
        self.0.await.unwrap_or(RefreshOutcome::Cancelled)
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
    /// `ReadSlot` books a repeat because its second caller knows something
    /// the read in flight does not — a write that landed after it started.
    /// Nothing here does: no local operation moves what a remote carries
    /// under `refs/tags/` (a push is branches only), and the two places
    /// that ask (an opening, and installing the interval) ask the same
    /// question of the same remotes. A request arriving while the interval
    /// was off cannot see this either, since the permission it brings is
    /// what lets a read run at all. So the read that made this `Busy` is
    /// the answer, and it publishes one.
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
    /// Waits until the refs and status readers already in flight have
    /// returned their single-flight slots.
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
        let run_cancel = self.take_log_token();
        let held = self.graph_passes.enter();
        let s = Arc::clone(self);
        let options = self.log_options();
        let (finished, task) = RefreshTask::pending();
        self.runtime.spawn(async move {
            let _held = held;
            let outcome = s.run_swap_pass(&workdir, options, &run_cancel).await;
            if finished.send(outcome).is_err() {
                tracing::trace!("refresh completion was not observed");
            }
        });
        task
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
    pub fn refresh_poll(self: &Arc<Self>) {
        drop(self.start_refresh_poll());
    }

    /// Runs one poll tick and returns a boundary that includes any graph
    /// rebuild the tick requested.
    pub fn refresh_poll_tracked(self: &Arc<Self>) -> RefreshTask {
        self.start_refresh_poll()
    }

    fn start_refresh_poll(self: &Arc<Self>) -> RefreshTask {
        if self.write_busy.load(Ordering::SeqCst) {
            tracing::trace!("poll skipped: a write is running");
            return RefreshTask::ready(RefreshOutcome::WriteBusy);
        }
        let Ok(permit) = Arc::clone(&self.poll_slot).try_acquire_owned() else {
            tracing::trace!("poll skipped: the previous one has not finished");
            return RefreshTask::ready(RefreshOutcome::Busy);
        };
        let held = self.graph_passes.enter();
        let s = Arc::clone(self);
        let (finished, task) = RefreshTask::pending();
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
            let (refs_moved, wip_flipped) = tokio::join!(s.publish_refs(true), s.publish_status());
            let outcome = if refs_moved || wip_flipped {
                let Some(workdir) = s.workdir() else {
                    drop(permit);
                    if finished.send(RefreshOutcome::Cancelled).is_err() {
                        tracing::trace!("poll completion was not observed");
                    }
                    return;
                };
                let run_cancel = s.take_log_token();
                let options = s.log_options();
                s.run_swap_pass(&workdir, options, &run_cancel).await
            } else {
                RefreshOutcome::Unchanged
            };
            // The completion is also the single-flight ownership boundary:
            // a caller woken by it must be able to start the following poll.
            drop(permit);
            if finished.send(outcome).is_err() {
                tracing::trace!("poll completion was not observed");
            }
        });
        task
    }
}
