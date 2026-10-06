//! What a write, or the window coming back, reads behind itself: each
//! listing once, and the history walked once — after the reads that decide
//! whether it is walked, and only where one of them moved.

use std::sync::Arc;

use platitude_core::session::{PaceBounds, PassStep, Recording, RepoSession, SessionEvent};

use crate::support::TestRepo;
use crate::support::remote::origin_and_clone;
use crate::support::session::{CaptureSink, open_with_doors, opened, write_settled};
use crate::support::wait::bounded;

/// The commands started from event `from` on, as the log shows them.
fn commands_since(sink: &CaptureSink, from: usize) -> Vec<String> {
    sink.events.lock().unwrap()[from..]
        .iter()
        .filter_map(|e| match e {
            SessionEvent::CommandStarted { display, .. } => Some(display.clone()),
            _ => None,
        })
        .collect()
}

fn times(commands: &[String], needle: &str) -> usize {
    commands.iter().filter(|c| c.contains(needle)).count()
}

/// Opened, with every read the opening started over, and the background
/// reads recorded from here on. Answers where the record starts. The
/// worktrees' pass first: one that lands moved asks for a walk, which the
/// graph's boundary then closes too.
async fn settled(sink: &CaptureSink, session: &Arc<RepoSession>) -> usize {
    sink.opening_settled(session).await;
    bounded(
        "the opening's other worktrees",
        session.wait_for_carried_pass(),
    )
    .await;
    bounded("the opening's reads", session.wait_for_snapshot_reads()).await;
    bounded("the opening's graph", session.wait_for_graph_passes()).await;
    session.set_recording(Recording::WithBackground);
    sink.events.lock().unwrap().len()
}

/// Waits for every read a focus or a poll started, the walk they asked for
/// included.
async fn reads_over(session: &Arc<RepoSession>) {
    bounded("the snapshot reads", session.wait_for_snapshot_reads()).await;
    bounded("the graph passes", session.wait_for_graph_passes()).await;
}

/// The stashes the graph on screen draws, by selector: the rows of the
/// newest picture, streamed or swapped in.
fn drawn_stashes(sink: &CaptureSink) -> Vec<String> {
    let mut generation = 0;
    let mut rows: Vec<platitude_core::session::LogRow> = Vec::new();
    for event in sink.events.lock().unwrap().iter() {
        match event {
            SessionEvent::LogStarted {
                generation: started,
            } => {
                generation = *started;
                rows.clear();
            }
            SessionEvent::LogChunk {
                generation: of,
                rows: more,
            } if *of == generation => rows.extend(more.iter().cloned()),
            SessionEvent::LogReplaced {
                generation: swapped,
                rows: all,
                ..
            } => {
                generation = *swapped;
                rows = all.clone();
            }
            _ => {}
        }
    }
    rows.into_iter()
        .map(|row| row.stash_ref)
        .filter(|stash| !stash.is_empty())
        .collect()
}

/// A repository with one stash and a clean tree.
fn with_a_stash() -> TestRepo {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("a.txt", "kept aside\n");
    repo.git(&["stash", "push", "-m", "kept"]);
    repo
}

/// Puts one commit on `bare`'s main from a clone of its own.
fn commit_upstream(bare: &TestRepo) {
    let mut other = TestRepo::init();
    other.git(&["remote", "add", "origin", &bare.file_url()]);
    other.git(&["fetch", "origin"]);
    other.git(&["reset", "--hard", "origin/main"]);
    other.commit_file("b.txt", "upstream\n", "upstream work");
    other.git(&["push", "origin", "HEAD:main"]);
}

/// A fetch that brings nothing moves nothing the graph, the listings or the
/// line-ending context are drawn from — so it reads none of them again:
/// only the refs, which are how it knows.
#[tokio::test(flavor = "multi_thread")]
async fn a_fetch_that_brings_nothing_down_reads_only_the_refs() {
    let (_bare, repo) = origin_and_clone();
    // Dirty, so the line-ending marks are something to read.
    repo.write_file("a.txt", "two\n");
    let (sink, session) = opened(&repo).await;
    settled(&sink, &session).await;
    // The opening's first refs read drops the line-ending context behind
    // the opening's status read; one more read takes that up, so what
    // follows counts the fetch's doing alone.
    session.refresh_status();
    reads_over(&session).await;
    let from = sink.events.lock().unwrap().len();

    let id = session.fetch(None).expect("accepted");
    assert_eq!(write_settled(&sink, id).await, [], "every read landed");
    let ran = commands_since(&sink, from);
    for (needle, expected) in [
        ("for-each-ref", 1),
        ("log -z", 0),
        ("status", 0),
        ("stash list", 0),
        ("worktree list", 0),
    ] {
        assert_eq!(times(&ran, needle), expected, "`{needle}`: {ran:#?}");
    }

    // The line-ending context was not dropped: the next status read over
    // the same tree reads no line endings again.
    let from = sink.events.lock().unwrap().len();
    session.refresh_status();
    reads_over(&session).await;
    let ran = commands_since(&sink, from);
    assert_eq!(times(&ran, "status"), 1, "{ran:#?}");
    assert_eq!(times(&ran, "--eol"), 0, "{ran:#?}");
    session.close();
}

/// One that brings nothing still walks where the graph is behind: a walk
/// that fell over left it so, and the fetch is the next read to try again.
#[tokio::test(flavor = "multi_thread")]
async fn a_fetch_that_brings_nothing_down_walks_a_graph_left_behind() {
    let (_bare, repo) = origin_and_clone();
    let (sink, session, doors) = open_with_doors(&repo);
    sink.opened_graph(&session, 1).await;
    doors.fail_every_pass(PassStep::Swapping);
    let fell = bounded(
        "a walk that falls over",
        session.refresh_log_tracked().outcome(),
    )
    .await;
    assert!(!fell.landed(), "{fell:?}");
    doors.stop_failing();
    session.set_recording(Recording::WithBackground);
    let from = sink.events.lock().unwrap().len();

    let id = session.fetch(None).expect("accepted");
    assert_eq!(
        write_settled(&sink, id).await,
        [],
        "the walk behind it landed"
    );
    let ran = commands_since(&sink, from);
    assert_eq!(times(&ran, "log -z"), 1, "{ran:#?}");
    let stale = sink.events.lock().unwrap()[from..]
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::LogStale { stale } => Some(*stale),
            _ => None,
        });
    assert_eq!(stale, Some(false), "the graph caught up");
    session.close();
}

/// One that brings a commit walks once, and lists the stashes once — the
/// walk's own read, published.
#[tokio::test(flavor = "multi_thread")]
async fn a_fetch_that_brings_a_commit_down_walks_once() {
    let (bare, repo) = origin_and_clone();
    let (sink, session) = opened(&repo).await;
    let from = settled(&sink, &session).await;
    commit_upstream(&bare);

    let id = session.fetch(None).expect("accepted");
    assert_eq!(write_settled(&sink, id).await, []);
    let ran = commands_since(&sink, from);
    for (needle, expected) in [("log -z", 1), ("status", 1), ("stash list", 1)] {
        assert_eq!(times(&ran, needle), expected, "`{needle}`: {ran:#?}");
    }
    session.close();
}

/// `Set upstream…` writes two config keys: the refs listing and the status
/// say what moved, and no ref did, so nothing is walked or listed.
#[tokio::test(flavor = "multi_thread")]
async fn setting_an_upstream_walks_nothing() {
    let (_bare, repo) = origin_and_clone();
    let (sink, session) = opened(&repo).await;
    let from = settled(&sink, &session).await;

    let id = session
        .set_upstream("main".into(), "origin".into(), "main".into())
        .expect("accepted");
    assert_eq!(write_settled(&sink, id).await, []);
    let ran = commands_since(&sink, from);
    for (needle, expected) in [
        ("for-each-ref", 1),
        ("status", 1),
        ("log -z", 0),
        ("stash list", 0),
        ("worktree list", 0),
    ] {
        assert_eq!(times(&ran, needle), expected, "`{needle}`: {ran:#?}");
    }
    session.close();
}

/// A commit walks once and lists the stashes once: the walk takes the
/// listing's answer instead of reading its own.
#[tokio::test(flavor = "multi_thread")]
async fn a_commit_lists_the_stashes_once() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("a.txt", "two\n");
    repo.git(&["add", "--", "a.txt"]);
    let (sink, session) = opened(&repo).await;
    let from = settled(&sink, &session).await;

    let id = session
        .commit("second".into(), Default::default())
        .expect("accepted");
    assert_eq!(write_settled(&sink, id).await, []);
    let ran = commands_since(&sink, from);
    for (needle, expected) in [("log -z", 1), ("stash list", 1), ("worktree list", 1)] {
        assert_eq!(times(&ran, needle), expected, "`{needle}`: {ran:#?}");
    }
    session.close();
}

/// A worktree on no branch is a row only the walk puts there: removing it is
/// walked once, with the listing that no longer has it — not before the
/// listing and again after.
#[tokio::test(flavor = "multi_thread")]
async fn removing_a_worktree_on_no_branch_walks_once() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let worktree = repo.path.with_file_name("loose");
    let worktree_arg = worktree.to_string_lossy().into_owned();
    repo.git(&["worktree", "add", "--detach", &worktree_arg]);
    repo.git(&[
        "-C",
        &worktree_arg,
        "commit",
        "--allow-empty",
        "-m",
        "only the worktree has this",
    ]);
    let (sink, session) = opened(&repo).await;
    let from = settled(&sink, &session).await;

    let id = session
        .remove_worktree(worktree_arg.clone(), "loose".into())
        .expect("accepted");
    assert_eq!(write_settled(&sink, id).await, []);
    let ran = commands_since(&sink, from);
    assert_eq!(times(&ran, "log -z"), 1, "{ran:#?}");
    assert_eq!(
        times(&ran, "worktree list"),
        2,
        "the write's and the listing's: {ran:#?}"
    );
    session.close();
}

/// The window coming back after a commit made in a terminal: the refs moved
/// and the tree came clean, and the graph is asked for once, with the
/// stash list read once for the listing and the walk together.
#[tokio::test(flavor = "multi_thread")]
async fn coming_back_to_a_commit_made_outside_reads_each_thing_once() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("a.txt", "two\n");
    let (sink, session) = opened(&repo).await;
    let from = settled(&sink, &session).await;
    repo.git(&["commit", "-am", "made in a terminal"]);

    session.refresh_quick();
    reads_over(&session).await;
    bounded("the other worktrees", session.wait_for_carried_pass()).await;
    let ran = commands_since(&sink, from);
    for (needle, expected) in [
        ("for-each-ref", 1),
        ("status", 1),
        ("stash list", 1),
        ("log -z", 1),
    ] {
        assert_eq!(times(&ran, needle), expected, "`{needle}`: {ran:#?}");
    }
    session.close();
}

/// Coming back to a repository nothing moved in reads each snapshot once
/// and walks nothing.
#[tokio::test(flavor = "multi_thread")]
async fn coming_back_to_nothing_new_walks_nothing() {
    let repo = with_a_stash();
    let (sink, session) = opened(&repo).await;
    let from = settled(&sink, &session).await;

    session.refresh_quick();
    reads_over(&session).await;
    bounded("the other worktrees", session.wait_for_carried_pass()).await;
    let ran = commands_since(&sink, from);
    for (needle, expected) in [("stash list", 1), ("log -z", 0)] {
        assert_eq!(times(&ran, needle), expected, "`{needle}`: {ran:#?}");
    }
    session.close();
}

/// A stash dropped in a terminal moves no ref the refs read lists, nor
/// HEAD, nor the tree: the stash listing is the only read that sees it,
/// and coming back walks the history for it.
#[tokio::test(flavor = "multi_thread")]
async fn a_stash_dropped_outside_is_walked_off_when_the_window_comes_back() {
    let mut repo = with_a_stash();
    let (sink, session) = opened(&repo).await;
    let from = settled(&sink, &session).await;
    assert_eq!(drawn_stashes(&sink), ["stash@{0}"], "the stash is drawn");
    repo.git(&["stash", "drop"]);

    session.refresh_quick();
    reads_over(&session).await;
    bounded("the other worktrees", session.wait_for_carried_pass()).await;
    let ran = commands_since(&sink, from);
    assert_eq!(times(&ran, "log -z"), 1, "{ran:#?}");
    assert_eq!(
        drawn_stashes(&sink),
        Vec::<String>::new(),
        "the dropped stash is gone"
    );
    session.close();
}

/// This tree's read the window coming back asks for at its pace
/// (`RepoSession::poll_now`) reads what the focus read does: each listing
/// once, the stashes included, and one walk for a commit made outside.
#[tokio::test(flavor = "multi_thread")]
async fn the_paced_read_the_window_asks_for_reads_each_thing_once() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("a.txt", "two\n");
    let (sink, session) = opened(&repo).await;
    let from = settled(&sink, &session).await;
    repo.git(&["commit", "-am", "made in a terminal"]);

    asked_paced_read(&sink, &session, from).await;
    let ran = commands_since(&sink, from);
    for (needle, expected) in [
        ("for-each-ref", 1),
        ("status", 1),
        ("stash list", 1),
        ("worktree list", 1),
        ("log -z", 1),
    ] {
        assert_eq!(times(&ran, needle), expected, "`{needle}`: {ran:#?}");
    }
    session.close();
}

/// A stash dropped in a terminal is walked off by the read the window
/// asks for, as by the focus read: the stash listing rides that read alone.
#[tokio::test(flavor = "multi_thread")]
async fn a_stash_dropped_outside_is_walked_off_by_the_paced_read_the_window_asks_for() {
    let mut repo = with_a_stash();
    let (sink, session) = opened(&repo).await;
    let from = settled(&sink, &session).await;
    assert_eq!(drawn_stashes(&sink), ["stash@{0}"], "the stash is drawn");
    repo.git(&["stash", "drop"]);

    asked_paced_read(&sink, &session, from).await;
    let ran = commands_since(&sink, from);
    assert_eq!(times(&ran, "log -z"), 1, "{ran:#?}");
    assert_eq!(
        drawn_stashes(&sink),
        Vec::<String>::new(),
        "the dropped stash is gone"
    );
    session.close();
}

/// Paces the page with bounds no read falls due under, asks for this
/// tree's read as the window coming back does, and waits for its end —
/// the walk it asked for included, which runs inside the read.
async fn asked_paced_read(sink: &CaptureSink, session: &Arc<RepoSession>, from: usize) {
    let hour = std::time::Duration::from_secs(3600);
    session.set_pace_bounds(PaceBounds {
        own_floor: hour,
        own_ceiling: hour,
        worktree_floor: hour,
        worktree_ceiling: hour,
    });
    session.set_paced(true);
    session.poll_now();
    sink.wait_for("this tree's paced read", |events| {
        events[from..]
            .iter()
            .any(|e| matches!(e, SessionEvent::PacedRead { worktree: None }))
            .then_some(())
    })
    .await;
    reads_over(session).await;
}

/// The stash list a focus read is not walked once a newer listing has
/// published: the focus asks for its walk last, and with the older list it
/// would draw back a stash the newer walk took off.
///
/// A refs read of its own is parked holding the refs flight, so the
/// focus's refs read waits behind it while its status and stash list come
/// in; the stash is then dropped and a walk lands without it; let go, the
/// focus walks (its status turned the tree dirty) and must not bring the
/// stash back. Parking the focus's own refs read would park its whole
/// task — the three reads are one join.
// `worker_threads = 2`: the hook below parks a worker (`CaptureSink::hook_once`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_focus_does_not_walk_a_stash_list_a_newer_one_replaced() {
    let mut repo = with_a_stash();
    let (sink, session) = opened(&repo).await;
    let from = settled(&sink, &session).await;
    assert_eq!(drawn_stashes(&sink), ["stash@{0}"], "the stash is drawn");
    // The tree turns dirty, so the focus walks once it is let go.
    repo.write_file("a.txt", "dirty\n");

    let (release, held) = std::sync::mpsc::channel::<()>();
    sink.hook_once(
        |e| matches!(e, SessionEvent::RefsLoaded { .. }),
        move || {
            held.recv().expect("the test releases the parked refs read");
        },
    );
    session.refresh_refs();
    sink.wait_for("the refs read parked", |events| {
        events[from..]
            .iter()
            .any(|e| matches!(e, SessionEvent::RefsLoaded { .. }))
            .then_some(())
    })
    .await;
    session.refresh_quick();
    sink.wait_for("the focus's stash list", |events| {
        events[from..]
            .iter()
            .any(|e| matches!(e, SessionEvent::StashesLoaded { stashes, .. } if stashes.len() == 1))
            .then_some(())
    })
    .await;

    repo.git(&["stash", "drop"]);
    let walked = bounded(
        "a walk with the stash dropped",
        session.refresh_log_tracked().outcome(),
    )
    .await;
    assert!(walked.landed(), "{walked:?}");
    assert_eq!(
        drawn_stashes(&sink),
        Vec::<String>::new(),
        "the newer walk drops it"
    );

    let released = sink.events.lock().unwrap().len();
    release.send(()).expect("let the parked refs read go");
    reads_over(&session).await;
    let ran = commands_since(&sink, released);
    assert_eq!(
        times(&ran, "log -z"),
        1,
        "the focus walked, or the check below proves nothing: {ran:#?}"
    );
    assert_eq!(
        drawn_stashes(&sink),
        Vec::<String>::new(),
        "the focus's walk drew the dropped stash back"
    );
    session.close();
}
