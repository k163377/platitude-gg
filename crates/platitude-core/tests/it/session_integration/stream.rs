//! Opening a repository, and the log stream that comes out of it.

use crate::support::TestRepo;
use crate::support::session::{open_unawaited, scenario};
use platitude_core::session::SessionEvent;

#[tokio::test(flavor = "multi_thread")]
async fn open_streams_the_full_pipeline() {
    let (repo, head) = scenario();
    let (sink, session) = open_unawaited(&repo);

    sink.wait_for("Opened", |evs| {
        evs.iter().find_map(|e| match e {
            SessionEvent::Opened { info } => Some(info.workdir.clone()),
            _ => None,
        })
    })
    .await;

    // Root + side + main + merge + stash row; `replay_rows` keeps a
    // superseded pass's rows out of the tally. The stash (newest child of
    // the merge) leads.
    let rows = sink
        .wait_for("all five rows", |evs| {
            let rows = crate::support::replay_rows(evs);
            (rows.len() == 5).then_some(rows)
        })
        .await;
    assert_eq!(rows[&0].stash_ref, "stash@{0}", "stash row leads");
    assert!(
        rows[&0].subject.contains("wip stash"),
        "stash subject is its reflog message: {:?}",
        rows[&0].subject
    );
    assert_eq!(rows[&1].oid_hex, head, "merge commit is the newest commit");
    assert!(rows.values().skip(1).all(|r| r.stash_ref.is_empty()));
    assert!(rows.values().all(|r| !r.subject.is_empty()));
    assert!(rows.values().all(|r| r.author == "Test User"));

    // The head row ends up carrying main + v1, inline or via `LabelsChanged`.
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
            SessionEvent::RefsLoaded { snapshot, .. } => {
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
            SessionEvent::StashesLoaded { stashes, .. } => {
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
    let (sink, session) = open_unawaited(&repo);

    // The opening settles first: a rebuild of it still in flight could
    // silently supersede the restart (hanging the wait) or land its pass in
    // the restart's place (passing with the restart broken).
    let first_gen = sink.opened_graph(&session, 5).await.generation;

    session.restart_log();

    let second = sink.pass_after("the restarted pass", first_gen).await;
    let second_gen = second.generation;

    // All rows again under the new generation (4 commits + the stash row).
    sink.wait_for("second-generation rows", move |evs| {
        let count: usize = evs
            .iter()
            .filter_map(|e| match e {
                SessionEvent::LogChunk { generation, rows }
                | SessionEvent::LogReplaced {
                    generation, rows, ..
                } if *generation == second_gen => Some(rows.len()),
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
    let (sink, session) = open_unawaited(&repo);

    let pass = sink.pass_after("the opening pass", 0).await;
    assert_eq!(pass.total, 0);

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

/// A `gh-pages`-shaped ref: a parentless commit only a remote-tracking ref
/// names. `--topo-order` would emit the whole main chain before it, burying
/// it at the bottom; `--date-order` puts it where its date says.
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
    // Only the remote-tracking ref keeps it.
    repo.git(&["update-ref", "refs/remotes/origin/gh-pages", &orphan]);
    repo.git(&["checkout", "-f", "main"]);
    repo.git(&["branch", "-D", "gh-pages"]);
    repo.git(&["clean", "-fd"]);
    // One more on main after the orphan: a newest-of-all tip comes first
    // under either ordering and proves nothing.
    repo.commit_file("f.txt", "3\n", "fourth");

    let (sink, session) = open_unawaited(&repo);
    let pass = sink.pass_after("the opening pass", 0).await;
    assert_eq!(pass.total, 5, "four on main plus the orphan");

    // The chip is what makes a row no branch reaches findable, so the row
    // number and the chip are asserted together.
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

/// A commit made in a detached working copy: `git log` reads only its own
/// tree's HEAD and no ref points here, so the row exists only because the
/// worktree read names it (`note_worktree_holders` → `walk_command`), and
/// its chip is all that says whose it is (デザイン規約 §ref の種別).
#[tokio::test(flavor = "multi_thread")]
async fn a_commit_only_a_detached_copy_holds_is_a_row_with_its_own_chip() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "first");
    repo.commit_file("a.txt", "two\n", "second");
    let spike = repo.path.parent().expect("a parent").join("spike");
    repo.git(&["worktree", "add", "--detach", &spike.to_string_lossy()]);
    std::fs::write(spike.join("idea.txt"), "an idea\n").expect("write the spike's file");
    repo.git_in(&spike, &["add", "idea.txt"]);
    repo.git_in(&spike, &["commit", "-m", "spike: try the idea"]);
    let only_theirs = repo.git_in(&spike, &["rev-parse", "HEAD"]);

    let (sink, session) = open_unawaited(&repo);
    // The worktree read comes in behind the first walk; this waits for the
    // walk it asks for.
    let seen = sink
        .wait_for("the row the copy is standing on", |evs| {
            let rows = crate::support::replay_graph(evs);
            rows.values()
                .find(|r| r.oid_hex == only_theirs)
                .filter(|r| !r.labels.is_empty())
                .cloned()
        })
        .await;
    let chip = seen
        .labels
        .iter()
        .find(|l| l.kind == platitude_core::session::LabelKind::Worktree)
        .unwrap_or_else(|| panic!("no chip says whose copy it is: {:?}", seen.labels));
    assert_eq!(
        chip.text.as_str(),
        "spike",
        "the chip carries the copy's name"
    );
    assert!(
        !chip.held_elsewhere,
        "it names no branch, so there is none for anybody to be holding"
    );
    session.close();
}

/// Conflicts all resolved as ours: `status` is empty with `MERGE_HEAD`
/// standing, and committing still writes a merge — so the row stays on a
/// clean tree; nothing else says the next commit has two parents.
#[tokio::test(flavor = "multi_thread")]
async fn a_merge_resolved_as_ours_keeps_the_row_a_clean_tree_would_not() {
    let mut repo = crate::support::integrate::conflicting_branches();
    let theirs = repo.git(&["rev-parse", "side"]);
    repo.git_expect_failure(&["merge", "side"]);
    repo.write_file("f.txt", "main\n");
    repo.git(&["add", "--", "f.txt"]);
    assert_eq!(
        repo.git(&["status", "--porcelain"]),
        "",
        "the point of the case is a clean tree under a standing merge"
    );

    let (sink, session) = crate::support::session::opened(&repo).await;
    sink.opened_graph(&session, 4).await;
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
    assert_eq!(rows[&2].oid_hex, theirs, "the side the leash reaches");
    session.close();
}

/// The same clean tree, with the side reachable only by `MERGE_HEAD`: the
/// walk starts there only for the row's dotted edge, so whatever arrives
/// this way arrives leashed.
#[tokio::test(flavor = "multi_thread")]
async fn a_side_only_merge_head_names_is_never_on_screen_unleashed() {
    let mut repo = crate::support::integrate::conflicting_branches();
    let theirs = repo.git(&["rev-parse", "side"]);
    repo.git(&["branch", "-D", "side"]);
    repo.git_expect_failure(&["merge", &theirs]);
    repo.write_file("f.txt", "main\n");
    repo.git(&["add", "--", "f.txt"]);
    assert_eq!(repo.git(&["status", "--porcelain"]), "", "a clean tree");

    let (sink, session) = crate::support::session::opened(&repo).await;
    sink.opened_graph(&session, 4).await;
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
        "and something reaches it: {:?}",
        rows[&2].segments
    );
    session.close();
}

/// A side no ref names (`git merge <sha>`, a one-off fetch's `FETCH_HEAD`)
/// is still a parent of the commit being written, so the walk starts there
/// — or its dotted edge runs off the bottom of the window.
#[tokio::test(flavor = "multi_thread")]
async fn a_side_no_ref_names_still_joins_the_walk() {
    let mut repo = crate::support::integrate::conflicting_branches();
    let theirs = repo.git(&["rev-parse", "side"]);
    repo.git(&["branch", "-D", "side"]);
    repo.git_expect_failure(&["merge", &theirs]);

    let (sink, session) = crate::support::session::opened(&repo).await;
    sink.opened_graph(&session, 4).await;
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

/// Aborting the merge takes the second dotted edge away: the next commit is
/// no longer a merge.
#[tokio::test(flavor = "multi_thread")]
async fn aborting_the_merge_takes_the_second_dotted_edge_away() {
    let mut repo = crate::support::integrate::conflicting_branches();
    repo.git_expect_failure(&["merge", "side"]);
    // Something uncommitted outlives the abort, or the row goes with the merge.
    repo.write_file("untracked.txt", "keep\n");

    let (sink, session) = crate::support::session::opened(&repo).await;
    sink.opened_graph(&session, 4).await;
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
