//! Rewrites fired while another operation stands: refused whole, with
//! nothing spawned and nothing stashed.
//!
//! The trap these are against is measured in
//! `integrate_integration::standing_op` — git refuses a rebase under a
//! standing merge in the words of a dirty tree, and the stash the carry
//! would take next puts the merge down. Before the guards, a plan run over
//! a settled-but-uncommitted merge went **through**: the merge marker
//! gone, the history replayed on top, the resolution handed back as an
//! ordinary staged edit, and the write reported as a success.

use crate::support::TestRepo;
use crate::support::session::{CaptureSink, install_todo_editor, opened, write_result};
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

/// Whether any git command that writes was spawned at all.
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

fn merge_stands(repo: &TestRepo) -> bool {
    repo.path.join(".git").join("MERGE_HEAD").exists()
}

/// **The plan's own premise check.** The tip has not moved, so the pin
/// the plan carries says nothing is wrong; the operation standing is what
/// the run is refused on, before a single command is spawned.
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
        // The very tip the plan was composed against: the pin agrees.
        head.clone(),
    );
    let refusal = write_result(&sink, "rebase")
        .await
        .expect("the run is refused");
    assert!(
        refusal.contains("merge") && refusal.contains("nothing was rewritten"),
        "the refusal names what is standing: {refusal}"
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

/// **The carry's own guard**, reached by the rewrites that have no plan
/// in front of them: git refuses in the words of a dirty tree, and the
/// stash that would follow is what would take the merge down. One
/// spawn — git's refusal — and then nothing.
#[tokio::test(flavor = "multi_thread")]
async fn a_squash_under_a_standing_merge_stops_at_the_carry() {
    install_todo_editor();
    let mut repo = standing_merge();
    let head = repo.git(&["rev-parse", "HEAD"]);

    let (sink, session) = opened(&repo).await;
    session.squash_into_parent(head.clone());
    let refusal = write_result(&sink, "squash")
        .await
        .expect("the squash is refused");
    assert!(
        refusal.contains("merge") && refusal.contains("nothing was rewritten"),
        "the refusal names what is standing rather than repeating git's \
         advice to commit or stash: {refusal}"
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

/// The same guard on the plain `rebase <upstream>` route, and under a
/// cherry-pick rather than a merge — the trap is the operation, not which
/// menu row was clicked.
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
    let refusal = write_result(&sink, "rebase")
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

/// And a bisect does **not** stand in the way: a stash leaves it running
/// (measured), so the work carries across as it does anywhere else. The
/// guard reads the four operations a stash puts down, not everything the
/// badge names.
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
    // The bisect leaves HEAD detached at its midpoint; the rewrite runs
    // from there, and what is being proved is only that the carry ran.
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
        write_result(&sink, "rebase").await,
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
