//! The auto-fetch timer, the fetch an opening fires, and the fetch a
//! refused push asks for.

use std::time::Duration;

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, opened, open_unawaited, write_result};
use platitude_core::session::{
    AutoFetchTicker, OPEN_FETCH_OP, OpenFetch, SessionEvent,
};

/// One hand-stepped tick, under the suite's backstop.
///
/// The tick resolves when the timer task acts on it, and a timer task that
/// is merely starved has nothing under it at all: no event goes to the
/// sink, so no [`crate::support::Patience`] is counting, and the binary
/// sits there until the CI kill with nothing named. 実測 2026-08-29: one
/// copy running beside seven of its own and a Linux container did exactly
/// that, and what was reported was a run that never ended rather than a
/// test that failed.
async fn stepped(ticker: &AutoFetchTicker, what: &str) -> bool {
    crate::support::wait::bounded(what, ticker.tick()).await
}

/// Waits for the `nth` automatic fetch to finish and returns git's error,
/// if any. Telling them apart is the point: only the second one can be laid
/// at the clock's door.
async fn auto_fetch_done(sink: &CaptureSink, nth: usize) -> Option<String> {
    sink.wait_for("an automatic fetch", |evs| {
        evs.iter()
            .filter_map(|e| match e {
                SessionEvent::WriteFinished { op, error, .. }
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
    assert!(
        stepped(&hourly, "the running timer's tick").await,
        "the running timer took the tick"
    );
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
        !stepped(&hourly, "the replaced timer's refusal").await,
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
        !stepped(&ticking, "the stopped timer's refusal").await,
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
        !stepped(&before, "the suspended timer's refusal").await,
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
    assert!(
        stepped(&after, "the resumed timer's tick").await,
        "the timer that came back takes a tick"
    );
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
                SessionEvent::WriteFinished { op, error, .. } if *op == "push" => {
                    Some(error.clone())
                }
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
            .any(|e| matches!(e, SessionEvent::WriteFinished { op, error: None, .. } if *op == "fetch"))
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

/// Opening a repository reaches the network itself: what a tab shows a
/// moment after it opens is what the remote holds, rather than what was
/// left behind the last time somebody looked.
#[tokio::test(flavor = "multi_thread")]
async fn opening_a_repository_fetches_without_being_asked() {
    let mut origin = TestRepo::init();
    origin.commit_file("f.txt", "0\n", "root");
    let mut clone = TestRepo::init();
    clone.git(&["remote", "add", "origin", &origin.file_url()]);

    let (sink, session) = opened(&clone).await;
    // The interval is the permission, and the application sets it before
    // it asks. An hour out, so nothing here can be the clock's doing.
    session.set_auto_fetch(Some(Duration::from_secs(3600)));
    assert_eq!(session.fetch_on_open(), OpenFetch::Started);
    assert_eq!(
        write_result(&sink, OPEN_FETCH_OP).await,
        None,
        "the file:// remote fetched cleanly"
    );
    assert_eq!(
        clone.git(&["rev-parse", "origin/main"]),
        origin.git(&["rev-parse", "main"]),
        "and what it brought down is in the repository"
    );
    assert_eq!(
        session.fetch_on_open(),
        OpenFetch::Spent,
        "one to a session: a second ask fires nothing"
    );
    session.close();
}

/// The ask can arrive before the repository is open — the application
/// makes it the instant it has handed the session its settings, and the
/// opening runs on the runtime — so the session keeps it until there is
/// something to fetch into.
///
/// Which of the two arrives first is the scheduler's business in the
/// application, and this test's to settle: the single-threaded runtime
/// polls a spawned task only once the task that spawned it awaits, so
/// the opening cannot have run by the line below however loaded the
/// machine is, and the ask is the one that waits. Left to the
/// multi-threaded runtime it is a race — a busy machine fits the whole
/// opening into the two calls after `open` — and what is under test
/// becomes the ask that arrives second, which
/// `opening_a_repository_fetches_without_being_asked` owns already
/// (実測 2026-08-20: that is how the local-only sibling below failed, in
/// a Linux container running beside a host build).
#[tokio::test]
async fn an_ask_that_beats_the_opening_is_kept_for_it() {
    let mut origin = TestRepo::init();
    origin.commit_file("f.txt", "0\n", "root");
    let mut clone = TestRepo::init();
    clone.git(&["remote", "add", "origin", &origin.file_url()]);

    let (sink, session) = open_unawaited(&clone);
    session.set_auto_fetch(Some(Duration::from_secs(3600)));
    assert_eq!(
        session.fetch_on_open(),
        OpenFetch::Held,
        "the repository is not open yet, so the session keeps the ask"
    );
    assert_eq!(write_result(&sink, OPEN_FETCH_OP).await, None);
    assert_eq!(
        clone.git(&["rev-parse", "origin/main"]),
        origin.git(&["rev-parse", "main"]),
        "the opening redeemed the ask it was holding"
    );
    session.close();
}

/// With automatic fetching off, the owner has said not to reach the
/// network unasked, and opening a tab is not the thing to break that for.
#[tokio::test(flavor = "multi_thread")]
async fn an_opening_reaches_nothing_where_automatic_fetching_is_off() {
    let mut origin = TestRepo::init();
    origin.commit_file("f.txt", "0\n", "root");
    let mut clone = TestRepo::init();
    clone.git(&["remote", "add", "origin", &origin.file_url()]);

    let (_sink, session) = opened(&clone).await;
    session.set_auto_fetch(None);
    // A synchronous refusal, so there is nothing queued to wait out.
    assert_eq!(session.fetch_on_open(), OpenFetch::Disabled);
    assert_eq!(
        clone.git(&["for-each-ref", "refs/remotes"]),
        "",
        "nothing came down: the remote was never reached"
    );
    session.close();
}

/// A repository with no remote is not fetched from at all.
///
/// Nothing there fails — `fetch --prune --all` with no remote configured
/// exits clean (実測 git 2.55) — so what an unasked one costs is a process
/// an interval, and a fetch button that spins while it is saying it cannot
/// be pressed. The list this reads is the one the refs listing keeps, which
/// is also what greys that button out, so the two cannot disagree.
#[tokio::test(flavor = "multi_thread")]
async fn a_repository_with_no_remote_is_not_fetched_from() {
    let mut only = TestRepo::init();
    only.commit_file("f.txt", "0\n", "root");

    let (sink, session) = opened(&only).await;
    // The refs listing reads the remote list before it publishes, so this
    // is where the answer below stops being a matter of timing.
    sink.opening_snapshots().await;
    session.set_auto_fetch(Some(Duration::from_secs(3600)));
    assert_eq!(session.fetch_on_open(), OpenFetch::NoRemote);
    session.close();
}

/// The same where the ask beats the opening, which is the shape the
/// application makes: it asks the instant it has handed the session its
/// settings, so the answer is read by the opening rather than by the ask.
///
/// On the single-threaded runtime for the reason
/// `an_ask_that_beats_the_opening_is_kept_for_it` gives, and here it is
/// the whole test: an ask that arrives after the opening instead reads a
/// remote list that has not been read yet, which is deliberately not a
/// "no" (`known_to_have_no_remote`), and fetches.
#[tokio::test]
async fn an_ask_held_for_a_local_only_repository_fires_nothing() {
    let mut only = TestRepo::init();
    only.commit_file("f.txt", "0\n", "root");

    let (sink, session) = open_unawaited(&only);
    session.set_auto_fetch(Some(Duration::from_secs(3600)));
    assert_eq!(
        session.fetch_on_open(),
        OpenFetch::Held,
        "the repository is not open yet, so the session keeps the ask"
    );
    // The opening settles the ask before it publishes anything, so a fetch
    // pressed after this point is behind whatever it queued.
    sink.opening_snapshots().await;
    assert_eq!(
        session.fetch_on_open(),
        OpenFetch::Spent,
        "the opening redeemed the ask rather than leaving it held"
    );
    session.fetch(None);
    assert_eq!(write_result(&sink, "fetch").await, None);
    assert_eq!(
        sink.count(|e| matches!(e, SessionEvent::WriteStarted { op } if *op == OPEN_FETCH_OP)),
        0,
        "the opening reached for nothing"
    );
    session.close();
}

/// And the timer says the same thing every interval without queueing a
/// write to find it out.
///
/// Read off the queue rather than off a stretch of quiet clock: the tick is
/// answered only once it has been acted on, and the queue is first in first
/// out, so a fetch pressed by hand afterwards can only finish behind
/// anything that tick queued.
#[tokio::test(flavor = "multi_thread")]
async fn the_timer_queues_nothing_where_there_is_no_remote() {
    let mut only = TestRepo::init();
    only.commit_file("f.txt", "0\n", "root");

    let (sink, session) = opened(&only).await;
    sink.opening_snapshots().await;
    session.set_auto_fetch(Some(Duration::from_secs(3600)));
    let hourly = session.auto_fetch_ticker().expect("auto fetch is on");
    assert!(
        stepped(&hourly, "the local-only timer's tick").await,
        "the timer took the tick and stayed on"
    );

    // Asked for by hand, the same fetch runs: the gate is on what nobody
    // asked for, not on the button.
    session.fetch(None);
    assert_eq!(write_result(&sink, "fetch").await, None);
    assert_eq!(
        sink.count(|e| matches!(
            e,
            SessionEvent::WriteStarted { op } if *op == platitude_core::session::AUTO_FETCH_OP
        )),
        0,
        "the tick in front of it queued nothing"
    );
    session.close();
}

/// A remote added in a terminal is picked up by the poll, and the timer
/// starts fetching from it.
///
/// `git remote add` moves no ref and lands no write in here, so nothing the
/// session watches would have noticed it; what does is the config file
/// having been written since the list was read (`RepoSession::remotes`).
#[tokio::test(flavor = "multi_thread")]
async fn a_remote_added_outside_the_app_is_picked_up_by_a_poll() {
    let mut origin = TestRepo::init();
    origin.commit_file("f.txt", "0\n", "root");
    let mut only = TestRepo::init();
    only.commit_file("g.txt", "0\n", "root");

    let (sink, session) = opened(&only).await;
    sink.opening_snapshots().await;
    session.set_auto_fetch(Some(Duration::from_secs(3600)));

    only.git(&["remote", "add", "origin", &origin.file_url()]);
    // A poll that was refused (busy, or a write in front of it) would
    // prove nothing about what it reads.
    let polled = session.refresh_poll_tracked().outcome().await;
    assert!(
        matches!(
            polled,
            platitude_core::session::RefreshOutcome::Changed
                | platitude_core::session::RefreshOutcome::Unchanged
        ),
        "the poll ran: {polled:?}"
    );
    sink.wait_for("the remote in the snapshot", |evs| {
        evs.iter()
            .any(|e| match e {
                SessionEvent::RefsLoaded { snapshot } => {
                    snapshot.remote_names.iter().any(|n| n == "origin")
                }
                _ => false,
            })
            .then_some(())
    })
    .await;

    let hourly = session.auto_fetch_ticker().expect("auto fetch is on");
    assert!(
        stepped(&hourly, "the running timer's tick").await,
        "the running timer took the tick"
    );
    assert_eq!(auto_fetch_done(&sink, 1).await, None);
    assert_eq!(
        only.git(&["rev-parse", "origin/main"]),
        origin.git(&["rev-parse", "main"]),
        "the timer fetched from the remote that appeared under it"
    );
    session.close();
}
