//! Making another working copy, against real git: the three things one can
//! stand on, and the report each refusal is.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use crate::support::TestRepo;
use crate::support::exec::env;
use crate::support::remote::origin_and_clone;
use platitude_core::report::ReportKind;
use platitude_core::worktrees::{self, CopyOn};

/// Two commits on `main`, and `free` — a branch no copy has out — on the
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

fn listed(repo: &mut TestRepo, copy: &Path) -> bool {
    let leaf = copy.file_name().unwrap().to_string_lossy().into_owned();
    repo.git(&["worktree", "list", "--porcelain"])
        .lines()
        .any(|l| l.starts_with("worktree ") && l.ends_with(&leaf))
}

async fn add(repo: &TestRepo, copy: &Path, on: &CopyOn) -> Result<(), platitude_core::GitError> {
    let (exec, cancel) = env();
    let name = copy.file_name().unwrap().to_string_lossy().into_owned();
    worktrees::add(
        &exec,
        &repo.path,
        &copy.to_string_lossy(),
        on,
        &name,
        &cancel,
    )
    .await
}

/// A branch made for the copy, on the commit asked for — not on HEAD.
#[tokio::test]
async fn a_new_branch_is_made_where_it_was_asked_for() {
    let (mut repo, _tip) = two_commits();
    let root = repo.git(&["rev-parse", "HEAD~1"]);
    let copy = beside(&repo, "fresh");

    add(
        &repo,
        &copy,
        &CopyOn::NewBranch {
            name: "feature/fresh".to_string(),
            start: root.clone(),
        },
    )
    .await
    .expect("git makes the copy");

    assert!(listed(&mut repo, &copy));
    assert_eq!(repo.git(&["rev-parse", "feature/fresh"]), root);
    assert_eq!(
        repo.git_in(&copy, &["symbolic-ref", "--short", "HEAD"]),
        "feature/fresh",
        "the copy has the new branch out"
    );
}

/// An existing branch is checked out as it is: no branch is made.
#[tokio::test]
async fn an_existing_branch_is_checked_out_in_the_new_copy() {
    let (mut repo, _tip) = two_commits();
    let copy = beside(&repo, "freecopy");
    let before = repo.git(&["for-each-ref", "--format=%(refname)", "refs/heads"]);

    add(&repo, &copy, &CopyOn::Branch("free".to_string()))
        .await
        .expect("git makes the copy");

    assert_eq!(
        repo.git_in(&copy, &["symbolic-ref", "--short", "HEAD"]),
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
    let copy = beside(&clone, "topic");

    add(
        &clone,
        &copy,
        &CopyOn::Tracking {
            local: "topic".to_string(),
            remote_ref: "origin/topic".to_string(),
        },
    )
    .await
    .expect("git makes the copy");

    assert_eq!(
        clone.git_in(&copy, &["symbolic-ref", "--short", "HEAD"]),
        "topic"
    );
    assert_eq!(
        clone.git(&["rev-parse", "--symbolic-full-name", "topic@{upstream}"]),
        "refs/remotes/origin/topic"
    );
}

/// The one the menus cannot see coming — another copy took the branch
/// after they opened: a report under the folder's name, in git's words,
/// with its progress line left out.
#[tokio::test]
async fn a_branch_out_in_another_copy_is_a_report_in_gits_words() {
    let (mut repo, _tip) = two_commits();
    let holder = beside(&repo, "holder");
    repo.git(&["worktree", "add", &holder.to_string_lossy(), "free"]);
    let copy = beside(&repo, "again");

    let err = add(&repo, &copy, &CopyOn::Branch("free".to_string()))
        .await
        .expect_err("git keeps a branch to one copy");

    let report = err.report().expect("a refusal is a report");
    assert_eq!(report.kind, ReportKind::WorktreeNotAdded);
    assert_eq!(report.name, "again", "the heading names the folder");
    assert!(
        report
            .reason
            .starts_with("fatal: 'free' is already used by worktree at"),
        "git's own sentence and nothing before it: {}",
        report.reason
    );
    assert!(
        err.to_string().contains("Preparing worktree"),
        "the log keeps all of it: {err}"
    );
    assert!(!copy.exists(), "nothing was made");
}

/// What git does with a folder that holds something: it makes the `-b`
/// branch and only then looks — the reason `worktrees::add` asks first.
#[test]
fn git_makes_the_branch_before_it_turns_the_folder_down() {
    let (mut repo, _tip) = two_commits();
    let copy = beside(&repo, "occupied");
    std::fs::create_dir_all(&copy).unwrap();
    std::fs::write(copy.join("mine.txt"), "mine\n").unwrap();

    repo.git_expect_failure(&[
        "worktree",
        "add",
        "-b",
        "occ",
        "--",
        &copy.to_string_lossy(),
    ]);

    assert!(repo.git_ok(&["rev-parse", "--verify", "--quiet", "refs/heads/occ"]));
}

/// The same for a place git still lists, its folder taken away by hand:
/// the branch is made, then the place turned down. The menus say this one
/// before the press (`NavSectionModel.newCopyFor`'s `listed`).
#[test]
fn git_makes_the_branch_before_it_turns_a_listed_place_down() {
    let (mut repo, _tip) = two_commits();
    let copy = beside(&repo, "gone");
    repo.git(&["worktree", "add", "--detach", "--", &copy.to_string_lossy()]);
    std::fs::remove_dir_all(&copy).unwrap();

    repo.git_expect_failure(&[
        "worktree",
        "add",
        "-b",
        "lst",
        "--",
        &copy.to_string_lossy(),
    ]);

    assert!(repo.git_ok(&["rev-parse", "--verify", "--quiet", "refs/heads/lst"]));
}

/// So a folder that holds something is turned down before git: what is
/// in it stays, and no branch is left behind.
#[tokio::test]
async fn a_folder_holding_something_is_turned_down_before_git() {
    let (mut repo, tip) = two_commits();
    let copy = beside(&repo, "occupied");
    std::fs::create_dir_all(&copy).unwrap();
    std::fs::write(copy.join("mine.txt"), "mine\n").unwrap();

    let err = add(
        &repo,
        &copy,
        &CopyOn::NewBranch {
            name: "occ".to_string(),
            start: tip,
        },
    )
    .await
    .expect_err("nothing goes into a folder that holds something");

    let report = err.report().expect("a refusal is a report");
    assert_eq!(report.kind, ReportKind::WorktreeFolderTaken);
    assert_eq!(report.name, "occupied");
    assert!(report.reason.is_empty(), "the screen writes the reason");
    assert!(
        matches!(err, platitude_core::GitError::Withheld { .. }),
        "git was never asked: {err}"
    );
    assert!(copy.join("mine.txt").exists());
    assert!(
        !repo.git_ok(&["rev-parse", "--verify", "--quiet", "refs/heads/occ"]),
        "no branch was left behind"
    );
}

/// What git does with a `-b` name opening with `-`: its own `git branch`
/// gets the name with nothing to end the options before it, so the name
/// runs as one — `-m` renames the branch out here to the start commit.
#[test]
fn git_hands_a_dash_led_name_to_its_branch_as_an_option() {
    let (mut repo, _tip) = two_commits();
    let root = repo.git(&["rev-parse", "HEAD~1"]);
    let copy = beside(&repo, "dashed");

    repo.git_expect_failure(&[
        "worktree",
        "add",
        "-b",
        "-m",
        "--",
        &copy.to_string_lossy(),
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

/// So such a name never reaches git: the branch out here keeps its name,
/// and nothing is made.
#[tokio::test]
async fn a_dash_led_name_is_turned_down_before_git() {
    let (mut repo, tip) = two_commits();
    let copy = beside(&repo, "dashed");

    for on in [
        CopyOn::NewBranch {
            name: "-m".to_string(),
            start: tip.clone(),
        },
        CopyOn::Tracking {
            local: "-m".to_string(),
            remote_ref: "origin/main".to_string(),
        },
    ] {
        let err = add(&repo, &copy, &on)
            .await
            .expect_err("git would take the name for an option");
        assert!(
            matches!(err, platitude_core::GitError::Rejected { .. }),
            "git was never asked: {err}"
        );
    }

    assert_eq!(repo.git(&["symbolic-ref", "--short", "HEAD"]), "main");
    assert!(!copy.exists());
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
    let copy = beside(&repo, "empty");
    std::fs::create_dir_all(&copy).unwrap();

    add(
        &repo,
        &copy,
        &CopyOn::NewBranch {
            name: "emp".to_string(),
            start: tip,
        },
    )
    .await
    .expect("git takes an empty folder");

    assert!(listed(&mut repo, &copy));
}

/// git's advice to the terminal is not quoted.
#[tokio::test]
async fn a_hint_is_not_quoted() {
    let (repo, tip) = two_commits();
    let copy = beside(&repo, "bad");

    let err = add(
        &repo,
        &copy,
        &CopyOn::NewBranch {
            name: "a..b".to_string(),
            start: tip,
        },
    )
    .await
    .expect_err("git will not take the name");

    let report = err.report().expect("a refusal is a report");
    assert_eq!(report.reason, "fatal: 'a..b' is not a valid branch name");
}
