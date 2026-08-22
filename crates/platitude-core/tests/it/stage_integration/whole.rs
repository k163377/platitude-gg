//! Staging and discarding whole files.

use std::sync::Arc;

use crate::support::exec::{env, observed_env};
use crate::support::stage::buckets;
use crate::support::{Ends, TestRepo};
use platitude_core::stage;

/// An empty selection is not "every path": every command here reads a
/// missing pathspec as the whole work tree, and `git clean -f -d --` on
/// its own would delete every untracked file. So nothing runs — not even
/// the read of HEAD that two of them need before they can choose a
/// command.
#[tokio::test]
async fn an_empty_selection_runs_nothing() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("a.txt", "edited\n");
    repo.write_file("new.txt", "fresh\n");

    let ends = Arc::new(Ends::default());
    let (exec, cancel) = observed_env(ends.clone(), true);
    let none: [String; 0] = [];

    stage::stage_paths(&exec, &repo.path, &none, &cancel)
        .await
        .expect("stage");
    stage::unstage_paths(&exec, &repo.path, &none, &cancel)
        .await
        .expect("unstage");
    stage::discard_worktree(&exec, &repo.path, &none, &cancel)
        .await
        .expect("discard worktree");
    stage::discard_to_head(&exec, &repo.path, &none, &cancel)
        .await
        .expect("discard to head");
    stage::remove_untracked(&exec, &repo.path, &none, &cancel)
        .await
        .expect("remove untracked");

    let ran = ends.0.lock().unwrap().clone();
    assert!(ran.is_empty(), "git was run anyway: {ran:?}");

    let (staged, unstaged, untracked) = buckets(&repo).await;
    assert!(staged.is_empty());
    assert_eq!(unstaged, vec!["a.txt"], "the edit is still there");
    assert_eq!(untracked, vec!["new.txt"], "and so is the untracked file");

    // The same observer does hear a call that has something to do, so the
    // silence above was the guard and not the watching.
    stage::stage_paths(&exec, &repo.path, &["a.txt".to_string()], &cancel)
        .await
        .expect("stage a.txt");
    assert_eq!(ends.0.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn stage_and_unstage_whole_files() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("a.txt", "one changed\n");
    repo.write_file("new.txt", "fresh\n");
    let (exec, cancel) = env();

    let (staged, unstaged, untracked) = buckets(&repo).await;
    assert!(staged.is_empty());
    assert_eq!(unstaged, vec!["a.txt"]);
    assert_eq!(untracked, vec!["new.txt"]);

    stage::stage_paths(
        &exec,
        &repo.path,
        &["a.txt".into(), "new.txt".into()],
        &cancel,
    )
    .await
    .expect("stage");
    let (staged, unstaged, untracked) = buckets(&repo).await;
    assert_eq!(staged, vec!["a.txt", "new.txt"]);
    assert!(unstaged.is_empty() && untracked.is_empty());

    stage::unstage_paths(&exec, &repo.path, &["a.txt".into()], &cancel)
        .await
        .expect("unstage");
    let (staged, unstaged, _) = buckets(&repo).await;
    assert_eq!(staged, vec!["new.txt"]);
    assert_eq!(unstaged, vec!["a.txt"], "worktree edit survives unstaging");
}

#[tokio::test]
async fn unstage_works_on_an_unborn_branch() {
    let mut repo = TestRepo::init();
    repo.write_file("first.txt", "hello\n");
    repo.git(&["add", "--", "first.txt"]);
    let (exec, cancel) = env();

    stage::unstage_paths(&exec, &repo.path, &["first.txt".into()], &cancel)
        .await
        .expect("unstage on unborn HEAD");
    let (staged, _, untracked) = buckets(&repo).await;
    assert!(staged.is_empty());
    assert_eq!(untracked, vec!["first.txt"], "file is back to untracked");
}

#[tokio::test]
async fn discard_and_clean_reset_the_working_tree() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("a.txt", "broken\n");
    repo.write_file("junk.txt", "junk\n");
    let (exec, cancel) = env();

    stage::discard_worktree(&exec, &repo.path, &["a.txt".into()], &cancel)
        .await
        .expect("discard");
    stage::remove_untracked(&exec, &repo.path, &["junk.txt".into()], &cancel)
        .await
        .expect("clean");

    let (staged, unstaged, untracked) = buckets(&repo).await;
    assert!(staged.is_empty() && unstaged.is_empty() && untracked.is_empty());
    assert!(!repo.path.join("junk.txt").exists());
}

/// Discarding from the staged side takes both sides with it, whatever
/// shape the staged change has: an edit goes back to HEAD, a file staged
/// as new leaves the disk, and a rename needs both of its names to be
/// undone in one go.
#[tokio::test]
async fn discard_to_head_undoes_every_staged_shape() {
    let mut repo = TestRepo::init();
    repo.commit_file("kept.txt", "one\n", "root");
    repo.commit_file("moved.txt", "move me\n", "second");
    repo.write_file("kept.txt", "staged\n");
    repo.git(&["add", "--", "kept.txt"]);
    // Staged on both sides: the worktree has gone on past the index.
    repo.write_file("kept.txt", "and dirty\n");
    repo.write_file("fresh.txt", "brand new\n");
    repo.git(&["add", "--", "fresh.txt"]);
    repo.git(&["mv", "moved.txt", "elsewhere.txt"]);
    let (exec, cancel) = env();

    stage::discard_to_head(
        &exec,
        &repo.path,
        &[
            "kept.txt".into(),
            "fresh.txt".into(),
            // Both names of the rename, or the old one stays staged as a
            // deletion.
            "elsewhere.txt".into(),
            "moved.txt".into(),
        ],
        &cancel,
    )
    .await
    .expect("discard to head");

    let (staged, unstaged, untracked) = buckets(&repo).await;
    assert!(
        staged.is_empty() && unstaged.is_empty() && untracked.is_empty(),
        "nothing is left over: {staged:?} {unstaged:?} {untracked:?}"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path.join("kept.txt")).unwrap(),
        "one\n"
    );
    assert!(repo.path.join("moved.txt").exists(), "the rename is undone");
    assert!(!repo.path.join("elsewhere.txt").exists());
    assert!(
        !repo.path.join("fresh.txt").exists(),
        "HEAD has no such file, so discarding it takes the file"
    );
}

#[tokio::test]
async fn discard_to_head_works_on_an_unborn_branch() {
    let mut repo = TestRepo::init();
    repo.write_file("first.txt", "hello\n");
    repo.git(&["add", "--", "first.txt"]);
    let (exec, cancel) = env();

    stage::discard_to_head(&exec, &repo.path, &["first.txt".into()], &cancel)
        .await
        .expect("discard to head on unborn HEAD");
    let (staged, _, untracked) = buckets(&repo).await;
    assert!(staged.is_empty() && untracked.is_empty());
    assert!(!repo.path.join("first.txt").exists());
}
