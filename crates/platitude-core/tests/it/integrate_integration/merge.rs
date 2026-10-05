//! Merging: fast-forward, `--no-ff`, and a conflict taken either to a
//! resolution or to an abort.

use crate::support::TestRepo;
use crate::support::exec::env;
use crate::support::integrate::{conflicting_branches, current_op};
use platitude_core::commit;
use platitude_core::conflict::{self, ConflictKind, Side};
use platitude_core::integrate::{self, Continuation, InProgress, Landing, MergeOptions};
use platitude_core::oid::Oid;
use platitude_core::opstate;
use platitude_core::status;

#[tokio::test]
async fn merge_fast_forward_and_no_ff() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["checkout", "-b", "side"]);
    repo.commit_file("b.txt", "two\n", "side work");
    repo.git(&["checkout", "main"]);
    let (exec, cancel) = env();

    assert_eq!(
        integrate::merge(&exec, &repo.path, "side", &MergeOptions::default(), &cancel)
            .await
            .expect("fast-forward merge"),
        Landing::Done
    );
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "side work");
    assert_eq!(repo.git(&["rev-list", "--count", "HEAD"]), "2");

    repo.git(&["checkout", "-b", "other", "HEAD~1"]);
    repo.commit_file("c.txt", "three\n", "other work");
    assert_eq!(
        integrate::merge(
            &exec,
            &repo.path,
            "main",
            &MergeOptions {
                no_ff: true,
                message: Some("explicit merge".into()),
                ..Default::default()
            },
            &cancel,
        )
        .await
        .expect("no-ff merge"),
        Landing::Done
    );
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "explicit merge");
    assert_eq!(
        repo.git(&["log", "-1", "--format=%P"]).split(' ').count(),
        2,
        "a merge commit was recorded"
    );
}

/// git exits 1 on a conflict and on a name it will not merge alike; only
/// the conflict leaves a merge standing, which is how they are told apart.
#[tokio::test]
async fn a_name_git_will_not_merge_is_still_a_failure() {
    let repo = conflicting_branches();
    let (exec, cancel) = env();
    let err = integrate::merge(
        &exec,
        &repo.path,
        "no-such-ref",
        &MergeOptions::default(),
        &cancel,
    )
    .await
    .expect_err("nothing to merge under that name");
    assert!(
        err.to_string().contains("not something we can merge"),
        "{err}"
    );
    assert_eq!(current_op(&repo).await, None);
}

/// git exits 128 here with `MERGE_HEAD` still standing, so reading the
/// marker without the code would call it a stop and hide the sentence
/// saying what is in the way.
#[tokio::test]
async fn a_second_merge_over_one_already_standing_is_still_a_failure() {
    let mut repo = conflicting_branches();
    let (exec, cancel) = env();
    let options = MergeOptions::default();

    assert_eq!(
        integrate::merge(&exec, &repo.path, "side", &options, &cancel)
            .await
            .expect("the first one stops"),
        Landing::Stopped
    );
    // Resolved and staged, so only the standing merge refuses the second.
    std::fs::write(repo.path.join("f.txt"), "resolved\n").expect("resolve");
    repo.git(&["add", "--", "f.txt"]);

    integrate::merge(&exec, &repo.path, "side", &options, &cancel)
        .await
        .expect_err("a merge over one already standing is a failure");
    assert_eq!(
        current_op(&repo).await,
        Some(InProgress::Merge),
        "and the one that was standing is still standing"
    );
}

#[tokio::test]
async fn a_conflicting_merge_is_reported_then_aborted() {
    let repo = conflicting_branches();
    let (exec, cancel) = env();

    assert_eq!(
        integrate::merge(&exec, &repo.path, "side", &MergeOptions::default(), &cancel)
            .await
            .expect("a conflict is an answer, not a failure"),
        Landing::Stopped
    );

    assert_eq!(current_op(&repo).await, Some(InProgress::Merge));
    let s = status::load(&exec, &repo.path, &cancel)
        .await
        .expect("status");
    let files = conflict::conflicted(&s);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "f.txt");
    assert_eq!(files[0].kind, ConflictKind::BothModified);
    assert!(files[0].kind.is_content_conflict());

    assert!(
        integrate::resolve_current(&exec, &repo.path, Continuation::Abort, &cancel)
            .await
            .expect("abort"),
        "something was in progress"
    );
    assert_eq!(current_op(&repo).await, None);
    assert_eq!(
        std::fs::read_to_string(repo.path.join("f.txt")).unwrap(),
        "main\n"
    );
}

/// The message is the one git left in `MERGE_MSG`, so the application can
/// put it in the box before the press (デザイン規約 §進行中の操作から出る).
/// The two runs are two repositories, so the ids differ.
#[tokio::test]
async fn a_stopped_merge_finished_by_committing_records_what_continue_would() {
    let (exec, cancel) = env();
    let mut landed = Vec::new();
    for way in ["continue", "commit"] {
        let mut repo = conflicting_branches();
        integrate::merge(&exec, &repo.path, "side", &MergeOptions::default(), &cancel)
            .await
            .expect("conflict");
        let waiting = integrate::stopped_message(&repo.path.join(".git"));
        assert_eq!(waiting, "Merge branch 'side'", "git's comment lines go");

        std::fs::write(repo.path.join("f.txt"), "resolved\n").expect("resolve");
        repo.git(&["add", "--", "f.txt"]);
        if way == "continue" {
            integrate::resolve_current(&exec, &repo.path, Continuation::Continue, &cancel)
                .await
                .expect("continue");
        } else {
            let info = crate::support::info(&repo).await;
            commit::commit(&exec, &info, &waiting, Default::default(), &cancel)
                .await
                .expect("commit");
        }
        assert_eq!(current_op(&repo).await, None);
        landed.push(repo.git(&["log", "-1", "--format=%T|%s|%P"]));
    }
    let by_continue: Vec<&str> = landed[0].split('|').collect();
    let by_commit: Vec<&str> = landed[1].split('|').collect();
    assert_eq!(by_continue[0], by_commit[0], "the same tree");
    assert_eq!(by_continue[1], by_commit[1], "the same message");
    assert_eq!(
        by_continue[2].split(' ').count(),
        2,
        "two parents: {}",
        by_continue[2]
    );
    assert_eq!(
        by_commit[2].split(' ').count(),
        2,
        "two parents: {}",
        by_commit[2]
    );
}

#[tokio::test]
async fn resolving_a_conflict_by_taking_one_side_lets_the_merge_continue() {
    let mut repo = conflicting_branches();
    let (exec, cancel) = env();
    integrate::merge(&exec, &repo.path, "side", &MergeOptions::default(), &cancel)
        .await
        .expect("conflict");

    conflict::take_side(&exec, &repo.path, &["f.txt".into()], Side::Theirs, &cancel)
        .await
        .expect("take theirs");
    assert_eq!(
        std::fs::read_to_string(repo.path.join("f.txt")).unwrap(),
        "side\n"
    );

    integrate::resolve_current(&exec, &repo.path, Continuation::Continue, &cancel)
        .await
        .expect("continue");
    assert_eq!(current_op(&repo).await, None);
    assert_eq!(
        repo.git(&["log", "-1", "--format=%P"]).split(' ').count(),
        2
    );
}

/// Only a real read answers `Some`: any other "no sides" is one a failed
/// read would answer too, and a caller would take it for a merge that ended.
#[tokio::test]
async fn merge_heads_says_nothing_read_rather_than_no_sides() {
    let repo = conflicting_branches();
    let (exec, cancel) = env();
    assert_eq!(
        opstate::merge_heads(&repo.path.join(".git")),
        None,
        "no merge is standing, so there is no MERGE_HEAD to read"
    );

    integrate::merge(&exec, &repo.path, "side", &MergeOptions::default(), &cancel)
        .await
        .expect("conflict");
    let sides =
        opstate::merge_heads(&repo.path.join(".git")).expect("a standing merge names its side");
    let mut repo = repo;
    assert_eq!(
        sides.iter().map(Oid::to_hex).collect::<Vec<_>>(),
        vec![repo.git(&["rev-parse", "side"])]
    );
}

/// git's own answers behind a fixed command line; the landings they rest on
/// are read by the tests above (rules-refs/core.md `periodic`).
mod periodic {
    use super::*;

    #[tokio::test]
    #[ignore = "git's own --ff-only refusal: not worth the pre-merge run"]
    async fn ff_only_merge_refuses_a_real_merge() {
        let repo = conflicting_branches();
        let (exec, cancel) = env();
        let err = integrate::merge(
            &exec,
            &repo.path,
            "side",
            &MergeOptions {
                ff_only: true,
                ..Default::default()
            },
            &cancel,
        )
        .await
        .expect_err("not a fast-forward");
        assert!(err.to_string().contains("fast-forward"), "{err}");
    }

    /// A `--squash` conflict reads as a failure: git writes `SQUASH_MSG` and
    /// no `MERGE_HEAD`, so nothing stands to be continued.
    #[tokio::test]
    #[ignore = "what git leaves after a --squash nothing asks for: not worth the pre-merge run"]
    async fn a_squashed_merge_leaves_nothing_standing_to_continue() {
        let repo = conflicting_branches();
        let (exec, cancel) = env();
        integrate::merge(
            &exec,
            &repo.path,
            "side",
            &MergeOptions {
                squash: true,
                ..Default::default()
            },
            &cancel,
        )
        .await
        .expect_err("a squash leaves no merge to continue");
        assert_eq!(current_op(&repo).await, None);
    }

    /// A resolution back to HEAD's content still leaves a merge that a plain
    /// commit writes, so the commit button cannot require something staged
    /// while a merge stands.
    #[tokio::test]
    #[ignore = "git writing an empty merge commit: not worth the pre-merge run"]
    async fn a_merge_that_records_nothing_is_still_committed() {
        let mut repo = conflicting_branches();
        let (exec, cancel) = env();
        integrate::merge(&exec, &repo.path, "side", &MergeOptions::default(), &cancel)
            .await
            .expect("conflict");
        repo.git(&["checkout", "HEAD", "--", "f.txt"]);
        assert_eq!(repo.git(&["diff", "--cached", "--name-only", "HEAD"]), "");

        let info = crate::support::info(&repo).await;
        commit::commit(
            &exec,
            &info,
            "Merge branch 'side'",
            Default::default(),
            &cancel,
        )
        .await
        .expect("a merge with nothing in it is still a merge");
        assert_eq!(current_op(&repo).await, None);
        assert_eq!(
            repo.git(&["log", "-1", "--format=%P"]).split(' ').count(),
            2
        );
    }
}
