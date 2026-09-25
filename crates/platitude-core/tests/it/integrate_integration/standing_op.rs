//! What git says when a rewrite is fired while another operation stands,
//! and what a stash does to that operation — the trap the session's guards
//! are for (held pre-merge by `session_integration::standing_op`).

use crate::support::TestRepo;
use crate::support::exec::env;
use crate::support::info;
use crate::support::integrate::helper;
use platitude_core::error::GitError;
use platitude_core::integrate::{RebaseOptions, RebaseOutcome};
use platitude_core::sequencer::{self, RebaseStep, TodoAction};

/// All of it git's own behaviour (rules-refs/core.md `periodic`).
mod periodic {
    use super::*;

    /// main and side both change `f.txt`, so every operation between them
    /// stops on a conflict.
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

    /// Stages a resolution and leaves the operation standing, uncommitted.
    fn resolve_and_stage(repo: &mut TestRepo) {
        repo.write_file("f.txt", "resolved by hand\n");
        repo.git(&["add", "f.txt"]);
    }

    /// The `stderr` field alone: the rendered error leads with the command
    /// line, whose `--no-rebase-merges` carries the word `merge`.
    async fn blocked_stderr(repo: &TestRepo, interactive: bool) -> String {
        let (exec, cancel) = env();
        let outcome = if interactive {
            let steps = vec![RebaseStep {
                action: TodoAction::Pick,
                oid: repo.path.to_string_lossy().to_string(),
                subject: "unused".into(),
                message: None,
            }];
            // Never read: git refuses before it opens an editor.
            sequencer::rebase_interactive(
                &exec,
                &info(repo).await,
                "main",
                &steps,
                &RebaseOptions::default(),
                &helper(),
                &cancel,
            )
            .await
        } else {
            platitude_core::integrate::rebase(
                &exec,
                &repo.path,
                "main",
                &RebaseOptions::default(),
                &cancel,
            )
            .await
        };
        match outcome.expect("git answered rather than failed") {
            RebaseOutcome::Blocked(GitError::Failed { stderr, .. }) => stderr,
            RebaseOutcome::Blocked(other) => {
                panic!("the refusal was expected to be git's: {other:?}")
            }
            other => panic!("expected a refusal the carry would act on, got {other:?}"),
        }
    }

    /// git reads the operation's staged result as a dirty index and never
    /// names the operation, in both rebases alike. The classifier would carry
    /// it through a stash, so the session guards on the operation itself
    /// (`session::build::carry_across_rewrite`).
    #[tokio::test]
    #[ignore = "git's refusal text under a standing operation: not worth the pre-merge run"]
    async fn a_rebase_under_a_standing_operation_is_refused_as_a_dirty_tree() {
        for (name, start) in [
            ("merge", vec!["merge", "side"]),
            ("cherry-pick", vec!["cherry-pick", "side"]),
            ("revert", vec!["revert", "--no-edit", "HEAD~2"]),
        ] {
            let mut repo = diverged();
            repo.git_expect_failure(&start);
            resolve_and_stage(&mut repo);
            for interactive in [false, true] {
                let stderr = blocked_stderr(&repo, interactive).await;
                assert!(
                    stderr.contains("cannot rebase:")
                        && stderr.contains("index contains uncommitted changes"),
                    "{name} (interactive={interactive}) — git said: {stderr}"
                );
                assert!(
                    !stderr.contains(name),
                    "and it does not name the operation: {stderr}"
                );
            }
        }
    }

    /// Still conflicted, the refusal also names the unstaged half. That
    /// nothing was lost here is an accident of the index being unmerged.
    #[tokio::test]
    #[ignore = "git's refusal text under an unmerged index: not worth the pre-merge run"]
    async fn an_unsettled_conflict_earns_the_same_refusal() {
        let mut repo = diverged();
        repo.git_expect_failure(&["merge", "side"]);
        let stderr = blocked_stderr(&repo, true).await;
        assert!(
            stderr.contains("cannot rebase:") && stderr.contains("unstaged changes"),
            "git said: {stderr}"
        );
    }

    /// `git stash push` takes the marker with the resolved index: a carry
    /// that stashed would lose the merge's second parent and leave the
    /// resolution an ordinary staged edit.
    #[test]
    #[ignore = "what git's stash does to a standing operation: not worth the pre-merge run"]
    fn a_stash_takes_a_standing_operation_down_with_it() {
        for (marker, start) in [
            ("MERGE_HEAD", vec!["merge", "side"]),
            ("CHERRY_PICK_HEAD", vec!["cherry-pick", "side"]),
            ("REVERT_HEAD", vec!["revert", "--no-edit", "HEAD~2"]),
        ] {
            let mut repo = diverged();
            repo.git_expect_failure(&start);
            resolve_and_stage(&mut repo);
            let path = repo.path.join(".git").join(marker);
            assert!(path.exists(), "{marker} stands before the stash");
            assert!(repo.git_ok(&["stash", "push", "--include-untracked"]));
            assert!(!path.exists(), "{marker} did not survive the stash");
        }
    }

    /// A bisect is the one thing `OpState::any` counts that a stash leaves
    /// alone, which is why the carry's guard reads `InProgress::from_state` —
    /// a dirty tree under a bisect still carries.
    #[test]
    #[ignore = "what git's stash leaves of a bisect: not worth the pre-merge run"]
    fn a_bisect_survives_a_stash() {
        let mut repo = diverged();
        repo.git(&["bisect", "start"]);
        repo.git(&["bisect", "bad"]);
        repo.git(&["bisect", "good", "main~2"]);
        repo.write_file("keep.txt", "dirty\n");
        assert!(repo.git_ok(&["stash", "push", "--include-untracked"]));
        assert!(
            repo.path.join(".git").join("BISECT_LOG").exists(),
            "the bisect is still running"
        );
        assert!(repo.git_ok(&["bisect", "log"]));
    }
}
