//! The working tree's own shapes, as real git spells them back. The pre-merge
//! part reads what the pane says of a repository inside the working tree
//! (`details::embedded`); how `status` lists one is git's answer, recorded
//! in [`periodic`].

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::expect_used)]

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::details::{self, DiffTarget, Embedded};
use platitude_core::status;

/// A repository of its own inside `repo`, at `rel`, with a file in it and
/// `commit` deciding whether that file has been recorded there.
fn embedded_repo(repo: &mut TestRepo, rel: &str, commit: bool) {
    let dir = repo.path.join(rel);
    std::fs::create_dir_all(&dir).expect("create the nested repository's directory");
    repo.git_in(&dir, &["init", "-b", "main"]);
    repo.write_file(&format!("{rel}/inside.txt"), "another repository's\n");
    if commit {
        repo.git_in(&dir, &["add", "--", "inside.txt"]);
        repo.git_in(&dir, &["commit", "-m", "the commit a pointer would name"]);
    }
}

/// What the row of a repository inside the working tree (one `dir/`
/// entry, [`periodic`]) shows instead of a patch: the commit a `git add`
/// would point at, since `--no-index` against a directory prints nothing.
#[tokio::test]
async fn a_stage_of_an_embedded_repository_would_point_at_its_head() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "root");
    embedded_repo(&mut repo, "vendor/nest", true);
    let head = repo.git_in(
        &repo.path.join("vendor").join("nest"),
        &["rev-parse", "HEAD"],
    );

    let (executor, cancel) = env();
    let target = DiffTarget::Untracked {
        path: "vendor/nest/".to_string(),
    };
    let raw = details::file_diff_raw(&executor, &repo.path, &target, &cancel)
        .await
        .expect("the diff of a directory");
    assert!(raw.is_empty(), "a directory has no patch: {raw:?}");
    let standing = details::embedded(&executor, &repo.path, "vendor/nest/", &cancel).await;
    assert_eq!(
        standing.map(|e| match e {
            Embedded::On(oid) => oid.to_hex(),
            Embedded::Unborn => "unborn".to_string(),
        }),
        Some(head)
    );
}

/// A repository with no commit has nothing for a pointer to name, and git
/// refuses to stage it at all (`does not have a commit checked out`).
#[tokio::test]
async fn an_embedded_repository_with_no_commit_has_nothing_to_point_at() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "root");
    embedded_repo(&mut repo, "vendor/nest", false);

    let (executor, cancel) = env();
    let standing = details::embedded(&executor, &repo.path, "vendor/nest/", &cancel).await;
    assert_eq!(standing, Some(Embedded::Unborn));
    repo.git_expect_failure(&["add", "--", "vendor/nest"]);
}

/// `rev-parse` walks up, so a directory that is no repository of its own
/// would answer with the HEAD of the repository above it. Nothing is said
/// then.
#[tokio::test]
async fn a_directory_that_is_no_repository_of_its_own_says_nothing() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "root");
    repo.write_file("plain/loose.txt", "in the same repository\n");

    let (executor, cancel) = env();
    assert_eq!(
        details::embedded(&executor, &repo.path, "plain/", &cancel).await,
        None
    );
    // The walk-up is real: the outer HEAD is what it would have said.
    let outer = repo.git(&["rev-parse", "HEAD"]);
    assert_eq!(
        repo.git_in(&repo.path.join("plain"), &["rev-parse", "HEAD"]),
        outer
    );
}

/// Left out of the pre-merge run: how `status` lists a repository inside
/// the working tree, which is git's boundary (`-uall` on a plain directory
/// is held by `stage_integration::partial::stage_part_of_an_untracked_file`).
/// Run by the full gate (`-- --ignored ::periodic::`).
mod periodic {
    use super::*;

    /// `-uall` opens every other new directory into its files; a repository
    /// inside the working tree comes back as the single entry `nest/`,
    /// trailing slash and all.
    ///
    /// That slash decides how `platitude_app::models::pathtree`, which cuts
    /// on `/`, files the row: a directory of its own or a folder with a
    /// nameless row inside it.
    #[tokio::test]
    #[ignore = "git's boundary at a nested repository: not worth the pre-merge run"]
    async fn a_repository_inside_the_working_tree_stays_one_entry() {
        let mut repo = TestRepo::init();
        repo.commit_file("a.txt", "1\n", "root");
        // For contrast: a plain new directory is listed file by file.
        repo.write_file("plain/loose.txt", "in the same repository\n");
        embedded_repo(&mut repo, "vendor/nest", false);

        let (executor, cancel) = env();
        let state = status::load(&executor, &repo.path, &cancel)
            .await
            .expect("status load");
        let untracked: Vec<&str> = state.untracked().map(|i| i.path()).collect();
        assert_eq!(untracked, ["plain/loose.txt", "vendor/nest/"]);
    }
}
