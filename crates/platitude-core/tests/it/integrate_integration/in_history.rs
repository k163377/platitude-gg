//! What merge, rebase and cherry-pick do with a commit the branch HEAD is
//! on already has — the git the menu's greys stand on
//! (`menu::Why::InHistory` / `SameCommit`, `menu::History`).

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::integrate::{self, Landing, MergeOptions, RebaseOptions, RebaseOutcome};

/// `main`: `root` → `a` → `b`, each its own file — `a` down HEAD's first
/// parents with no merge on the way, and nothing since touching its lines.
fn straight() -> (TestRepo, String) {
    let mut repo = TestRepo::init();
    repo.commit_file("root.txt", "root\n", "root");
    let a = repo.commit_file_id("a.txt", "a\n", "a");
    repo.commit_file("b.txt", "b\n", "b");
    (repo, a)
}

fn head(repo: &mut TestRepo) -> String {
    repo.git(&["rev-parse", "HEAD"])
}

/// A merge of a commit behind HEAD brings nothing in (`Already up to
/// date.`): the branch stands where it was.
#[tokio::test]
async fn a_merge_of_a_commit_behind_head_brings_nothing_in() {
    let (mut repo, a) = straight();
    let before = head(&mut repo);
    let (exec, cancel) = env();
    let landing = integrate::merge(&exec, &repo.path, &a, &MergeOptions::default(), &cancel)
        .await
        .expect("nothing to bring in is no failure");
    assert_eq!(landing, Landing::Done);
    assert_eq!(head(&mut repo), before);
}

/// A rebase onto a commit HEAD runs down to straight has nothing to move
/// (`Current branch … is up to date.`).
#[tokio::test]
async fn a_rebase_onto_a_commit_down_a_straight_line_moves_nothing() {
    let (mut repo, a) = straight();
    let before = head(&mut repo);
    let (exec, cancel) = env();
    let outcome = integrate::rebase(&exec, &repo.path, &a, &RebaseOptions::default(), &cancel)
        .await
        .expect("nothing to move is no failure");
    assert!(matches!(outcome, RebaseOutcome::Done), "{outcome:?}");
    assert_eq!(head(&mut repo), before);
}

/// Past a merge on the way down HEAD's first parents, a rebase lays the
/// branch's commits again in one line, the merge left out: it moves the
/// branch, so the row stays live there.
#[tokio::test]
async fn a_rebase_onto_a_commit_past_a_merge_lays_the_branch_again() {
    let mut repo = TestRepo::init();
    repo.commit_file("root.txt", "root\n", "root");
    let root = head(&mut repo);
    repo.git(&["checkout", "-b", "side"]);
    repo.commit_file("side.txt", "side\n", "side");
    repo.git(&["checkout", "main"]);
    repo.commit_file("main.txt", "main\n", "main");
    repo.git(&["merge", "--no-ff", "-m", "merge side", "side"]);
    repo.commit_file("top.txt", "top\n", "top");
    let before = head(&mut repo);
    let (exec, cancel) = env();
    let outcome = integrate::rebase(&exec, &repo.path, &root, &RebaseOptions::default(), &cancel)
        .await
        .expect("a rebase that lays the branch again");
    assert!(matches!(outcome, RebaseOutcome::Done), "{outcome:?}");
    assert_ne!(head(&mut repo), before, "the branch moved");
    assert_eq!(repo.git(&["rev-list", "--merges", "--count", "HEAD"]), "0");
    assert_eq!(
        repo.git(&["rev-list", "--count", "HEAD"]),
        "4",
        "root and the three laid again"
    );
}

/// A cherry-pick of HEAD's own commit comes out empty, and the press walks
/// past git's stop: nothing written, nothing left standing.
#[tokio::test]
async fn a_cherry_pick_of_heads_own_commit_writes_nothing() {
    let (mut repo, _) = straight();
    let before = head(&mut repo);
    let (exec, cancel) = env();
    let landing = integrate::cherry_pick(&exec, &repo.path, std::slice::from_ref(&before), &cancel)
        .await
        .expect("an empty pick is no failure");
    assert_eq!(landing, Landing::Done);
    assert_eq!(head(&mut repo), before);
}

/// A cherry-pick of a commit in the history is no part of it: one the
/// branch undid since is made again…
#[tokio::test]
async fn a_cherry_pick_of_a_commit_undone_since_makes_it_again() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "root\n", "root");
    let added = repo.commit_file_id("f.txt", "root\nadded\n", "add");
    repo.git(&["revert", "--no-edit", "HEAD"]);
    let before = head(&mut repo);
    let (exec, cancel) = env();
    let landing = integrate::cherry_pick(&exec, &repo.path, &[added], &cancel)
        .await
        .expect("the pick lands");
    assert_eq!(landing, Landing::Done);
    assert_ne!(head(&mut repo), before, "a commit written");
    let file = std::fs::read_to_string(repo.path.join("f.txt")).expect("read f.txt");
    assert_eq!(file.replace("\r\n", "\n"), "root\nadded\n");
}

/// …and one whose lines were written over since stops on a conflict.
#[tokio::test]
async fn a_cherry_pick_of_a_commit_written_over_since_stops() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\n", "root");
    let two = repo.commit_file_id("f.txt", "two\n", "two");
    repo.commit_file("f.txt", "three\n", "three");
    let (exec, cancel) = env();
    let landing = integrate::cherry_pick(&exec, &repo.path, &[two], &cancel)
        .await
        .expect("a stop is no failure");
    assert_eq!(landing, Landing::Stopped);
}
