//! What git says when a rewrite is fired while another operation stands,
//! and what a stash does to the operation standing there.
//!
//! Both halves of the trap the session's guards are for: git's refusal is
//! worded as a dirty tree and never names the operation, and the stash the
//! carry would take next puts that operation down.

use crate::support::TestRepo;
use crate::support::exec::env;
use crate::support::info;
use crate::support::integrate::helper;
use platitude_core::integrate::{RebaseOptions, RebaseOutcome};
use platitude_core::sequencer::{self, RebaseStep, TodoAction};

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

/// Settles the conflict the way a person at a terminal would: write the
/// file, stage it, and leave the operation standing for the commit.
fn resolve_and_stage(repo: &mut TestRepo) {
    repo.write_file("f.txt", "resolved by hand\n");
    repo.git(&["add", "f.txt"]);
}

/// The `Blocked` refusal git gave, or a panic naming what came instead —
/// `Blocked` is the classifier's own verdict, so reaching it *is* the
/// observation.
async fn blocked_stderr(repo: &TestRepo, interactive: bool) -> String {
    let (exec, cancel) = env();
    let outcome = if interactive {
        let steps = vec![RebaseStep {
            action: TodoAction::Pick,
            oid: repo.path.to_string_lossy().to_string(),
            subject: "unused".into(),
            message: None,
        }];
        // The todo is never reached: git refuses before it opens an
        // editor, which is the whole point of the observation.
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
        RebaseOutcome::Blocked(refusal) => refusal.to_string(),
        other => panic!("expected a refusal the carry would act on, got {other:?}"),
    }
}

/// **The refusal a standing operation earns is worded as a dirty tree.**
/// git sees the operation's own staged result in the index and says so;
/// it never names the operation, and both the plain and the interactive
/// rebase word it identically. The classifier therefore calls it work in
/// the way and the carry would go round through a stash — which is why
/// the session guards on the operation itself rather than on the wording
/// (`session::build::carry_across_rewrite`).
#[tokio::test]
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

/// Still conflicted, the same refusal arrives with the unstaged half
/// named as well — the stash that would follow is the only reason this
/// case has never lost anything, and it is an accident of the index being
/// unmerged rather than a guard.
#[tokio::test]
async fn an_unsettled_conflict_earns_the_same_refusal() {
    let mut repo = diverged();
    repo.git_expect_failure(&["merge", "side"]);
    let stderr = blocked_stderr(&repo, true).await;
    assert!(
        stderr.contains("cannot rebase:") && stderr.contains("unstaged changes"),
        "git said: {stderr}"
    );
}

/// **A stash puts a standing operation down.** `git stash push` writes
/// the resolved index away and the marker goes with it, so a carry that
/// stashed here would leave the merge's second parent unrecoverable and
/// the resolution looking like an ordinary staged edit.
#[test]
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
/// alone, which is why the carry's guard reads `InProgress::from_state`
/// rather than `any` — a dirty tree under a bisect still carries.
#[test]
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
