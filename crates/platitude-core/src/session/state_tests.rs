//! Tests of [`super::state`]'s derived reads, in a file of their own
//! (structure.md §分割).

// Both globs: `Arc` / `AtomicU64` / `Ordering` come off the session's
// prelude, as in `state` (rules-refs/structure.md §分割の各論).
use super::state::*;
use super::*;

/// Two callers miss at once: the second, driven by hand past the fast-path
/// miss onto the gate the first holds ([`crate::wait::poll_once`]), is
/// answered by the first's read.
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

    let mut second = Box::pin(derived.get_or_try_init(|| {
        calls.fetch_add(1, Ordering::SeqCst);
        async { Ok::<u32, ()>(99) }
    }));
    assert!(
        crate::wait::poll_once(&mut second).is_pending(),
        "the second caller is parked on the single-flight gate"
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "and read nothing on its way there"
    );

    release.notify_one();
    assert_eq!(first.await.expect("first caller finished"), Ok(42));
    assert_eq!(second.await, Ok(42), "answered by the read in flight");
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

/// The chase after a lost generation is one read long
/// (`Derived::get_or_try_init`).
#[tokio::test]
async fn an_answer_invalidated_on_every_read_is_still_an_answer() {
    let derived = Derived::<u32>::default();
    let calls = AtomicU64::new(0);
    let got = derived
        .get_or_try_init(|| {
            let call = calls.fetch_add(1, Ordering::SeqCst) + 1;
            derived.forget();
            async move { Ok::<u32, ()>(call as u32) }
        })
        .await;
    assert_eq!(got, Ok(2), "the second reading is the answer");
    assert_eq!(calls.load(Ordering::SeqCst), 2, "the chase is bounded");

    let after = derived.get_or_try_init(|| async { Ok::<u32, ()>(9) }).await;
    assert_eq!(after, Ok(9), "the outrun reading was not cached");
}
