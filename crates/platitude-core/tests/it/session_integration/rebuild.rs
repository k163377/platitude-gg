//! When a pass rebuilds the graph, and when it leaves the one on screen
//! alone.

use std::sync::Arc;

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, is_stream_event, open_unawaited, opened, scenario};
use platitude_core::OperationKind;
use platitude_core::session::{Recording, RefreshOutcome, RepoSession, SessionEvent};

/// A write rebuilds the graph exactly once. Committing turns a dirty tree
/// clean, which removes the WIP row; reacting to that separately from the
/// write itself would stream the whole graph twice for one action.
#[tokio::test(flavor = "multi_thread")]
async fn a_write_rebuilds_the_graph_once() {
    let mut repo = TestRepo::init();
    repo.commit_file("root.txt", "0\n", "root");
    repo.write_file("new.txt", "content\n");

    let (sink, session) = open_unawaited(&repo);
    // Wait until the WIP row is on screen (root + WIP = 2 rows) behind an
    // explicit opening boundary, so the commit below is the transition
    // that removes it and every later stream event is a reaction to a write.
    sink.opened_graph(&session, 2).await;

    session.stage_all();
    session.commit(
        "add new file".into(),
        platitude_core::commit::CommitOptions::default(),
    );
    // A no-op write behind the commit is a queue barrier. `WriteStarted`
    // cannot be delivered until the commit's refresh has itself completed,
    // so it cannot cancel or overlook a trailing rebuild.
    session.stage_all();

    let barrier_at = sink
        .wait_for("the write behind the commit", |evs| {
            let commit_at = position_of(evs, OperationKind::Commit)?;
            evs[commit_at..]
                .iter()
                .position(|e| {
                    matches!(
                        e,
                        SessionEvent::WriteStarted {
                            kind: OperationKind::Stage,
                            ..
                        }
                    )
                })
                .map(|after| commit_at + after)
        })
        .await;

    {
        let events = sink.events.lock().unwrap();
        let stage_at = position_of(&events, OperationKind::Stage).expect("stage finished");
        let commit_at = position_of(&events, OperationKind::Commit).expect("commit finished");
        assert_eq!(
            log_starts(&events[stage_at..commit_at]),
            0,
            "staging left the tree dirty, so the graph did not change"
        );
        assert_eq!(
            log_starts(&events[commit_at..barrier_at]),
            0,
            "the rebuild replaces atomically; it never resets and re-streams"
        );
        assert_eq!(
            events[commit_at..barrier_at]
                .iter()
                .filter(|e| matches!(e, SessionEvent::LogReplaced { .. }))
                .count(),
            1,
            "the commit replaced the rebuilt graph exactly once"
        );
    }
    sink.wait_for("the barrier write to finish", |events| {
        (events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    SessionEvent::WriteFinished {
                        kind: OperationKind::Stage,
                        ..
                    }
                )
            })
            .count()
            == 2)
            .then_some(())
    })
    .await;
    session.close();
}

/// Opens `scenario()` and takes a baseline only after the opening graph
/// and snapshot reads have answered. The operation under test supplies
/// its own completion boundary, and that is what this helper waits
/// on.
async fn settled_graph() -> (TestRepo, Arc<CaptureSink>, Arc<RepoSession>, usize) {
    let (repo, _) = scenario();
    let (sink, session) = open_unawaited(&repo);
    sink.opened_graph(&session, 5).await;
    let baseline = sink.count(is_stream_event);
    (repo, sink, session, baseline)
}

/// A background rebuild that finds nothing changed must stay silent — no
/// reset, no chunk, no repaint. This is what keeps a quiet auto-fetch
/// interval (or any other background refresh) from flickering the graph.
#[tokio::test(flavor = "multi_thread")]
async fn background_refresh_swaps_only_on_change() {
    let (mut repo, sink, session, baseline) = settled_graph().await;

    let quiet = crate::support::wait::bounded(
        "the tracked graph refresh",
        session.refresh_log_tracked().outcome(),
    )
    .await;
    assert_eq!(quiet, RefreshOutcome::Unchanged);
    assert_eq!(
        sink.count(is_stream_event),
        baseline,
        "an unchanged rebuild stayed silent: {:?}",
        sink.events.lock().unwrap()
    );

    // History moved outside the session: the same call now delivers one
    // atomic replacement — a single LogReplaced carrying every row, so
    // the consumer never holds an empty model in between.
    repo.commit_file("h.txt", "x\n", "outside commit");
    let changed = crate::support::wait::bounded(
        "the tracked graph refresh",
        session.refresh_log_tracked().outcome(),
    )
    .await;
    assert_eq!(changed, RefreshOutcome::Changed);
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
            assert!(*generation > 0);
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
    let (mut repo, sink, session, baseline) = settled_graph().await;
    // Where the opening ends in the record. Whether its own stream got
    // as far as starting before the tracked refresh took it over is the
    // scheduler's to decide (`a_rebuild_taken_over_before_it_started_never_walks`
    // is the shape where it did not), so the streams counted below are
    // the ones after this mark.
    let opened_at = sink.events.lock().unwrap().len();

    let quiet_refs = sink.count(|event| matches!(event, SessionEvent::RefsLoaded { .. }));
    session.refresh_refs();
    sink.wait_for("the unchanged refs read", |events| {
        (events
            .iter()
            .filter(|event| matches!(event, SessionEvent::RefsLoaded { .. }))
            .count()
            > quiet_refs)
            .then_some(())
    })
    .await;
    assert_eq!(
        sink.count(is_stream_event),
        baseline,
        "unchanged refs requested no graph refresh"
    );

    // Now main moves under the session, with the working tree clean on
    // both sides: nothing but the refs can report this.
    repo.commit_file("outside.txt", "x\n", "outside commit");
    session.refresh_refs();
    sink.settled_pass(6).await;
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
        log_starts(&events[opened_at..]),
        0,
        "the rebuild replaced in place: nothing has streamed since the opening"
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
/// The hook makes the interleaving exact: it holds the read at the
/// sink call that publishes its snapshot while the test rebuilds the
/// graph under it.
// `worker_threads = 2` is the test's own premise: the hook below
// parks a worker on a blocking `recv`, and a pool inherited from
// `available_parallelism` can be a single thread on a small runner —
// the parked hook then owns the only worker and nothing else runs.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn chips_read_from_one_graph_do_not_land_on_another() {
    let mut repo = TestRepo::init();
    let root = repo.commit_file_id("f.txt", "0\n", "root");
    repo.commit_file("f.txt", "1\n", "middle");
    let head = repo.commit_file_id("f.txt", "2\n", "head");

    let (sink, session) = open_unawaited(&repo);
    sink.opened_graph(&session, 3).await;

    // Something for the read to find, on the last row of the graph it
    // reads it from: a chip that travels as a diff.
    repo.git(&["tag", "v2", &root]);

    let (release, held) = std::sync::mpsc::channel::<()>();
    sink.hook_once(
        |e| {
            matches!(e, SessionEvent::RefsLoaded { snapshot, .. }
                if snapshot.tags.iter().any(|t| t.short == "v2"))
        },
        move || {
            held.recv().expect("the test releases the refs read");
        },
    );
    session.refresh_refs();
    sink.wait_for("the read reached the window", |events| {
        events
            .iter()
            .any(|event| {
                matches!(event, SessionEvent::RefsLoaded { snapshot, .. }
                    if snapshot.tags.iter().any(|tag| tag.short == "v2"))
            })
            .then_some(())
    })
    .await;

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

    sink.wait_for("the tag chips on the rebuilt graph", |evs| {
        let rows = crate::support::replay_graph(evs);
        rows.values()
            .any(|row| row.oid_hex == root && row.labels.iter().any(|label| label.text == "v2"))
            .then_some(())
    })
    .await;

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
/// alone. Which pass is in charge is decided when somebody asks —
/// both entry points cancel the running token before spawning — so a
/// reset that arrives late leaves the screen as it is. Clearing it
/// would wipe the record a rebuild compares against and leave every
/// later chip diff numbered for a graph nobody was ever
/// shown.
///
/// Held under the graph lock, the interleaving is exact: the losing pass
/// cannot reach its reset before the cancel that supersedes it.
// `worker_threads = 2`: the parked hook needs a worker to spare
// (see the sibling above).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_pass_nobody_asked_for_any_more_leaves_the_graph_alone() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    repo.commit_file("f.txt", "1\n", "middle");
    repo.commit_file("f.txt", "2\n", "head");

    let (sink, session) = open_unawaited(&repo);
    sink.opened_graph(&session, 3).await;

    // Park in the swap that adds the WIP row: it sends under the graph
    // lock, so everything else is stopped at the door with the graph
    // fully installed behind it.
    let (release, held) = std::sync::mpsc::channel::<()>();
    sink.hook_once(
        |e| matches!(e, SessionEvent::LogReplaced { rows, .. } if rows.len() == 4),
        move || {
            held.recv().expect("the test releases the graph swap");
        },
    );
    repo.write_file("f.txt", "dirty\n");
    session.refresh_status();
    sink.wait_for("the rebuild reached the window", |events| {
        events
            .iter()
            .any(|event| matches!(event, SessionEvent::LogReplaced { rows, .. } if rows.len() == 4))
            .then_some(())
    })
    .await;
    let settled = sink.count(|_| true);

    // Asked for, then superseded while it waits for the lock.
    session.restart_log();
    let refresh = session.refresh_log_tracked();
    release.send(()).expect("let the rebuild finish");
    assert_eq!(
        crate::support::wait::bounded("refresh", refresh.outcome()).await,
        RefreshOutcome::Unchanged
    );

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

    let (sink, session) = open_unawaited(&repo);
    // root + WIP row.
    sink.opened_graph(&session, 2).await;

    let replacements = || sink.count(|e| matches!(e, SessionEvent::LogReplaced { .. }));
    let starts = || sink.count(|e| matches!(e, SessionEvent::LogStarted { .. }));
    let (quiet_replacements, quiet_starts) = (replacements(), starts());

    // An idle repository is what the poll spends nearly all its ticks on.
    let idle =
        crate::support::wait::bounded("the tracked poll", session.refresh_poll_tracked().outcome())
            .await;
    assert_eq!(idle, RefreshOutcome::Unchanged);
    assert_eq!(
        crate::support::wait::bounded("the tracked poll", session.refresh_poll_tracked().outcome())
            .await,
        RefreshOutcome::Unchanged,
        "the completion returned the single-flight slot before waking us"
    );
    assert_eq!(
        (replacements(), starts()),
        (quiet_replacements, quiet_starts),
        "a poll over an unchanged repository stayed silent: {:?}",
        sink.events.lock().unwrap()
    );

    // Both signals move at once: `new.txt` becomes a commit, so the ref
    // advances and the WIP row goes away.
    repo.commit_file("new.txt", "content\n", "outside commit");
    let changed =
        crate::support::wait::bounded("the tracked poll", session.refresh_poll_tracked().outcome())
            .await;
    assert_eq!(changed, RefreshOutcome::Changed);
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

/// **One ask, one set of reads.** A restart runs two passes where tags
/// are drawn — the tag-less one that paints and the rebuild that is owed
/// the picture the tags are in (`restart_log`) — and the second starts
/// the moment the first lands, on a repository neither of them moved.
/// Asking git the walk's own questions again there is a process launch
/// apiece, which is most of what a read costs on Windows
/// (ci/baseline/code-costs-windows-x64.md).
///
/// The stash listing is the one of those reads that always goes to git:
/// HEAD is answered by the settled refs read (`known_head_tip`) and the
/// remote tips by the snapshot beside it, so counting `stash list` is
/// counting how many times the pair asked.
#[tokio::test(flavor = "multi_thread")]
async fn the_two_passes_of_one_ask_read_the_stashes_once() {
    let (_repo, sink, session, _baseline) = settled_graph().await;
    // From here the background reads are in the command log, and the
    // opening's own passes are behind this line (the baseline closed
    // them).
    session.set_recording(Recording::WithBackground);
    let ran = |needle: &'static str| {
        sink.count(
            |e| matches!(e, SessionEvent::CommandStarted { display, .. } if display.contains(needle)),
        )
    };
    assert_eq!(ran("stash list"), 0, "nothing from before the baseline");

    // A window nobody has asked for yet, which is one ask and two passes:
    // the options carry tags (`LogOptions::default`).
    session.set_log_limit(Some(platitude_core::session::DEFAULT_LOG_LIMIT + 1));
    crate::support::wait::bounded("the graph passes", session.wait_for_graph_passes()).await;

    assert_eq!(
        ran("log -z"),
        2,
        "the ask ran both of its passes: {:?}",
        commands(&sink)
    );
    assert_eq!(
        ran("stash list"),
        1,
        "and read the stashes once between them: {:?}",
        commands(&sink)
    );
    session.close();
}

/// A rebuild taken over before it started does not walk.
///
/// Asking for one cancels the pass that held the stream, and a cancelled
/// pass has nobody left to answer: what it would build is a graph that
/// has already been replaced. The walk is the most expensive read in the
/// app, so starting it to find that out is the whole of it spent.
///
/// **The opening is where this lands.** Its tag-inclusive pass waits out
/// the tag-less one that paints (`restart_log`), so a write, a poll tick
/// or a test closing its baseline in between used to leave a full history
/// walk running — and that walk's command turns up *after* the boundary
/// that was meant to close the opening, which is what made
/// `remote_tags_integration::learning_what_the_remotes_carry_…` fail
/// under load and nowhere else.
///
/// The single-threaded runtime is what makes the order a fact: a
/// spawned pass is not polled until this test awaits, so the second
/// ask is known to arrive before the first has read anything.
#[tokio::test]
async fn a_rebuild_taken_over_before_it_started_never_walks() {
    let (_repo, sink, session, _baseline) = settled_graph().await;
    // From here the background reads are in the command log, so what the
    // two asks below spend is countable — and the opening's own passes
    // are behind this line, both closed by the baseline.
    session.set_recording(Recording::WithBackground);
    let walks = || {
        sink.count(
            |e| matches!(e, SessionEvent::CommandStarted { display, .. } if display.contains("log -z")),
        )
    };
    assert_eq!(walks(), 0, "nothing is recorded from before the baseline");

    let taken_over = session.refresh_log_tracked();
    let winner = session.refresh_log_tracked();
    assert_eq!(
        crate::support::wait::bounded("taken_over", taken_over.outcome()).await,
        RefreshOutcome::Cancelled,
        "the second ask owns the stream"
    );
    assert_eq!(
        crate::support::wait::bounded("winner", winner.outcome()).await,
        RefreshOutcome::Unchanged
    );
    crate::support::wait::bounded("the graph passes", session.wait_for_graph_passes()).await;
    assert_eq!(
        walks(),
        1,
        "only the pass that still owned the stream walked: {:?}",
        commands(&sink)
    );
    session.close();
}

/// **The snapshot is built out of more than where the refs point.**
/// Setting an upstream moves no ref at all — it is two config keys — but
/// the listing carries what each branch reads and how far it stands from
/// it, and the sidebar's badge and the row's chips are built from that.
///
/// A key that answered only for positions republished the snapshot from
/// before the write, so the answer just given was nowhere on screen until
/// something else moved a ref (observed: `Set upstream…` redrew nothing).
#[tokio::test(flavor = "multi_thread")]
async fn setting_an_upstream_rebuilds_what_the_rows_read() {
    let mut origin = TestRepo::init();
    origin.commit_file("a.txt", "one\n", "root");
    origin.git(&["branch", "other"]);

    let mut work = TestRepo::init();
    let url = origin.file_url();
    work.git(&["remote", "add", "origin", &url]);
    work.git(&["fetch", "origin"]);
    work.git(&["checkout", "-b", "topic", "origin/main"]);

    let (sink, session) = opened(&work).await;
    sink.opening_snapshots().await;

    session.set_upstream("topic".into(), "origin".into(), "other".into());

    let upstream = sink
        .wait_for("the snapshot published after the upstream write", |evs| {
            let done = evs.iter().position(|e| match e {
                SessionEvent::WriteFinished {
                    kind: OperationKind::Branch,
                    error,
                    ..
                } => {
                    assert!(error.is_none(), "the write failed: {error:?}");
                    true
                }
                _ => false,
            })?;
            evs[done..].iter().find_map(|e| match e {
                SessionEvent::RefsLoaded { snapshot, .. } => Some(
                    snapshot
                        .local_named("topic")
                        .map(|branch| branch.upstream.as_str().to_string())
                        .unwrap_or_default(),
                ),
                _ => None,
            })
        })
        .await;

    assert_eq!(
        upstream, "origin/other",
        "the rows read what the write said, not what they read before it"
    );
    session.close();
}

fn position_of(events: &[SessionEvent], kind: OperationKind) -> Option<usize> {
    events
        .iter()
        .position(|e| matches!(e, SessionEvent::WriteFinished { kind: got, .. } if *got == kind))
}

/// What the session actually spawned, for a failure that is about
/// the commands themselves.
fn commands(sink: &CaptureSink) -> Vec<String> {
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

fn log_starts(events: &[SessionEvent]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, SessionEvent::LogStarted { .. }))
        .count()
}
