//! The interval-driven `git fetch --prune`: timer install, suspend and
//! resume, hand-stepped ticks, the fetch an opening fires, and the
//! remote-tag catch-up they permit.

use super::state::AutoFetchTick;
use super::*;

/// What [`RepoSession::fetch_on_open`] did with the ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenFetch {
    /// On the write queue: the repository is open and the interval is on.
    Started,
    /// Kept until the repository finishes opening, which is what fires it.
    Held,
    /// Declined: automatic fetching is off.
    Disabled,
    /// An automatic fetch was already running.
    Busy,
    NoRemote,
    /// This session has already had its opening fetch.
    Spent,
    /// The session is closed; nothing was started.
    Closed,
}

/// What wakes the timer: the interval clock in the product, a hand clock
/// in tests. Acting on a due tick, and the stop that outranks it, are
/// [`drive_auto_fetch`]'s.
pub(super) trait AutoFetchClock: Send + 'static {
    fn due(&mut self) -> impl Future<Output = ()> + Send;
}

/// The product's clock: one tick per interval, the first a whole interval
/// away (tokio's default fires at once, and the opening has just read).
///
/// Build it inside a task on the runtime: an interval takes its time
/// driver from the runtime it is created in, and panics off one.
struct IntervalClock(tokio::time::Interval);

impl IntervalClock {
    fn every(interval: std::time::Duration) -> Self {
        let first = tokio::time::Instant::now() + interval;
        Self(tokio::time::interval_at(first, interval))
    }
}

impl AutoFetchClock for IntervalClock {
    async fn due(&mut self) {
        self.0.tick().await;
    }
}

/// The timer's loop. `act` runs one tick and answers whether the timer
/// still runs; `false` ends the loop, as does the stop.
///
/// Biased towards the stop, so one that arrives with ticks already
/// overdue (a starved timer catching up in a burst) wins over them — an
/// unbiased `select!` picks at random. A hand-stepped tick is acked only
/// after it was acted on.
pub(super) async fn drive_auto_fetch(
    mut clock: impl AutoFetchClock,
    stop: CancellationToken,
    mut by_hand: tokio::sync::mpsc::UnboundedReceiver<AutoFetchTick>,
    mut act: impl FnMut() -> bool + Send,
) {
    loop {
        tokio::select! {
            biased;
            () = stop.cancelled() => return,
            () = clock.due() => {
                if !act() {
                    return;
                }
            }
            Some(ack) = by_hand.recv() => {
                if !act() {
                    return;
                }
                if ack.send(()).is_err() {
                    tracing::debug!("auto fetch tick: nobody waiting for it");
                }
            }
        }
    }
}

/// The one remote-tag read at a time, and the word that it has let go.
///
/// A second ask while a read is in flight is dropped
/// ([`RemoteTagRefreshOutcome::Busy`]) and its ack waits for the word.
/// Waiting by taking the permit would hold it for an instant, and an ask
/// in that instant would be told `Busy` with nothing reading.
pub(super) struct RemoteTagSlot {
    permits: Arc<tokio::sync::Semaphore>,
    freed: tokio::sync::watch::Sender<u64>,
}

impl Default for RemoteTagSlot {
    fn default() -> Self {
        Self {
            permits: Arc::new(tokio::sync::Semaphore::new(1)),
            freed: tokio::sync::watch::channel(0).0,
        }
    }
}

impl RemoteTagSlot {
    /// Takes the slot, or hands back what to wait on for its release.
    /// Subscribes first, so a release landing between the two is seen.
    pub(super) fn take(&self) -> Result<SlotHeld, tokio::sync::watch::Receiver<u64>> {
        let freed = self.freed.subscribe();
        match Arc::clone(&self.permits).try_acquire_owned() {
            Ok(permit) => Ok(SlotHeld {
                permit: Some(permit),
                freed: self.freed.clone(),
            }),
            Err(_) => Err(freed),
        }
    }
}

/// The slot, held: letting it go is what tells the asks booked behind it.
#[derive(Debug)]
pub(super) struct SlotHeld {
    permit: Option<tokio::sync::OwnedSemaphorePermit>,
    freed: tokio::sync::watch::Sender<u64>,
}

impl Drop for SlotHeld {
    fn drop(&mut self) {
        // The permit first, then the word: an ask made on the word must
        // find the slot already free.
        self.permit.take();
        self.freed
            .send_modify(|releases| *releases = releases.wrapping_add(1));
    }
}

impl RepoSession {
    /// Starts, restarts or stops the periodic `git fetch --prune`.
    ///
    /// `None` (or zero) turns it off, and nothing is queued once it has
    /// returned (a fetch already in the write queue still runs). A tick
    /// that finds the previous fetch unfinished is skipped.
    /// [`Self::auto_fetch_ticker`] steps the timer this starts.
    pub fn set_auto_fetch(self: &Arc<Self>, interval: Option<std::time::Duration>) {
        let wanted = interval.filter(|i| !i.is_zero());
        *self.lock_auto_fetch_interval() = wanted;
        self.install_auto_fetch(wanted);
    }

    /// Stops the timer without forgetting its interval, for
    /// [`Self::resume_auto_fetch`]. Answers whether there was one to stop
    /// (`false` where automatic fetching is off).
    pub fn suspend_auto_fetch(&self) -> bool {
        let mut guard = relock(&self.auto_fetch);
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

    /// Fetches once, as soon as the repository is open.
    ///
    /// Permitted only while the auto-fetch interval is on; the app asks
    /// right after handing the session its settings, so it is already set.
    ///
    /// Either end can arrive first — this call or the opening — so
    /// whichever arrives second fires it and the other gets
    /// [`OpenFetch::Held`]; timing one against the other would lose the
    /// fetch to a slow `git rev-parse`.
    ///
    /// Its own op name keeps it out of the command log, so a tab opened
    /// offline does not open the panel (デザイン規約 §リモートから取り込む).
    pub fn fetch_on_open(self: &Arc<Self>) -> OpenFetch {
        let open = self.workdir().is_some();
        let mut state = self.lock_open_fetch();
        match *state {
            OpenFetchState::Settled => return OpenFetch::Spent,
            _ if !open => {
                *state = OpenFetchState::Held;
                return OpenFetch::Held;
            }
            _ => *state = OpenFetchState::Settled,
        }
        drop(state);
        self.start_open_fetch()
    }

    /// Fires the fetch asked for before the repository was open, and
    /// answers whether one started — if so the opening skips the
    /// remote-tag catch-up, the fetch having read them.
    ///
    /// Reads the remote list first (the decision needs it); the refs
    /// listing that follows finds it cached (`RepoSession::remotes`).
    pub(super) async fn take_open_fetch(self: &Arc<Self>, workdir: &Path) -> bool {
        {
            let mut state = self.lock_open_fetch();
            if *state != OpenFetchState::Held {
                return false;
            }
            *state = OpenFetchState::Settled;
        }
        let cancel = self.root_cancel.clone();
        drop(self.remotes(workdir, &cancel).await);
        self.start_open_fetch() == OpenFetch::Started
    }

    fn start_open_fetch(self: &Arc<Self>) -> OpenFetch {
        if !self.auto_fetch_is_on() {
            return OpenFetch::Disabled;
        }
        if self.known_to_have_no_remote() {
            return OpenFetch::NoRemote;
        }
        let Ok(permit) = Arc::clone(&self.auto_fetch_slot).try_acquire_owned() else {
            tracing::debug!("opening fetch skipped: an automatic fetch is already running");
            return OpenFetch::Busy;
        };
        let timeout = self.network_timeout();
        let s = Arc::clone(self);
        let accepted = self.write(
            OperationKind::OpenFetch,
            AfterWrite::Refs,
            move |exec, repo, cancel| async move {
                let _permit = permit;
                s.fetch_and_read_tags(&exec, &repo.workdir, None, timeout, &cancel)
                    .await
            },
        );
        // The permit went with the request the queue would not take.
        if accepted.is_some() {
            OpenFetch::Started
        } else {
            OpenFetch::Closed
        }
    }

    fn lock_open_fetch(&self) -> std::sync::MutexGuard<'_, OpenFetchState> {
        relock(&self.open_fetch)
    }

    /// Whether the repository is known to have no remote, where an unasked
    /// fetch exits 0 having reached nothing — a process per interval and a
    /// spinner on a greyed-out button.
    ///
    /// Read off the refs listing's cached list, the one that greys the
    /// fetch button, so the two cannot disagree. Not read yet means
    /// "fetch": that way round costs only a process.
    fn known_to_have_no_remote(&self) -> bool {
        self.remotes
            .peek(|read| read.list.is_empty())
            .unwrap_or(false)
    }

    /// Whether the interval is installed — the permission for every
    /// unasked reach for the network.
    fn auto_fetch_is_on(&self) -> bool {
        relock(&self.auto_fetch).is_some()
    }

    fn lock_auto_fetch_interval(&self) -> std::sync::MutexGuard<'_, Option<std::time::Duration>> {
        relock(&self.auto_fetch_interval)
    }

    fn install_auto_fetch(self: &Arc<Self>, interval: Option<std::time::Duration>) {
        let mut guard = relock(&self.auto_fetch);
        if let Some(previous) = guard.take() {
            previous.cancel.cancel();
        }
        let Some(interval) = interval.filter(|i| !i.is_zero()) else {
            return;
        };
        let cancel = self.root_cancel.child_token();
        let (ticks, by_hand) = tokio::sync::mpsc::unbounded_channel();
        *guard = Some(AutoFetch {
            cancel: cancel.clone(),
            ticks,
        });
        drop(guard);

        let s = Arc::clone(self);
        let acting = cancel.clone();
        // The clock is built inside the task: this is the UI thread, off
        // the runtime (`IntervalClock`).
        self.runtime.spawn(async move {
            drive_auto_fetch(IntervalClock::every(interval), cancel, by_hand, move || {
                s.auto_fetch_tick(&acting)
            })
            .await;
        });
        self.catch_up_remote_tags();
    }

    /// Reads what the remotes carry under `refs/tags/` without waiting for
    /// a fetch, when automatic fetching is on (the permission) and the
    /// graph shows tags (otherwise the chips it feeds are off screen).
    ///
    /// A background read off the write queue and out of the command log,
    /// republishing only if the answer moved: on the queue it would hold
    /// the poll out and every later write behind it
    /// (`RepoSession::running_write`).
    ///
    /// Entered from an opening that fired no fetch ([`Self::fetch_on_open`])
    /// and from [`Self::set_auto_fetch`], so granting the permission is
    /// itself a reason to look.
    pub(super) fn catch_up_remote_tags(self: &Arc<Self>) {
        drop(self.start_remote_tag_refresh());
    }

    /// The remote-tag catch-up under the same gates, returning its
    /// completion boundary — for callers that must tell "declined" from
    /// "not answered yet" (the app uses the fire-and-forget path).
    pub fn refresh_remote_tags_tracked(self: &Arc<Self>) -> RemoteTagRefreshTask {
        self.start_remote_tag_refresh()
    }

    fn start_remote_tag_refresh(self: &Arc<Self>) -> RemoteTagRefreshTask {
        if !self.auto_fetch_is_on() {
            return RemoteTagRefreshTask::ready(RemoteTagRefreshOutcome::Disabled);
        }
        if !self.log_options().include_tags {
            return RemoteTagRefreshTask::ready(RemoteTagRefreshOutcome::Hidden);
        }
        // Dropped: the read in flight asks the same remotes the same
        // question, and a repeat would only bring a badge one round trip
        // earlier ([`RemoteTagRefreshOutcome::Busy`]).
        let held = match self.remote_tags_slot.take() {
            Ok(held) => held,
            Err(freed) => {
                tracing::debug!("remote tags: the previous read has not finished");
                return self.busy_once_the_slot_is_free(freed);
            }
        };
        let s = Arc::clone(self);
        let timeout = self.network_timeout();
        let (finished, task) = RemoteTagRefreshTask::pending();
        self.runtime.spawn(async move {
            let Some(workdir) = s.workdir() else {
                drop(held);
                if finished.send(RemoteTagRefreshOutcome::Unavailable).is_err() {
                    tracing::trace!("remote-tag refresh completion was not observed");
                }
                return;
            };
            let cancel = s.root_cancel.clone();
            let read = s
                .read_remote_tags(&s.exec_background, &workdir, None, timeout, &cancel)
                .await;
            let outcome = if cancel.is_cancelled() {
                RemoteTagRefreshOutcome::Cancelled
            } else {
                if read.moved {
                    s.refresh_refs();
                }
                if read.unread.is_empty() {
                    if read.moved {
                        RemoteTagRefreshOutcome::Changed
                    } else {
                        RemoteTagRefreshOutcome::Unchanged
                    }
                } else {
                    // Outranks `Changed`: what moved shows in the refs,
                    // what went unanswered shows nowhere else.
                    tracing::debug!(unread = ?read.unread, "remote tags: not every remote answered");
                    RemoteTagRefreshOutcome::Unanswered
                }
            };
            // Released before the ack, so a request made on `outcome()`
            // does not find the slot held.
            drop(held);
            if finished.send(outcome).is_err() {
                tracing::trace!("remote-tag refresh completion was not observed");
            }
        });
        task
    }

    /// The ack of an ask the slot turned away
    /// ([`RemoteTagRefreshOutcome::Busy`]), sent once the holding read has
    /// let go (`RemoteTagSlot`).
    fn busy_once_the_slot_is_free(
        &self,
        mut freed: tokio::sync::watch::Receiver<u64>,
    ) -> RemoteTagRefreshTask {
        let (finished, task) = RemoteTagRefreshTask::pending();
        self.runtime.spawn(async move {
            if freed.changed().await.is_err() {
                tracing::debug!("remote tags: the slot went with the session under a waiting ack");
            }
            if finished.send(RemoteTagRefreshOutcome::Busy).is_err() {
                tracing::trace!("remote-tag refresh completion was not observed");
            }
        });
        task
    }

    /// Handle for stepping the running timer, in place of waiting out its
    /// interval — see [`AutoFetchTicker`]. `None` while auto fetch is off.
    pub fn auto_fetch_ticker(&self) -> Option<AutoFetchTicker> {
        let guard = relock(&self.auto_fetch);
        guard.as_ref().map(|a| AutoFetchTicker {
            ticks: a.ticks.clone(),
            stopped: a.cancel.clone(),
        })
    }

    /// Queues one automatic fetch, unless the previous one is still going.
    /// Its own op name keeps an offline laptop's repeated failures out of
    /// the error banner.
    ///
    /// Answers whether the timer still runs. The token is read under the
    /// lock a stop cancels it under, so a tick either queues before
    /// `set_auto_fetch` returns or sees the stop and queues nothing.
    fn auto_fetch_tick(self: &Arc<Self>, cancel: &CancellationToken) -> bool {
        let _stop = relock(&self.auto_fetch);
        if cancel.is_cancelled() {
            return false;
        }
        if self.known_to_have_no_remote() {
            tracing::debug!("auto fetch skipped: the repository has no remote");
            return true;
        }
        let Ok(permit) = Arc::clone(&self.auto_fetch_slot).try_acquire_owned() else {
            tracing::debug!("auto fetch skipped: the previous one has not finished");
            return true;
        };
        let timeout = self.network_timeout();
        let s = Arc::clone(self);
        let accepted = self.write(
            OperationKind::AutoFetch,
            AfterWrite::Refs,
            move |exec, repo, cancel| async move {
                let _permit = permit;
                s.fetch_and_read_tags(&exec, &repo.workdir, None, timeout, &cancel)
                    .await
            },
        );
        // A queue that takes nothing is a closed session's: the timer ends.
        accepted.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stopped timer's refusal comes off the stop token alone: nothing
    /// polls a cancelled task on any deadline, and a `tick` waiting on it
    /// would hang. Modelled as a timer whose task never runs — the channel
    /// open, nobody answering.
    #[tokio::test]
    async fn a_stopped_timer_refuses_the_tick_without_being_scheduled() {
        let stopped = CancellationToken::new();
        let (ticks, keep_open) = tokio::sync::mpsc::unbounded_channel();
        let ticker = AutoFetchTicker {
            ticks,
            stopped: stopped.clone(),
        };
        stopped.cancel();
        let refused = crate::wait::bounded("the refusal of a stopped timer", ticker.tick()).await;
        assert!(!refused, "a stopped timer never takes a tick");
        drop(keep_open);
    }

    /// A clock stepped by hand: due only when the test says so, and
    /// pending forever once the test has dropped its end.
    struct HandClock(tokio::sync::mpsc::UnboundedReceiver<()>);

    impl AutoFetchClock for HandClock {
        async fn due(&mut self) {
            if self.0.recv().await.is_none() {
                std::future::pending::<()>().await;
            }
        }
    }

    /// The timer's loop over a hand clock, reporting each act on a channel.
    fn driven(
        stop: &CancellationToken,
    ) -> (
        tokio::sync::mpsc::UnboundedSender<()>,
        tokio::sync::mpsc::UnboundedSender<AutoFetchTick>,
        tokio::sync::mpsc::UnboundedReceiver<()>,
        tokio::task::JoinHandle<()>,
    ) {
        let (due, clock) = tokio::sync::mpsc::unbounded_channel();
        let (hand, by_hand) = tokio::sync::mpsc::unbounded_channel();
        let (acted, acts) = tokio::sync::mpsc::unbounded_channel();
        let timer = tokio::spawn(drive_auto_fetch(
            HandClock(clock),
            stop.clone(),
            by_hand,
            move || acted.send(()).is_ok(),
        ));
        (due, hand, acts, timer)
    }

    #[tokio::test]
    async fn a_due_tick_is_acted_on_and_the_stop_ends_the_timer() {
        let stop = CancellationToken::new();
        let (due, _hand, mut acts, timer) = driven(&stop);
        due.send(()).expect("the first due");
        due.send(()).expect("the second due");
        crate::wait::bounded("the first act", acts.recv()).await;
        crate::wait::bounded("the second act", acts.recv()).await;
        stop.cancel();
        crate::wait::bounded("the stopped timer's task", timer)
            .await
            .expect("the timer's task ended cleanly");
        assert!(
            acts.try_recv().is_err(),
            "nothing was acted on past the two ticks"
        );
    }

    /// Set up before the loop is first polled, so its first look sees the
    /// stop and the overdue ticks together.
    #[tokio::test]
    async fn a_stop_wins_over_the_ticks_it_finds_overdue() {
        let stop = CancellationToken::new();
        stop.cancel();
        let (due, clock) = tokio::sync::mpsc::unbounded_channel();
        let (_hand, by_hand) = tokio::sync::mpsc::unbounded_channel();
        for _ in 0..3 {
            due.send(()).expect("an overdue tick");
        }
        let acts = std::sync::atomic::AtomicUsize::new(0);
        crate::wait::bounded(
            "the stopped timer's loop",
            drive_auto_fetch(HandClock(clock), stop, by_hand, || {
                acts.fetch_add(1, Ordering::SeqCst);
                true
            }),
        )
        .await;
        assert_eq!(
            acts.load(Ordering::SeqCst),
            0,
            "a stopped timer acts on none of the ticks it found overdue"
        );
    }

    #[tokio::test]
    async fn an_act_that_reports_the_timer_stopped_ends_the_loop() {
        let (due, clock) = tokio::sync::mpsc::unbounded_channel();
        let (_hand, by_hand) = tokio::sync::mpsc::unbounded_channel();
        due.send(()).expect("a due tick");
        crate::wait::bounded(
            "a loop whose act said stop",
            drive_auto_fetch(HandClock(clock), CancellationToken::new(), by_hand, || {
                false
            }),
        )
        .await;
    }

    /// The ack that reaches the caller stands for a fetch already queued.
    #[tokio::test]
    async fn a_hand_stepped_tick_is_answered_once_it_was_acted_on() {
        let stop = CancellationToken::new();
        let (_due, hand, mut acts, timer) = driven(&stop);
        let (ack, taken) = tokio::sync::oneshot::channel();
        hand.send(ack).expect("the tick reaches the timer");
        crate::wait::bounded("the tick's ack", taken)
            .await
            .expect("answered");
        assert!(acts.try_recv().is_ok(), "the act came before the answer");
        stop.cancel();
        crate::wait::bounded("the stopped timer's task", timer)
            .await
            .expect("the timer's task ended cleanly");
    }

    /// Nothing is taken to learn of the release (`RemoteTagSlot`).
    #[tokio::test]
    async fn a_release_is_told_to_the_ask_behind_it_and_the_slot_is_free_at_once() {
        let slot = RemoteTagSlot::default();
        let held = slot.take().expect("the slot was free");
        let mut behind = slot.take().expect_err("the slot was held");
        drop(held);
        crate::wait::bounded("the word of the release", behind.changed())
            .await
            .expect("the slot outlives the ask");
        assert!(slot.take().is_ok(), "free the instant it was let go");
    }

    /// On a paused clock, so both edges are judged: pending one instant
    /// before the interval is out, ready once it is. Real time bounds one
    /// side only — a clock at twice the interval passes a floor.
    #[tokio::test(start_paused = true)]
    async fn a_due_falls_at_the_end_of_each_interval_and_the_first_is_a_whole_one_out() {
        let interval = std::time::Duration::from_secs(30);
        let an_instant = std::time::Duration::from_millis(1);
        let mut clock = IntervalClock::every(interval);
        for nth in 1..=3 {
            let mut due = Box::pin(clock.due());
            tokio::time::advance(interval - an_instant).await;
            assert!(
                crate::wait::poll_once(&mut due).is_pending(),
                "due {nth} came before its interval was out"
            );
            tokio::time::advance(an_instant).await;
            assert!(
                crate::wait::poll_once(&mut due).is_ready(),
                "due {nth} was still owed with its interval out"
            );
        }
    }
}
