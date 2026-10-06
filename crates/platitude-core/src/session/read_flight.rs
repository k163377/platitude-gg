//! Single-flight ownership for snapshot readers.

use super::*;

/// One read of a snapshot at a time, answering every caller that asked
/// before the pass it waited on started.
///
/// Nothing coordinates the places that ask for a re-read (the tick, a
/// write settling, the window activating, a dialog), and a
/// `status --porcelain=v2 -uall` is a pass over every file of the tree
/// (ci/baseline/code-costs-windows-x64.md) — run once per asker, it is
/// spent twice exactly where the reader is waiting.
///
/// The ordering rides on the same gate: a pass holds it from before it
/// looks until after it has published, so an older answer can never
/// overtake a newer one and nothing downstream needs a generation.
///
/// A caller that arrives while a pass is running waits for the gate,
/// and then either
///
/// * a pass **that started after it asked** has landed, and its answer
///   is the caller's — no process at all; or
/// * it runs the next pass itself, and everyone who asked before that
///   pass began reads the answer it lands.
///
/// No caller is ever answered by a read that looked before its reason
/// existed: a write settled by an older pass would rebuild the graph
/// from the repository as it was *before* the write.
///
/// `A` is what a pass answers its callers with (`Reread`, `StashRead`,
/// `WorktreeRead`; a `bool` where nothing but "it landed" is wanted).
pub(super) struct ReadFlight<A = bool> {
    /// One pass at a time. tokio hands it on in the order it was asked
    /// for, so no caller is passed over; which pass answers whom is the
    /// stamps' to decide.
    gate: tokio::sync::Mutex<()>,
    passes: Mutex<Passes<A>>,
    /// Callers that have taken a stamp and not yet left, the waiting ones
    /// included, so a boundary can close the work already in flight.
    live: std::sync::atomic::AtomicUsize,
    /// Bumped on both edges of `live`, so a waiter on either count is
    /// woken at once.
    changed: tokio::sync::watch::Sender<u64>,
}

#[derive(Default)]
struct Passes<A> {
    /// Read by a caller before it queues: a landed pass numbered above it
    /// began after the caller had its reason.
    started: u64,
    /// Which pass landed last, and what it answered.
    landed: u64,
    answer: A,
}

impl<A: Default> Default for ReadFlight<A> {
    fn default() -> Self {
        let (changed, _) = tokio::sync::watch::channel(0);
        Self {
            gate: tokio::sync::Mutex::default(),
            passes: Mutex::new(Passes::default()),
            live: std::sync::atomic::AtomicUsize::new(0),
            changed,
        }
    }
}

/// A caller's place in the flight, taken before it asks for the gate.
/// Separate from the run for the caller that asks from inside a pass: a
/// fenced read (`Standing::current`) stamps while it still holds the
/// gate, so the next pass answers it instead of a third read. Counted
/// as a caller from here until it is run.
#[must_use = "a stamp is a caller until it is run"]
pub(super) struct Stamp {
    /// Passes that had started when this caller asked.
    asked: u64,
}

impl<A> ReadFlight<A> {
    /// Takes a caller's place now, to be run later ([`Self::run_from`]).
    pub(super) fn stamp(&self) -> Stamp {
        // One lock for both: a caller counted in `live` has already
        // chosen its pass.
        let asked = {
            let passes = relock(&self.passes);
            self.live.fetch_add(1, Ordering::SeqCst);
            passes.started
        };
        self.woken();
        Stamp { asked }
    }

    /// Answers this caller, running the read only where no pass that
    /// started after it asked has already answered the same question.
    pub(super) async fn run<F, Fut>(&self, read: F) -> A
    where
        A: Clone,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = A>,
    {
        self.run_from(self.stamp(), read).await
    }

    /// [`Self::run`] for a caller whose place was taken earlier
    /// ([`Self::stamp`]).
    pub(super) async fn run_from<F, Fut>(&self, stamp: Stamp, read: F) -> A
    where
        A: Clone,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = A>,
    {
        let stamped = Live {
            flight: self,
            asked: stamp.asked,
        };
        let _gate = self.gate.lock().await;
        {
            let passes = relock(&self.passes);
            if passes.landed > stamped.asked {
                return passes.answer.clone();
            }
        }
        let mine = {
            let mut passes = relock(&self.passes);
            passes.started += 1;
            passes.started
        };
        let answer = read().await;
        // Recorded under the gate. A pass that never gets here (dropped
        // or unwound) leaves `started` ahead of `landed`, which costs the
        // next caller a read of its own, never its answer.
        let mut passes = relock(&self.passes);
        passes.landed = mine;
        passes.answer = answer.clone();
        answer
    }

    /// Waits until no stamped caller is left — the boundary that closes
    /// the work already in flight.
    pub(super) async fn wait_idle(&self) {
        self.wait_for_live(|live| live == 0).await;
    }

    /// Counts a caller in beyond its read, until [`Self::leave`]: for one
    /// that acts on the answer after leaving the flight — asks for the
    /// rebuild it implies once its other reads are in — so the boundary
    /// that closes the flight ([`Self::wait_idle`]) closes that act too.
    pub(super) fn enter(&self) {
        self.live.fetch_add(1, Ordering::SeqCst);
        self.woken();
    }

    /// Counts out a caller [`Self::enter`] counted in.
    pub(super) fn leave(&self) {
        self.live.fetch_sub(1, Ordering::SeqCst);
        self.woken();
    }

    async fn wait_for_live(&self, settled: impl Fn(usize) -> bool) {
        let mut changed = self.changed.subscribe();
        while !settled(self.live.load(Ordering::SeqCst)) {
            if changed.changed().await.is_err() {
                return;
            }
        }
    }

    fn woken(&self) {
        self.changed
            .send_modify(|generation| *generation = generation.wrapping_add(1));
    }
}

/// Counts a caller of [`ReadFlight::run_from`] out, as the stamp counted
/// it in — on drop, so a caller that unwound or went down with the
/// runtime still leaves and the boundary can close.
struct Live<'a, A> {
    flight: &'a ReadFlight<A>,
    asked: u64,
}

impl<A> Drop for Live<'_, A> {
    fn drop(&mut self) {
        self.flight.live.fetch_sub(1, Ordering::SeqCst);
        self.flight.woken();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl<A> ReadFlight<A> {
        /// Waits until `asking` callers hold a stamp: each has chosen the
        /// pass that must answer it, so releasing the read in flight can
        /// no longer be mistaken for the repeat.
        async fn wait_for_askers(&self, asking: usize) {
            self.wait_for_live(move |live| live >= asking).await;
        }
    }

    #[derive(Default)]
    struct Ran {
        passes: std::sync::atomic::AtomicUsize,
        /// Passes reading at this moment, and the most there ever were.
        inside: std::sync::atomic::AtomicUsize,
        most: std::sync::atomic::AtomicUsize,
    }

    impl Ran {
        fn enter(&self) {
            self.passes.fetch_add(1, Ordering::SeqCst);
            let inside = self.inside.fetch_add(1, Ordering::SeqCst) + 1;
            self.most.fetch_max(inside, Ordering::SeqCst);
        }

        fn leave(&self) {
            self.inside.fetch_sub(1, Ordering::SeqCst);
        }

        fn passes(&self) -> usize {
            self.passes.load(Ordering::SeqCst)
        }

        fn most(&self) -> usize {
            self.most.load(Ordering::SeqCst)
        }
    }

    #[tokio::test]
    async fn a_caller_with_the_flight_to_itself_reads() {
        let flight = ReadFlight::default();
        let ran = Ran::default();
        assert!(
            flight
                .run(|| async {
                    ran.enter();
                    ran.leave();
                    true
                })
                .await
        );
        assert_eq!(ran.passes(), 1);
    }

    /// The second caller is polled onto the gate by hand while the first
    /// read stands open ([`crate::wait::poll_once`]), so the overlap is
    /// tried exactly where it could happen.
    #[tokio::test]
    async fn passes_of_one_flight_never_overlap() {
        let flight = ReadFlight::default();
        let ran = Ran::default();
        let release = tokio::sync::Notify::new();
        let mut first = Box::pin(flight.run(|| async {
            ran.enter();
            release.notified().await;
            ran.leave();
            false
        }));
        assert!(
            crate::wait::poll_once(&mut first).is_pending(),
            "the first read is in flight"
        );
        let mut second = Box::pin(flight.run(|| async {
            ran.enter();
            ran.leave();
            true
        }));
        assert!(
            crate::wait::poll_once(&mut second).is_pending(),
            "the second caller is parked on the gate"
        );
        assert_eq!(ran.passes(), 1, "and did not read past the first");

        release.notify_one();
        assert!(!first.await);
        assert!(second.await, "answered by the repeat behind the first");
        assert_eq!(ran.most(), 1, "two reads were in flight at once");
        assert_eq!(ran.passes(), 2, "one read and one repeat");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn callers_that_arrive_during_a_pass_share_one_repeat() {
        let flight = Arc::new(ReadFlight::default());
        let ran = Arc::new(Ran::default());
        let (reading, started) = tokio::sync::oneshot::channel();
        let (release, held) = tokio::sync::oneshot::channel();

        let first = {
            let flight = Arc::clone(&flight);
            let ran = Arc::clone(&ran);
            tokio::spawn(async move {
                flight
                    .run(move || async move {
                        ran.enter();
                        let _ = reading.send(());
                        let _ = held.await;
                        ran.leave();
                        false
                    })
                    .await
            })
        };
        started.await.expect("the first pass began");

        let mut waiting = Vec::new();
        for _ in 0..3 {
            let flight = Arc::clone(&flight);
            let ran = Arc::clone(&ran);
            waiting.push(tokio::spawn(async move {
                flight
                    .run(move || async move {
                        ran.enter();
                        ran.leave();
                        true
                    })
                    .await
            }));
        }
        // All three have chosen their pass before the read in flight is
        // let go, so the repeat they share cannot be that read.
        flight.wait_for_askers(4).await;
        let _ = release.send(());

        assert!(!first.await.expect("the first pass answered"));
        for caller in waiting {
            assert!(
                caller.await.expect("the caller answered"),
                "the repeat's answer reached everyone waiting on it"
            );
        }
        assert_eq!(ran.passes(), 2, "one read and one repeat, not one each");
    }

    /// The fenced read's shape: a stamp taken inside a pass is answered by
    /// the next pass, where one taken after the gate was let go would be
    /// numbered past it and spend a third read.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_caller_stamped_inside_a_pass_shares_the_repeat_behind_it() {
        let flight = Arc::new(ReadFlight::default());
        let ran = Arc::new(Ran::default());
        let (reading, started) = tokio::sync::oneshot::channel();
        let (release, held) = tokio::sync::oneshot::channel();

        let first = {
            let flight = Arc::clone(&flight);
            let ran = Arc::clone(&ran);
            tokio::spawn(async move {
                flight
                    .run(move || async move {
                        ran.enter();
                        let _ = reading.send(());
                        let _ = held.await;
                        ran.leave();
                        false
                    })
                    .await
            })
        };
        started.await.expect("the first pass began");
        let again = flight.stamp();

        let other = {
            let flight = Arc::clone(&flight);
            let ran = Arc::clone(&ran);
            tokio::spawn(async move {
                flight
                    .run(move || async move {
                        ran.enter();
                        ran.leave();
                        true
                    })
                    .await
            })
        };
        flight.wait_for_askers(3).await;
        let _ = release.send(());
        assert!(!first.await.expect("the first pass answered"));
        assert!(other.await.expect("the other caller answered"));

        assert!(
            flight
                .run_from(again, || async {
                    ran.enter();
                    ran.leave();
                    false
                })
                .await,
            "answered by the other caller's pass, which started after the stamp"
        );
        assert_eq!(
            ran.passes(),
            2,
            "the first read and one repeat, not a third"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_caller_is_not_answered_by_a_read_older_than_itself() {
        let flight = Arc::new(ReadFlight::default());
        // The repository: changed below while the first read holds the
        // answer it took before.
        let dirty = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (reading, started) = tokio::sync::oneshot::channel();
        let (release, held) = tokio::sync::oneshot::channel();

        let first = {
            let flight = Arc::clone(&flight);
            let dirty = Arc::clone(&dirty);
            tokio::spawn(async move {
                flight
                    .run(move || async move {
                        let seen = dirty.load(Ordering::SeqCst);
                        let _ = reading.send(());
                        let _ = held.await;
                        seen
                    })
                    .await
            })
        };
        started.await.expect("the first pass began");

        dirty.store(true, Ordering::SeqCst);
        let second = {
            let flight = Arc::clone(&flight);
            let dirty = Arc::clone(&dirty);
            tokio::spawn(async move {
                flight
                    .run(move || async move { dirty.load(Ordering::SeqCst) })
                    .await
            })
        };
        flight.wait_for_askers(2).await;
        let _ = release.send(());

        assert!(!first.await.expect("the first pass answered"));
        assert!(
            second.await.expect("the second caller answered"),
            "the caller that changed the repository read it after the change"
        );
    }
}
