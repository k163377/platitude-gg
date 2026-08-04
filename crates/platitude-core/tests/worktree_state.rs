//! status / op-state / stash against real git, plus captured-fixture
//! parser checks (raw bytes committed under tests/fixtures/).

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

mod support;

use platitude_core::status::StatusItem;
use platitude_core::{GitExecutor, opstate, stash, status};
use support::TestRepo;
use tokio_util::sync::CancellationToken;

/// staged add + staged rename + unstaged modify + untracked, on main with
/// one commit of history.
fn dirty_scenario() -> TestRepo {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "line1\nline2\n", "initial");
    repo.commit_file("old name.txt", "rename me\n", "add file to rename");
    // Staged new file.
    repo.write_file("staged.txt", "new\n");
    repo.git(&["add", "--", "staged.txt"]);
    // Staged rename (with spaces and unicode in names).
    repo.git(&["mv", "old name.txt", "новый 名前.txt"]);
    // Unstaged modification.
    repo.write_file("a.txt", "line1 changed\nline2\n");
    // Untracked.
    repo.write_file("untracked dir/inner.txt", "u\n");
    repo
}

#[tokio::test]
async fn status_buckets_reflect_the_working_tree() {
    let repo = dirty_scenario();
    let executor = GitExecutor::new();
    let cancel = CancellationToken::new();

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
async fn conflict_and_merge_state_are_detected() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "base");
    repo.git(&["checkout", "-b", "side"]);
    repo.commit_file("f.txt", "side change\n", "side edit");
    repo.git(&["checkout", "main"]);
    repo.commit_file("f.txt", "main change\n", "main edit");

    let executor = GitExecutor::new();
    let cancel = CancellationToken::new();

    let before = opstate::detect(&executor, &repo.path, &cancel)
        .await
        .unwrap();
    assert!(!before.any());

    repo.git_expect_failure(&["merge", "side"]);

    let s = status::load(&executor, &repo.path, &cancel).await.unwrap();
    assert!(s.has_conflicts());
    let conflicted: Vec<&str> = s.conflicted().map(StatusItem::path).collect();
    assert_eq!(conflicted, vec!["f.txt"]);

    let during = opstate::detect(&executor, &repo.path, &cancel)
        .await
        .unwrap();
    assert!(during.merging);
    assert!(!during.rebasing);

    repo.git(&["merge", "--abort"]);
    let after = opstate::detect(&executor, &repo.path, &cancel)
        .await
        .unwrap();
    assert!(!after.any());
}

#[tokio::test]
async fn rebase_state_is_detected() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "base");
    repo.git(&["checkout", "-b", "topic"]);
    repo.commit_file("f.txt", "topic change\n", "topic edit");
    repo.git(&["checkout", "main"]);
    repo.commit_file("f.txt", "main change\n", "main edit");
    repo.git(&["checkout", "topic"]);

    repo.git_expect_failure(&["rebase", "main"]);

    let executor = GitExecutor::new();
    let cancel = CancellationToken::new();
    let state = opstate::detect(&executor, &repo.path, &cancel)
        .await
        .unwrap();
    assert!(state.rebasing);

    repo.git(&["rebase", "--abort"]);
    let after = opstate::detect(&executor, &repo.path, &cancel)
        .await
        .unwrap();
    assert!(!after.any());
}

#[tokio::test]
async fn cherry_pick_state_is_detected() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "base");
    repo.git(&["checkout", "-b", "donor"]);
    repo.commit_file("f.txt", "donor change\n", "donor edit");
    let donor = repo.git(&["rev-parse", "HEAD"]);
    repo.git(&["checkout", "main"]);
    repo.commit_file("f.txt", "main change\n", "main edit");

    repo.git_expect_failure(&["cherry-pick", &donor]);

    let executor = GitExecutor::new();
    let cancel = CancellationToken::new();
    let state = opstate::detect(&executor, &repo.path, &cancel)
        .await
        .unwrap();
    assert!(state.cherry_picking);

    repo.git(&["cherry-pick", "--abort"]);
}

#[tokio::test]
async fn stash_list_round_trips() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "base");

    repo.write_file("f.txt", "wip 1\n");
    repo.git(&["stash", "push", "-m", "first stash メッセージ"]);
    repo.write_file("f.txt", "wip 2\n");
    repo.git(&["stash", "push"]);

    let executor = GitExecutor::new();
    let cancel = CancellationToken::new();
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

    let executor = GitExecutor::new();
    let cancel = CancellationToken::new();
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
/// `cargo test -p platitude-core --test worktree_state -- --ignored capture`
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

fn read_fixture(name: &str) -> Vec<u8> {
    let path = fixture_dir().join(name);
    std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "missing fixture {} ({e}); regenerate with: cargo test -p platitude-core \
             --test worktree_state -- --ignored capture",
            path.display()
        )
    })
}

#[test]
fn committed_status_fixture_parses() {
    let bytes = read_fixture("status_v2.bin");
    let s = status::parse_status(&bytes).unwrap();
    assert_eq!(s.branch_head.as_deref(), Some("main"));
    assert!(s.staged().any(|i| i.path() == "staged.txt"));
    assert!(s.unstaged().any(|i| i.path() == "a.txt"));
    assert!(s.untracked().any(|i| i.path() == "untracked dir/inner.txt"));
    assert!(
        s.items.iter().any(|i| matches!(
            i,
            StatusItem::Tracked { staged: 'R', orig_path: Some(o), .. } if o == "old name.txt"
        )),
        "rename with unicode target present"
    );
}

#[test]
fn committed_stash_fixture_parses() {
    let bytes = read_fixture("stash_list.bin");
    let stashes = stash::parse_stashes(&bytes).unwrap();
    assert_eq!(stashes.len(), 2);
    assert_eq!(stashes[0].name, "stash@{0}");
    assert!(stashes[1].message.contains("first stash メッセージ"));
}
