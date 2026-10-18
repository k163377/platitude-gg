//! Making another worktree, against real git: the three things one can
//! stand on, and the report each refusal is.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use crate::support::TestRepo;
use crate::support::exec::env;
use crate::support::remote::origin_and_clone;
use platitude_core::report::ReportKind;
use platitude_core::worktrees::{self, WorktreeOn};

/// Two commits on `main`, and `free` — a branch no worktree has out — on the
/// first. Answers the second commit's id.
fn two_commits() -> (TestRepo, String) {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    repo.git(&["branch", "free"]);
    let tip = repo.commit_file_id("b.txt", "b\n", "second");
    (repo, tip)
}

fn beside(repo: &TestRepo, leaf: &str) -> PathBuf {
    repo.path.with_file_name(leaf)
}

fn listed(repo: &mut TestRepo, worktree: &Path) -> bool {
    let leaf = worktree.file_name().unwrap().to_string_lossy().into_owned();
    repo.git(&["worktree", "list", "--porcelain"])
        .lines()
        .any(|l| l.starts_with("worktree ") && l.ends_with(&leaf))
}

async fn add(
    repo: &TestRepo,
    worktree: &Path,
    on: &WorktreeOn,
) -> Result<(), platitude_core::GitError> {
    let (exec, cancel) = env();
    let name = worktree.file_name().unwrap().to_string_lossy().into_owned();
    worktrees::add(
        &exec,
        &repo.path,
        &worktree.to_string_lossy(),
        on,
        &name,
        &cancel,
    )
    .await
}

/// A branch made for the worktree, on the commit asked for — not on HEAD.
#[tokio::test]
async fn a_new_branch_is_made_where_it_was_asked_for() {
    let (mut repo, _tip) = two_commits();
    let root = repo.git(&["rev-parse", "HEAD~1"]);
    let worktree = beside(&repo, "fresh");

    add(
        &repo,
        &worktree,
        &WorktreeOn::NewBranch {
            name: "feature/fresh".to_string(),
            start: root.clone(),
        },
    )
    .await
    .expect("git makes the worktree");

    assert!(listed(&mut repo, &worktree));
    assert_eq!(repo.git(&["rev-parse", "feature/fresh"]), root);
    assert_eq!(
        repo.git_in(&worktree, &["symbolic-ref", "--short", "HEAD"]),
        "feature/fresh",
        "the worktree has the new branch out"
    );
}

/// An existing branch is checked out as it is: no branch is made.
#[tokio::test]
async fn an_existing_branch_is_checked_out_in_the_new_worktree() {
    let (mut repo, _tip) = two_commits();
    let worktree = beside(&repo, "freeworktree");
    let before = repo.git(&["for-each-ref", "--format=%(refname)", "refs/heads"]);

    add(&repo, &worktree, &WorktreeOn::Branch("free".to_string()))
        .await
        .expect("git makes the worktree");

    assert_eq!(
        repo.git_in(&worktree, &["symbolic-ref", "--short", "HEAD"]),
        "free"
    );
    assert_eq!(
        repo.git(&["for-each-ref", "--format=%(refname)", "refs/heads"]),
        before,
        "no branch was made"
    );
}

/// A remote branch with no local one: a local one is made off it and set
/// to follow it, as `switch` does.
#[tokio::test]
async fn a_remote_branch_gets_a_local_one_that_follows_it() {
    let (_origin, mut clone) = origin_and_clone();
    clone.git(&["push", "origin", "main:topic"]);
    clone.git(&["fetch", "origin"]);
    // A local branch spelled like the remote one makes `origin/topic`
    // ambiguous to git; the full name is not.
    clone.git(&["branch", "origin/topic"]);
    let worktree = beside(&clone, "topic");

    add(
        &clone,
        &worktree,
        &WorktreeOn::Tracking {
            local: "topic".to_string(),
            remote_ref: "origin/topic".to_string(),
        },
    )
    .await
    .expect("git makes the worktree");

    assert_eq!(
        clone.git_in(&worktree, &["symbolic-ref", "--short", "HEAD"]),
        "topic"
    );
    assert_eq!(
        clone.git(&["rev-parse", "--symbolic-full-name", "topic@{upstream}"]),
        "refs/remotes/origin/topic"
    );
}

/// The one the menus cannot see coming — another worktree took the branch
/// after they opened: a report under the folder's name, git's words the
/// log's, and nothing of git's ahead of them (`--quiet`).
#[tokio::test]
async fn a_branch_out_in_another_worktree_is_a_report() {
    let (mut repo, _tip) = two_commits();
    let holder = beside(&repo, "holder");
    repo.git(&["worktree", "add", &holder.to_string_lossy(), "free"]);
    let worktree = beside(&repo, "again");

    let err = add(&repo, &worktree, &WorktreeOn::Branch("free".to_string()))
        .await
        .expect_err("git keeps a branch to one worktree");

    let report = err.report().expect("a refusal is a report");
    assert_eq!(report.kind, ReportKind::WorktreeNotAdded);
    assert_eq!(report.name, "again", "the heading names the folder");
    assert!(report.fact.is_empty(), "the screen says it: {report:?}");
    assert!(
        err.to_string()
            .contains("fatal: 'free' is already used by worktree at"),
        "git's own sentence is the log's: {err}"
    );
    assert!(
        !err.to_string().contains("Preparing worktree"),
        "no progress line in what the log keeps: {err}"
    );
    assert!(!worktree.exists(), "nothing was made");
}

/// A place git still lists for a worktree whose folder went: git makes the
/// `-b` branch first and turns the place down after — the report says the
/// place, or the branch it left where it made one.
#[tokio::test]
async fn a_place_still_listed_is_a_report_and_says_what_it_left() {
    let (mut repo, tip) = two_commits();
    let worktree = beside(&repo, "gone");
    repo.git(&[
        "worktree",
        "add",
        "-b",
        "elsewhere",
        &worktree.to_string_lossy(),
    ]);
    std::fs::remove_dir_all(&worktree).expect("the folder goes, the record stays");

    let err = add(&repo, &worktree, &WorktreeOn::Branch("free".to_string()))
        .await
        .expect_err("git keeps the place for the listed one");
    let report = err.report().expect("a refusal is a report");
    assert_eq!(report.kind, ReportKind::WorktreePlaceTaken, "{err}");

    let err = add(
        &repo,
        &worktree,
        &WorktreeOn::NewBranch {
            name: "topic".to_string(),
            start: tip,
        },
    )
    .await
    .expect_err("the same place, with a branch made for it");
    let report = err.report().expect("a refusal is a report");
    assert_eq!(report.kind, ReportKind::WorktreeBranchLeft, "{err}");
    assert_eq!(report.fact, "topic", "the branch git made before saying no");
    assert!(repo.git_ok(&["rev-parse", "--verify", "--quiet", "refs/heads/topic"]));
}

/// A `post-checkout` hook that fails leaves the worktree made — git keeps
/// it — so the report says that, not that nothing was made.
#[tokio::test]
async fn a_post_checkout_hook_that_fails_leaves_the_worktree_made() {
    let (mut repo, _tip) = two_commits();
    repo.write_hook(
        "post-checkout",
        "echo \"git-lfs was not found\" >&2\nexit 2\n",
    );
    let worktree = beside(&repo, "hooked");

    let err = add(&repo, &worktree, &WorktreeOn::Branch("free".to_string()))
        .await
        .expect_err("the hook's failure is git's answer");
    let report = err.report().expect("a report");
    assert_eq!(report.kind, ReportKind::WorktreeHookFailed, "{err}");
    assert_eq!(report.name, "hooked");
    assert!(listed(&mut repo, &worktree), "git kept the worktree");
    assert!(
        err.to_string().contains("git-lfs was not found"),
        "the hook's words are the log's: {err}"
    );
}

/// A checkout past two seconds has git's child `reset` write its progress
/// to the pipe as well, ahead of whatever fails after it; `--quiet` reaches
/// that `reset`. The delay is taken to zero, and a failing `post-checkout`
/// hook is the failure after the checkout.
#[tokio::test]
async fn a_slow_checkout_says_nothing_before_a_refusal() {
    let (repo, _tip) = two_commits();
    repo.write_hook("post-checkout", "echo \"the hook says no\" >&2\nexit 1\n");
    let worktree = beside(&repo, "slow");
    let exec = crate::support::exec::progress_at_once();
    let cancel = tokio_util::sync::CancellationToken::new();

    let err = worktrees::add(
        &exec,
        &repo.path,
        &worktree.to_string_lossy(),
        &WorktreeOn::Branch("free".to_string()),
        "slow",
        &cancel,
    )
    .await
    .expect_err("the hook fails the add");

    let said = err.to_string();
    assert!(said.contains("the hook says no"), "{said}");
    assert!(
        !said.contains("Updating files"),
        "the checkout's progress stays out of it: {said}"
    );
}

/// A worktree stopped part-way through a rebase, or in a bisect, is
/// listed `detached`, yet git refuses its branch to any other worktree.
/// The listing names the branch it holds, so the menus can say so first.
#[tokio::test]
async fn a_branch_a_stopped_rebase_or_bisect_holds_is_named_by_the_listing() {
    let (mut repo, _tip) = two_commits();
    repo.git(&["branch", "rebasing"]);
    repo.git(&["branch", "bisecting"]);
    let rebasing = beside(&repo, "rebasing");
    let bisecting = beside(&repo, "bisecting");
    repo.git(&["worktree", "add", &rebasing.to_string_lossy(), "rebasing"]);
    repo.git(&["worktree", "add", &bisecting.to_string_lossy(), "bisecting"]);
    let rebased = repo.git_ok(&[
        "-C",
        &rebasing.to_string_lossy(),
        "rebase",
        "--exec",
        "exit 1",
        "HEAD~1",
    ]);
    assert!(!rebased, "the exec stops the rebase");
    // Two more commits, so the bisect has a middle to check out: one with
    // nothing between good and bad leaves the branch out as it was.
    repo.git_in(&bisecting, &["commit", "--allow-empty", "-m", "third"]);
    repo.git_in(&bisecting, &["commit", "--allow-empty", "-m", "fourth"]);
    repo.git_in(&bisecting, &["bisect", "start", "HEAD", "HEAD~3"]);
    let (exec, cancel) = env();

    let listed = worktrees::load(&exec, &repo.path, &cancel)
        .await
        .expect("a listing");
    let held = |leaf: &str| {
        listed
            .iter()
            .find(|entry| entry.path.ends_with(leaf))
            .map(|entry| (entry.detached, entry.held.clone()))
    };
    let by = |branch: &str, by| worktrees::Held {
        branch: branch.to_string(),
        by,
    };
    assert_eq!(
        held("rebasing"),
        Some((true, Some(by("rebasing", worktrees::HeldBy::Rebase))))
    );
    assert_eq!(
        held("bisecting"),
        Some((true, Some(by("bisecting", worktrees::HeldBy::Bisect))))
    );
    assert_eq!(
        listed
            .iter()
            .find(|entry| entry.main)
            .map(|entry| entry.held.clone()),
        Some(None),
        "a worktree on its branch holds nothing more"
    );
    for branch in ["rebasing", "bisecting"] {
        assert!(
            !repo.git_ok(&["switch", branch]),
            "git keeps {branch} to the worktree it stopped in"
        );
        // A rename of it is turned down from any worktree, the one it stopped
        // in too — the name box asks first (`NavSectionModel.heldByOperation`).
        assert!(
            !repo.git_ok(&["branch", "-m", branch, "renamed"]),
            "git renames no {branch} here"
        );
    }
    assert!(
        !repo.git_ok(&[
            "-C",
            &rebasing.to_string_lossy(),
            "branch",
            "-m",
            "rebasing",
            "renamed"
        ]),
        "nor in the worktree it stopped in"
    );
    // A branch checked out elsewhere, nothing standing, is renamed: git moves
    // that worktree's HEAD with it.
    let plain = beside(&repo, "plain");
    repo.git(&["worktree", "add", "-b", "plain", &plain.to_string_lossy()]);
    assert!(repo.git_ok(&["branch", "-m", "plain", "plain-renamed"]));
}

/// So a folder that holds something is turned down before git: what is
/// in it stays, and no branch is left behind.
#[tokio::test]
async fn a_folder_holding_something_is_turned_down_before_git() {
    let (mut repo, tip) = two_commits();
    let worktree = beside(&repo, "occupied");
    std::fs::create_dir_all(&worktree).unwrap();
    std::fs::write(worktree.join("mine.txt"), "mine\n").unwrap();

    let err = add(
        &repo,
        &worktree,
        &WorktreeOn::NewBranch {
            name: "occ".to_string(),
            start: tip,
        },
    )
    .await
    .expect_err("nothing goes into a folder that holds something");

    let report = err.report().expect("a refusal is a report");
    assert_eq!(report.kind, ReportKind::WorktreeFolderTaken);
    assert_eq!(report.name, "occupied");
    assert!(report.fact.is_empty(), "the screen writes the reason");
    assert!(
        matches!(err, platitude_core::GitError::Withheld { .. }),
        "git was never asked: {err}"
    );
    assert!(worktree.join("mine.txt").exists());
    assert!(
        !repo.git_ok(&["rev-parse", "--verify", "--quiet", "refs/heads/occ"]),
        "no branch was left behind"
    );
}

/// So such a name never reaches git: the branch out here keeps its name,
/// and nothing is made.
#[tokio::test]
async fn a_dash_led_name_is_turned_down_before_git() {
    let (mut repo, tip) = two_commits();
    let worktree = beside(&repo, "dashed");

    for on in [
        WorktreeOn::NewBranch {
            name: "-m".to_string(),
            start: tip.clone(),
        },
        WorktreeOn::Tracking {
            local: "-m".to_string(),
            remote_ref: "origin/main".to_string(),
        },
    ] {
        let err = add(&repo, &worktree, &on)
            .await
            .expect_err("git would take the name for an option");
        assert!(
            matches!(err, platitude_core::GitError::Rejected { .. }),
            "git was never asked: {err}"
        );
    }

    assert_eq!(repo.git(&["symbolic-ref", "--short", "HEAD"]), "main");
    assert!(!worktree.exists());
}

/// The branch name rules are git's own for a branch: a tag's, less a name
/// opening with `-` or one that is `HEAD`.
#[test]
fn the_branch_name_rules_are_the_ones_git_applies() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    for name in [
        "feature/login",
        "a-b",
        "feature/-inner",
        "-dash",
        "-m",
        "HEAD",
        "a..b",
        "with space",
    ] {
        let git_says = repo.git_ok(&["check-ref-format", "--branch", name]);
        assert_eq!(
            platitude_core::branch::is_valid_name(name),
            git_says,
            "disagreed about {name:?} (git says {git_says})"
        );
    }
}

/// An empty folder is git's to fill, as it would be from a terminal.
#[tokio::test]
async fn an_empty_folder_is_filled() {
    let (mut repo, tip) = two_commits();
    let worktree = beside(&repo, "empty");
    std::fs::create_dir_all(&worktree).unwrap();

    add(
        &repo,
        &worktree,
        &WorktreeOn::NewBranch {
            name: "emp".to_string(),
            start: tip,
        },
    )
    .await
    .expect("git takes an empty folder");

    assert!(listed(&mut repo, &worktree));
}

/// A `-b` name git's own `git branch` turns down ends the add with 255
/// before anything is made: no branch left to speak of.
#[tokio::test]
async fn a_branch_git_would_not_make_leaves_nothing_behind() {
    let (mut repo, tip) = two_commits();
    let worktree = beside(&repo, "bad");

    let err = add(
        &repo,
        &worktree,
        &WorktreeOn::NewBranch {
            name: "a..b".to_string(),
            start: tip,
        },
    )
    .await
    .expect_err("git will not take the name");

    let report = err.report().expect("a refusal is a report");
    assert_eq!(report.kind, ReportKind::WorktreeNotAdded, "{err}");
    assert!(report.fact.is_empty());
    assert!(!listed(&mut repo, &worktree));
}

/// A `-b` name taken since the menu opened: git's `git branch` turns it
/// down (255), and the branch there is somebody else's, not one git left
/// behind — the one case where the branch exists after the refusal.
#[tokio::test]
async fn a_name_taken_since_is_not_a_branch_left_behind() {
    let (mut repo, tip) = two_commits();
    let worktree = beside(&repo, "taken");

    let err = add(
        &repo,
        &worktree,
        &WorktreeOn::NewBranch {
            name: "free".to_string(),
            start: tip,
        },
    )
    .await
    .expect_err("the name is taken");

    let report = err.report().expect("a refusal is a report");
    assert_eq!(report.kind, ReportKind::WorktreeNotAdded, "{err}");
    assert!(
        report.fact.is_empty(),
        "nothing of git's own was left: {report:?}"
    );
    assert!(!listed(&mut repo, &worktree));
}

/// What git itself does behind the fixed command lines the tests above
/// pass — left to the full gate (rules-refs/core.md `periodic`).
mod periodic {
    use super::*;

    /// What git does with a folder that holds something: it makes the `-b`
    /// branch and only then looks — the reason `worktrees::add` asks first.
    #[test]
    #[ignore = "git making the -b branch before it looks at the folder: not worth the pre-merge run"]
    fn git_makes_the_branch_before_it_turns_the_folder_down() {
        let (mut repo, _tip) = two_commits();
        let worktree = beside(&repo, "occupied");
        std::fs::create_dir_all(&worktree).unwrap();
        std::fs::write(worktree.join("mine.txt"), "mine\n").unwrap();

        repo.git_expect_failure(&[
            "worktree",
            "add",
            "-b",
            "occ",
            "--",
            &worktree.to_string_lossy(),
        ]);

        assert!(repo.git_ok(&["rev-parse", "--verify", "--quiet", "refs/heads/occ"]));
    }

    /// The same for a place git still lists, its folder taken away by hand:
    /// the branch is made, then the place turned down. The menus say this one
    /// before the press (`NavSectionModel.newWorktreeFor`'s `listed`).
    #[test]
    #[ignore = "git making the -b branch before it turns a listed place down: not worth the pre-merge run"]
    fn git_makes_the_branch_before_it_turns_a_listed_place_down() {
        let (mut repo, _tip) = two_commits();
        let worktree = beside(&repo, "gone");
        repo.git(&[
            "worktree",
            "add",
            "--detach",
            "--",
            &worktree.to_string_lossy(),
        ]);
        std::fs::remove_dir_all(&worktree).unwrap();

        repo.git_expect_failure(&[
            "worktree",
            "add",
            "-b",
            "lst",
            "--",
            &worktree.to_string_lossy(),
        ]);

        assert!(repo.git_ok(&["rev-parse", "--verify", "--quiet", "refs/heads/lst"]));
    }

    /// What git does with a `-b` name opening with `-`: its own `git branch`
    /// gets the name with nothing to end the options before it, so the name
    /// runs as one — `-m` renames the branch out here to the start commit.
    #[test]
    #[ignore = "git's own branch taking a dash-led -b name as an option: not worth the pre-merge run"]
    fn git_hands_a_dash_led_name_to_its_branch_as_an_option() {
        let (mut repo, _tip) = two_commits();
        let root = repo.git(&["rev-parse", "HEAD~1"]);
        let worktree = beside(&repo, "dashed");

        repo.git_expect_failure(&[
            "worktree",
            "add",
            "-b",
            "-m",
            "--",
            &worktree.to_string_lossy(),
            &root,
        ]);

        assert!(!repo.git_ok(&["rev-parse", "--verify", "--quiet", "refs/heads/main"]));
        assert!(repo.git_ok(&[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/heads/{root}")
        ]));
    }
}
