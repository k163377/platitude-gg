//! Merging: fast-forward, `--no-ff`, `--ff-only`, and a conflict taken
//! either to a resolution or to an abort.

use crate::support::TestRepo;
use crate::support::exec::env;
use crate::support::integrate::{conflicting_branches, current_op};
use platitude_core::conflict::{self, ConflictKind, Side};
use platitude_core::integrate::{self, Continuation, InProgress, MergeOptions, MergeOutcome};
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
        MergeOutcome::Done
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
        MergeOutcome::Done
    );
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "explicit merge");
    assert_eq!(
        repo.git(&["log", "-1", "--format=%P"]).split(' ').count(),
        2,
        "a merge commit was recorded"
    );
}

#[tokio::test]
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

/// The exit code cannot sort a merge's answers on its own: git spends 1
/// on a conflict and on a name it will not merge alike, so the second
/// has to keep reading as the failure it is. Nothing is left standing
/// after it, which is how they are told apart.
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

/// `--squash` conflicts are the one stop that reads as a failure: git
/// writes `SQUASH_MSG` and no `MERGE_HEAD` (実測 2.55), so there is no
/// operation standing to be continued. Recorded rather than worked
/// around — nothing on screen asks for a squashed merge.
#[tokio::test]
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

#[tokio::test]
async fn a_conflicting_merge_is_reported_then_aborted() {
    let repo = conflicting_branches();
    let (exec, cancel) = env();

    // Not an error: git stopped and left the merge standing, which is a
    // landing of its own (2026-08-22 ユーザー報告 — it read as a failure).
    assert_eq!(
        integrate::merge(&exec, &repo.path, "side", &MergeOptions::default(), &cancel)
            .await
            .expect("a conflict is an answer, not a failure"),
        MergeOutcome::Stopped
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

#[tokio::test]
async fn merge_does_not_offer_skip() {
    let repo = conflicting_branches();
    let (exec, cancel) = env();
    let err = integrate::resolve(
        &exec,
        &repo.path,
        InProgress::Merge,
        Continuation::Skip,
        &cancel,
    )
    .await
    .expect_err("merge cannot skip");
    assert!(err.to_string().contains("does not support"), "{err}");
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
