//! The auto-fetch timer, the fetch an opening fires, and the fetch a
//! refused push asks for.

use std::sync::Arc;
use std::time::Duration;

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, open_unawaited, opened, write_result};
use platitude_core::OperationKind;
use platitude_core::session::{AutoFetchTicker, OpenFetch, SessionEvent};

/// One hand-stepped tick, under the suite's backstop: a starved timer task
/// sends the sink nothing, so no [`crate::support::Patience`] is counting
/// and the binary would sit until the CI kill with nothing named.
async fn stepped(ticker: &AutoFetchTicker, what: &str) -> bool {
    crate::support::wait::bounded(what, ticker.tick()).await
}

/// Waits for the `nth` automatic fetch to finish and returns git's error,
/// if any.
async fn auto_fetch_done(sink: &CaptureSink, nth: usize) -> Option<String> {
    sink.wait_for("an automatic fetch", |evs| {
        evs.iter()
            .filter_map(|e| match e {
                SessionEvent::WriteFinished {
                    kind: OperationKind::AutoFetch,
                    error,
                    ..
                } => Some(error.clone()),
                _ => None,
            })
            .nth(nth - 1)
    })
    .await
}

/// Both halves are read off the timer — a tick taken, a tick refused once
/// stopped — because a fetch queued just before the stop still starts
/// later, so silence cannot tell "stopped" from "slow". That the clock
/// brings ticks on its own is the timer's unit tests' (`session::auto_fetch`).
#[tokio::test(flavor = "multi_thread")]
async fn auto_fetch_runs_on_its_interval_and_stops() {
    let mut origin = TestRepo::init();
    origin.commit_file("f.txt", "0\n", "root");
    let mut clone = TestRepo::init();
    let url = origin.file_url();
    clone.git(&["remote", "add", "origin", &url]);

    let (sink, session) = opened(&clone).await;
    // An hour, so only the hand-stepped tick fires.
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

    session.set_auto_fetch(Some(Duration::from_secs(1800)));
    assert!(
        !stepped(&hourly, "the replaced timer's refusal").await,
        "setting an interval stops the timer it replaces"
    );
    let ticking = session.auto_fetch_ticker().expect("auto fetch is on");
    assert!(
        stepped(&ticking, "the new timer's tick").await,
        "the timer that replaced it takes the tick"
    );
    assert_eq!(
        auto_fetch_done(&sink, 2).await,
        None,
        "and its tick fetched"
    );

    session.set_auto_fetch(None);
    assert!(
        !stepped(&ticking, "the stopped timer's refusal").await,
        "turning it off stops the timer, so no further fetch can start"
    );
    session.close();
}

/// The application sets the interval from the UI thread, outside any tokio
/// context, so the timer's clock has to be built on the runtime.
#[tokio::test(flavor = "multi_thread")]
async fn the_interval_is_set_from_outside_the_runtime() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    let (_sink, session) = opened(&repo).await;

    let from_the_ui = {
        let session = Arc::clone(&session);
        std::thread::spawn(move || session.set_auto_fetch(Some(Duration::from_secs(3600))))
    };
    from_the_ui
        .join()
        .expect("setting the interval off the runtime is not a panic");

    let ticker = session.auto_fetch_ticker().expect("auto fetch is on");
    assert!(
        stepped(&ticker, "a tick of the timer set from outside").await,
        "the timer set from the UI thread runs"
    );
    session.close();
}

/// Suspending keeps the interval for resuming, and says whether there was a
/// timer — how the caller tells "stopped for failing" from "never on".
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

/// The fetch puts what the remote holds on screen before anything else is
/// decided; the push itself still fails and is not retried.
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

    let mut other = TestRepo::init();
    other.git(&["remote", "add", "origin", &origin.file_url()]);
    other.git(&["fetch", "origin"]);
    other.git(&["checkout", "-B", "main", "origin/main"]);
    other.commit_file("theirs.txt", "t\n", "their work");
    other.git(&["push", "origin", "main"]);

    clone.commit_file("mine.txt", "m\n", "my work");

    let (sink, session) = opened(&clone).await;
    session.push_current(String::new(), platitude_core::remote::PushForce::None);

    let error = sink
        .wait_for("the push to be refused", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::WriteFinished {
                    kind: OperationKind::Push,
                    error,
                    ..
                } => Some(error.clone()),
                _ => None,
            })
        })
        .await;
    assert!(
        error.is_some_and(|e| e.contains("rejected")),
        "the refusal is reported as it stands"
    );

    // A failed fetch ends the wait: one catch-up is queued per refusal, so
    // a broken environment would otherwise time out naming the wait.
    sink.wait_for("the fetch that answers it", |evs| {
        evs.iter()
            .any(|e| match e {
                SessionEvent::WriteFinished {
                    kind: OperationKind::Fetch,
                    error,
                    ..
                } => {
                    assert!(error.is_none(), "the catch-up fetch failed: {error:?}");
                    true
                }
                _ => false,
            })
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

/// What a tab shows a moment after it opens is what the remote holds now.
#[tokio::test(flavor = "multi_thread")]
async fn opening_a_repository_fetches_without_being_asked() {
    let mut origin = TestRepo::init();
    origin.commit_file("f.txt", "0\n", "root");
    let mut clone = TestRepo::init();
    clone.git(&["remote", "add", "origin", &origin.file_url()]);

    let (sink, session) = opened(&clone).await;
    // The interval is the permission, set before the ask; an hour, so the
    // clock does nothing here.
    session.set_auto_fetch(Some(Duration::from_secs(3600)));
    assert_eq!(session.fetch_on_open(), OpenFetch::Started);
    assert_eq!(
        write_result(&sink, OperationKind::OpenFetch).await,
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

/// The application asks the instant it hands the session its settings,
/// which can be before the repository is open, so the session keeps the ask.
///
/// Single-threaded on purpose: a spawned task is polled only once its
/// spawner awaits, so the opening cannot have run by the ask. On the
/// multi-threaded runtime a busy machine fits the whole opening in first,
/// and the test becomes `opening_a_repository_fetches_without_being_asked`.
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
    assert_eq!(write_result(&sink, OperationKind::OpenFetch).await, None);
    assert_eq!(
        clone.git(&["rev-parse", "origin/main"]),
        origin.git(&["rev-parse", "main"]),
        "the opening redeemed the ask it was holding"
    );
    session.close();
}

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

/// `fetch --prune --all` with no remote exits clean, so an unasked one
/// would cost a process per interval and a fetch button spinning while
/// greyed out. The list read is the refs listing's, which also greys the
/// button, so the two cannot disagree.
#[tokio::test(flavor = "multi_thread")]
async fn a_repository_with_no_remote_is_not_fetched_from() {
    let mut only = TestRepo::init();
    only.commit_file("f.txt", "0\n", "root");

    let (sink, session) = opened(&only).await;
    // The refs listing reads the remote list before it publishes, so past
    // here the answer below is not a matter of timing.
    sink.opening_snapshots().await;
    session.set_auto_fetch(Some(Duration::from_secs(3600)));
    assert_eq!(session.fetch_on_open(), OpenFetch::NoRemote);
    session.close();
}

/// The same where the ask beats the opening — the application's shape — so
/// the opening reads the answer.
///
/// Single-threaded for the reason `an_ask_that_beats_the_opening_is_kept_for_it`
/// gives, and here it is the whole test: an ask arriving after the opening
/// reads a remote list not yet read, which is deliberately not a "no"
/// (`known_to_have_no_remote`), and fetches.
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
    // The opening settles the ask before it publishes, so a fetch pressed
    // after this is behind whatever it queued.
    sink.opening_snapshots().await;
    assert_eq!(
        session.fetch_on_open(),
        OpenFetch::Spent,
        "the opening redeemed the ask rather than leaving it held"
    );
    session.fetch(None);
    assert_eq!(write_result(&sink, OperationKind::Fetch).await, None);
    assert_eq!(
        sink.count(|e| matches!(
            e,
            SessionEvent::WriteStarted {
                kind: OperationKind::OpenFetch,
                ..
            }
        )),
        0,
        "the opening reached for nothing"
    );
    session.close();
}

/// Read off the queue: the tick is answered only once acted on and the
/// queue is first in first out, so a fetch pressed by hand afterwards
/// finishes behind anything that tick queued.
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

    // By hand the same fetch runs: the gate is only on what nobody asked for.
    session.fetch(None);
    assert_eq!(write_result(&sink, OperationKind::Fetch).await, None);
    assert_eq!(
        sink.count(|e| matches!(
            e,
            SessionEvent::WriteStarted {
                kind: OperationKind::AutoFetch,
                ..
            }
        )),
        0,
        "the tick in front of it queued nothing"
    );
    session.close();
}

/// `git remote add` moves no ref and lands no write here; what notices it
/// is the config file written since the list was read (`RepoSession::remotes`).
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
    let polled =
        crate::support::wait::bounded("the tracked poll", session.refresh_poll_tracked().outcome())
            .await;
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
                SessionEvent::RefsLoaded { snapshot, .. } => {
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

/// A fetch never touches the index or the working tree, so its refresh
/// reads a status only where refs moved: a tick that brings nothing down —
/// nearly every one — spends no `git status`, the longest read in the app.
///
/// Counted between the writes' starts: `WriteFinished` precedes the
/// refreshes it triggers, and the serial queue advances only once they have
/// landed, so the next write's start closes the window.
#[tokio::test(flavor = "multi_thread")]
async fn a_fetch_that_brings_nothing_down_reads_no_status() {
    let mut origin = TestRepo::init();
    origin.commit_file("f.txt", "0\n", "root");
    let mut clone = TestRepo::init();
    clone.git(&["remote", "add", "origin", &origin.file_url()]);

    let (sink, session) = opened(&clone).await;
    // A status read publishes from inside its reader, and a caller waiting
    // behind it runs a pass of its own that would land in the counted window.
    sink.opening_settled(&session).await;
    session.set_auto_fetch(Some(Duration::from_secs(3600)));
    let hourly = session.auto_fetch_ticker().expect("auto fetch is on");

    // The first tick brings `origin/main` down; the second finds nothing new.
    assert!(stepped(&hourly, "the first tick").await);
    assert_eq!(auto_fetch_done(&sink, 1).await, None);
    assert!(stepped(&hourly, "the second tick").await);
    assert_eq!(auto_fetch_done(&sink, 2).await, None);

    // The write that closes the second tick's window.
    session.fetch(None);
    assert_eq!(write_result(&sink, OperationKind::Fetch).await, None);

    let events = sink.events.lock().unwrap();
    let ticked: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            matches!(
                e,
                SessionEvent::WriteStarted {
                    kind: OperationKind::AutoFetch,
                    ..
                }
            )
        })
        .map(|(at, _)| at)
        .collect();
    assert_eq!(ticked.len(), 2, "two automatic fetches ran: {events:?}");
    let asked = events
        .iter()
        .position(|e| {
            matches!(
                e,
                SessionEvent::WriteStarted {
                    kind: OperationKind::Fetch,
                    ..
                }
            )
        })
        .expect("the fetch asked for by hand started");
    let statuses = |from: usize, to: usize| {
        events[from..to]
            .iter()
            .filter(|e| matches!(e, SessionEvent::StatusLoaded { .. }))
            .count()
    };
    assert_eq!(
        statuses(ticked[0], ticked[1]),
        1,
        "the tick that brought a ref down read the status behind it: {events:?}"
    );
    assert_eq!(
        statuses(ticked[1], asked),
        0,
        "the tick that brought nothing down read no status: {events:?}"
    );
    drop(events);
    session.close();
}

/// The panel the first failure raises has to hold the command that raised
/// it (デザイン規約 §git が言ったことを読む場所); without the row it shows whatever
/// the reader last did, blaming a push for a fetch that could not reach its remote.
///
/// Both halves in one test: a log that keeps every unasked fetch passes the
/// second assertion alone, and one that keeps none passes the first.
#[tokio::test(flavor = "multi_thread")]
async fn an_unasked_fetch_leaves_a_row_only_when_git_says_no() {
    let mut origin = TestRepo::init();
    origin.commit_file("f.txt", "0\n", "root");
    let mut clone = TestRepo::init();
    clone.git(&["remote", "add", "origin", &origin.file_url()]);

    let (sink, session) = opened(&clone).await;
    session.set_auto_fetch(Some(Duration::from_secs(3600)));
    let hourly = session.auto_fetch_ticker().expect("auto fetch is on");
    assert!(stepped(&hourly, "the tick that reaches the remote").await);
    assert_eq!(
        auto_fetch_done(&sink, 1).await,
        None,
        "the file:// remote fetched cleanly"
    );
    assert_eq!(
        sink.count(|e| matches!(e, SessionEvent::CommandStarted { .. })),
        0,
        "a fetch that landed is nobody's doing, and leaves no row"
    );

    let gone = format!("{}-gone", origin.file_url());
    clone.git(&["remote", "set-url", "origin", &gone]);
    assert!(stepped(&hourly, "the tick that cannot reach it").await);
    assert!(
        auto_fetch_done(&sink, 2).await.is_some(),
        "the fetch could not reach the remote"
    );
    let (id, asked) = sink
        .wait_for("the row of the fetch git turned down", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::CommandStarted {
                    id, display, asked, ..
                } if display.contains(" fetch ") => Some((*id, *asked)),
                _ => None,
            })
        })
        .await;
    assert!(
        !asked,
        "the row is the record of a git that said no, not the reader's doing"
    );
    let end = sink
        .wait_for("its end", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::CommandFinished { id: got, end, .. } if *got == id => Some(*end),
                _ => None,
            })
        })
        .await;
    assert_ne!(
        end,
        platitude_core::CommandEnd::Exited(0),
        "the row carries what the fetch actually did"
    );
    session.close();
}
