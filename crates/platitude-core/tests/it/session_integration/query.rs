//! Reads that reuse what a read already landed, rather than asking git again.

use std::sync::Arc;

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, opened, write_result};
use platitude_core::details::DiffTarget;
use platitude_core::session::{RepoSession, SessionEvent};

/// Opening a repository asks for a read, and so does the window becoming
/// active a moment later; on a large repository that pair would be two
/// `for-each-ref` and two `status -uall` for one answer. The second
/// caller books a repeat instead of starting its own — and the point
/// of booking rather than dropping is that the repeat still sees what
/// happened in between.
#[tokio::test(flavor = "multi_thread")]
async fn a_request_made_while_a_read_runs_gets_a_read_of_its_own() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    let (sink, session) = opened(&repo).await;
    sink.wait_for("the opening status read", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::StatusLoaded { .. }))
            .then_some(())
    })
    .await;

    // Park a read on its way out, which is where a reader stands after it
    // has seen the repository and before it asks whether to go round
    // again. Everything below happens inside that window.
    let (release, held) = std::sync::mpsc::channel::<()>();
    let clean_reads = sink
        .count(|e| matches!(e, SessionEvent::StatusLoaded { status, .. } if !status.is_dirty()));
    sink.hook_once(
        |e| matches!(e, SessionEvent::StatusLoaded { status, .. } if !status.is_dirty()),
        move || {
            held.recv().expect("the test releases the status read");
        },
    );
    session.refresh_status();
    sink.wait_for("the read reached the window", |events| {
        (events
            .iter()
            .filter(
                |e| matches!(e, SessionEvent::StatusLoaded { status, .. } if !status.is_dirty()),
            )
            .count()
            > clean_reads)
            .then_some(())
    })
    .await;

    // The tree turns dirty behind the parked read — so what it is holding
    // is already out of date — and somebody asks again. Dropping that ask
    // for being a duplicate is what this is here to catch: the only read
    // that could answer it is the one that already looked.
    repo.write_file("f.txt", "dirty\n");
    session.refresh_status();
    release.send(()).expect("let the read finish");

    sink.wait_for("a status read that sees the dirty tree", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::StatusLoaded { status, .. } if status.is_dirty()))
            .then_some(())
    })
    .await;
    session.close();
}

fn commands_of(sink: &CaptureSink) -> Vec<String> {
    sink.events
        .lock()
        .unwrap()
        .iter()
        .filter_map(|e| match e {
            SessionEvent::CommandStarted { display, .. } => Some(display.clone()),
            _ => None,
        })
        .collect()
}

/// The refs listing already marks the branch HEAD is on, so a refs read
/// does not ask a second and third process where HEAD is.
#[tokio::test(flavor = "multi_thread")]
async fn a_refs_read_takes_head_out_of_the_listing_it_already_has() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    let (sink, session) = opened(&repo).await;
    // Complete the opening work before clearing the observer; `Opened`
    // alone only accepts the path.
    sink.opened_graph_gen(&session, 1).await;
    session.set_record_background(true);
    sink.events.lock().unwrap().clear();

    session.refresh_refs();
    sink.wait_for("the refs answer", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::RefsLoaded { .. }))
            .then_some(())
    })
    .await;
    let seen = commands_of(&sink);
    assert!(
        !seen.iter().any(|c| c.contains("symbolic-ref")),
        "HEAD came out of the listing: {seen:?}"
    );
    assert!(
        !seen.iter().any(|c| c.contains("rev-parse --verify")),
        "and so did the commit it is on: {seen:?}"
    );
    session.close();
}

/// Detached HEAD is the case the listing cannot answer — no ref is marked
/// — and it still gets a right answer, by asking.
#[tokio::test(flavor = "multi_thread")]
async fn a_detached_head_is_still_read_correctly() {
    let mut repo = TestRepo::init();
    let root = repo.commit_file("f.txt", "0\n", "root");
    repo.commit_file("g.txt", "1\n", "second");
    repo.git(&["checkout", "--detach", &root]);

    let (sink, session) = opened(&repo).await;
    let head = sink
        .wait_for("the refs snapshot", |evs| {
            evs.iter().rev().find_map(|e| match e {
                SessionEvent::RefsLoaded { snapshot } => snapshot.head.clone(),
                _ => None,
            })
        })
        .await;
    assert!(head.detached, "{head:?}");
    assert_eq!(head.branch, None);
    assert_eq!(head.oid.map(|o| o.to_hex()), Some(root));
    session.close();
}

/// A read that finds nothing moved publishes the snapshot it published
/// last — the same one, by pointer — instead of building an equal one.
///
/// Sorting every ref into a snapshot and a label map on every tick just
/// to compare the result equal costs 39ms of a core against
/// `JetBrains/kotlin`, ten seconds apart, for an answer the key already
/// had.
#[tokio::test(flavor = "multi_thread")]
async fn an_unmoved_repository_republishes_the_snapshot_it_already_built() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    repo.git(&["branch", "side"]);
    let (sink, session) = opened(&repo).await;

    let latest = |sink: &CaptureSink| {
        sink.events
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find_map(|e| match e {
                SessionEvent::RefsLoaded { snapshot } => Some(Arc::clone(snapshot)),
                _ => None,
            })
            .expect("a snapshot")
    };
    let settle = async |want: usize| {
        sink.wait_for("a refs snapshot", move |evs| {
            (evs.iter()
                .filter(|e| matches!(e, SessionEvent::RefsLoaded { .. }))
                .count()
                >= want)
                .then_some(())
        })
        .await
    };

    settle(1).await;
    let first = latest(&sink);
    let before = sink.count(|e| matches!(e, SessionEvent::RefsLoaded { .. }));

    session.refresh_refs();
    settle(before + 1).await;
    let again = latest(&sink);
    assert!(
        Arc::ptr_eq(&first, &again),
        "the same snapshot, not an equal one"
    );

    // A ref really moving still rebuilds, and the sidebar is told.
    repo.git(&["branch", "-D", "side"]);
    let before = sink.count(|e| matches!(e, SessionEvent::RefsLoaded { .. }));
    session.refresh_refs();
    settle(before + 1).await;
    let after = latest(&sink);
    assert!(
        !Arc::ptr_eq(&first, &after),
        "a moved ref is a new snapshot"
    );
    assert_eq!(
        after.locals.len(),
        1,
        "and it is the repository as it stands: {:?}",
        after.locals
    );
    session.close();
}

/// Once a refs read has said where HEAD is, the walk stops asking.
///
/// Spawning `symbolic-ref` and `rev-parse` before every rebuild would put
/// two processes in front of the first chunk, on the path a commit or a
/// fetch takes to reach the screen.
#[tokio::test(flavor = "multi_thread")]
async fn the_walk_reads_head_from_the_refs_read_that_already_landed() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    let (sink, session) = opened(&repo).await;
    sink.opened_graph_gen(&session, 1).await;
    session.set_record_background(true);
    sink.events.lock().unwrap().clear();

    // An external commit moves the refs, which is what rebuilds the graph.
    repo.commit_file("g.txt", "1\n", "second");
    session.refresh_refs();
    sink.settled_stream_gen(2).await;

    let seen = commands_of(&sink);
    assert!(
        seen.iter().any(|c| c.contains("log -z")),
        "the walk did run: {seen:?}"
    );
    assert!(
        !seen.iter().any(|c| c.contains("symbolic-ref")),
        "and did not ask where HEAD is: {seen:?}"
    );
    session.close();
}

/// The remotes are read once per refs listing no more: a poll tick that
/// finds nothing moved spawns no `git config` to re-read them. A write
/// puts the question back, because a write is what can add one.
#[tokio::test(flavor = "multi_thread")]
async fn the_remotes_are_read_once_until_something_could_have_changed_them() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    let (sink, session) = opened(&repo).await;
    sink.opened_graph_gen(&session, 1).await;
    session.set_record_background(true);
    sink.events.lock().unwrap().clear();

    let reads = |sink: &CaptureSink| {
        commands_of(sink)
            .iter()
            .filter(|c| c.contains("remote\\..*\\.(url|pushurl)"))
            .count()
    };
    // Counted in snapshots delivered, not in elapsed time: a read that is
    // merely slow must not read as a read that did not happen.
    let snapshots =
        |sink: &CaptureSink| sink.count(|e| matches!(e, SessionEvent::RefsLoaded { .. })) as u32;
    let settle = async |want: u32| {
        sink.wait_for("a refs snapshot", move |evs| {
            (evs.iter()
                .filter(|e| matches!(e, SessionEvent::RefsLoaded { .. }))
                .count() as u32
                >= want)
                .then_some(())
        })
        .await
    };

    // The opening boundary already warmed it. An unchanged listing reuses
    // that answer, even though background command recording only starts now.
    session.refresh_refs();
    settle(1).await;
    assert_eq!(reads(&sink), 0, "{:?}", commands_of(&sink));

    // A write can add a remote, so it drops the answer. Its post-write refs
    // snapshot is the causal boundary after the replacement read.
    let before_write = snapshots(&sink);
    session.create_branch("side".into(), None, false);
    write_result(&sink, "branch").await;
    settle(before_write + 1).await;
    assert_eq!(reads(&sink), 1, "{:?}", commands_of(&sink));

    // Now nothing moves, and the listings that follow ask git nothing.
    let from = snapshots(&sink);
    for n in 1..=3 {
        session.refresh_refs();
        settle(from + n).await;
    }
    assert_eq!(reads(&sink), 1, "still the one: {:?}", commands_of(&sink));
    session.close();
}

/// Whether git normalises line endings is repository configuration, so
/// concurrent diff reads share one in-flight settings query.
#[tokio::test(flavor = "multi_thread")]
async fn concurrent_diffs_share_the_line_ending_setting_read() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "two\n", "second");
    let (sink, session) = opened(&repo).await;
    sink.opening_snapshots().await;
    session.set_record_background(true);
    sink.events.lock().unwrap().clear();

    let head = platitude_core::Oid::from_hex_str(&repo.git(&["rev-parse", "HEAD"])).unwrap();
    let parent = platitude_core::Oid::from_hex_str(&repo.git(&["rev-parse", "HEAD^"])).unwrap();
    for path in ["a.txt", "b.txt"] {
        session.load_diff(DiffTarget::Commit {
            oid: head,
            parent: Some(parent),
            path: path.to_string(),
            orig_path: None,
        });
    }
    sink.wait_for("both concurrent diffs", |events| {
        ["a.txt", "b.txt"]
            .iter()
            .all(|path| {
                events.iter().any(|event| {
                    matches!(event, SessionEvent::DiffLoaded { target, .. }
                        if matches!(target, DiffTarget::Commit { path: seen, .. }
                            if seen == path))
                })
            })
            .then_some(())
    })
    .await;

    let reads = commands_of(&sink)
        .iter()
        .filter(|c| c.contains("autocrlf"))
        .count();
    assert_eq!(reads, 1, "{:?}", commands_of(&sink));
    session.close();
}

/// The publish check answers through its own event, so a UI can warn
/// before rewriting history a remote already has.
#[tokio::test(flavor = "multi_thread")]
async fn publish_check_answers_through_the_session() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "two\n", "second");

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        crate::support::exec::isolated(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    sink.wait_for("Opened", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::Opened { .. }))
            .then_some(())
    })
    .await;

    session.check_publish("HEAD~1..HEAD".into());
    let state = sink
        .wait_for("PublishChecked", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::PublishChecked { range, state } if range == "HEAD~1..HEAD" => {
                    Some(*state)
                }
                _ => None,
            })
        })
        .await;
    assert_eq!(state.total, 1);
    assert!(!state.rewrites_published(), "nothing is on a remote");
    session.close();
}
