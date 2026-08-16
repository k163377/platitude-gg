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
    /// Another remote-tag read already owns the single-flight slot.
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
        let s = Arc::clone(self);
        let options = self.log_options();
        let (finished, task) = RefreshTask::pending();
        self.runtime.spawn(async move {
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
        let s = Arc::clone(self);
        let (finished, task) = RefreshTask::pending();
        self.runtime.spawn(async move {
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
