//! Local branch operations on real repositories: creating, switching,
//! renaming and deleting. What git itself does behind a switch and a
//! reset — leaving a detached HEAD, carrying or refusing uncommitted work,
//! each reset mode's index and tree — is in [`periodic`].

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::branch::{self, CheckoutOutcome, CheckoutTarget, ResetMode};

#[tokio::test]
async fn branch_create_switch_rename_delete() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let (exec, cancel) = env();

    branch::create(&exec, &repo.path, "feature", None, false, &cancel)
        .await
        .expect("create");
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "main");

    branch::checkout(
        &exec,
        &repo.path,
        &CheckoutTarget::Branch {
            name: "feature".into(),
        },
        &cancel,
    )
    .await
    .expect("switch");
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "feature");

    branch::rename(&exec, &repo.path, "feature", "renamed", false, &cancel)
        .await
        .expect("rename");
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "renamed");

    branch::checkout(
        &exec,
        &repo.path,
        &CheckoutTarget::Branch {
            name: "main".into(),
        },
        &cancel,
    )
    .await
    .expect("switch back");
    branch::delete(&exec, &repo.path, "renamed", false, &cancel)
        .await
        .expect("delete merged branch");
    assert!(!repo.git(&["branch", "--list"]).contains("renamed"));
}

#[tokio::test]
async fn unmerged_branch_needs_the_forced_delete() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["checkout", "-b", "wip"]);
    repo.commit_file("b.txt", "two\n", "unmerged work");
    repo.git(&["checkout", "main"]);
    let (exec, cancel) = env();

    assert!(
        !branch::is_merged_into(&exec, &repo.path, "wip", "HEAD", &cancel)
            .await
            .expect("merge check"),
        "wip is not reachable from main"
    );
    let err = branch::delete(&exec, &repo.path, "wip", false, &cancel)
        .await
        .expect_err("plain delete refused");
    assert!(err.to_string().contains("not fully merged"), "{err}");

    branch::delete(&exec, &repo.path, "wip", true, &cancel)
        .await
        .expect("forced delete");
    assert!(!repo.git(&["branch", "--list"]).contains("wip"));
}

/// **What the pre-merge run leaves out**: what git does behind the fixed
/// command lines of a switch and a reset. Leaving a detached HEAD,
/// carrying work that is not in the way and refusing work that is are
/// git's; the refusals' wording is read by `branch::tests`, and the
/// session's carry acts before every merge on the move and on both refusals in
/// `session_integration::carry_move`. What each reset mode does to the
/// index and the tree is git's too; the command line each mode asks for
/// is `branch::tests::a_reset_names_its_mode_and_the_revision_and_nothing_more`.
/// So the full gate runs them (`-- --ignored ::periodic::`) rather than
/// every change.
mod periodic {
    use super::*;

    /// Nothing offered here detaches HEAD, but git and the command line still
    /// leave it detached — after a bisect, a `checkout <tag>`, an interrupted
    /// rebase. That is a state to be worked from and left, and leaving it is
    /// an ordinary switch.
    #[tokio::test]
    #[ignore = "git's switch off a detached HEAD: not worth the pre-merge run"]
    async fn a_detached_head_switches_back_onto_a_branch() {
        let mut repo = TestRepo::init();
        let root = repo.commit_file_id("a.txt", "one\n", "root");
        repo.commit_file("a.txt", "two\n", "second");
        let (exec, cancel) = env();
        repo.git(&["checkout", "--detach", &root]);
        assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "HEAD");

        branch::checkout(
            &exec,
            &repo.path,
            &CheckoutTarget::Branch {
                name: "main".into(),
            },
            &cancel,
        )
        .await
        .expect("switch off the detached HEAD");
        assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "main");
    }

    /// Two commits on `main`; the returned id is the first of them, which is
    /// where each reset takes the branch back to.
    fn one_commit_back() -> (TestRepo, String) {
        let mut repo = TestRepo::init();
        let root = repo.commit_file_id("a.txt", "one\n", "root");
        repo.commit_file("a.txt", "two\n", "second");
        (repo, root)
    }

    /// The keeping-it-staged move: the branch goes back, the files stay,
    /// and what the dropped commit wrote is ready to be committed again.
    #[tokio::test]
    #[ignore = "what git's reset modes do: not worth the pre-merge run"]
    async fn a_soft_reset_moves_the_branch_and_leaves_the_work_staged() {
        let (mut repo, root) = one_commit_back();
        let (exec, cancel) = env();

        branch::reset(&exec, &repo.path, &root, ResetMode::Soft, &cancel)
            .await
            .expect("soft reset");

        assert_eq!(repo.git(&["rev-parse", "HEAD"]), root);
        assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "main");
        assert_eq!(
            std::fs::read_to_string(repo.path.join("a.txt")).unwrap(),
            "two\n",
            "the working tree kept what the commit left behind wrote"
        );
        assert_eq!(
            repo.git(&["diff", "--cached", "--name-only"]),
            "a.txt",
            "and the difference is staged"
        );
    }

    /// The same move with the index cleared: the content is still on disk,
    /// but nothing of it is staged.
    #[tokio::test]
    #[ignore = "what git's reset modes do: not worth the pre-merge run"]
    async fn a_mixed_reset_moves_the_branch_and_unstages_the_work() {
        let (mut repo, root) = one_commit_back();
        let (exec, cancel) = env();

        branch::reset(&exec, &repo.path, &root, ResetMode::Mixed, &cancel)
            .await
            .expect("mixed reset");

        assert_eq!(repo.git(&["rev-parse", "HEAD"]), root);
        assert_eq!(
            std::fs::read_to_string(repo.path.join("a.txt")).unwrap(),
            "two\n"
        );
        assert_eq!(repo.git(&["diff", "--cached", "--name-only"]), "");
        assert_eq!(
            repo.git(&["diff", "--name-only"]),
            "a.txt",
            "the difference is there to stage again"
        );
    }

    /// The discarding move — the one the UI asks about. Tracked work goes,
    /// staged or not; untracked files are none of a reset's business.
    #[tokio::test]
    #[ignore = "what git's reset modes do: not worth the pre-merge run"]
    async fn a_hard_reset_throws_tracked_work_away_and_leaves_untracked_files() {
        let (mut repo, root) = one_commit_back();
        repo.write_file("a.txt", "uncommitted\n");
        repo.write_file("staged.txt", "also mine\n");
        repo.git(&["add", "--", "staged.txt"]);
        repo.write_file("untracked.txt", "never recorded\n");
        let (exec, cancel) = env();

        branch::reset(&exec, &repo.path, &root, ResetMode::Hard, &cancel)
            .await
            .expect("hard reset");

        assert_eq!(repo.git(&["rev-parse", "HEAD"]), root);
        assert_eq!(
            std::fs::read_to_string(repo.path.join("a.txt")).unwrap(),
            "one\n",
            "back to the content of the commit it landed on"
        );
        assert_eq!(repo.git(&["status", "--porcelain"]), "?? untracked.txt");
        assert!(
            !repo.path.join("staged.txt").exists(),
            "a staged new file is tracked work: it goes with the rest"
        );
        assert!(repo.path.join("untracked.txt").exists());
    }

    /// `main` and `other` differ in `both.txt` and agree on `same.txt`;
    /// `theirs.txt` exists only on `other`. HEAD is left on `main`.
    fn two_branches() -> TestRepo {
        let mut repo = TestRepo::init();
        repo.write_file("both.txt", "base\n");
        repo.write_file("same.txt", "shared\n");
        repo.git(&["add", "--all"]);
        repo.git(&["commit", "-m", "root"]);
        repo.git(&["switch", "-c", "other"]);
        repo.write_file("both.txt", "theirs\n");
        repo.write_file("theirs.txt", "only over there\n");
        repo.git(&["add", "--all"]);
        repo.git(&["commit", "-m", "other"]);
        repo.git(&["switch", "main"]);
        repo
    }

    async fn move_to_other(repo: &TestRepo) -> CheckoutOutcome {
        let (exec, cancel) = env();
        branch::checkout(
            &exec,
            &repo.path,
            &CheckoutTarget::Branch {
                name: "other".into(),
            },
            &cancel,
        )
        .await
        .expect("switch")
    }

    /// The everyday case: work that is not in the way travels with the move,
    /// and nothing has to be asked (デザイン規約 §未コミット変更がある状態での移動).
    #[tokio::test]
    #[ignore = "what git's switch carries: not worth the pre-merge run"]
    async fn a_move_carries_uncommitted_work_along() {
        let mut repo = two_branches();
        repo.write_file("same.txt", "mine\n");

        assert!(matches!(move_to_other(&repo).await, CheckoutOutcome::Moved));
        assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "other");
        assert_eq!(
            std::fs::read_to_string(repo.path.join("same.txt")).unwrap(),
            "mine\n",
            "the change came along"
        );
    }

    /// Work that *is* in the way stops the move dead — git changes nothing,
    /// which is what makes it safe to go round the long way afterwards.
    #[tokio::test]
    #[ignore = "git's own refusal wording: not worth the pre-merge run"]
    async fn a_move_is_refused_when_the_changes_are_in_the_way() {
        let mut repo = two_branches();
        repo.write_file("both.txt", "mine\n");

        let outcome = move_to_other(&repo).await;
        let CheckoutOutcome::Blocked(refusal) = outcome else {
            panic!("expected a refusal, got {outcome:?}");
        };
        assert!(
            refusal.to_string().contains("would be overwritten"),
            "git's own words came back: {refusal}"
        );
        assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "main");
        assert_eq!(
            std::fs::read_to_string(repo.path.join("both.txt")).unwrap(),
            "mine\n",
            "the refusal left the working tree alone"
        );
    }

    /// Untracked files are refused in words of their own, and the session has
    /// to recognise those too: they are the case a stash gets *most* of the
    /// way past, carrying every tracked change while the untracked file stays
    /// behind in the entry.
    #[tokio::test]
    #[ignore = "git's own refusal wording: not worth the pre-merge run"]
    async fn untracked_files_in_the_way_are_a_refusal_too() {
        let mut repo = two_branches();
        repo.write_file("theirs.txt", "mine, uncommitted\n");

        let outcome = move_to_other(&repo).await;
        let CheckoutOutcome::Blocked(refusal) = outcome else {
            panic!("expected a refusal, got {outcome:?}");
        };
        assert!(
            refusal.to_string().contains("untracked working tree file"),
            "{refusal}"
        );
        assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "main");
    }
}
