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
    /// Declined: with automatic fetching off, the owner has said not to
    /// reach the network unasked, and an opening is not the thing to
    /// break that for.
    Disabled,
    /// An automatic fetch was already running, which is what this asks
    /// for — nothing more to start.
    Busy,
    /// The repository has no remote, so there is nothing to fetch from.
    NoRemote,
    /// This session has already had its opening fetch.
    Spent,
}

/// What wakes the timer between hand-stepped ticks: the interval clock
/// in the product, and whatever a test hands it. A due tick is only ever
/// *asked for* here — acting on it, and the stop that outranks it, are
/// [`drive_auto_fetch`]'s.
pub(super) trait AutoFetchClock: Send + 'static {
    /// Resolves when the next tick is due.
    fn due(&mut self) -> impl Future<Output = ()> + Send;
}

/// The product's clock: one tick per interval, the first a whole interval
/// away. tokio would fire it at once; opening the repository has just
/// read it, so that one is not owed.
///
/// **Built inside a task on the runtime.** An interval takes its time
/// driver from the runtime it is created in, and off one it panics; the
/// interval is set from the UI thread.
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

/// The timer's loop, apart from what it acts on. `act` runs one tick and
/// answers whether the timer still runs: a `false` ends the loop, as
/// does the stop.
///
/// Biased towards the stop: one that arrives while ticks are already
/// overdue (a starved timer catches up in a burst) wins over them instead
/// of being picked at random. A hand-stepped tick is answered only once
/// it has been acted on, and never by a timer that has been stopped.
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
/// ([`RemoteTagRefreshOutcome::Busy`]), and what its ack waits for is
/// the word — never the slot itself. A waiter that took the permit to
/// learn of the release would hold it for an instant, and an ask arriving
/// in that instant would be told the slot was taken when nothing was
/// reading at all.
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
    /// Takes the slot, or hands back what to wait on for the read that
    /// holds it to let it go. Subscribed before the slot is asked for, so
    /// a release landing between the two is seen rather than waited for
    /// a second time.
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

    /// Fetches once, as soon as the repository is open — the first thing
    /// a session reaches the network for.
    ///
    /// The permission is the interval, as it is everywhere else here: a
    /// repository whose owner turned automatic fetching off has said not
    /// to reach the network unasked. The application asks the instant it
    /// has handed the session its settings, so the answer is already
    /// there to read.
    ///
    /// Either end can arrive first — this call, or the opening that
    /// learns where the repository is — so whichever arrives second
    /// fires it and the other one gets [`OpenFetch::Held`]. Neither is
    /// timed against the other, which is what keeps the fetch from being
    /// lost to a slow `git rev-parse` or a main thread that was held up.
    ///
    /// Carries an op name of its own: nobody asked for this one, so it
    /// stays out of the command log, and a machine that opens a tab
    /// offline is not to have the panel thrown up at it
    /// (デザイン規約 §リモートから取り込む).
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

    /// Fires the fetch that was asked for before the repository was open,
    /// and answers whether one started — which is the opening's reason to
    /// skip the remote-tag catch-up, a fetch having read them on its way
    /// out.
    ///
    /// Reads the remote list on the way, which is the one thing the
    /// decision needs and the opening has not asked for yet. It costs a
    /// `git config` in front of the fetch, and nothing beyond it: the refs
    /// listing that follows wanted the same answer and now finds it read
    /// (`RepoSession::remotes`).
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
        self.write(
            OPEN_FETCH_OP,
            AfterWrite::Refs,
            move |exec, repo, cancel| async move {
                let _permit = permit;
                s.fetch_and_read_tags(&exec, &repo.workdir, None, timeout, &cancel)
                    .await
            },
        );
        OpenFetch::Started
    }

    fn lock_open_fetch(&self) -> std::sync::MutexGuard<'_, OpenFetchState> {
        relock(&self.open_fetch)
    }

    /// Whether the repository is known to have no remote at all, in which
    /// case an unasked fetch has nowhere to go: `fetch --prune --all`
    /// there exits clean without reaching anything (measured, git 2.55), so
    /// what it costs is a process an interval and a button that spins
    /// while it says it cannot be pressed.
    ///
    /// Read off the list the refs listing already keeps, so this asks git
    /// nothing — and the same list is what greys the fetch button out, so
    /// the two cannot disagree. An answer that is not in yet is not a
    /// "no": it fetches, which is the way round that can only cost a
    /// process.
    fn known_to_have_no_remote(&self) -> bool {
        self.remotes
            .peek(|read| read.list.is_empty())
            .unwrap_or(false)
    }

    /// Whether the interval is installed — the permission that every
    /// unasked reach for the network is measured against.
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
        // The clock is built on the runtime, not here: an interval takes
        // its time driver from the runtime it is created in, and this is
        // the thread that set the interval — the UI's, in the app.
        self.runtime.spawn(async move {
            drive_auto_fetch(IntervalClock::every(interval), cancel, by_hand, move || {
                s.auto_fetch_tick(&acting)
            })
            .await;
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
    /// Entered twice: from an opening that fired no fetch of its own
    /// ([`Self::fetch_on_open`] — one that did has read the remotes on
    /// its way out), and from [`Self::set_auto_fetch`] afterwards, so
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
        if !self.auto_fetch_is_on() {
            return RemoteTagRefreshTask::ready(RemoteTagRefreshOutcome::Disabled);
        }
        if !self.log_options().include_tags {
            return RemoteTagRefreshTask::ready(RemoteTagRefreshOutcome::Hidden);
        }
        // Dropped, not booked behind the read in flight: that read is
        // asking the same remotes the same question, and no repeat would
        // carry anything it cannot already see
        // ([`RemoteTagRefreshOutcome::Busy`]). What a second `ls-remote`
        // per collision would buy on the reference repository — 45,000
        // tags — is one badge round trip earlier, on the path whose whole
        // budget is a badge.
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
            drop(held);
            if finished.send(outcome).is_err() {
                tracing::trace!("remote-tag refresh completion was not observed");
            }
        });
        task
    }

    /// The ack of an ask the slot turned away, booked behind the read
    /// that holds it ([`RemoteTagRefreshOutcome::Busy`]): sent once that
    /// read has let the slot go, with nothing read and nothing taken on
    /// the way — the word of the release is waited for, never the slot,
    /// so the ask that comes next finds it free. The app drops the task,
    /// and what the booking costs it is a spawn per collision — one a
    /// session at most, the opening's catch-up against the interval's.
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
        self.write(
            AUTO_FETCH_OP,
            AfterWrite::Refs,
            move |exec, repo, cancel| async move {
                let _permit = permit;
                s.fetch_and_read_tags(&exec, &repo.workdir, None, timeout, &cancel)
                    .await
            },
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The refusal of a stopped timer must not wait for its task to be
    /// polled: nothing schedules a cancelled task on any deadline, and
    /// under load one sat unpolled for a test suite's whole overall
    /// budget while the caller of `tick` hung with it. Modelled
    /// as a timer whose task never runs at all — the channel stays open,
    /// nobody will answer, and the stop token is the only word there is.
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

    /// A clock stepped by hand: due when the test says so, and never on
    /// its own. Once the test has dropped its end, never again.
    struct HandClock(tokio::sync::mpsc::UnboundedReceiver<()>);

    impl AutoFetchClock for HandClock {
        async fn due(&mut self) {
            if self.0.recv().await.is_none() {
                std::future::pending::<()>().await;
            }
        }
    }

    /// The timer's loop over a hand clock, saying each act on a channel
    /// as it happens — an act is proved by the word of it.
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

    /// Each due tick is acted on once, and the stop ends the loop with
    /// nothing acted on after it.
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

    /// A stop that finds ticks already overdue wins over every one of
    /// them: the loop is biased that way, so a starved timer catching up
    /// in a burst does not fetch on the way out. Set up before the loop
    /// is ever polled, so the first look sees both at once.
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

    /// An act that answers "the timer has stopped" ends the loop by
    /// itself, with no stop token needed.
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

    /// A hand-stepped tick is answered after it was acted on: the ack
    /// that reaches the caller stands for a fetch already queued.
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

    /// The word of a release reaches the ask booked behind the read, and
    /// the slot is free the instant it is said: nothing was taken to
    /// learn of it.
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

    /// The product's clock owes its first due a whole interval out — not
    /// the tick tokio fires at once — and one to every interval after it.
    ///
    /// On a paused clock, which is what lets both edges of the interval
    /// be judged: the due is still owed one instant before the interval
    /// is out, and owed no longer once it is. Real time can judge either
    /// of those as a bound only, and a clock running at twice the
    /// interval passes a floor.
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
