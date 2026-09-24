//! Stash operations against real repositories. What git does with the two
//! push options nothing here asks for is in [`periodic`].

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::stash::{self, PushOptions};
use platitude_core::status;

#[tokio::test]
async fn stash_push_drop_and_pop() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("a.txt", "edited\n");
    let (exec, cancel) = env();

    stash::push(
        &exec,
        &repo.path,
        "my work in progress",
        PushOptions::default(),
        &[],
        &cancel,
    )
    .await
    .expect("stash push");

    let entries = stash::load(&exec, &repo.path, &cancel)
        .await
        .expect("stash list");
    assert_eq!(entries.len(), 1);
    assert!(
        entries[0].message.contains("my work in progress"),
        "custom message kept: {}",
        entries[0].message
    );
    assert_eq!(
        std::fs::read_to_string(repo.path.join("a.txt")).unwrap(),
        "one\n",
        "working tree restored"
    );

    stash::pop(&exec, &repo.path, &entries[0].name, &cancel)
        .await
        .expect("stash pop");
    assert_eq!(
        std::fs::read_to_string(repo.path.join("a.txt")).unwrap(),
        "edited\n"
    );
    assert!(
        stash::load(&exec, &repo.path, &cancel)
            .await
            .expect("stash list")
            .is_empty(),
        "pop removes the entry"
    );
}

#[tokio::test]
async fn stash_includes_untracked_when_asked() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("new.txt", "fresh\n");
    let (exec, cancel) = env();

    stash::push(
        &exec,
        &repo.path,
        "with untracked",
        PushOptions {
            include_untracked: true,
            ..Default::default()
        },
        &[],
        &cancel,
    )
    .await
    .expect("stash push -u");
    assert!(
        !repo.path.join("new.txt").exists(),
        "untracked file stashed"
    );

    let entries = stash::load(&exec, &repo.path, &cancel).await.expect("list");
    stash::apply(&exec, &repo.path, &entries[0].name, &cancel)
        .await
        .expect("apply");
    assert!(repo.path.join("new.txt").exists());
    assert_eq!(
        stash::load(&exec, &repo.path, &cancel)
            .await
            .expect("list")
            .len(),
        1,
        "apply keeps the entry"
    );

    stash::drop(&exec, &repo.path, &entries[0].name, &cancel)
        .await
        .expect("drop");
    assert!(
        stash::load(&exec, &repo.path, &cancel)
            .await
            .expect("list")
            .is_empty()
    );
}

#[tokio::test]
async fn stash_limited_to_one_path_leaves_the_rest_behind() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "one\n", "second");
    // The file that goes is changed on both sides at once, which the
    // path form handles: unlike `--staged`, it takes the whole path.
    repo.write_file("a.txt", "staged\n");
    repo.git(&["add", "--", "a.txt"]);
    repo.write_file("a.txt", "staged then edited\n");
    repo.write_file("b.txt", "stays\n");
    repo.write_file("new.txt", "untracked, stays\n");
    let (exec, cancel) = env();

    stash::push(
        &exec,
        &repo.path,
        "just a.txt",
        PushOptions {
            include_untracked: true,
            ..Default::default()
        },
        &["a.txt".to_string()],
        &cancel,
    )
    .await
    .expect("stash push -- a.txt");

    assert_eq!(
        std::fs::read_to_string(repo.path.join("a.txt")).unwrap(),
        "one\n",
        "the named path went"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path.join("b.txt")).unwrap(),
        "stays\n",
        "everything else stayed"
    );
    assert!(
        repo.path.join("new.txt").exists(),
        "an untracked file outside the path stays"
    );

    stash::pop_with_index(&exec, &repo.path, "stash@{0}", &cancel)
        .await
        .expect("pop --index");
    let s = status::load(&exec, &repo.path, &cancel)
        .await
        .expect("status");
    assert_eq!(
        s.partially_staged().count(),
        1,
        "the staged/unstaged split of the stashed path came back"
    );
}

/// **What the pre-merge run leaves out**: git's `--keep-index` and `--staged`.
/// `PushOptions` can ask for either, and nothing in the application does —
/// every stash it makes sets both off — so these record what git does with
/// them: the index kept, the index alone taken, and a file changed on both
/// sides that `--staged` writes an entry for and then fails on. So the
/// full gate runs them (`-- --ignored ::periodic::`) rather than every
/// change.
mod periodic {
    use super::*;

    #[tokio::test]
    #[ignore = "git's --keep-index, which nothing here asks for: not worth the pre-merge run"]
    async fn stash_keep_index_leaves_the_staged_part_alone() {
        let mut repo = TestRepo::init();
        repo.commit_file("a.txt", "one\n", "root");
        repo.write_file("a.txt", "staged\n");
        repo.git(&["add", "--", "a.txt"]);
        repo.write_file("b.txt", "unstaged\n");
        repo.git(&["add", "--", "b.txt"]);
        repo.write_file("b.txt", "unstaged edited\n");
        let (exec, cancel) = env();

        stash::push(
            &exec,
            &repo.path,
            "keep index",
            PushOptions {
                keep_index: true,
                ..Default::default()
            },
            &[],
            &cancel,
        )
        .await
        .expect("stash --keep-index");

        let s = status::load(&exec, &repo.path, &cancel)
            .await
            .expect("status");
        assert_eq!(s.staged().count(), 2, "index survives");
        assert_eq!(s.unstaged().count(), 0, "worktree matches the index");
    }

    #[tokio::test]
    #[ignore = "git's --staged, which nothing here asks for: not worth the pre-merge run"]
    async fn staged_only_stash_needs_a_tree_split_git_can_separate() {
        let mut repo = TestRepo::init();
        repo.commit_file("a.txt", "one\n", "root");
        repo.commit_file("b.txt", "one\n", "second");
        repo.write_file("a.txt", "staged\n");
        repo.git(&["add", "--", "a.txt"]);
        repo.write_file("b.txt", "unstaged\n");
        let (exec, cancel) = env();

        let staged_only = PushOptions {
            staged_only: true,
            ..Default::default()
        };
        stash::push(&exec, &repo.path, "index only", staged_only, &[], &cancel)
            .await
            .expect("stash --staged");
        let s = status::load(&exec, &repo.path, &cancel)
            .await
            .expect("status");
        assert_eq!(s.staged().count(), 0, "the index went");
        assert_eq!(s.unstaged().count(), 1, "the rest of the tree stayed");

        // Changed on both sides: git writes the entry and then fails to take
        // the staged half out of the tree, leaving the entry behind with
        // nothing else done. The UI refuses before reaching this (measured).
        repo.git(&["add", "--", "b.txt"]);
        repo.write_file("b.txt", "unstaged again\n");
        let before = stash::load(&exec, &repo.path, &cancel).await.expect("list");
        let err = stash::push(&exec, &repo.path, "doomed", staged_only, &[], &cancel)
            .await
            .expect_err("git cannot separate a file changed on both sides");
        assert!(
            err.to_string().contains("Cannot remove worktree changes"),
            "git's own wording: {err}"
        );
        let after = stash::load(&exec, &repo.path, &cancel).await.expect("list");
        assert_eq!(after.len(), before.len() + 1, "the entry is written anyway");
        let s = status::load(&exec, &repo.path, &cancel)
            .await
            .expect("status");
        assert_eq!(s.partially_staged().count(), 1, "and the tree is untouched");
    }
}
