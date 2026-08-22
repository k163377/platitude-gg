//! Merging: fast-forward, `--no-ff`, `--ff-only`, and a conflict taken
//! either to a resolution or to an abort.

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

/// The failure that leaves `MERGE_HEAD` standing: a merge asked for
/// while one is already in progress. git spends 128 on it with the
/// marker right there (実測 2.55), so reading the marker without the
/// code would call it a stop — and the sentence saying what is really in
/// the way would never reach the screen.
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
    // Resolved and staged, so what refuses the second one is the merge
    // standing rather than the unmerged paths.
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

    // Not an error: git stopped and left the merge standing, which is a
    // landing of its own (2026-08-22 ユーザー報告 — it read as a failure).
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

/// Finishing a stopped merge with a plain commit records what
/// `--continue` would: same tree, same two parents, same message — and
/// the message is the one git left in `MERGE_MSG`, which is why the
/// application can put it in the box before the press
/// (デザイン規約 §進行中の操作から出る).
///
/// The two runs are two repositories, so the ids differ; everything the
/// two commits are made of does not.
#[tokio::test]
async fn a_stopped_merge_finished_by_committing_records_what_continue_would() {
    let (exec, cancel) = env();
    let mut landed = Vec::new();
    for way in ["continue", "commit"] {
        let mut repo = conflicting_branches();
        integrate::merge(&exec, &repo.path, "side", &MergeOptions::default(), &cancel)
            .await
            .expect("conflict");
        let waiting = integrate::stopped_message(&exec, &repo.path, &cancel).await;
        assert_eq!(waiting, "Merge branch 'side'", "git's comment lines go");

        std::fs::write(repo.path.join("f.txt"), "resolved\n").expect("resolve");
        repo.git(&["add", "--", "f.txt"]);
        if way == "continue" {
            integrate::resolve_current(&exec, &repo.path, Continuation::Continue, &cancel)
                .await
                .expect("continue");
        } else {
            let info = crate::support::integrate::info(&repo).await;
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

/// A resolution that puts back exactly what HEAD already had still has a
/// merge to finish, and a plain commit writes it — the merge commit
/// records nothing and git makes it anyway (実測 2.55). Which is why the
/// commit button cannot ask for something staged while a merge stands.
#[tokio::test]
async fn a_merge_that_records_nothing_is_still_committed() {
    let mut repo = conflicting_branches();
    let (exec, cancel) = env();
    integrate::merge(&exec, &repo.path, "side", &MergeOptions::default(), &cancel)
        .await
        .expect("conflict");
    repo.git(&["checkout", "HEAD", "--", "f.txt"]);
    assert_eq!(repo.git(&["diff", "--cached", "--name-only", "HEAD"]), "");

    let info = crate::support::integrate::info(&repo).await;
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

/// The sides come back as `Some` only when they were really read. Nothing
/// there is `None` rather than an empty list, so a caller cannot mistake a
/// read that told it nothing for a merge that ended — every way of
/// answering "no sides" other than `None` is one that a failed read would
/// answer too.
#[tokio::test]
async fn merge_heads_says_nothing_read_rather_than_no_sides() {
    let repo = conflicting_branches();
    let (exec, cancel) = env();
    assert_eq!(
        opstate::merge_heads(&exec, &repo.path, &cancel).await,
        None,
        "no merge is standing, so there is no MERGE_HEAD to read"
    );

    integrate::merge(&exec, &repo.path, "side", &MergeOptions::default(), &cancel)
        .await
        .expect("conflict");
    let sides = opstate::merge_heads(&exec, &repo.path, &cancel)
        .await
        .expect("a standing merge names its side");
    let mut repo = repo;
    assert_eq!(
        sides.iter().map(Oid::to_hex).collect::<Vec<_>>(),
        vec![repo.git(&["rev-parse", "side"])]
    );
}
