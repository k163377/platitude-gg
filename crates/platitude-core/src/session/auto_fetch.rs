//! The interval-driven `git fetch --prune`: timer install, suspend and
//! resume, hand-stepped ticks, and the remote-tag catch-up it permits.

use super::*;

impl RepoSession {
    /// Starts, restarts or stops the periodic `git fetch --prune`.
    ///
    /// `None` (or zero) turns it off, and nothing is queued once it has
    /// returned (a fetch already in the write queue still runs). Only one
    /// fetch is ever outstanding: on a slow link or a repository whose
    /// credential helper is taking its time, a tick that finds the previous
    /// fetch unfinished is skipped rather than queued behind it.
    ///
    /// The clock is not the only way in: [`Self::auto_fetch_ticker`] steps
    /// the timer this starts.
    pub fn set_auto_fetch(self: &Arc<Self>, interval: Option<std::time::Duration>) {
        let wanted = interval.filter(|i| !i.is_zero());
        *self.lock_auto_fetch_interval() = wanted;
        self.install_auto_fetch(wanted);
    }

    /// Stops the timer without forgetting what it was set to, so
    /// [`Self::resume_auto_fetch`] can put it back.
    ///
    /// Answers whether there was one to stop. A repository whose owner
    /// turned automatic fetching off has nothing suspended and nothing to
    /// say about it — the caller uses that to tell the two apart.
    pub fn suspend_auto_fetch(&self) -> bool {
        let mut guard = match self.auto_fetch.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        match guard.take() {
            Some(previous) => {
                previous.cancel.cancel();
                true
            }
            None => false,
        }
    }

    /// Starts the timer again on the interval it was last set to. Does
    /// nothing where automatic fetching was never on.
    pub fn resume_auto_fetch(self: &Arc<Self>) {
        let wanted = *self.lock_auto_fetch_interval();
        self.install_auto_fetch(wanted);
    }

    fn lock_auto_fetch_interval(&self) -> std::sync::MutexGuard<'_, Option<std::time::Duration>> {
        match self.auto_fetch_interval.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        }
    }

    fn install_auto_fetch(self: &Arc<Self>, interval: Option<std::time::Duration>) {
        let mut guard = match self.auto_fetch.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        if let Some(previous) = guard.take() {
            previous.cancel.cancel();
        }
        let Some(interval) = interval.filter(|i| !i.is_zero()) else {
            return;
        };
        let cancel = self.root_cancel.child_token();
        let (ticks, mut by_hand) = tokio::sync::mpsc::unbounded_channel();
        *guard = Some(AutoFetch {
            cancel: cancel.clone(),
            ticks,
        });
        drop(guard);

        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            // tokio fires the first tick immediately; opening the
            // repository has just read it, so wait out a full interval.
            ticker.tick().await;
            loop {
                tokio::select! {
                    // Biased: a stop that arrives while ticks are already
                    // overdue (a starved timer catches up in a burst) wins
                    // over them instead of being picked at random.
                    biased;
                    _ = cancel.cancelled() => return,
                    _ = ticker.tick() => {
                        if !s.auto_fetch_tick(&cancel) {
                            return;
                        }
                    }
                    Some(ack) = by_hand.recv() => {
                        // Answered only once the tick has been acted on,
                        // and never by a timer that has been stopped.
                        if !s.auto_fetch_tick(&cancel) {
                            return;
                        }
                        if ack.send(()).is_err() {
                            tracing::debug!("auto fetch tick: nobody waiting for it");
                        }
                    }
                }
            }
        });
        self.catch_up_remote_tags();
    }

    /// Reads what the remotes carry under `refs/tags/` without waiting for
    /// a fetch, when two things are true: automatic fetching is on, and the
    /// graph is showing tags.
    ///
    /// The first is the permission — a repository whose owner turned the
    /// timer off has said not to reach the network unasked, and a badge is
    /// not the thing to break that for. The second is the priority: with
    /// tags out of the walk the chips that carry this reading are not on
    /// screen, and the timer will fill it in within the interval anyway.
    ///
    /// Deliberately **not** on the write queue. A request there sets
    /// `write_busy`, which holds the poll out and puts every later write
    /// behind this one — far too much to spend on a badge. It runs as a
    /// plain background read instead, on the handle that keeps it out of
    /// the command log, and only republishes if the answer moved.
    ///
    /// Entered twice: once the repository is open (the interval is already
    /// installed by then — the application sets it the moment the session
    /// is handed over), and from [`Self::set_auto_fetch`] afterwards, so
    /// granting the permission in settings is itself a reason to look.
    pub(super) fn catch_up_remote_tags(self: &Arc<Self>) {
        drop(self.start_remote_tag_refresh());
    }

    /// Applies the same permission and visibility gates as the automatic
    /// remote-tag catch-up, and returns its causal completion boundary.
    ///
    /// The app uses the fire-and-forget path. Tests and coordinating callers
    /// use this form when "the read was declined" must be distinguished from
    /// "the network read has not answered yet" without a quiet-time guess.
    pub fn refresh_remote_tags_tracked(self: &Arc<Self>) -> RemoteTagRefreshTask {
        self.start_remote_tag_refresh()
    }

    fn start_remote_tag_refresh(self: &Arc<Self>) -> RemoteTagRefreshTask {
        let interval_is_on = match self.auto_fetch.lock() {
            Ok(g) => g.is_some(),
            Err(e) => e.into_inner().is_some(),
        };
        if !interval_is_on {
            return RemoteTagRefreshTask::ready(RemoteTagRefreshOutcome::Disabled);
        }
        if !self.log_options().include_tags {
            return RemoteTagRefreshTask::ready(RemoteTagRefreshOutcome::Hidden);
        }
        let Ok(permit) = Arc::clone(&self.remote_tags_slot).try_acquire_owned() else {
            tracing::debug!("remote tags: the previous read has not finished");
            return RemoteTagRefreshTask::ready(RemoteTagRefreshOutcome::Busy);
        };
        let s = Arc::clone(self);
        let timeout = self.network_timeout();
        let (finished, task) = RemoteTagRefreshTask::pending();
        self.runtime.spawn(async move {
            let Some(workdir) = s.workdir() else {
                drop(permit);
                if finished.send(RemoteTagRefreshOutcome::Unavailable).is_err() {
                    tracing::trace!("remote-tag refresh completion was not observed");
                }
                return;
            };
            let cancel = s.root_cancel.clone();
            let moved = s
                .read_remote_tags(&s.executor, &workdir, None, timeout, &cancel)
                .await;
            let outcome = if cancel.is_cancelled() {
                RemoteTagRefreshOutcome::Cancelled
            } else if moved {
                s.refresh_refs();
                RemoteTagRefreshOutcome::Changed
            } else {
                RemoteTagRefreshOutcome::Unchanged
            };
            // `outcome()` closes ownership as well as the read: an immediate
            // following request must not race the old permit's destructor.
            drop(permit);
            if finished.send(outcome).is_err() {
                tracing::trace!("remote-tag refresh completion was not observed");
            }
        });
        task
    }

    /// Handle for stepping the running timer, in place of waiting out its
    /// interval — see [`AutoFetchTicker`]. `None` while auto fetch is off.
    pub fn auto_fetch_ticker(&self) -> Option<AutoFetchTicker> {
        let guard = match self.auto_fetch.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        guard.as_ref().map(|a| AutoFetchTicker(a.ticks.clone()))
    }

    /// Queues one automatic fetch, unless the previous one is still going.
    ///
    /// Reported under its own op name: a laptop that is simply offline must
    /// not put a fresh error banner on screen every interval.
    ///
    /// Answers whether the timer is still running. Its token is read under
    /// the lock a stop cancels it under, so the two cannot interleave: a
    /// tick either has its fetch in the write queue before `set_auto_fetch`
    /// returns, or sees the stop and queues nothing. Turning it off leaves
    /// nothing still to come.
    fn auto_fetch_tick(self: &Arc<Self>, cancel: &CancellationToken) -> bool {
        let _stop = match self.auto_fetch.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        if cancel.is_cancelled() {
            return false;
        }
        let Ok(permit) = Arc::clone(&self.auto_fetch_slot).try_acquire_owned() else {
            tracing::debug!("auto fetch skipped: the previous one has not finished");
            return true;
        };
        let timeout = self.network_timeout();
        let s = Arc::clone(self);
        self.write(
            AUTO_FETCH_OP,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let _permit = permit;
                s.fetch_and_read_tags(&exec, &repo.workdir, None, timeout, &cancel)
                    .await
            },
        );
        true
    }
}
