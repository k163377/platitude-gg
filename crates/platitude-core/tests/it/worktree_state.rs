//! status / stash against real git. The committed parser fixtures under
//! tests/fixtures/ are regenerated here (`capture_fixtures`); the checks
//! that parse them live beside each parser as unit tests. Op-state
//! detection is pinned where the operations that produce it live —
//! `integrate_integration` stops a real merge / rebase / cherry-pick and
//! reads the state back.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::status::StatusItem;
use platitude_core::{stash, status};

/// staged add + staged rename + unstaged modify + untracked, on main with
/// two commits of history.
fn dirty_scenario() -> TestRepo {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "line1\nline2\n", "initial");
    repo.commit_file("old name.txt", "rename me\n", "add file to rename");
    repo.write_file("staged.txt", "new\n");
    repo.git(&["add", "--", "staged.txt"]);
    // The rename keeps spaces and unicode in both names.
    repo.git(&["mv", "old name.txt", "новый 名前.txt"]);
    repo.write_file("a.txt", "line1 changed\nline2\n");
    repo.write_file("untracked dir/inner.txt", "u\n");
    repo
}

#[tokio::test]
async fn status_buckets_reflect_the_working_tree() {
    let repo = dirty_scenario();
    let (executor, cancel) = env();

    let s = status::load(&executor, &repo.path, &cancel).await.unwrap();
    assert_eq!(s.branch_head.as_deref(), Some("main"));
    assert!(s.branch_oid.is_some());
    assert!(!s.has_conflicts());

    let staged: Vec<&str> = s.staged().map(StatusItem::path).collect();
    assert!(staged.contains(&"staged.txt"));
    assert!(staged.contains(&"новый 名前.txt"), "renamed target staged");

    let rename = s
        .items
        .iter()
        .find_map(|i| match i {
            StatusItem::Tracked {
                staged: 'R',
                path,
                orig_path: Some(orig),
                ..
            } => Some((path.as_str(), orig.as_str())),
            _ => None,
        })
        .expect("rename entry present");
    assert_eq!(rename, ("новый 名前.txt", "old name.txt"));

    let unstaged: Vec<&str> = s.unstaged().map(StatusItem::path).collect();
    assert_eq!(unstaged, vec!["a.txt"]);

    let untracked: Vec<&str> = s.untracked().map(StatusItem::path).collect();
    assert_eq!(
        untracked,
        vec!["untracked dir/inner.txt"],
        "-uall expands a new directory into its files"
    );
}

#[tokio::test]
async fn stash_list_round_trips() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "base");

    repo.write_file("f.txt", "wip 1\n");
    repo.git(&["stash", "push", "-m", "first stash メッセージ"]);
    repo.write_file("f.txt", "wip 2\n");
    repo.git(&["stash", "push"]);

    let (executor, cancel) = env();
    let stashes = stash::load(&executor, &repo.path, &cancel).await.unwrap();

    assert_eq!(stashes.len(), 2);
    assert_eq!(stashes[0].name, "stash@{0}");
    assert_eq!(stashes[1].name, "stash@{1}");
    assert!(stashes[0].message.starts_with("WIP on main"));
    assert!(stashes[1].message.contains("first stash メッセージ"));
    assert!(stashes.iter().all(|s| s.time > 0));
}

#[tokio::test]
async fn clean_repo_has_empty_stash_and_status() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "base");

    let (executor, cancel) = env();
    let s = status::load(&executor, &repo.path, &cancel).await.unwrap();
    assert!(s.items.is_empty());
    let stashes = stash::load(&executor, &repo.path, &cancel).await.unwrap();
    assert!(stashes.is_empty());
}

// --- captured fixtures ----------------------------------------------------

fn fixture_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Regenerates the committed fixture bytes from real git output. Run with:
/// `cargo test -p platitude-core --test it -- --ignored capture`
/// then review the diff (never hand-edit the .bin files).
#[tokio::test]
#[ignore = "regenerates committed fixtures; run explicitly and review the diff"]
async fn capture_fixtures() {
    let mut repo = dirty_scenario();
    let status_bytes = repo.git_raw(&["status", "--porcelain=v2", "-z", "--branch", "-uall"]);
    std::fs::create_dir_all(fixture_dir()).unwrap();
    std::fs::write(fixture_dir().join("status_v2.bin"), &status_bytes).unwrap();

    let mut stash_repo = TestRepo::init();
    stash_repo.commit_file("f.txt", "base\n", "base");
    stash_repo.write_file("f.txt", "wip 1\n");
    stash_repo.git(&["stash", "push", "-m", "first stash メッセージ"]);
    stash_repo.write_file("f.txt", "wip 2\n");
    stash_repo.git(&["stash", "push"]);
    let stash_bytes = stash_repo.git_raw(&["stash", "list", "-z", stash::STASH_FORMAT_ARG]);
    std::fs::write(fixture_dir().join("stash_list.bin"), &stash_bytes).unwrap();
}
