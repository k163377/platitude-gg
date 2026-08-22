//! Opening a repository, and the log stream that comes out of it.

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, scenario};
use platitude_core::session::{RepoSession, SessionEvent};

#[tokio::test(flavor = "multi_thread")]
async fn open_streams_the_full_pipeline() {
    let (repo, head) = scenario();
    let sink = CaptureSink::new();
    let session = RepoSession::open(
        crate::support::exec::isolated(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );

    sink.wait_for("Opened", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::Opened { info } => Some(info.workdir.clone()),
            _ => None,
        })
    })
    .await;

    let total = sink
        .wait_for("LogFinished", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::LogFinished { total, .. } => Some(*total),
                _ => None,
            })
        })
        .await;
    assert_eq!(total, 5, "root + side + main + merge + stash row");

    // All rows delivered, topo-consistent; the stash (newest child of the
    // merge) streams first, the head commit right after.
    let rows = sink
        .wait_for("chunk rows", |evs| {
            let mut rows = Vec::new();
            for e in evs {
                if let SessionEvent::LogChunk { rows: r, .. } = e {
                    rows.extend(r.iter().cloned());
                }
            }
            (rows.len() == 5).then_some(rows)
        })
        .await;
    assert_eq!(rows[0].stash_ref, "stash@{0}", "stash row leads");
    assert!(
        rows[0].subject.contains("wip stash"),
        "stash subject is its reflog message: {:?}",
        rows[0].subject
    );
    assert_eq!(rows[1].oid_hex, head, "merge commit is the newest commit");
    assert_eq!(rows[1].row, 1);
    assert!(rows.iter().skip(1).all(|r| r.stash_ref.is_empty()));
    assert!(rows.iter().all(|r| !r.subject.is_empty()));
    assert!(rows.iter().all(|r| r.author == "Test User"));

    // Labels: the head row must end up carrying main (+ v1 tag), either
    // inline or via a LabelsChanged update.
    sink.wait_for("labels on head row", |evs| {
        let seen = crate::support::replay_graph(evs);
        let latest: Vec<&str> = seen
            .get(&1)
            .map(|row| row.labels.iter().map(|l| l.text.as_str()).collect())
            .unwrap_or_default();
        (latest.contains(&"main") && latest.contains(&"v1")).then_some(())
    })
    .await;

    sink.wait_for("RefsLoaded", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::RefsLoaded { snapshot } => {
                let locals: Vec<&str> = snapshot.locals.iter().map(|b| b.short.as_str()).collect();
                assert_eq!(locals, vec!["main", "side"], "sorted locals");
                assert!(snapshot.locals[0].is_head);
                assert_eq!(snapshot.tags.len(), 1);
                Some(())
            }
            _ => None,
        })
    })
    .await;

    sink.wait_for("StatusLoaded", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::StatusLoaded {
                status, op_state, ..
            } => {
                assert_eq!(status.branch_head.as_deref(), Some("main"));
                assert!(!op_state.any());
                Some(())
            }
            _ => None,
        })
    })
    .await;

    sink.wait_for("StashesLoaded", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::StashesLoaded { stashes } => {
                assert_eq!(stashes.len(), 1);
                assert!(stashes[0].message.contains("wip stash"));
                Some(())
            }
            _ => None,
        })
    })
    .await;

    session.close();
}

#[tokio::test(flavor = "multi_thread")]
async fn restart_log_delivers_a_new_generation() {
    let (repo, _) = scenario();
    let sink = CaptureSink::new();
    let session = RepoSession::open(
        crate::support::exec::isolated(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );

    let first_gen = sink
        .wait_for("first LogFinished", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::LogFinished { generation, .. } => Some(*generation),
                _ => None,
            })
        })
        .await;

    session.restart_log();

    let second_gen = sink
        .wait_for("second LogFinished", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::LogFinished { generation, .. } if *generation > first_gen => {
                    Some(*generation)
                }
                _ => None,
            })
        })
        .await;
    assert!(second_gen > first_gen);

    // The restarted stream re-delivers all rows under the new generation
    // (4 commits + the stash row).
    sink.wait_for("second-generation rows", |evs| {
        let count: usize = evs
            .iter()
            .filter_map(|e| match e {
                SessionEvent::LogChunk { generation, rows } if *generation == second_gen => {
                    Some(rows.len())
                }
                _ => None,
            })
            .sum();
        (count == 5).then_some(())
    })
    .await;

    session.close();
}

#[tokio::test(flavor = "multi_thread")]
async fn unborn_repository_finishes_with_zero_rows() {
    let repo = TestRepo::init();
    let sink = CaptureSink::new();
    let session = RepoSession::open(
        crate::support::exec::isolated(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );

    let total = sink
        .wait_for("LogFinished", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::LogFinished { total, .. } => Some(*total),
                _ => None,
            })
        })
        .await;
    assert_eq!(total, 0);

    sink.wait_for("StatusLoaded", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::StatusLoaded { status, .. } => {
                assert_eq!(status.branch_oid, None, "unborn branch");
                Some(())
            }
            _ => None,
        })
    })
    .await;

    session.close();
}

#[tokio::test(flavor = "multi_thread")]
async fn open_failure_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let sink = CaptureSink::new();
    let _session = RepoSession::open(
        crate::support::exec::isolated(),
        tokio::runtime::Handle::current(),
        dir.path().to_path_buf(),
        sink.clone(),
    );
    sink.wait_for("OpenFailed", |evs| {
        evs.iter()
            .any(|e| matches!(e, SessionEvent::OpenFailed { .. }))
            .then_some(())
    })
    .await;
}

/// A `gh-pages`-shaped ref: a parentless commit that only a
/// remote-tracking ref names, sharing no history with anything else.
///
/// The walk offers it either way — `--remotes` names it — but the row it
/// lands on is the whole point. `--topo-order` refuses to intermix
/// independent lines of history, so it emits every commit of the main
/// chain before starting this one and the newest commit in the repository
/// arrives dead last. `--date-order` keeps the same parents-after-children
/// guarantee the graph builder needs and puts it where its timestamp says.
#[tokio::test(flavor = "multi_thread")]
async fn an_independent_history_sits_where_its_date_puts_it() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    repo.commit_file("f.txt", "1\n", "second");
    repo.commit_file("f.txt", "2\n", "third");

    repo.git(&["checkout", "--orphan", "gh-pages"]);
    repo.git(&["rm", "-rf", "."]);
    repo.write_file("index.html", "docs\n");
    repo.git(&["add", "index.html"]);
    repo.git(&["commit", "-m", "docs: api documentation"]);
    let orphan = repo.git(&["rev-parse", "HEAD"]);
    // Only the remote-tracking ref keeps it: no local branch, and nothing
    // reaches it from HEAD.
    repo.git(&["update-ref", "refs/remotes/origin/gh-pages", &orphan]);
    repo.git(&["checkout", "-f", "main"]);
    repo.git(&["branch", "-D", "gh-pages"]);
    repo.git(&["clean", "-fd"]);
    // One more on main *after* the orphan: it is now neither the newest
    // commit nor the oldest, which is what buries it. A tip that is newest
    // of all gets emitted first under either ordering, so a repository
    // shaped that way proves nothing.
    repo.commit_file("f.txt", "3\n", "fourth");

    let sink = CaptureSink::new();
    let session = RepoSession::open(
        crate::support::exec::isolated(),
        tokio::runtime::Handle::current(),
        repo.path.clone(),
        sink.clone(),
    );
    let total = sink
        .wait_for("LogFinished", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::LogFinished { total, .. } => Some(*total),
                _ => None,
            })
        })
        .await;
    assert_eq!(total, 5, "four on main plus the orphan");

    // Chips are what make a row unreachable from every branch findable at
    // all, so the row number and the chip are asserted together.
    let seen = sink
        .wait_for("its chip", |evs| {
            let rows = crate::support::replay_graph(evs);
            rows.iter()
                .find(|(_, r)| r.oid_hex == orphan)
                .filter(|(_, r)| !r.labels.is_empty())
                .map(|(row, r)| (*row, r.clone()))
        })
        .await;
    let (row, seen) = seen;
    assert_eq!(
        row, 1,
        "the second-newest commit belongs on the second row, not {row} rows down"
    );
    assert!(
        seen.labels.iter().any(|l| l.text == "origin/gh-pages"),
        "the orphan carries no chip naming it: {:?}",
        seen.labels
    );
    session.close();
}

/// A merge stopped in the working tree. The row for the uncommitted files
/// is the merge commit it is about to become, so it draws that commit's
/// fork: a dotted edge to HEAD and another to the side being brought in,
/// each landing on the tip it names.
#[tokio::test(flavor = "multi_thread")]
async fn a_standing_merge_dots_the_side_it_is_bringing_in() {
    let mut repo = crate::support::integrate::conflicting_branches();
    let ours = repo.git(&["rev-parse", "HEAD"]);
    let theirs = repo.git(&["rev-parse", "side"]);
    repo.git_expect_failure(&["merge", "side"]);

    let (sink, session) = crate::support::session::opened(&repo).await;
    // WIP + main + side + root. The WIP row only appears once the opening
    // status read has found the tree dirty, and that same read is what
    // reports the merge — so the pass that shows four rows is the pass
    // that knows about both.
    sink.opened_graph_gen(&session, 4).await;
    let rows = sink
        .wait_for("the four-row graph", |evs| {
            let rows = crate::support::replay_rows(evs);
            (rows.len() == 4).then_some(rows)
        })
        .await;

    let wip = &rows[&0];
    assert!(
        wip.oid_hex.bytes().all(|b| b == b'0'),
        "the uncommitted row leads: {:?}",
        wip.oid_hex
    );
    let outs: Vec<(u16, bool)> = wip
        .segments
        .iter()
        .filter(|s| s.kind == platitude_core::graph::SegmentKind::OutOfNode)
        .map(|s| (s.lane, s.dashed))
        .collect();
    assert_eq!(
        outs,
        vec![(0, true), (1, true)],
        "both parents of the pending merge leave the row dotted: {:?}",
        wip.segments
    );

    // Each edge lands on its own tip, and the side arrives on the lane the
    // dotted edge reserved for it.
    assert_eq!(rows[&1].oid_hex, ours);
    assert_eq!(rows[&1].node_lane, 0);
    assert_eq!(rows[&2].oid_hex, theirs);
    assert_eq!(rows[&2].node_lane, 1);
    assert!(
        rows[&2]
            .segments
            .iter()
            .any(|s| s.kind == platitude_core::graph::SegmentKind::IntoNode
                && s.lane == 1
                && s.dashed),
        "the side's tip gathers the dotted edge: {:?}",
        rows[&2].segments
    );
    session.close();
}

/// A side no ref names — `git merge <sha>`, or the `FETCH_HEAD` of a
/// one-off fetch — is still a parent of the commit being written, so the
/// walk is told to start there. Nothing else would offer it, and a dotted
/// edge to a commit the graph does not hold runs off the bottom of the
/// window instead of landing.
#[tokio::test(flavor = "multi_thread")]
async fn a_side_no_ref_names_still_joins_the_walk() {
    let mut repo = crate::support::integrate::conflicting_branches();
    let theirs = repo.git(&["rev-parse", "side"]);
    repo.git(&["branch", "-D", "side"]);
    repo.git_expect_failure(&["merge", &theirs]);

    let (sink, session) = crate::support::session::opened(&repo).await;
    sink.opened_graph_gen(&session, 4).await;
    let rows = sink
        .wait_for("the four-row graph", |evs| {
            let rows = crate::support::replay_rows(evs);
            (rows.len() == 4).then_some(rows)
        })
        .await;
    assert_eq!(rows[&2].oid_hex, theirs, "the side is on screen");
    assert!(
        rows[&2]
            .segments
            .iter()
            .any(|s| s.kind == platitude_core::graph::SegmentKind::IntoNode
                && s.lane == 1
                && s.dashed),
        "its dotted edge lands: {:?}",
        rows[&2].segments
    );
    session.close();
}

/// Aborting the merge takes the second dotted edge away again — the row
/// stands for whatever the next commit will be, and that is no longer a
/// merge.
#[tokio::test(flavor = "multi_thread")]
async fn aborting_the_merge_takes_the_second_dotted_edge_away() {
    let mut repo = crate::support::integrate::conflicting_branches();
    repo.git_expect_failure(&["merge", "side"]);
    // Something uncommitted has to outlive the abort, or the row it draws
    // goes with the merge and there is nothing left to assert on.
    repo.write_file("untracked.txt", "keep\n");

    let (sink, session) = crate::support::session::opened(&repo).await;
    sink.opened_graph_gen(&session, 4).await;
    sink.wait_for("the merge's two dotted edges", |evs| {
        let rows = crate::support::replay_rows(evs);
        (rows.len() == 4 && rows.get(&0).is_some_and(|r| r.width == 2)).then_some(())
    })
    .await;

    repo.git(&["merge", "--abort"]);
    session.refresh_quick();
    let rows = sink
        .wait_for("the row after the abort", |evs| {
            let rows = crate::support::replay_rows(evs);
            (rows.len() == 4 && rows.get(&0).is_some_and(|r| r.width == 1)).then_some(rows)
        })
        .await;
    let outs: Vec<(u16, bool)> = rows[&0]
        .segments
        .iter()
        .filter(|s| s.kind == platitude_core::graph::SegmentKind::OutOfNode)
        .map(|s| (s.lane, s.dashed))
        .collect();
    assert_eq!(
        outs,
        vec![(0, true)],
        "only HEAD is left on a leash: {:?}",
        rows[&0].segments
    );
    session.close();
}
