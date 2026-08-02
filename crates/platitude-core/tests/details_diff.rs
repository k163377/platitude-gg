//! Commit details and file diffs against real git.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

mod support;

use platitude_core::details::{self, DiffTarget};
use platitude_core::parse::diff::DiffLineKind;
use platitude_core::{GitExecutor, Oid};
use support::TestRepo;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn details_of_a_merge_commit_use_the_first_parent() {
    let mut repo = TestRepo::init();
    repo.commit_file("base.txt", "base\n", "root");
    repo.git(&["checkout", "-b", "side"]);
    repo.commit_file("side.txt", "side\n", "side work");
    repo.git(&["checkout", "main"]);
    repo.commit_file("main.txt", "main\n", "main work");
    repo.git(&[
        "merge",
        "--no-ff",
        "-m",
        "merge side\n\nbody of merge 日本語",
        "side",
    ]);
    let merge_sha = repo.git(&["rev-parse", "HEAD"]);
    let main_sha = repo.git(&["rev-parse", "HEAD^1"]);

    let executor = GitExecutor::new();
    let cancel = CancellationToken::new();
    let oid = Oid::from_hex_str(&merge_sha).unwrap();
    let d = details::commit_details(&executor, &repo.path, &oid, &cancel)
        .await
        .unwrap();

    assert_eq!(d.oid, oid);
    assert_eq!(d.parents.len(), 2);
    assert_eq!(d.parents[0], Oid::from_hex_str(&main_sha).unwrap());
    assert_eq!(d.author_name, "Test User");
    assert_eq!(d.message, "merge side\n\nbody of merge 日本語");
    // First-parent diff of the merge = what the merge brought in: side.txt.
    let paths: Vec<&str> = d.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, vec!["side.txt"]);
    assert_eq!(d.files[0].status, 'A');
}

#[tokio::test]
async fn details_of_the_root_commit_show_created_files() {
    let mut repo = TestRepo::init();
    let root = repo.commit_file("first.txt", "hello\n", "root commit");

    let executor = GitExecutor::new();
    let cancel = CancellationToken::new();
    let oid = Oid::from_hex_str(&root).unwrap();
    let d = details::commit_details(&executor, &repo.path, &oid, &cancel)
        .await
        .unwrap();
    assert!(d.parents.is_empty());
    assert_eq!(d.files.len(), 1);
    assert_eq!(d.files[0].status, 'A');
    assert_eq!(d.files[0].path, "first.txt");
}

#[tokio::test]
async fn details_report_renames_with_scores() {
    let mut repo = TestRepo::init();
    repo.commit_file("before.txt", "stable content here\n", "add file");
    repo.git(&["mv", "before.txt", "after.txt"]);
    repo.git(&["commit", "-m", "rename it"]);
    let sha = repo.git(&["rev-parse", "HEAD"]);

    let executor = GitExecutor::new();
    let cancel = CancellationToken::new();
    let oid = Oid::from_hex_str(&sha).unwrap();
    let d = details::commit_details(&executor, &repo.path, &oid, &cancel)
        .await
        .unwrap();
    assert_eq!(d.files.len(), 1);
    let f = &d.files[0];
    assert_eq!(f.status, 'R');
    assert_eq!(f.path, "after.txt");
    assert_eq!(f.orig_path.as_deref(), Some("before.txt"));
    assert_eq!(f.score, Some(100));
}

#[tokio::test]
async fn commit_file_diff_has_hunks_and_line_numbers() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\ntwo\nthree\n", "add");
    repo.write_file("f.txt", "one\ntwo changed\nthree\n");
    repo.git(&["commit", "-am", "change line two"]);
    let sha = repo.git(&["rev-parse", "HEAD"]);
    let parent = repo.git(&["rev-parse", "HEAD^"]);

    let executor = GitExecutor::new();
    let cancel = CancellationToken::new();
    let target = DiffTarget::Commit {
        oid: Oid::from_hex_str(&sha).unwrap(),
        parent: Some(Oid::from_hex_str(&parent).unwrap()),
        path: "f.txt".to_string(),
        orig_path: None,
    };
    let patches = details::file_diff(&executor, &repo.path, &target, &cancel)
        .await
        .unwrap();
    assert_eq!(patches.len(), 1);
    let patch = &patches[0];
    assert_eq!(patch.path(), "f.txt");
    assert_eq!(patch.hunks.len(), 1);
    let lines = &patch.hunks[0].lines;
    let del = lines
        .iter()
        .find(|l| l.kind == DiffLineKind::Deletion)
        .unwrap();
    assert_eq!(del.text, "two");
    assert_eq!(del.old_no, Some(2));
    let add = lines
        .iter()
        .find(|l| l.kind == DiffLineKind::Addition)
        .unwrap();
    assert_eq!(add.text, "two changed");
    assert_eq!(add.new_no, Some(2));
}

#[tokio::test]
async fn staged_and_unstaged_diffs_are_separate() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "committed\n", "base");
    repo.write_file("f.txt", "staged version\n");
    repo.git(&["add", "--", "f.txt"]);
    repo.write_file("f.txt", "worktree version\n");

    let executor = GitExecutor::new();
    let cancel = CancellationToken::new();

    let staged = details::file_diff(
        &executor,
        &repo.path,
        &DiffTarget::Staged {
            path: "f.txt".to_string(),
            orig_path: None,
        },
        &cancel,
    )
    .await
    .unwrap();
    let staged_adds: Vec<&str> = staged[0].hunks[0]
        .lines
        .iter()
        .filter(|l| l.kind == DiffLineKind::Addition)
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(staged_adds, vec!["staged version"]);

    let unstaged = details::file_diff(
        &executor,
        &repo.path,
        &DiffTarget::Unstaged {
            path: "f.txt".to_string(),
        },
        &cancel,
    )
    .await
    .unwrap();
    let unstaged_adds: Vec<&str> = unstaged[0].hunks[0]
        .lines
        .iter()
        .filter(|l| l.kind == DiffLineKind::Addition)
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(unstaged_adds, vec!["worktree version"]);
}

#[tokio::test]
async fn untracked_file_renders_as_all_additions() {
    let mut repo = TestRepo::init();
    repo.commit_file("base.txt", "x\n", "base");
    repo.write_file("brand new.txt", "line 1\nline 2\n");

    let executor = GitExecutor::new();
    let cancel = CancellationToken::new();
    let patches = details::file_diff(
        &executor,
        &repo.path,
        &DiffTarget::Untracked {
            path: "brand new.txt".to_string(),
        },
        &cancel,
    )
    .await
    .unwrap();
    assert_eq!(patches.len(), 1);
    let lines = &patches[0].hunks[0].lines;
    assert_eq!(lines.len(), 2);
    assert!(lines.iter().all(|l| l.kind == DiffLineKind::Addition));
    assert_eq!(lines[0].text, "line 1");
}

#[tokio::test]
async fn binary_file_diff_is_flagged() {
    let mut repo = TestRepo::init();
    repo.commit_file("t.txt", "x\n", "base");
    std::fs::write(repo.path.join("blob.bin"), [0u8, 159, 146, 150, 0, 1, 2]).unwrap();
    repo.git(&["add", "--", "blob.bin"]);
    repo.git(&["commit", "-m", "add binary"]);
    let sha = repo.git(&["rev-parse", "HEAD"]);
    let parent = repo.git(&["rev-parse", "HEAD^"]);

    let executor = GitExecutor::new();
    let cancel = CancellationToken::new();
    let patches = details::file_diff(
        &executor,
        &repo.path,
        &DiffTarget::Commit {
            oid: Oid::from_hex_str(&sha).unwrap(),
            parent: Some(Oid::from_hex_str(&parent).unwrap()),
            path: "blob.bin".to_string(),
            orig_path: None,
        },
        &cancel,
    )
    .await
    .unwrap();
    assert_eq!(patches.len(), 1);
    assert!(patches[0].is_binary);
    assert!(patches[0].hunks.is_empty());
}
