//! The auto-fetch timer, and the fetch a refused push asks for.

use std::time::Duration;

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, opened};
use platitude_core::session::SessionEvent;

/// Waits for the `nth` automatic fetch to finish and returns git's error,
/// if any. Telling them apart is the point: only the second one can be laid
/// at the clock's door.
async fn auto_fetch_done(sink: &CaptureSink, nth: usize) -> Option<String> {
    sink.wait_for("an automatic fetch", |evs| {
        evs.iter()
            .filter_map(|e| match e {
                SessionEvent::WriteFinished { op, error }
                    if *op == platitude_core::session::AUTO_FETCH_OP =>
                {
                    Some(error.clone())
                }
                _ => None,
            })
            .nth(nth - 1)
    })
    .await
}

/// The auto-fetch timer runs the fetch it promises, and turns off again.
///
/// Both halves are read off the timer — a tick it takes, a tick it refuses
/// once stopped — rather than off a stretch of quiet clock: a fetch queued
/// a moment before the stop starts whenever the write queue reaches it,
/// which on a loaded machine is long after any margin worth waiting, so no
/// amount of silence tells "stopped" from "slow".
#[tokio::test(flavor = "multi_thread")]
async fn auto_fetch_runs_on_its_interval_and_stops() {
    let mut origin = TestRepo::init();
    origin.commit_file("f.txt", "0\n", "root");
    let mut clone = TestRepo::init();
    let url = origin.file_url();
    clone.git(&["remote", "add", "origin", &url]);

    let (sink, session) = opened(&clone).await;
    // An hour, so nothing but the tick below can fire this one and the
    // fetch that follows is that tick's doing and nothing else's.
    session.set_auto_fetch(Some(Duration::from_secs(3600)));
    let hourly = session.auto_fetch_ticker().expect("auto fetch is on");
    assert!(hourly.tick().await, "the running timer took the tick");
    assert_eq!(
        auto_fetch_done(&sink, 1).await,
        None,
        "the file:// remote fetched cleanly"
    );
    assert_eq!(
        clone.git(&["rev-parse", "origin/main"]),
        origin.git(&["rev-parse", "main"]),
    );

    // Now hand it to the clock. The hourly timer is replaced, so a second
    // fetch can only be the new interval's.
    session.set_auto_fetch(Some(Duration::from_millis(120)));
    assert!(
        !hourly.tick().await,
        "setting an interval stops the timer it replaces"
    );
    let ticking = session.auto_fetch_ticker().expect("auto fetch is on");
    assert_eq!(
        auto_fetch_done(&sink, 2).await,
        None,
        "the interval came round and fetched on its own"
    );

    session.set_auto_fetch(None);
    assert!(
        !ticking.tick().await,
        "turning it off stops the timer, so no further fetch can start"
    );
    session.close();
}

/// Suspending stops the timer without forgetting what it was set to, so
/// resuming needs no one to say the interval again — and a repository that
/// never had one says so, which is how the caller tells "stopped because it
/// kept failing" from "never fetched on its own in the first place".
#[tokio::test(flavor = "multi_thread")]
async fn a_suspended_timer_comes_back_on_the_interval_it_had() {
    let mut origin = TestRepo::init();
    origin.commit_file("f.txt", "0\n", "root");
    let mut clone = TestRepo::init();
    clone.git(&["remote", "add", "origin", &origin.file_url()]);

    let (sink, session) = opened(&clone).await;
    assert!(
        !session.suspend_auto_fetch(),
        "nothing to suspend before an interval is ever set"
    );
    // Resuming what was never on leaves it off.
    session.resume_auto_fetch();
    assert!(
        session.auto_fetch_ticker().is_none(),
        "resume does not invent an interval of its own"
    );

    session.set_auto_fetch(Some(Duration::from_secs(3600)));
    let before = session.auto_fetch_ticker().expect("auto fetch is on");
    assert!(session.suspend_auto_fetch(), "there was a timer to stop");
    assert!(
        !before.tick().await,
        "the suspended timer refuses the tick it would have taken"
    );
    assert!(
        session.auto_fetch_ticker().is_none(),
        "and there is no timer to reach while it is suspended"
    );
    assert!(
        !session.suspend_auto_fetch(),
        "suspending twice has nothing left to stop"
    );

    session.resume_auto_fetch();
    let after = session
        .auto_fetch_ticker()
        .expect("resume put the interval back");
    assert!(after.tick().await, "the timer that came back takes a tick");
    assert_eq!(
        auto_fetch_done(&sink, 1).await,
        None,
        "and the fetch it queued ran"
    );
    session.close();
}

/// A push refused for looking at an older remote is followed by a fetch,
/// so what the remote actually holds is on screen before anything else is
/// decided. The push itself still fails and nothing is retried.
#[tokio::test(flavor = "multi_thread")]
async fn a_push_refused_as_out_of_date_fetches_what_it_was_missing() {
    let mut origin = TestRepo::init();
    origin.commit_file("f.txt", "0\n", "root");
    origin.git(&["config", "core.bare", "true"]);

    let mut clone = TestRepo::init();
    clone.git(&["remote", "add", "origin", &origin.file_url()]);
    clone.git(&["fetch", "origin"]);
    clone.git(&["reset", "--hard", "origin/main"]);
    clone.git(&["branch", "--set-upstream-to=origin/main", "main"]);
    let known = clone.git(&["rev-parse", "origin/main"]);

    // Someone else pushes while this clone is not looking.
    let mut other = TestRepo::init();
    other.git(&["remote", "add", "origin", &origin.file_url()]);
    other.git(&["fetch", "origin"]);
    other.git(&["checkout", "-B", "main", "origin/main"]);
    other.commit_file("theirs.txt", "t\n", "their work");
    other.git(&["push", "origin", "main"]);

    // Ours goes its own way, still believing the remote is where it was.
    clone.commit_file("mine.txt", "m\n", "my work");

    let (sink, session) = opened(&clone).await;
    session.push_current(String::new(), platitude_core::remote::PushForce::None);

    let error = sink
        .wait_for("the push to be refused", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::WriteFinished { op, error } if *op == "push" => Some(error.clone()),
                _ => None,
            })
        })
        .await;
    assert!(
        error.is_some_and(|e| e.contains("rejected")),
        "the refusal is reported as it stands"
    );

    sink.wait_for("the fetch that answers it", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::WriteFinished { op, error: None } if *op == "fetch"))
            .then_some(())
    })
    .await;
    assert_ne!(
        clone.git(&["rev-parse", "origin/main"]),
        known,
        "the tracking ref caught up, so the graph can show what would be overwritten"
    );
    assert_eq!(
        origin.git(&["log", "-1", "--format=%s", "main"]),
        "their work",
        "nothing was retried: the remote still holds only their commit"
    );
    session.close();
}
