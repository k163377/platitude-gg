//! When a pass rebuilds the graph, and when it leaves the one on screen
//! alone.

use std::sync::Arc;
use std::time::Duration;

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, is_stream_event, scenario};
use platitude_core::GitExecutor;
use platitude_core::session::{RepoSession, SessionEvent};

/// A write rebuilds the graph exactly once. Committing turns a dirty tree
/// clean, which removes the WIP row; reacting to that separately from the
/// write itself would stream the whole graph twice for one action.
#[tokio::test(flavor = "multi_thread")]
async fn a_write_rebuilds_the_graph_once() {
    let mut repo = TestRepo::init();
    repo.commit_file("root.txt", "0\n", "root");
    repo.write_file("new.txt", "content\n");

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    // Wait until the WIP row is on screen (root + WIP = 2 rows) and the
    // stream has settled, so the commit below is the transition that
    // removes it and every later stream event is a reaction to a write.
    sink.settled_stream_gen(2).await;

    session.stage_all();
    session.commit(
        "add new file".into(),
        platitude_core::commit::CommitOptions::default(),
    );

    // Counting from where each write finished ignores whatever the open
    // sequence was still doing, which a wall-clock delay would not.
    sink.wait_for("the commit's rebuild finished", |evs| {
        let commit_at = position_of(evs, "commit")?;
        evs[commit_at..]
            .iter()
            .any(|e| matches!(e, SessionEvent::LogReplaced { .. }))
            .then_some(())
    })
    .await;
    // Give a trailing second rebuild (the regression this guards against)
    // time to show up before counting.
    tokio::time::sleep(Duration::from_millis(400)).await;

    let events = sink.events.lock().unwrap();
    let stage_at = position_of(&events, "stage").expect("stage finished");
    let commit_at = position_of(&events, "commit").expect("commit finished");
    assert_eq!(
        log_starts(&events[stage_at..commit_at]),
        0,
        "staging left the tree dirty, so the graph did not change"
    );
    assert_eq!(
        log_starts(&events[commit_at..]),
        0,
        "the rebuild replaces atomically; it never resets and re-streams"
    );
    assert_eq!(
        events[commit_at..]
            .iter()
            .filter(|e| matches!(e, SessionEvent::LogReplaced { .. }))
            .count(),
        1,
        "the commit replaced the rebuilt graph in exactly once"
    );
    drop(events);
    session.close();
}

/// Opens a session over `scenario()`, settles the first 5-row graph, then
/// holds `refresh` to silence: a background pass over an unchanged
/// repository must not emit a single stream event ("nothing happens" can
/// only be observed by giving the pass ample time to run). Returns the
/// stream-event count to measure "after" against. The quiet half of both
/// refresh entry points is the same promise, so it is written once.
async fn settled_and_silent(
    refresh: impl Fn(&Arc<RepoSession>),
) -> (TestRepo, Arc<CaptureSink>, Arc<RepoSession>, usize) {
    let (repo, _) = scenario();
    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    sink.settled_stream_gen(5).await;
    let baseline = sink.count(is_stream_event);

    refresh(&session);
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(
        sink.count(is_stream_event),
        baseline,
        "an unchanged rebuild stayed silent: {:?}",
        sink.events.lock().unwrap()
    );
    (repo, sink, session, baseline)
}

/// A background rebuild that finds nothing changed must stay silent — no
/// reset, no chunk, no repaint. This is what keeps a quiet auto-fetch
/// interval (or any other background refresh) from flickering the graph.
#[tokio::test(flavor = "multi_thread")]
async fn background_refresh_swaps_only_on_change() {
    let (mut repo, sink, session, baseline) = settled_and_silent(|s| s.refresh_log()).await;

    // History moved outside the session: the same call now delivers one
    // atomic replacement — a single LogReplaced carrying every row, so
    // the consumer never holds an empty model in between.
    repo.commit_file("h.txt", "x\n", "outside commit");
    session.refresh_log();
    let swap_gen = sink.settled_stream_gen(6).await;
    let events = sink.events.lock().unwrap();
    let after: Vec<&SessionEvent> = events
        .iter()
        .filter(|e| is_stream_event(e))
        .skip(baseline)
        .collect();
    assert_eq!(after.len(), 1, "one event for the whole change: {after:?}");
    match after[0] {
        SessionEvent::LogReplaced {
            generation, rows, ..
        } => {
            assert_eq!(*generation, swap_gen);
            assert_eq!(rows.len(), 6, "the replacement carries the whole graph");
        }
        other => panic!("expected LogReplaced, got {other:?}"),
    }
    drop(events);
    session.close();
}

/// A ref that moved outside the session (a commit in a terminal, a fetch,
/// a switch by another tool) points at commits this graph has never
/// walked, so re-reading the refs has to rebuild — chips alone cannot show
/// them. A re-read that finds every ref where it left it stays silent.
#[tokio::test(flavor = "multi_thread")]
async fn an_external_ref_move_rebuilds_the_graph() {
    let (mut repo, sink, session, _) = settled_and_silent(|s| s.refresh_refs()).await;

    // Now main moves under the session, with the working tree clean on
    // both sides: nothing but the refs can report this.
    repo.commit_file("outside.txt", "x\n", "outside commit");
    session.refresh_refs();
    sink.settled_stream_gen(6).await;
    let events = sink.events.lock().unwrap();
    let replacements = events[..]
        .iter()
        .skip_while(|e| !matches!(e, SessionEvent::LogReplaced { rows, .. } if rows.len() == 6))
        .filter(|e| matches!(e, SessionEvent::LogReplaced { .. }))
        .count();
    assert_eq!(
        replacements,
        1,
        "the moved ref rebuilt once: {:?}",
        events[..].iter().collect::<Vec<_>>()
    );
    assert_eq!(
        log_starts(&events[..]),
        1,
        "the rebuild replaced in place; only opening resets and streams"
    );
    drop(events);
    session.close();
}

/// Chips are diffed against the graph that is on screen, so they are only
/// ever sent for that one. A rebuild landing in the middle of a refs read
/// moves every commit down a row (the WIP row goes in at the top), and row
/// numbers taken before it name other commits after it. Nothing takes such
/// a mistake back either: the session believes those chips are on screen,
/// so the next read has nothing to say and the next rebuild nothing to
/// swap.
///
/// The hook makes the interleaving exact rather than hoped for: it holds
/// the read at the sink call that publishes its snapshot while the test
/// rebuilds the graph under it.
#[tokio::test(flavor = "multi_thread")]
async fn chips_read_from_one_graph_do_not_land_on_another() {
    let mut repo = TestRepo::init();
    let root = repo.commit_file("f.txt", "0\n", "root");
    repo.commit_file("f.txt", "1\n", "middle");
    let head = repo.commit_file("f.txt", "2\n", "head");

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    sink.settled_stream_gen(3).await;

    // Something for the read to find, on the last row of the graph it
    // reads it from: a chip that travels as a diff instead of with a walk.
    repo.git(&["tag", "v2", &root]);

    let (arrived, at_the_window) = tokio::sync::oneshot::channel();
    let (release, held) = std::sync::mpsc::channel::<()>();
    sink.hook_once(
        |e| {
            matches!(e, SessionEvent::RefsLoaded { snapshot }
                if snapshot.tags.iter().any(|t| t.short == "v2"))
        },
        move || {
            let _ = arrived.send(());
            let _ = held.recv_timeout(Duration::from_secs(20));
        },
    );
    session.refresh_refs();
    at_the_window.await.expect("the read reached the window");

    // Rebuilt from here, with the read held: dirtying the tree puts the
    // WIP row at the top, so every row number that read took moves down
    // one.
    repo.write_file("f.txt", "dirty\n");
    session.refresh_status();
    sink.wait_for("the rebuild that adds the WIP row", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::LogReplaced { rows, .. } if rows.len() == 4))
            .then_some(())
    })
    .await;
    release.send(()).expect("let the read finish");

    sink.settled_stream_gen(4).await;
    // Chips travel on an event of their own: let a late one land rather
    // than reading the graph before it could have arrived.
    tokio::time::sleep(Duration::from_millis(400)).await;

    let events = sink.events.lock().unwrap();
    let rows = crate::support::replay_graph(&events[..]);
    let wearing = |name: &str| -> Vec<&str> {
        rows.values()
            .filter(|seen| seen.labels.iter().any(|l| l.text == name))
            .map(|seen| seen.oid_hex.as_str())
            .collect()
    };
    assert_eq!(
        wearing("v2"),
        vec![root.as_str()],
        "the tag reached the commit it names, and only it: {rows:?}"
    );
    assert_eq!(
        wearing("main"),
        vec![head.as_str()],
        "and the branch stayed where it was: {rows:?}"
    );
    drop(events);
    session.close();
}

/// A pass that was superseded before it could start leaves the graph
/// alone. Which pass is in charge is decided when somebody asks (both
/// entry points cancel the running token before spawning), not by the
/// order the tasks happen to reach the lock — so a reset that arrives
/// late must not clear what is on screen, wiping the record a rebuild
/// compares against and leaving every later chip diff numbered for a
/// graph nobody was ever shown.
///
/// Held under the graph lock, the interleaving is exact: the losing pass
/// cannot reach its reset before the cancel that supersedes it.
#[tokio::test(flavor = "multi_thread")]
async fn a_pass_nobody_asked_for_any_more_leaves_the_graph_alone() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    repo.commit_file("f.txt", "1\n", "middle");
    repo.commit_file("f.txt", "2\n", "head");

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    sink.settled_stream_gen(3).await;

    // Park in the swap that adds the WIP row: it sends under the graph
    // lock, so everything else is stopped at the door with the graph
    // fully installed behind it.
    let (arrived, at_the_window) = tokio::sync::oneshot::channel();
    let (release, held) = std::sync::mpsc::channel::<()>();
    sink.hook_once(
        |e| matches!(e, SessionEvent::LogReplaced { rows, .. } if rows.len() == 4),
        move || {
            let _ = arrived.send(());
            let _ = held.recv_timeout(Duration::from_secs(20));
        },
    );
    repo.write_file("f.txt", "dirty\n");
    session.refresh_status();
    at_the_window.await.expect("the rebuild reached the window");
    let settled = sink.count(|_| true);

    // Asked for, then superseded while it waits for the lock.
    session.restart_log();
    tokio::time::sleep(Duration::from_millis(200)).await;
    session.refresh_log();
    release.send(()).expect("let the rebuild finish");

    // Long enough for both to have run: the superseded stream (which
    // only has to take the lock) and the rebuild behind it (a whole
    // walk, which then finds the graph unchanged and skips its swap).
    tokio::time::sleep(Duration::from_millis(1500)).await;

    let events = sink.events.lock().unwrap();
    let after: Vec<&SessionEvent> = events[settled..]
        .iter()
        .filter(|e| {
            matches!(
                e,
                SessionEvent::LogStarted { .. } | SessionEvent::LogReplaced { .. }
            )
        })
        .collect();
    assert!(
        after.is_empty(),
        "nothing repainted the graph: {after:?}\nall: {:?}",
        events[settled..].iter().collect::<Vec<_>>()
    );

    let rows = crate::support::replay_graph(&events[..]);
    assert_eq!(rows.len(), 4, "the WIP row and three commits: {rows:?}");
    drop(events);
    session.close();
}

/// One tick, one rebuild. A commit made outside the session moves a ref
/// *and* turns the tree clean, and the poll reads both: walking the
/// history once per reader would throw a whole pass away every time
/// someone else commits.
#[tokio::test(flavor = "multi_thread")]
async fn a_poll_rebuilds_the_graph_once() {
    let mut repo = TestRepo::init();
    repo.commit_file("root.txt", "0\n", "root");
    repo.write_file("new.txt", "content\n");

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        GitExecutor::new(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    // root + WIP row.
    sink.settled_stream_gen(2).await;

    let replacements = || sink.count(|e| matches!(e, SessionEvent::LogReplaced { .. }));
    let starts = || sink.count(|e| matches!(e, SessionEvent::LogStarted { .. }));
    let (quiet_replacements, quiet_starts) = (replacements(), starts());

    // An idle repository is what the poll spends nearly all its ticks on.
    let quiet_refs = sink.count(|e| matches!(e, SessionEvent::RefsLoaded { .. }));
    let quiet_status = sink.count(|e| matches!(e, SessionEvent::StatusLoaded { .. }));
    session.refresh_poll();
    // What has to be over before the commit below is this tick, and the
    // tick says so itself: it publishes both of its reads whatever it
    // finds, so one more of each is it landing (規約 §「もう起きない」を
    // sleep で確かめない). A fixed wait fails the assertion two
    // paragraphs down the moment the machine is busy enough for the tick
    // to outlast it — the poll's two reads then straddle the commit,
    // report different worlds, and the graph rebuilds once for each.
    sink.wait_for("the idle poll's two reads", |evs| {
        let refs = evs
            .iter()
            .filter(|e| matches!(e, SessionEvent::RefsLoaded { .. }))
            .count();
        let status = evs
            .iter()
            .filter(|e| matches!(e, SessionEvent::StatusLoaded { .. }))
            .count();
        (refs > quiet_refs && status > quiet_status).then_some(())
    })
    .await;
    assert_eq!(
        (replacements(), starts()),
        (quiet_replacements, quiet_starts),
        "a poll over an unchanged repository stayed silent: {:?}",
        sink.events.lock().unwrap()
    );

    // Both signals move at once: `new.txt` becomes a commit, so the ref
    // advances and the WIP row goes away.
    repo.commit_file("new.txt", "content\n", "outside commit");
    // A poll steps aside while another one is still running, so the tick
    // that sees the commit need not be the first one asked for — the
    // ticker would simply ask again. Asking again cannot add a rebuild of
    // its own: a poll over a repository that has not moved is silent,
    // which is exactly what the paragraph above established.
    let rebuilt = sink.wait_for("the poll's rebuild", |evs| {
        evs.iter()
            .filter(|e| matches!(e, SessionEvent::LogReplaced { .. }))
            .count()
            .gt(&quiet_replacements)
            .then_some(())
    });
    tokio::pin!(rebuilt);
    loop {
        session.refresh_poll();
        tokio::select! {
            () = &mut rebuilt => break,
            () = tokio::time::sleep(Duration::from_millis(100)) => {}
        }
    }
    // Give the second rebuild this guards against time to show up.
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert_eq!(
        replacements(),
        quiet_replacements + 1,
        "the moved ref and the cleaned tree rebuilt once between them: {:?}",
        sink.events.lock().unwrap()
    );
    assert_eq!(
        starts(),
        quiet_starts,
        "the rebuild replaced in place; a poll never resets the graph"
    );
    session.close();
}

fn position_of(events: &[SessionEvent], op: &str) -> Option<usize> {
    events
        .iter()
        .position(|e| matches!(e, SessionEvent::WriteFinished { op: got, .. } if *got == op))
}

fn log_starts(events: &[SessionEvent]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, SessionEvent::LogStarted { .. }))
        .count()
}
