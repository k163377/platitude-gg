//! A merge or a pull over uncommitted work git would write over: the carry
//! that takes the work round through a stash, and the stash that keeps it
//! when the merge stops on a conflict (デザイン規約 §履歴を合流させる).

use crate::support::TestRepo;
use crate::support::remote::origin_and_clone;
use crate::support::session::{CaptureSink, command_words, opened, write_result, write_stopped};
use platitude_core::OperationKind;
use platitude_core::integrate::MergeOptions;
use platitude_core::session::SessionEvent;

/// The route taken, as the commands issued.
fn route(sink: &CaptureSink) -> Vec<&'static str> {
    let mut out = Vec::new();
    for event in sink.events.lock().unwrap().iter() {
        let SessionEvent::CommandStarted { display, .. } = event else {
            continue;
        };
        let words = command_words(display);
        // Longest first: "stash pop --index" also starts with "stash pop".
        for step in [
            "merge",
            "pull",
            "stash push",
            "stash pop --index",
            "stash pop",
        ] {
            if words.starts_with(step) {
                out.push(step);
                break;
            }
        }
    }
    out
}

/// Six lines: a change at the top here and one at the bottom over there
/// merge cleanly when the work comes back.
fn six(top: &str, bottom: &str) -> String {
    format!("{top}\n2\n3\n4\n5\n{bottom}\n")
}

fn read(repo: &TestRepo, rel: &str) -> String {
    std::fs::read_to_string(repo.path.join(rel)).unwrap()
}

/// A fast-forward git refuses over a changed file goes round through a
/// stash — `merge` → `stash push` → `merge` → `stash pop --index` — and
/// the work comes back over the merged file.
#[tokio::test(flavor = "multi_thread")]
async fn a_merge_over_work_in_its_way_carries_the_work_across() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", &six("1", "6"), "root");
    repo.git(&["checkout", "-b", "side"]);
    repo.commit_file("a.txt", &six("1", "six"), "side work");
    repo.git(&["checkout", "main"]);
    repo.write_file("a.txt", &six("one", "6"));
    let side = repo.git(&["rev-parse", "side"]);

    let (sink, session) = opened(&repo).await;
    session.merge("side".into(), MergeOptions::default());
    assert_eq!(write_result(&sink, OperationKind::Merge).await, None);
    assert_eq!(
        route(&sink),
        vec!["merge", "stash push", "merge", "stash pop --index"]
    );
    assert_eq!(
        repo.git(&["rev-parse", "HEAD"]),
        side,
        "the fast-forward landed"
    );
    assert_eq!(
        read(&repo, "a.txt"),
        six("one", "six"),
        "both the work and the merge's change stand"
    );
    assert_eq!(repo.git(&["stash", "list"]), "", "the entry was put back");
    session.close_and_wait().await;
}

/// The same round for a pull: the fast-forward from the upstream, over a
/// file changed here that the upstream changed too.
#[tokio::test(flavor = "multi_thread")]
async fn a_pull_over_work_in_its_way_carries_the_work_across() {
    let (bare, mut work) = origin_and_clone();
    work.commit_file("b.txt", &six("1", "6"), "six lines");
    work.git(&["push", "origin", "main"]);
    work.git(&["branch", "--set-upstream-to=origin/main", "main"]);
    let mut far = TestRepo::init();
    far.git(&["remote", "add", "origin", &bare.file_url()]);
    far.git(&["fetch", "origin"]);
    far.git(&["switch", "--force-create", "main", "origin/main"]);
    far.commit_file("b.txt", &six("1", "six"), "their bottom line");
    far.git(&["push", "origin", "main"]);
    let theirs = far.git(&["rev-parse", "main"]);
    work.write_file("b.txt", &six("one", "6"));

    let (sink, session) = opened(&work).await;
    session.pull();
    assert_eq!(write_result(&sink, OperationKind::Pull).await, None);
    assert_eq!(
        route(&sink),
        vec!["pull", "stash push", "pull", "stash pop --index"]
    );
    assert_eq!(
        work.git(&["rev-parse", "HEAD"]),
        theirs,
        "the fast-forward landed"
    );
    assert_eq!(read(&work, "b.txt"), six("one", "six"));
    assert_eq!(work.git(&["stash", "list"]), "", "the entry was put back");
    session.close_and_wait().await;
}

/// A merge that stops on a conflict after the carry leaves the work in the
/// stash, as a stopped rewrite does: git holds the tree, and the person
/// settling the conflict takes the stash back.
#[tokio::test(flavor = "multi_thread")]
async fn a_merge_that_stops_after_the_carry_leaves_the_work_in_the_stash() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "base\n", "root");
    repo.commit_file("b.txt", &six("1", "6"), "six lines");
    repo.git(&["checkout", "-b", "side"]);
    repo.commit_file("a.txt", "side\n", "side a");
    repo.commit_file("b.txt", &six("1", "six"), "side b");
    repo.git(&["checkout", "main"]);
    repo.commit_file("a.txt", "main\n", "main a");
    repo.write_file("b.txt", &six("one", "6"));

    let (sink, session) = opened(&repo).await;
    session.merge("side".into(), MergeOptions::default());
    assert_eq!(
        write_result(&sink, OperationKind::Merge).await,
        None,
        "a conflict stop is an answer"
    );
    assert!(write_stopped(&sink, OperationKind::Merge));
    assert_eq!(route(&sink), vec!["merge", "stash push", "merge"]);
    assert!(repo.path.join(".git").join("MERGE_HEAD").exists());
    assert_eq!(
        repo.git(&["stash", "list"]).lines().count(),
        1,
        "the work waits in the stash"
    );
    assert_eq!(
        read(&repo, "b.txt"),
        six("1", "six"),
        "the tree is the merge's, the work the stash's"
    );
    session.close_and_wait().await;
}

/// A merge refused for any other reason stops there: nothing is stashed.
#[tokio::test(flavor = "multi_thread")]
async fn a_merge_that_fails_outright_stashes_nothing() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "root");
    repo.write_file("a.txt", "uncommitted\n");

    let (sink, session) = opened(&repo).await;
    session.merge("no-such-branch".into(), MergeOptions::default());
    assert!(
        write_result(&sink, OperationKind::Merge).await.is_some(),
        "git rejected the name"
    );
    assert_eq!(route(&sink), vec!["merge"]);
    assert_eq!(
        read(&repo, "a.txt"),
        "uncommitted\n",
        "the work never left the tree"
    );
    assert_eq!(repo.git(&["stash", "list"]), "", "no entry left behind");
    session.close_and_wait().await;
}

/// What git itself does behind the fixed command lines the tests above
/// pass — left to the full gate (rules-refs/core.md `periodic`).
mod periodic {
    use super::*;

    /// A reader's `merge.autoStash=true` does not take the carry's place:
    /// `--no-autostash` is pinned, so the round is this end's and staged work
    /// comes back staged (git's own would restore with a plain `apply`).
    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "a reader's merge.autoStash behind --no-autostash: not worth the pre-merge run"]
    async fn a_readers_merge_autostash_leaves_the_carry_to_the_session() {
        let mut repo = TestRepo::init();
        repo.commit_file("a.txt", &six("1", "6"), "root");
        repo.git(&["checkout", "-b", "side"]);
        repo.commit_file("a.txt", &six("1", "six"), "side work");
        repo.git(&["checkout", "main"]);
        repo.git(&["config", "merge.autoStash", "true"]);
        repo.write_file("a.txt", &six("one", "6"));
        repo.write_file("staged.txt", "staged\n");
        repo.git(&["add", "staged.txt"]);

        let (sink, session) = opened(&repo).await;
        session.merge("side".into(), MergeOptions::default());
        assert_eq!(write_result(&sink, OperationKind::Merge).await, None);
        assert_eq!(
            route(&sink),
            vec!["merge", "stash push", "merge", "stash pop --index"],
            "git's own autostash did not run in the carry's place"
        );
        assert_eq!(
            repo.git(&["diff", "--cached", "--name-only"]),
            "staged.txt",
            "the staged file came back staged"
        );
        assert_eq!(read(&repo, "a.txt"), six("one", "six"));
        assert_eq!(repo.git(&["stash", "list"]), "", "the entry was put back");
        session.close_and_wait().await;
    }
}
