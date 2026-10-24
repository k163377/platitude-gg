//! Pulls, exercised entirely offline: the remote is a `file://` URL of a
//! second local repository (実装計画 §10). The branch the worktree is on
//! goes through `remote::pull`, one it is not on through
//! `remote::fast_forward`.
//!
//! What git alone decides about them is in [`periodic`].

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::exec::env;
use crate::support::remote::{a_branch_over_there, moved_over_there, origin_and_clone};
use platitude_core::GitError;
use platitude_core::remote;

/// A `file://` remote answers instantly; the budget just has to exist.
const NET: std::time::Duration = remote::DEFAULT_NETWORK_TIMEOUT;

/// The fast-forward git refuses over work in the way exits 1 with nothing
/// standing, and its row keeps git's list of the files (as a merge's does).
#[tokio::test]
async fn a_pull_refused_over_work_in_the_way_keeps_gits_words_in_the_log() {
    let (bare, mut work) = origin_and_clone();
    work.git(&["branch", "--set-upstream-to=origin/main", "main"]);
    moved_over_there(&bare, "main", "a.txt", "their line");
    work.write_file("a.txt", "mine\n");
    let said = std::sync::Arc::new(crate::support::exec::Said::default());
    let (exec, cancel) =
        crate::support::exec::observed_env(said.clone(), platitude_core::process::Kept::Asked);

    let err = remote::pull(&exec, &work.path, NET, &cancel)
        .await
        .expect_err("the work in the way is refused");
    assert!(matches!(err, GitError::Failed { code: 1, .. }), "{err}");
    let row = said
        .message_of(platitude_core::process::CommandEnd::Exited(1))
        .expect("the pull's row is a failure, not an answer");
    assert!(
        row.contains("a.txt") && row.contains("would be overwritten"),
        "the files in the way are under the row: {row}"
    );
}

/// The pull of a branch the worktree is not on moves that branch and its
/// reading of the remote, and nothing the worktree stands on.
#[tokio::test]
async fn a_branch_aside_is_fast_forwarded_to_what_its_upstream_holds() {
    let (bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();
    a_branch_over_there(&mut work, "topic");
    let theirs = moved_over_there(&bare, "topic", "theirs.txt", "pushed since our fetch");
    let head = work.git(&["rev-parse", "HEAD"]);

    remote::fast_forward(&exec, &work.path, "topic", NET, &cancel)
        .await
        .expect("fast-forward");
    assert_eq!(
        work.git(&["rev-parse", "topic"]),
        theirs,
        "the branch is on what the far side holds"
    );
    assert_eq!(
        work.git(&["rev-parse", "origin/topic"]),
        theirs,
        "and the reading of the remote came down with it"
    );
    assert_eq!(work.git(&["rev-parse", "HEAD"]), head);
    assert_eq!(
        work.git(&["symbolic-ref", "--short", "HEAD"]),
        "main",
        "the worktree stays on its own branch"
    );
}

/// No `+` on the refspec: a branch with commits its upstream lacks is never
/// set back to the upstream's tip. git turns the fetch down instead, which
/// is why the row is greyed before the press (`offers::pull_row`).
#[tokio::test]
async fn a_branch_aside_with_commits_of_its_own_keeps_them() {
    let (bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();
    a_branch_over_there(&mut work, "topic");
    moved_over_there(&bare, "topic", "theirs.txt", "pushed since our fetch");
    work.git(&["switch", "topic"]);
    work.commit_file("mine.txt", "mine\n", "work of our own");
    work.git(&["switch", "main"]);
    let ours = work.git(&["rev-parse", "topic"]);

    let err = remote::fast_forward(&exec, &work.path, "topic", NET, &cancel)
        .await
        .expect_err("not a fast-forward");
    assert!(
        matches!(err, GitError::Failed { code: 1, .. }),
        "git's own refusal, passed through: {err}"
    );
    assert_eq!(work.git(&["rev-parse", "topic"]), ours, "nothing was lost");
}

/// A branch that has stopped tracking anything since the menu opened is
/// refused here, before git is handed a refspec with half of it missing.
#[tokio::test]
async fn a_branch_aside_tracking_nothing_is_refused_before_git() {
    let (_bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();
    work.git(&["branch", "solo"]);

    let err = remote::fast_forward(&exec, &work.path, "solo", NET, &cancel)
        .await
        .expect_err("nowhere to pull from");
    assert!(matches!(err, GitError::Rejected { .. }), "{err:?}");
}

/// What git itself does behind the fixed `pull` line (unit-tested beside
/// `remote::pull`). What is read of its answer is held above, so only the
/// full gate runs these.
mod periodic {
    use super::*;

    #[tokio::test]
    #[ignore = "git's own dirty-tree refusal: not worth the pre-merge run"]
    async fn a_pull_over_changes_it_would_write_on_is_a_failure() {
        let (bare, mut work) = origin_and_clone();
        let (exec, cancel) = env();
        work.git(&["branch", "--set-upstream-to=origin/main", "main"]);
        moved_over_there(&bare, "main", "a.txt", "their line");
        work.write_file("a.txt", "uncommitted\n");

        remote::pull(&exec, &work.path, NET, &cancel)
            .await
            .expect_err("git will not write over what is not committed");
        assert!(
            !work.git_ok(&["rev-parse", "--verify", "-q", "MERGE_HEAD"]),
            "and it left nothing standing, so the words are the whole answer"
        );
    }

    /// The row is offered only where the branch is behind, so a branch that
    /// diverged since the last read is refused whole, `pull.rebase` set either
    /// way or not at all: nothing merges, nothing rebases, nothing is left
    /// standing, and git is never left to ask how to reconcile.
    #[tokio::test]
    #[ignore = "git's --ff-only refusal whatever pull.rebase says: not worth the pre-merge run"]
    async fn a_diverged_pull_is_refused_whatever_pull_rebase_says() {
        for reconcile in [None, Some("false"), Some("true")] {
            let (bare, mut work) = origin_and_clone();
            let (exec, cancel) = env();
            work.git(&["branch", "--set-upstream-to=origin/main", "main"]);
            if let Some(reconcile) = reconcile {
                work.git(&["config", "pull.rebase", reconcile]);
            }
            moved_over_there(&bare, "main", "a.txt", "their line");
            work.commit_file("b.txt", "our line\n", "ours");

            let err = remote::pull(&exec, &work.path, NET, &cancel)
                .await
                .expect_err("a diverged branch is no fast-forward");
            assert!(
                matches!(err, GitError::Failed { code: 128, .. })
                    && err.to_string().contains("Not possible to fast-forward"),
                "pull.rebase={reconcile:?}: {err}"
            );
            for marker in ["MERGE_HEAD", "rebase-merge"] {
                assert!(
                    !work.path.join(".git").join(marker).exists(),
                    "nothing was left standing: {marker}"
                );
            }
            assert_eq!(work.git(&["log", "-1", "--format=%s"]), "ours");
        }
    }

    /// `pull.rebase` would have git check the tree is clean before any
    /// fast-forward (`cannot pull with rebase: You have unstaged changes`);
    /// pinned off, a fast-forward goes through over work it does not touch.
    #[tokio::test]
    #[ignore = "git's pull.rebase clean-tree check kept out by the fixed --no-rebase: not worth the pre-merge run"]
    async fn a_pull_rebase_setting_does_not_keep_a_fast_forward_from_a_dirty_tree() {
        let (bare, mut work) = origin_and_clone();
        let (exec, cancel) = env();
        work.git(&["branch", "--set-upstream-to=origin/main", "main"]);
        work.git(&["config", "pull.rebase", "true"]);
        moved_over_there(&bare, "main", "b.txt", "over there");
        work.write_file("a.txt", "mine\n");

        remote::pull(&exec, &work.path, NET, &cancel)
            .await
            .expect("a fast-forward over work it does not touch");
        assert_eq!(
            work.git(&["log", "-1", "--format=%s", "main"]),
            "over there"
        );
        assert_eq!(
            std::fs::read_to_string(work.path.join("a.txt")).unwrap(),
            "mine\n",
            "the work stayed where it was"
        );
    }

    /// A refusal stays a failure: no tracking information and a tree the pull
    /// would write over both exit 1 with nothing standing.
    #[tokio::test]
    #[ignore = "git's exit 1 for a branch with no tracking information: not worth the pre-merge run"]
    async fn a_pull_with_nothing_to_pull_from_is_a_failure() {
        let (_bare, mut work) = origin_and_clone();
        let (exec, cancel) = env();
        work.git(&["switch", "--create", "solo"]);

        let err = remote::pull(&exec, &work.path, NET, &cancel)
            .await
            .expect_err("a branch with no tracking information has nowhere to go");
        assert!(
            matches!(err, GitError::Failed { code: 1, .. }),
            "git spends 1 on it: {err}"
        );
    }

    #[tokio::test]
    #[ignore = "git bringing the remote-tracking ref down with a pull: not worth the pre-merge run"]
    async fn a_pull_brings_the_upstream_in_and_moves_the_branch() {
        let (bare, mut work) = origin_and_clone();
        let (exec, cancel) = env();
        work.git(&["branch", "--set-upstream-to=origin/main", "main"]);
        moved_over_there(&bare, "main", "b.txt", "over there");

        remote::pull(&exec, &work.path, NET, &cancel)
            .await
            .expect("pull");
        assert_eq!(
            work.git(&["log", "-1", "--format=%s", "main"]),
            "over there",
            "the branch is on what the far side holds"
        );
        assert_eq!(
            work.git(&["log", "-1", "--format=%s", "origin/main"]),
            "over there",
            "and the reading of the remote came down with it"
        );
    }
}
