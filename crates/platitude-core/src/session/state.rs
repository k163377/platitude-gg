//! What one session holds between operations: the derived answers, the
//! gates and slots that keep concurrent reads from tripping over each
//! other, and the shapes the log and refresh tasks share.

use super::*;

/// What the refs read last saw of the branch tip, which is everything the
/// reachability walk needs to start (see [`crate::reachable`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HeadHold {
    /// The commit HEAD is on.
    pub(super) tip: Oid,
    /// Short name of the branch HEAD is on; empty when detached.
    pub(super) branch: String,
    /// Some other ref already sits exactly on `tip`, which the listing
    /// answers on its own — no walk needed.
    pub(super) on_a_ref: bool,
}

/// One tick handed to the timer by hand rather than by the clock; the
/// timer answers on it once it has acted (see [`AutoFetchTicker`]).
pub(super) type AutoFetchTick = tokio::sync::oneshot::Sender<()>;

/// The auto-fetch timer that is running: what stops it, and the way in for
/// a tick that does not come from the clock.
pub(super) struct AutoFetch {
    pub(super) cancel: CancellationToken,
    pub(super) ticks: tokio::sync::mpsc::UnboundedSender<AutoFetchTick>,
}

/// Steps one auto-fetch timer by hand, in place of waiting out its
/// interval.
///
/// Handed out by [`RepoSession::auto_fetch_ticker`] and bound to the timer
/// that was running when it was taken, so a tick reports back whether that
/// timer is still there to take it. That is what makes "this timer fetches
/// nothing any more" something to wait for rather than a wall-clock margin
/// to guess at, which is all the tests have to go on otherwise: a fetch
/// queued a moment before the stop can start much later on a loaded
/// machine, and no length of quiet proves the next one is not coming. The
/// app only ever sets an interval.
pub struct AutoFetchTicker(pub(super) tokio::sync::mpsc::UnboundedSender<AutoFetchTick>);

impl AutoFetchTicker {
    /// Fires one tick and resolves once the timer has acted on it: `true`
    /// when it took the tick, `false` once that timer has stopped — turned
    /// off, replaced by another interval, or gone with the session. A
    /// stopped timer never takes a tick, not even one that raced its own
    /// cancellation, so `false` is the last word on it.
    pub async fn tick(&self) -> bool {
        let (ack, taken) = tokio::sync::oneshot::channel();
        if self.0.send(ack).is_err() {
            return false;
        }
        taken.await.is_ok()
    }
}

/// A queued write: what to run, what it invalidates, what to call it.
pub(super) struct WriteRequest {
    pub(super) op: &'static str,
    pub(super) after: AfterWrite,
    #[expect(
        clippy::type_complexity,
        reason = "a boxed async job needs its shape spelled out"
    )]
    pub(super) run: Box<
        dyn FnOnce(
                GitExecutor,
                RepoInfo,
                CancellationToken,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<(), GitError>> + Send>,
            > + Send,
    >,
}

/// What a pass reported *under* its graph rather than in it: how far the
/// walk got, and whether it stopped because the window ran out.
///
/// Kept beside the rows because it does not follow from them. The walk
/// count drifts from the shown count in both directions (the WIP row is
/// shown but never walked, sifted stash parents are walked but never
/// shown), and truncation is a property of the walk alone — so two passes
/// can draw the very same graph and still owe the consumer different
/// answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Footer {
    pub(super) walked: u32,
    pub(super) truncated: bool,
}

/// Shared mutable state between the log task and refs joins.
#[derive(Default)]
pub(super) struct Shared {
    pub(super) builder: GraphBuilder,
    /// The graph the consumer is on — the last pass that reached it, in
    /// the row numbers `builder`, `applied` and `sent_rows` speak in.
    /// Not `log_gen`: that counter is bumped before a pass takes this
    /// lock and bumped by passes that send nothing at all, and either way
    /// everything here still belongs to the walk that was shown.
    pub(super) generation: u64,
    /// Label chips per commit id, derived from the last refs snapshot.
    pub(super) label_map: LabelIndex,
    /// Labels currently shown per row (for diffing on refs refresh).
    pub(super) applied: HashMap<u32, Vec<RefLabel>>,
    /// A print of each row exactly as delivered to the UI (chips
    /// included), kept so a background rebuild can tell "same picture"
    /// from "changed" and skip the swap entirely — an unchanged
    /// repository must not repaint. Prints rather than the rows because
    /// this is the part of the graph that grows with the window
    /// ([`RowPrint`]).
    pub(super) sent_rows: Vec<RowPrint>,
    /// The footer delivered with them, compared alongside the rows for the
    /// same decision: a window change can leave every row where it is and
    /// still change the answer beneath them (two commits through a window
    /// of two are cut; through a window of three they are not), and that
    /// change reaches the consumer through a rebuild whenever one overtakes
    /// the stream the change asked for. `None` = no pass has answered for
    /// this graph yet, so the next one to finish has something to say.
    pub(super) sent_footer: Option<Footer>,
}

/// One fact read out of the repository and kept until something that
/// could have changed it happens.
///
/// **The point is that the invalidation is one place.** These are not a
/// cache of a keyed lookup — each is a single answer about the repository
/// as a whole (does git normalise line endings here, what remotes are
/// configured), and every one of them is invalidated by the same two
/// events: a write landed, or the refs moved. Scattering an
/// `Option<T>` and its `= None` across the modules that happen to read it
/// is how one gets forgotten, so they are cleared together in
/// [`RepoSession::forget_derived`].
///
/// Deliberately not a cache crate. `salsa` tracks dependencies between
/// pure synchronous queries; these are async subprocess reads that can be
/// cancelled. `moka` evicts by age and size; these expire on an event and
/// never on a clock. Neither axis is this one.
pub(super) struct Derived<T> {
    state: Mutex<DerivedState<T>>,
    /// One repository read at a time. This is an async mutex because the
    /// protected work is an async subprocess, not CPU work.
    reading: tokio::sync::Mutex<()>,
}

struct DerivedState<T> {
    generation: u64,
    value: Option<T>,
}

impl<T> Default for Derived<T> {
    fn default() -> Self {
        Self {
            state: Mutex::new(DerivedState {
                generation: 0,
                value: None,
            }),
            reading: tokio::sync::Mutex::new(()),
        }
    }
}

impl<T: Clone> Derived<T> {
    /// Returns the current answer, or lets exactly one caller read it.
    ///
    /// A plain `get` followed by an async read followed by `put` lets every
    /// concurrent caller observe the same miss and spawn the same git
    /// process. The async gate makes that sequence single-flight; callers
    /// waiting behind it re-check the value rather than repeat the read.
    ///
    /// Invalidation does not wait for a slow read. It advances the
    /// generation and clears the value synchronously. A reader that then
    /// returns from git sees that its answer belonged to the old generation
    /// and goes round again instead of restoring stale state.
    pub(super) async fn get_or_try_init<E, F, Fut>(&self, mut read: F) -> Result<T, E>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        if let Some(value) = self.lock_state().value.clone() {
            return Ok(value);
        }

        let _reading = self.reading.lock().await;
        loop {
            let generation = {
                let state = self.lock_state();
                if let Some(value) = state.value.clone() {
                    return Ok(value);
                }
                state.generation
            };

            let value = read().await?;
            let mut state = self.lock_state();
            if state.generation == generation {
                state.value = Some(value.clone());
                return Ok(value);
            }
            // Something invalidated the answer while git was reading it.
            // Keep the gate and read the current generation before waking
            // callers that are waiting for the same answer.
        }
    }

    pub(super) fn forget(&self) {
        let mut state = self.lock_state();
        state.generation = state.generation.wrapping_add(1);
        state.value = None;
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, DerivedState<T>> {
        match self.state.lock() {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

/// Guards snapshot-replacing ops against out-of-order completion.
#[derive(Default)]
pub(super) struct OpGate(AtomicU64);

impl OpGate {
    pub(super) fn begin(&self) -> u64 {
        self.0.fetch_add(1, Ordering::SeqCst) + 1
    }
    pub(super) fn is_current(&self, generation: u64) -> bool {
        self.0.load(Ordering::SeqCst) == generation
    }
}

/// One read of a snapshot at a time, with at most one repeat booked
/// behind it.
///
/// Nothing coordinates the places that ask for a re-read — opening a
/// repository asks, and so does the window becoming active a moment
/// later, which at startup is the same moment. Granting both has
/// [`OpGate`] throw the older answer away: two `for-each-ref` and two
/// `status -uall` for one snapshot, which on `JetBrains/kotlin` is about
/// a second of disk work landing exactly where the first click goes.
///
/// The second caller does not start its own read and does not lose its
/// request either — it books the repeat, and the read in flight goes
/// round again when it lands. That distinction is the whole point: a
/// dropped request would leave a write's own refresh reading the
/// repository as it was *before* the write, with the correction waiting
/// on the next poll tick.
#[derive(Default)]
pub(super) struct ReadSlot(std::sync::atomic::AtomicU8);

/// Nobody is reading.
const SLOT_IDLE: u8 = 0;
/// A read is in flight.
const SLOT_RUNNING: u8 = 1;
/// A read is in flight and somebody asked for another behind it.
const SLOT_AGAIN: u8 = 2;

impl ReadSlot {
    /// Whether this caller is the one that runs. `false` = a read is
    /// already in flight and has been booked to repeat.
    pub(super) fn claim(&self) -> bool {
        let mut seen = self.0.load(Ordering::SeqCst);
        loop {
            let next = if seen == SLOT_IDLE {
                SLOT_RUNNING
            } else {
                SLOT_AGAIN
            };
            match self
                .0
                .compare_exchange(seen, next, Ordering::SeqCst, Ordering::SeqCst)
            {
                Ok(_) => return seen == SLOT_IDLE,
                Err(actual) => seen = actual,
            }
        }
    }

    /// Called by the reader when its pass lands. `true` = somebody asked
    /// while it was running, so it goes round once more.
    pub(super) fn finish(&self) -> bool {
        loop {
            if self
                .0
                .compare_exchange(SLOT_RUNNING, SLOT_IDLE, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return false;
            }
            if self
                .0
                .compare_exchange(SLOT_AGAIN, SLOT_RUNNING, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return true;
            }
        }
    }

    /// Opens the slot from wherever it was left.
    ///
    /// For the reader that never reached [`Self::finish`] — its task was
    /// dropped with the runtime, or it unwound. Without this, a slot left
    /// running turns one lost pass into a section of the window that
    /// never updates again and never says why, which is a far worse
    /// failure than the duplicate read the slot exists to stop.
    pub(super) fn abandon(&self) {
        self.0.store(SLOT_IDLE, Ordering::SeqCst);
    }
}

/// Opens a [`ReadSlot`] when the reader holding it goes away without
/// finishing (see [`ReadSlot::abandon`]).
pub(super) struct SlotHeld<'a>(pub(super) &'a ReadSlot);

impl Drop for SlotHeld<'_> {
    fn drop(&mut self) {
        self.0.abandon();
    }
}

/// What is known about a path's line endings before its patch is read.
pub(super) enum EndingContext {
    /// git calls the path something other than text, so nothing is said
    /// about it. Also where a failed reading lands: not knowing whether a
    /// path is binary is a reason to stay quiet, not to guess.
    Excluded,
    /// Worth reading, with the neighbours' opinion if one was needed and
    /// could be had.
    Open(Option<crate::eol::Baseline>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn concurrent_callers_share_one_derived_read() {
        let derived = Arc::new(Derived::<u32>::default());
        let calls = Arc::new(AtomicU64::new(0));
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let release = Arc::new(tokio::sync::Notify::new());

        let first = {
            let derived = Arc::clone(&derived);
            let calls = Arc::clone(&calls);
            let release = Arc::clone(&release);
            tokio::spawn(async move {
                let mut entered_tx = Some(entered_tx);
                derived
                    .get_or_try_init(|| {
                        calls.fetch_add(1, Ordering::SeqCst);
                        if let Some(tx) = entered_tx.take() {
                            let _ = tx.send(());
                        }
                        let release = Arc::clone(&release);
                        async move {
                            release.notified().await;
                            Ok::<u32, ()>(42)
                        }
                    })
                    .await
            })
        };
        entered_rx.await.expect("the first read started");

        let second = {
            let derived = Arc::clone(&derived);
            let calls = Arc::clone(&calls);
            tokio::spawn(async move {
                derived
                    .get_or_try_init(|| {
                        calls.fetch_add(1, Ordering::SeqCst);
                        async { Ok::<u32, ()>(99) }
                    })
                    .await
            })
        };
        release.notify_one();

        assert_eq!(first.await.expect("first caller finished"), Ok(42));
        assert_eq!(second.await.expect("second caller finished"), Ok(42));
        assert_eq!(calls.load(Ordering::SeqCst), 1, "one shared read");
    }

    #[tokio::test]
    async fn invalidation_during_a_derived_read_retries_before_publishing() {
        let derived = Arc::new(Derived::<u32>::default());
        let calls = Arc::new(AtomicU64::new(0));
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let release = Arc::new(tokio::sync::Notify::new());

        let reader = {
            let derived = Arc::clone(&derived);
            let calls = Arc::clone(&calls);
            let release = Arc::clone(&release);
            tokio::spawn(async move {
                let mut entered_tx = Some(entered_tx);
                derived
                    .get_or_try_init(|| {
                        let call = calls.fetch_add(1, Ordering::SeqCst) + 1;
                        if let Some(tx) = entered_tx.take() {
                            let _ = tx.send(());
                        }
                        let release = Arc::clone(&release);
                        async move {
                            if call == 1 {
                                release.notified().await;
                            }
                            Ok::<u32, ()>(call as u32)
                        }
                    })
                    .await
            })
        };

        entered_rx.await.expect("the old-generation read started");
        derived.forget();
        release.notify_one();

        assert_eq!(reader.await.expect("reader finished"), Ok(2));
        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "the stale read was retried"
        );
        let held = derived
            .get_or_try_init(|| async { Ok::<u32, ()>(99) })
            .await;
        assert_eq!(held, Ok(2), "only the current generation was cached");
    }

    #[test]
    fn the_first_caller_reads_and_the_second_books_one_more_pass() {
        let slot = ReadSlot::default();
        assert!(slot.claim(), "nothing was running");
        assert!(!slot.claim(), "a read is in flight, so this one waits");
        assert!(!slot.claim(), "and so does the next");
        // Two callers queued behind one read, and one repeat answers both:
        // they asked for the same thing.
        assert!(slot.finish(), "somebody asked while it ran");
        assert!(!slot.finish(), "nobody asked during the repeat");
        assert!(slot.claim(), "and the slot is free again");
    }

    #[test]
    fn a_reader_that_never_finishes_does_not_take_the_slot_with_it() {
        let slot = ReadSlot::default();
        assert!(slot.claim());
        assert!(!slot.claim(), "and somebody is waiting behind it");
        drop(SlotHeld(&slot));
        assert!(
            slot.claim(),
            "the next ask runs rather than waiting on a reader that is gone"
        );
    }

    #[test]
    fn a_request_that_arrives_as_a_read_lands_is_not_lost() {
        let slot = ReadSlot::default();
        assert!(slot.claim());
        assert!(!slot.claim());
        assert!(slot.finish());
        // Mid-repeat, a third caller: still exactly one more pass.
        assert!(!slot.claim());
        assert!(slot.finish());
        assert!(!slot.finish());
    }
}
