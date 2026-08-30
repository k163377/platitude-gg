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

/// How a config file looked, closely enough to tell "somebody wrote this"
/// from "nobody has touched it".
///
/// A missing file has a stamp of its own, so one that appears later reads
/// as the change it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConfigStamp {
    modified: Option<std::time::SystemTime>,
    len: u64,
}

impl ConfigStamp {
    pub(super) fn of(path: &Path) -> Self {
        match std::fs::metadata(path) {
            Ok(meta) => Self {
                modified: meta.modified().ok(),
                len: meta.len(),
            },
            Err(_) => Self {
                modified: None,
                len: 0,
            },
        }
    }
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

/// Where the fetch that opening a repository fires stands.
///
/// Two ends reach for it and either can arrive first: the application asks
/// once it has handed the session its settings, and the opening is what
/// learns where the repository is. Held under one lock, so whichever
/// arrives second fires it and it is fired exactly once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OpenFetchState {
    /// Nobody has asked for one.
    Unasked,
    /// Asked for before the repository was open; the opening redeems it.
    Held,
    /// Started, declined, or already spent — this session asks no more.
    Settled,
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
pub struct AutoFetchTicker {
    pub(super) ticks: tokio::sync::mpsc::UnboundedSender<AutoFetchTick>,
    /// The stop of the timer this ticker is bound to, read directly: the
    /// refusal must not wait for the stopped task to come round and drop
    /// the channel, because nothing schedules a cancelled task on any
    /// deadline — under load one sat unpolled for the whole of a test
    /// suite's overall budget while the rest of the session ran on
    /// (measured), and a caller awaiting its answer hung with it.
    pub(super) stopped: CancellationToken,
}

impl AutoFetchTicker {
    /// Fires one tick and resolves once the timer has acted on it: `true`
    /// when it took the tick, `false` once that timer has stopped — turned
    /// off, replaced by another interval, or gone with the session. A
    /// stopped timer never takes a tick, not even one that raced its own
    /// cancellation, so `false` is the last word on it.
    pub async fn tick(&self) -> bool {
        let (ack, taken) = tokio::sync::oneshot::channel();
        if self.ticks.send(ack).is_err() {
            return false;
        }
        // Biased towards the answer: a tick that was acted on says so even
        // when the stop lands right behind it. The stop token is the other
        // half of the race — a stopped timer's task answers by dropping
        // the channel, but only when it is next polled, and its stop must
        // not hang on that.
        tokio::select! {
            biased;
            answered = taken => answered.is_ok(),
            () = self.stopped.cancelled() => false,
        }
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
/// These are not a cache of a keyed lookup — each is a single answer about
/// the repository as a whole (does git normalise line endings here, what
/// remotes are configured). Invalidation stays in the two event boundaries
/// that own it: before a write's refresh, and after an external ref move.
/// The latter has already read the current remote answer, so line-ending
/// context is dropped immediately while remotes are dropped only for the
/// following read (see [`RepoSession::forget_derived`]).
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

    /// Reads the answer as it stands, without asking git for one. `None`
    /// where nothing has been read yet or the last answer was dropped —
    /// which a caller must not read as an answer of its own.
    pub(super) fn peek<R>(&self, read: impl FnOnce(&T) -> R) -> Option<R> {
        self.lock_state().value.as_ref().map(read)
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, DerivedState<T>> {
        super::relock(&self.state)
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
            let (started_tx, started_rx) = tokio::sync::oneshot::channel();
            let task = tokio::spawn(async move {
                let _ = started_tx.send(());
                derived
                    .get_or_try_init(|| {
                        calls.fetch_add(1, Ordering::SeqCst);
                        async { Ok::<u32, ()>(99) }
                    })
                    .await
            });
            started_rx
                .await
                .expect("the second caller reached the read");
            // Let it run through the fast-path miss and park on the
            // single-flight gate before the first answer is published.
            tokio::task::yield_now().await;
            task
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
}
