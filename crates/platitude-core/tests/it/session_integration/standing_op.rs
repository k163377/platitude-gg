//! Rewrites fired while another operation stands: refused whole, with
//! nothing rewritten and nothing stashed.
//!
//! The trap (measured in `integrate_integration::standing_op`): git refuses
//! a rebase under a standing merge in the words of a dirty tree, and the
//! carry's stash would then put the merge down and report a success.

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, install_todo_editor, opened, write_result};
use platitude_core::OperationKind;
use platitude_core::sequencer::{RebaseStep, TodoAction};
use platitude_core::session::SessionEvent;

fn diverged() -> TestRepo {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "root");
    repo.commit_file("keep.txt", "keep\n", "second");
    repo.git(&["checkout", "-b", "side"]);
    repo.commit_file("f.txt", "side\n", "side change");
    repo.git(&["checkout", "main"]);
    repo.commit_file("f.txt", "main\n", "main change");
    repo
}

/// A merge stopped on a conflict, settled by hand and staged, waiting for
/// its commit — HEAD has not moved, which is what makes the tip check
/// blind to it.
fn standing_merge() -> TestRepo {
    let mut repo = diverged();
    repo.git_expect_failure(&["merge", "side"]);
    repo.write_file("f.txt", "resolved by hand\n");
    repo.git(&["add", "f.txt"]);
    repo
}

/// Every writing git command the session spawned.
fn wrote_anything(sink: &CaptureSink) -> Vec<String> {
    sink.events
        .lock()
        .unwrap()
        .iter()
        .filter_map(|event| match event {
            SessionEvent::CommandStarted { display, .. } => Some(display.clone()),
            _ => None,
        })
        .filter(|display| {
            [
                "rebase",
                "stash push",
                "stash pop",
                "merge",
                "cherry-pick",
                "revert",
            ]
            .iter()
            .any(|verb| display.starts_with(&format!("git {verb}")))
        })
        .collect()
}

/// The report the write of `kind` carried — what the notice bar is written
/// from, which the error string alone cannot say.
async fn report_kind(
    sink: &CaptureSink,
    kind: OperationKind,
) -> Option<platitude_core::report::ReportKind> {
    sink.wait_for(kind.label(), |evs| {
        evs.iter().find_map(|e| match e {
            platitude_core::session::SessionEvent::WriteFinished {
                kind: got, report, ..
            } if *got == kind => Some(report.as_ref().map(|r| r.kind)),
            _ => None,
        })
    })
    .await
}

fn merge_stands(repo: &TestRepo) -> bool {
    repo.path.join(".git").join("MERGE_HEAD").exists()
}

/// The plan's premise check: the pin agrees (the tip has not moved), so the
/// standing operation is what refuses the run, before anything spawns.
#[tokio::test(flavor = "multi_thread")]
async fn a_plan_run_under_a_standing_merge_is_refused_before_anything_is_spawned() {
    install_todo_editor();
    let mut repo = standing_merge();
    let head = repo.git(&["rev-parse", "HEAD"]);
    let base = repo.git(&["rev-parse", "HEAD~1"]);
    let resolved = std::fs::read_to_string(repo.path.join("f.txt")).expect("read");

    let (sink, session) = opened(&repo).await;
    session.rebase_interactive(
        base,
        vec![RebaseStep {
            action: TodoAction::Pick,
            oid: head.clone(),
            subject: "main change".into(),
            message: None,
        }],
        platitude_core::integrate::RebaseOptions::default(),
        // The very tip the plan was composed against.
        head.clone(),
    );
    let refusal = write_result(&sink, OperationKind::Rebase)
        .await
        .expect("the run is refused");
    assert!(
        refusal.contains("merge") && refusal.contains("nothing was rewritten"),
        "the refusal names what is standing: {refusal}"
    );
    // Nothing ran, so no command-log row raises the panel: the report is
    // what reaches the notice bar (デザイン規約 §答えの要らない報せ).
    assert_eq!(
        report_kind(&sink, OperationKind::Rebase).await,
        Some(platitude_core::report::ReportKind::RewriteWhileStanding)
    );
    assert_eq!(
        wrote_anything(&sink),
        Vec::<String>::new(),
        "nothing was spawned at all"
    );
    assert!(merge_stands(&repo), "the merge is still standing");
    assert_eq!(
        repo.git(&["rev-parse", "HEAD"]),
        head,
        "the tip is untouched"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path.join("f.txt")).expect("read"),
        resolved,
        "and the settled conflict is where it was left"
    );
    assert_eq!(repo.git(&["stash", "list"]), "", "nothing was stashed");
    session.close();
}

/// The carry's own guard, for rewrites with no plan in front: one spawn —
/// git's refusal — and no stash after it.
#[tokio::test(flavor = "multi_thread")]
async fn a_squash_under_a_standing_merge_stops_at_the_carry() {
    install_todo_editor();
    let mut repo = standing_merge();
    let head = repo.git(&["rev-parse", "HEAD"]);

    let (sink, session) = opened(&repo).await;
    session.squash_into_parent(head.clone());
    let refusal = write_result(&sink, OperationKind::Squash)
        .await
        .expect("the squash is refused");
    assert!(
        refusal.contains("merge") && refusal.contains("nothing was rewritten"),
        "the refusal names what is standing rather than repeating git's \
         advice to commit or stash: {refusal}"
    );
    // The same report as the plan's guard, though git did run here and
    // refused in a dirty tree's words.
    assert_eq!(
        report_kind(&sink, OperationKind::Squash).await,
        Some(platitude_core::report::ReportKind::RewriteWhileStanding)
    );
    let spawned = wrote_anything(&sink);
    assert_eq!(
        spawned.len(),
        1,
        "git refused once and the carry did not go round: {spawned:?}"
    );
    assert!(
        spawned[0].starts_with("git rebase --interactive"),
        "and the one spawn is the replay git turned down: {spawned:?}"
    );
    assert!(merge_stands(&repo), "the merge is still standing");
    assert_eq!(
        repo.git(&["rev-parse", "HEAD"]),
        head,
        "the tip is untouched"
    );
    assert_eq!(repo.git(&["stash", "list"]), "", "nothing was stashed");
    session.close();
}

/// The same guard on plain `rebase <upstream>`, under a cherry-pick.
#[tokio::test(flavor = "multi_thread")]
async fn a_rebase_onto_under_a_standing_cherry_pick_stops_at_the_carry() {
    let mut repo = diverged();
    repo.git_expect_failure(&["cherry-pick", "side"]);
    repo.write_file("f.txt", "resolved by hand\n");
    repo.git(&["add", "f.txt"]);
    let head = repo.git(&["rev-parse", "HEAD"]);

    let (sink, session) = opened(&repo).await;
    session.rebase(
        "side".into(),
        platitude_core::integrate::RebaseOptions {
            update_refs: true,
            ..Default::default()
        },
    );
    let refusal = write_result(&sink, OperationKind::Rebase)
        .await
        .expect("the rebase is refused");
    assert!(
        refusal.contains("cherry-pick"),
        "the refusal names what is standing: {refusal}"
    );
    assert!(
        repo.path.join(".git").join("CHERRY_PICK_HEAD").exists(),
        "the cherry-pick is still standing"
    );
    assert_eq!(
        repo.git(&["rev-parse", "HEAD"]),
        head,
        "the tip is untouched"
    );
    assert_eq!(repo.git(&["stash", "list"]), "", "nothing was stashed");
    session.close();
}

/// A bisect stands aside: a stash leaves it running, so the work carries as
/// usual. The guard reads only the four operations a stash puts down.
#[tokio::test(flavor = "multi_thread")]
async fn a_dirty_tree_under_a_bisect_still_carries() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["switch", "-c", "topic"]);
    repo.commit_file("t.txt", "topic\n", "topic one");
    repo.git(&["switch", "main"]);
    repo.commit_file("m.txt", "main\n", "main moved");
    repo.git(&["switch", "topic"]);
    repo.git(&["bisect", "start"]);
    repo.git(&["bisect", "bad", "topic"]);
    repo.git(&["bisect", "good", "main~1"]);
    // HEAD is detached at the bisect's midpoint; what is proved is only that
    // the carry ran.
    repo.write_file("a.txt", "uncommitted\n");

    let (sink, session) = opened(&repo).await;
    session.rebase(
        "main".into(),
        platitude_core::integrate::RebaseOptions {
            update_refs: true,
            ..Default::default()
        },
    );
    assert_eq!(
        write_result(&sink, OperationKind::Rebase).await,
        None,
        "the rewrite was not refused for the bisect"
    );
    assert!(
        wrote_anything(&sink)
            .iter()
            .any(|display| display.starts_with("git stash push")),
        "the carry went round through a stash as usual: {:?}",
        wrote_anything(&sink)
    );
    assert!(
        repo.path.join(".git").join("BISECT_LOG").exists(),
        "and the bisect is still running"
    );
    session.close();
}
