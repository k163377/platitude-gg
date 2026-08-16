//! The log window: what the walk covers, what of it is shown, and what
//! moving the window lands.

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, pass_of};
use platitude_core::GitExecutor;
use platitude_core::session::{RepoSession, SessionEvent};

#[tokio::test(flavor = "multi_thread")]
async fn tag_only_commits_follow_the_include_tags_option() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "base");
    // A commit reachable only through a tag (detached, then back to main).
    repo.git(&["checkout", "--detach", "HEAD"]);
    repo.commit_file("g.txt", "t\n", "tag only work");
    repo.git(&["tag", "islet"]);
    repo.git(&["checkout", "main"]);

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );

    // Tags are walked by default → the tag-only commit has a row. The
    // tag-inclusive pass differs from the fast pass here, so it arrives
    // as an atomic replacement.
    let first_gen = sink
        .wait_for("tags-on LogReplaced", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::LogReplaced {
                    generation, rows, ..
                } if rows.len() == 2 => Some(*generation),
                _ => None,
            })
        })
        .await;

    // Two-phase streaming: a fast tag-less pass must have painted first.
    {
        let events = sink.events.lock().unwrap();
        let fast_pass = events.iter().find_map(|e| match e {
            SessionEvent::LogFinished {
                generation, total, ..
            } if *total == 1 => Some(*generation),
            _ => None,
        });
        assert!(
            fast_pass.is_some_and(|g| g < first_gen),
            "expected a tag-less fast pass before the tag-inclusive swap"
        );
    }

    session.set_include_tags(false);
    let off = sink.pass_after("the tag-less graph", first_gen).await;
    assert_eq!(off.total, 1, "the tag-only commit left the walk");

    session.close();
}

#[tokio::test(flavor = "multi_thread")]
async fn log_limit_truncates_the_window() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "one");
    repo.commit_file("f.txt", "2\n", "two");
    repo.commit_file("f.txt", "3\n", "three");

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );

    sink.wait_for("full LogFinished", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::LogFinished {
                total, truncated, ..
            } if *total == 3 => {
                assert!(!truncated, "3 commits fit in the default window");
                Some(())
            }
            _ => None,
        })
    })
    .await;
    let first_gen = sink.opened_graph_gen(&session, 3).await;

    session.set_log_limit(Some(2));
    let limited = sink.pass_after("the limited window", first_gen).await;
    assert_eq!(limited.total, 2);
    assert_eq!(limited.walked, 2, "the footer's number is the limit itself");
    assert!(limited.truncated);

    session.close();
}

/// A stash's synthetic index parent is walked but sifted out of the
/// shown rows, so the shown count sits below the window limit even when
/// the walk was cut — truncation must follow the walk, not the rows.
#[tokio::test(flavor = "multi_thread")]
async fn truncation_follows_the_walk_not_the_shown_rows() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "one");
    repo.commit_file("f.txt", "2\n", "two");
    repo.commit_file("f.txt", "3\n", "three");
    repo.write_file("f.txt", "wip\n");
    repo.git(&["stash", "push", "-m", "wip stash"]);

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );

    // Full pass first: stash row + three commits, nothing truncated.
    sink.wait_for("full LogFinished", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::LogFinished {
                total, truncated, ..
            } if *total == 4 => {
                assert!(!truncated);
                Some(())
            }
            _ => None,
        })
    })
    .await;
    let first_gen = sink.opened_graph_gen(&session, 4).await;

    // The walk emits 4 rows (stash, its index parent, "three", "two") and
    // is cut before "one"; the sifted index parent leaves 3 shown rows.
    session.set_log_limit(Some(4));
    let limited = sink.pass_after("the limited window", first_gen).await;
    assert_eq!(limited.total, 3, "stash + three + two, index parent sifted");
    assert_eq!(limited.walked, 4, "the walk count stays on the limit");
    assert!(limited.truncated, "the walk was cut before the root commit");

    session.close();
}

/// The synthetic WIP row is shown but never walked: a window that holds
/// the whole history must not report truncation just because the WIP row
/// pushes the shown count up to the limit.
///
/// Reached by widening a window that really was cut, rather than by
/// opening the wide one straight away. Both say the same thing about the
/// WIP row, but only the widening changes the graph — and a change is
/// what makes the answer arrive at all. Going straight to the wide window
/// leaves the rows *and the footer* exactly as the opening pass left them
/// (the default window holds this history whole either way), so a rebuild
/// that overtakes the stream (this repository opens dirty, and the status
/// read that notices it asks for one) finds nothing to swap and says
/// nothing, and the test waits for an event that was never sent.
#[tokio::test(flavor = "multi_thread")]
async fn the_wip_row_does_not_trigger_truncation() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "one");
    repo.commit_file("f.txt", "2\n", "two");
    repo.write_file("f.txt", "wip\n"); // dirty → synthetic WIP row

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );

    // Wait until the dirty state is reflected and the stream settles, so
    // the next pass is the reaction to the limit change.
    let first_gen = sink.opened_graph_gen(&session, 3).await;

    // One commit through a window of one: cut, and the WIP row rides on
    // top of it regardless.
    session.set_log_limit(Some(1));
    let cut = sink.pass_after("the cut window", first_gen).await;
    assert_eq!(cut.total, 2, "WIP row + the one commit walked");
    assert_eq!(cut.walked, 1, "the walk stopped on the limit");
    assert!(cut.truncated, "older history exists and is not shown");

    // Two commits walk through a window of three; the WIP row makes three
    // shown rows, which is not a truncated window.
    session.set_log_limit(Some(3));
    let whole = sink.pass_after("the widened window", cut.generation).await;
    assert_eq!(whole.total, 3, "WIP row + two commits");
    assert_eq!(whole.walked, 2, "the WIP row is shown but never walked");
    assert!(!whole.truncated, "the whole history fits the window");

    session.close();
}

/// A window change asks for a stream, but only until somebody else asks
/// for the graph: the stream drops without a word when a background
/// rebuild supersedes it, and that rebuild — reading the options the
/// change just wrote — lands the new window as an atomic replacement
/// instead. The two are the same answer, and which one arrives is a
/// scheduling accident, so waiting for one shape is waiting on a race.
///
/// Held in the swap that adds the WIP row, the interleaving is exact: the
/// stream cannot reach its cancel check until the rebuild behind it has
/// taken its place. Left to the scheduler it is rare — it turned up as a
/// flake on a machine running three other builds, not as a test.
#[tokio::test(flavor = "multi_thread")]
async fn a_window_change_a_rebuild_overtakes_still_lands_the_new_window() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "one");
    repo.commit_file("f.txt", "2\n", "two");

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    sink.opened_graph_gen(&session, 2).await;

    // Park in the swap that adds the WIP row: it sends under the graph
    // lock, so every pass asked for from here waits at that door.
    let (release, held) = std::sync::mpsc::channel::<()>();
    sink.hook_once(
        |e| matches!(e, SessionEvent::LogReplaced { rows, .. } if rows.len() == 3),
        move || {
            held.recv().expect("the test releases the graph swap");
        },
    );
    repo.write_file("f.txt", "wip\n");
    session.refresh_status();
    sink.wait_for("the rebuild reached the window", |events| {
        events
            .iter()
            .any(|event| matches!(event, SessionEvent::LogReplaced { rows, .. } if rows.len() == 3))
            .then_some(())
    })
    .await;
    // The sink records before it runs the hook, so the graph the change
    // is measured against is already readable from where it is parked.
    let wip_gen = sink
        .wait_for("the WIP row's generation", |evs| {
            evs.iter()
                .filter_map(pass_of)
                .find(|p| p.total == 3)
                .map(|p| p.generation)
        })
        .await;

    // The change's stream is stopped at that door; the rebuild asked for
    // behind it takes its place before the stream gets through.
    session.set_log_limit(Some(1));
    session.refresh_log();
    release.send(()).expect("let the rebuild finish");

    let cut = sink.pass_after("the new window", wip_gen).await;
    assert_eq!(cut.total, 2, "WIP row + the one commit walked");
    assert_eq!(cut.walked, 1, "the walk stopped on the limit");
    assert!(cut.truncated, "older history exists and is not shown");
    assert_eq!(
        sink.count(
            |e| matches!(e, SessionEvent::LogFinished { generation, .. } if *generation > wip_gen)
        ),
        0,
        "the superseded stream stayed silent, so the replacement is the \
         only thing that could have carried the window: {:?}",
        sink.events.lock().unwrap()
    );
    session.close();
}

/// The same interleaving over a window change that moves nothing but the
/// footer: two commits through a window of two are reported cut (the walk
/// stopped on the limit, which is all truncation can mean), and widening
/// to three leaves every row exactly where it was. The rebuild that
/// overtakes the stream carries the new window, so it is the only thing
/// that can say the history is no longer cut — and a comparison that only
/// looks at rows finds nothing to do, leaving the notice claiming history
/// the user just asked to see.
#[tokio::test(flavor = "multi_thread")]
async fn a_window_change_only_the_footer_notices_still_lands() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "one");
    repo.commit_file("f.txt", "2\n", "two");

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    let first_gen = sink.opened_graph_gen(&session, 2).await;

    // A window exactly as wide as the history: every commit is shown, and
    // the walk stopping on the limit is what makes it cut all the same.
    session.set_log_limit(Some(2));
    let cut = sink.pass_after("the window on the limit", first_gen).await;
    assert_eq!(cut.total, 2, "both commits fit");
    assert_eq!(cut.walked, 2, "the walk stopped on the limit");
    assert!(cut.truncated, "which is all the footer knows");

    // Park in the swap that adds the WIP row (see the test above): from
    // here every pass waits at the graph lock.
    let (release, held) = std::sync::mpsc::channel::<()>();
    sink.hook_once(
        |e| matches!(e, SessionEvent::LogReplaced { rows, .. } if rows.len() == 3),
        move || {
            held.recv().expect("the test releases the graph swap");
        },
    );
    repo.write_file("f.txt", "wip\n");
    session.refresh_status();
    sink.wait_for("the rebuild reached the window", |events| {
        events
            .iter()
            .any(|event| matches!(event, SessionEvent::LogReplaced { rows, .. } if rows.len() == 3))
            .then_some(())
    })
    .await;
    let wip = sink
        .wait_for("the WIP row's pass", |evs| {
            evs.iter().filter_map(pass_of).find(|p| p.total == 3)
        })
        .await;
    assert!(wip.truncated, "the window is still sitting on the limit");

    // Widen past the end of the history. The stream this asks for is
    // stopped at the door and dropped; the rebuild behind it walks the
    // same two commits, draws the same three rows, and carries the only
    // thing that did change.
    session.set_log_limit(Some(3));
    session.refresh_log();
    release.send(()).expect("let the rebuild finish");

    let whole = sink.pass_after("the widened window", wip.generation).await;
    assert_eq!(whole.total, 3, "WIP row + both commits, exactly as before");
    assert_eq!(whole.walked, 2, "the walk ran out of history");
    assert!(!whole.truncated, "so nothing is being kept from the user");
    session.close();
}
