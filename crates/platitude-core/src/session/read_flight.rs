//! Single-flight ownership for snapshot readers.

use super::*;

/// One read of a snapshot at a time, answering every caller that asked
/// before the pass it waited on started.
///
/// Nothing coordinates the places that ask for a re-read: the periodic
/// tick, a write settling behind itself, the window becoming active, a
/// dialog that wants one field of the answer. Letting each start its own
/// read has [`OpGate`] throw the older answer away, but by then both
/// processes have run — and a `status --porcelain=v2 -uall` over the
/// benchmark corpus is 2.9 seconds of wall clock and 2.3 of CPU, spent
/// twice, exactly where the reader is waiting.
///
/// A caller that arrives while a pass is running neither starts its own
/// nor loses its request. It waits for the gate, and then either
///
/// * a pass **that started after it asked** has landed, so that pass
///   looked at the repository the caller is asking about and its answer
///   is the caller's answer — no process at all; or
/// * it runs the next pass itself, and everyone who asked before that
///   pass began reads the answer it lands.
///
/// So a burst collapses to one repeat rather than one read each, and no
/// caller is ever answered by a read that looked before its reason
/// existed. That second half is the one that cannot be traded away: a
/// write settled by a pass older than itself would have the graph
/// rebuilt from the repository as it was *before* the write, with the
/// correction waiting on the next poll tick.
pub(super) struct ReadFlight {
    /// One pass at a time. An async mutex because what it guards is a git
    /// subprocess rather than CPU work; tokio hands it on in the order it
    /// was asked for, which is what keeps a caller from being passed over
    /// while others read. Which pass may answer whom is decided by the
    /// stamps below and not by that order.
    gate: tokio::sync::Mutex<()>,
    passes: Mutex<Passes>,
    /// Callers that have taken a stamp and not yet left, the waiting ones
    /// included, so a boundary can close the work already in flight.
    live: std::sync::atomic::AtomicUsize,
    /// Bumped on both edges of `live`, so a waiter on either count is
    /// woken rather than left to look again.
    changed: tokio::sync::watch::Sender<u64>,
}

#[derive(Default)]
struct Passes {
    /// How many passes have started. A caller reads this before it queues
    /// and compares it with what landed: anything numbered above it began
    /// after the caller had its reason.
    started: u64,
    /// Which pass landed last, and what it answered.
    landed: u64,
    answer: bool,
}

impl Default for ReadFlight {
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

impl ReadFlight {
    /// Answers this caller, running the read only where no pass that
    /// started after it asked has already answered the same question.
    ///
    /// `read` reports whatever its callers act on — the working-tree row
    /// that moved, the refs that moved. A reader with nothing to answer
    /// says `false`.
    pub(super) async fn run<F, Fut>(&self, read: F) -> bool
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = bool>,
    {
        // The stamp is taken before the gate is asked for, so a pass
        // numbered above it is known to have started afterwards.
        let stamped = Live::stamp(self);
        let _gate = self.gate.lock().await;
        {
            let passes = relock(&self.passes);
            if passes.landed > stamped.asked {
                return passes.answer;
            }
        }
        let mine = {
            let mut passes = relock(&self.passes);
            passes.started += 1;
            passes.started
        };
        let answer = read().await;
        // Recorded while the gate is still held, so the caller it goes to
        // reads this pass rather than the one before it. A pass that never
        // reaches here — its task dropped with the runtime, or it unwound
        // — leaves `started` ahead of `landed`, which costs the next
        // caller a read of its own and never an answer.
        let mut passes = relock(&self.passes);
        passes.landed = mine;
        passes.answer = answer;
        answer
    }

    /// Waits until the callers that had taken a stamp when this was
    /// called have left. A later one may arrive; this closes the work
    /// already in flight rather than reserving silence.
    pub(super) async fn wait_idle(&self) {
        self.wait_for_live(|live| live == 0).await;
    }

    /// Waits until the count of stamped callers is what `settled` accepts.
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

/// One caller of [`ReadFlight::run`], from the stamp it took to the
/// answer it leaves with.
///
/// Dropped by the caller itself, so one that unwound or went down with
/// the runtime still reports that it has left rather than holding the
/// boundary open forever.
struct Live<'a> {
    flight: &'a ReadFlight,
    /// Passes that had started when this caller asked.
    asked: u64,
}

impl<'a> Live<'a> {
    fn stamp(flight: &'a ReadFlight) -> Self {
        // One lock for both, so that a caller counted here is a caller
        // whose stamp is already taken: what the count means is "asking
        // for a pass no older than this one", and a caller registered
        // before its stamp would not have chosen its pass yet.
        let asked = {
            let passes = relock(&flight.passes);
            flight.live.fetch_add(1, Ordering::SeqCst);
            passes.started
        };
        flight.woken();
        Self { flight, asked }
    }
}

impl Drop for Live<'_> {
    fn drop(&mut self) {
        self.flight.live.fetch_sub(1, Ordering::SeqCst);
        self.flight.woken();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl ReadFlight {
        /// Callers that have taken a stamp and not yet left. A test waits
        /// on this rather than on a moment: a caller counted here has
        /// chosen the pass that must answer it, so releasing the read in
        /// flight can no longer be mistaken for the repeat.
        async fn wait_for_askers(&self, asking: usize) {
            self.wait_for_live(move |live| live >= asking).await;
        }
    }

    /// What the passes of one flight did, as the reads themselves saw it.
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

    /// The reason the gate is here at all: however many callers pile up,
    /// two reads of the same snapshot never run at once.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn passes_of_one_flight_never_overlap() {
        let flight = Arc::new(ReadFlight::default());
        let ran = Arc::new(Ran::default());
        let mut callers = Vec::new();
        for _ in 0..8 {
            let flight = Arc::clone(&flight);
            let ran = Arc::clone(&ran);
            callers.push(tokio::spawn(async move {
                flight
                    .run(move || async move {
                        ran.enter();
                        tokio::task::yield_now().await;
                        ran.leave();
                        false
                    })
                    .await
            }));
        }
        for caller in callers {
            caller.await.expect("the caller answered");
        }
        assert_eq!(ran.most(), 1, "two reads were in flight at once");
    }

    /// Everyone who asked while one pass was running is answered by the
    /// single repeat behind it.
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
        // All three have chosen the pass that must answer them before the
        // read in flight is let go, so the repeat they share cannot be
        // that read.
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

    /// The distinction the whole thing is for: a caller is never handed
    /// the answer of a read that looked before its reason existed.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_caller_is_not_answered_by_a_read_older_than_itself() {
        let flight = Arc::new(ReadFlight::default());
        // What the repository says. The caller below changes it while the
        // read in flight is holding the answer it took before that.
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
