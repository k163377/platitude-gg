//! Renaming a worktree's folder, against real git (`worktree move`): what
//! goes with it, what git keeps, and the report each refusal is — and the
//! worktree a session stands in, which that session stands down for.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use crate::support::TestRepo;
use crate::support::exec::env;
use crate::support::session::{opened_at, write_answer, write_settled};
use platitude_core::report::ReportKind;
use platitude_core::session::SessionEvent;
use platitude_core::worktrees;

/// A repository with one commit and a linked worktree beside it on no
/// branch, as git lists it (`/` throughout).
fn with_loose_worktree(folder: &str) -> (TestRepo, String) {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    let at = repo.path.with_file_name(folder);
    repo.git(&["worktree", "add", "--detach", &at.to_string_lossy()]);
    let listed = listed_path(&mut repo, folder).expect("git lists the new worktree");
    (repo, listed)
}

/// The worktree whose folder is `folder`, as `git worktree list` spells it.
fn listed_path(repo: &mut TestRepo, folder: &str) -> Option<String> {
    repo.git(&["worktree", "list", "--porcelain"])
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .find(|path| path.ends_with(&format!("/{folder}")))
        .map(str::to_string)
}

async fn rename(repo: &TestRepo, from: &str, to: &str) -> Result<(), platitude_core::GitError> {
    let (exec, cancel) = env();
    let name = from.rsplit('/').next().unwrap_or_default();
    worktrees::move_to(&exec, &repo.path, from, to, name, &cancel).await
}

/// git turned the rename down: a report the screen writes the line of, git's
/// words — `says` among them — left to the log.
fn refused_by_git(err: &platitude_core::GitError, says: &str) {
    let report = err.report().expect("a refusal is a report");
    assert_eq!(report.kind, ReportKind::WorktreeNotRenamed);
    assert!(
        report.fact.is_empty(),
        "git's words are the log's: {}",
        report.fact
    );
    assert!(err.to_string().contains(says), "the log keeps them: {err}");
}

/// The folder takes the new name where it stands, and everything in it goes
/// along: the commit it is on, its detached HEAD and what it has not committed.
#[tokio::test]
async fn a_worktree_on_no_branch_takes_its_new_name_where_it_stands() {
    let (mut repo, from) = with_loose_worktree("loose");
    std::fs::write(Path::new(&from).join("a.txt"), "changed\n").unwrap();
    std::fs::write(Path::new(&from).join("draft.txt"), "untracked\n").unwrap();
    let to = worktrees::renamed_worktree_path(&from, "feature-x");

    rename(&repo, &from, &to)
        .await
        .expect("git moves a worktree with work in it");

    assert!(!Path::new(&from).exists(), "the old folder is gone");
    assert_eq!(
        listed_path(&mut repo, "feature-x").as_deref(),
        Some(to.as_str()),
        "git lists it at the new name, in the same parent"
    );
    let moved = PathBuf::from(&to);
    assert_eq!(
        std::fs::read_to_string(moved.join("a.txt")).unwrap(),
        "changed\n"
    );
    assert!(
        moved.join("draft.txt").exists(),
        "untracked work went along"
    );
    let status = repo.git_in(&moved, &["status", "--porcelain=v2", "--branch"]);
    assert!(
        status.contains("# branch.head (detached)"),
        "still on no branch: {status}"
    );
}

/// git moves a worktree *into* a folder already at the new path — an empty
/// one too — so anything there is turned down before git is asked.
#[tokio::test]
async fn anything_at_the_new_path_is_turned_down_before_git() {
    let (mut repo, from) = with_loose_worktree("loose");
    let to = worktrees::renamed_worktree_path(&from, "taken");
    std::fs::create_dir(&to).unwrap();

    let err = rename(&repo, &from, &to)
        .await
        .expect_err("an empty folder is in the way");

    let report = err.report().expect("a refusal is a report");
    assert_eq!(report.kind, ReportKind::WorktreeRenameTaken);
    assert_eq!(report.name, "loose", "the heading names the old folder");
    assert!(report.fact.is_empty(), "the screen writes the reason");
    assert!(
        matches!(err, platitude_core::GitError::Withheld { .. }),
        "git was never asked: {err}"
    );
    assert!(Path::new(&from).exists(), "nothing moved");
    assert!(listed_path(&mut repo, "loose").is_some());
    assert_eq!(std::fs::read_dir(&to).unwrap().count(), 0);
}

/// A name that is not one folder in the same parent is no rename: the box
/// never sends one, and nothing is asked of git if it did.
#[tokio::test]
async fn a_path_out_of_the_parent_is_no_rename() {
    let (repo, from) = with_loose_worktree("loose");
    for to in [
        format!("{from}/inside"),
        worktrees::renamed_worktree_path(&from, ".."),
        repo.path
            .join("elsewhere")
            .to_string_lossy()
            .replace('\\', "/"),
    ] {
        let err = rename(&repo, &from, &to)
            .await
            .expect_err("not a folder beside it");
        assert!(
            err.report().is_none(),
            "a caller's bug, not a report: {err}"
        );
    }
    assert!(Path::new(&from).exists());
}

/// A name Windows keeps for itself goes no further than here there. Asked of
/// git, a device's name moves the folder somewhere only a `\\?\` path reaches
/// and leaves the worktree broken; a kept character git turns down itself.
#[cfg(windows)]
#[tokio::test]
async fn a_name_windows_keeps_is_no_rename_there() {
    let (mut repo, from) = with_loose_worktree("loose");
    for name in ["CON", "con.txt", "why?", "dot."] {
        let err = rename(&repo, &from, &worktrees::renamed_worktree_path(&from, name))
            .await
            .expect_err("Windows keeps it");
        assert!(
            matches!(err, platitude_core::GitError::Rejected { .. }),
            "git was never asked for {name:?}: {err}"
        );
    }
    assert!(Path::new(&from).exists(), "nothing moved");
    assert!(listed_path(&mut repo, "loose").is_some());
}

/// A lock keeps the worktree where it is — one taken after the row was read;
/// the gesture opens no box on a lock it sees.
#[tokio::test]
async fn a_lock_keeps_the_folder() {
    let (mut repo, from) = with_loose_worktree("locked");
    repo.git(&["worktree", "lock", "--reason", "held", &from]);
    let to = worktrees::renamed_worktree_path(&from, "free");

    let err = rename(&repo, &from, &to)
        .await
        .expect_err("git keeps a locked worktree");

    refused_by_git(&err, "cannot move a locked working tree");
    assert!(Path::new(&from).exists());
}

/// The worktree a session stands in: the session stands down and git moves
/// the folder from the repository's own directory. Nothing is read behind
/// it: the tab stands in the new place on a session of its own.
///
/// This pins the road, not the wait: a folder a process stands in renames on
/// Linux, and on Windows only a read still running at the move would turn it
/// down. The wait is the count's (`process::Live`, and `exec.rs`'s witness
/// that a git is counted from its spawn to its reap).
#[tokio::test(flavor = "multi_thread")]
async fn the_worktree_a_session_stands_in_moves_from_the_repositorys_own_directory() {
    let (mut repo, from) = with_loose_worktree("standing");
    let to = worktrees::renamed_worktree_path(&from, "renamed");
    let (sink, session) = opened_at(Path::new(&from)).await;

    let id = session
        .move_worktree(from.clone(), to.clone(), "standing".into())
        .expect("accepted");

    assert_eq!(write_answer(&sink, id).await, None, "git moved it");
    assert_eq!(write_settled(&sink, id).await, [], "and nothing was read");
    assert!(!Path::new(&from).exists());
    assert_eq!(
        listed_path(&mut repo, "renamed").as_deref(),
        Some(to.as_str())
    );
    let moves = sink.count(|event| {
        matches!(event, SessionEvent::CommandStarted { display, .. }
            if display.contains("worktree move"))
    });
    assert_eq!(moves, 1, "one move, run once");
    session.close_and_wait().await;
}

/// Another worktree than the one the session stands in moves as a plain
/// write, and the listing behind it names the new folder.
#[tokio::test(flavor = "multi_thread")]
async fn another_worktree_moves_and_the_listing_names_it_anew() {
    let (mut repo, from) = with_loose_worktree("other");
    let to = worktrees::renamed_worktree_path(&from, "other-renamed");
    let (sink, session) = opened_at(&repo.path).await;

    let id = session
        .move_worktree(from.clone(), to.clone(), "other".into())
        .expect("accepted");

    assert_eq!(write_answer(&sink, id).await, None);
    assert_eq!(write_settled(&sink, id).await, []);
    assert!(listed_path(&mut repo, "other-renamed").is_some());
    let named = sink.count(|event| {
        matches!(event, SessionEvent::WorktreesLoaded { worktrees, .. }
            if worktrees.iter().any(|entry| entry.path == to))
    });
    assert!(named > 0, "a listing names the new folder");
    session.close_and_wait().await;
}

/// What git itself does behind the fixed command lines the tests above
/// pass — left to the full gate (rules-refs/core.md `periodic`).
mod periodic {
    use super::*;

    /// The repository's own worktree is never git's to move.
    #[tokio::test]
    #[ignore = "git's own refusal to move its main worktree: not worth the pre-merge run"]
    async fn the_main_worktree_is_refused() {
        let (mut repo, _loose) = with_loose_worktree("loose");
        let main = repo.git(&["rev-parse", "--show-toplevel"]);
        let to = worktrees::renamed_worktree_path(&main, "elsewhere");

        let err = rename(&repo, &main, &to)
            .await
            .expect_err("git refuses the main worktree");

        refused_by_git(&err, "is a main working tree");
        assert!(repo.path.join("a.txt").exists());
    }

    /// A worktree whose folder is gone has nothing to rename, and a path git
    /// still lists for one is not free.
    #[tokio::test]
    #[ignore = "git's refusals over a folder that is gone and a place it still lists: not worth the pre-merge run"]
    async fn a_gone_folder_and_the_path_still_listed_for_one_are_gits_to_refuse() {
        let (mut repo, from) = with_loose_worktree("here");
        let gone_at = repo.path.with_file_name("gone");
        repo.git(&["worktree", "add", "--detach", &gone_at.to_string_lossy()]);
        let gone = listed_path(&mut repo, "gone").unwrap();
        std::fs::remove_dir_all(&gone_at).unwrap();

        let err = rename(
            &repo,
            &gone,
            &worktrees::renamed_worktree_path(&gone, "back"),
        )
        .await
        .expect_err("nothing there to move");
        refused_by_git(&err, "does not exist");

        let err = rename(&repo, &from, &gone)
            .await
            .expect_err("git still lists a worktree there");
        refused_by_git(&err, "already registered worktree");
        assert!(Path::new(&from).exists());
    }

    /// Submodules checked out in a worktree keep it where it is.
    #[tokio::test]
    #[ignore = "git's refusal to move a worktree with submodules checked out: not worth the pre-merge run"]
    async fn checked_out_submodules_keep_the_folder() {
        let mut sub = TestRepo::init();
        sub.commit_file("s.txt", "s\n", "sub root");
        let mut repo = TestRepo::init();
        repo.commit_file("a.txt", "a\n", "root");
        let url = sub.path.to_string_lossy().replace('\\', "/");
        repo.git(&[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            &url,
            "sub",
        ]);
        repo.git(&["commit", "-m", "add sub"]);
        let at = repo.path.with_file_name("with-sub");
        repo.git(&["worktree", "add", "--detach", &at.to_string_lossy()]);
        repo.git_in(
            &at,
            &[
                "-c",
                "protocol.file.allow=always",
                "submodule",
                "update",
                "--init",
            ],
        );
        let from = listed_path(&mut repo, "with-sub").unwrap();

        let err = rename(
            &repo,
            &from,
            &worktrees::renamed_worktree_path(&from, "moved"),
        )
        .await
        .expect_err("git keeps a worktree with submodules");

        refused_by_git(&err, "submodules");
        assert!(at.exists());
    }
}
