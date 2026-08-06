//! Renaming the things git has no rename for: tags and stash entries.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

mod support;

use platitude_core::process::GitExecutor;
use platitude_core::{stash, tag};
use support::TestRepo;
use tokio_util::sync::CancellationToken;

fn env() -> (GitExecutor, CancellationToken) {
    (GitExecutor::new(), CancellationToken::new())
}

#[tokio::test]
async fn renaming_a_tag_keeps_the_object_it_names() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    repo.git(&["tag", "-a", "v1.0.0", "-m", "first release"]);
    let object = repo.git(&["rev-parse", "v1.0.0"]);
    let (exec, cancel) = env();

    tag::rename(&exec, &repo.path, "v1.0.0", "v1.0.1", &cancel)
        .await
        .expect("rename");

    let tags = repo.git(&["tag", "--list"]);
    assert_eq!(tags, "v1.0.1", "the old name is gone");
    assert_eq!(
        repo.git(&["rev-parse", "v1.0.1"]),
        object,
        "the same tag object, not a new one"
    );
    assert_eq!(
        repo.git(&["cat-file", "-t", "v1.0.1"]),
        "tag",
        "an annotated tag stays annotated"
    );
    assert_eq!(
        repo.git(&["tag", "-l", "--format=%(contents:subject)", "v1.0.1"]),
        "first release",
        "and keeps its message"
    );
}

#[tokio::test]
async fn a_lightweight_tag_renames_too() {
    let mut repo = TestRepo::init();
    let head = repo.commit_file("a.txt", "a\n", "root");
    repo.git(&["tag", "nightly"]);
    let (exec, cancel) = env();

    tag::rename(&exec, &repo.path, "nightly", "nightly-old", &cancel)
        .await
        .expect("rename");

    assert_eq!(repo.git(&["tag", "--list"]), "nightly-old");
    assert_eq!(repo.git(&["rev-parse", "nightly-old"]), head);
}

#[tokio::test]
async fn renaming_onto_a_name_in_use_changes_nothing() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    repo.git(&["tag", "v1"]);
    repo.git(&["tag", "v2"]);
    let (exec, cancel) = env();

    let err = tag::rename(&exec, &repo.path, "v1", "v2", &cancel)
        .await
        .expect_err("git refuses an existing tag name");
    assert!(
        format!("{err}").contains("already exists"),
        "git's own words: {err}"
    );
    let tags = repo.git(&["tag", "--list"]);
    assert!(
        tags.contains("v1") && tags.contains("v2"),
        "both stay: {tags}"
    );
}

#[tokio::test]
async fn renaming_a_stash_keeps_its_contents_and_says_so_everywhere() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    repo.write_file("a.txt", "changed\n");
    repo.write_file("new.txt", "untracked\n");
    repo.git(&["stash", "push", "-u", "-m", "half of the work"]);
    let (exec, cancel) = env();

    stash::rename(
        &exec,
        &repo.path,
        "stash@{0}",
        "the login refactor",
        &cancel,
    )
    .await
    .expect("rename");

    let list = repo.git(&["stash", "list", "--format=%gd %gs"]);
    assert_eq!(list, "stash@{0} the login refactor", "one entry, renamed");
    let selector = "stash@{0}";
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s", selector]),
        "the login refactor",
        "the commit says the same as the list"
    );

    // The work is still there, untracked file included.
    repo.git(&["stash", "pop"]);
    let status = repo.git(&["status", "--porcelain"]);
    assert!(status.contains("a.txt"), "the change came back: {status}");
    assert!(status.contains("?? new.txt"), "untracked too: {status}");
    assert_eq!(
        std::fs::read_to_string(repo.path.join("a.txt")).expect("read back"),
        "changed\n"
    );
}

#[tokio::test]
async fn renaming_a_stash_leaves_the_others_where_they_were() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    repo.write_file("a.txt", "one\n");
    repo.git(&["stash", "push", "-m", "first"]);
    repo.write_file("a.txt", "two\n");
    repo.git(&["stash", "push", "-m", "second"]);
    let (exec, cancel) = env();

    // The older of the two (`stash@{1}`) is the one renamed.
    stash::rename(&exec, &repo.path, "stash@{1}", "renamed first", &cancel)
        .await
        .expect("rename");

    let list = repo.git(&["stash", "list", "--format=%gs"]);
    assert_eq!(
        list.lines().collect::<Vec<_>>(),
        vec!["renamed first", "On main: second"],
        "the renamed entry is at the top; the reflog only grows at the front"
    );
}

/// The rules the rename box enforces are git's own — held to them by
/// asking git itself about every shape the box has to judge.
#[tokio::test]
async fn the_name_rules_are_the_ones_git_applies() {
    let mut repo = TestRepo::init();
    let names = [
        "v1.0.0",
        "feature/login",
        "release/2026-08",
        "a",
        "with.dots",
        "under_score",
        "日本語",
        "a-b",
        "",
        "@",
        "with space",
        "tilde~1",
        "caret^",
        "colon:name",
        "question?",
        "star*",
        "bracket[",
        "back\\slash",
        "a..b",
        "at@{0}",
        "/leading",
        "trailing/",
        "double//slash",
        "ends.",
        ".hidden",
        "dir/.hidden",
        "name.lock",
        "dir/name.lock",
        "dir//",
        "a.lock/b",
        "-dash",
    ];
    for name in names {
        let git_says = repo.git_ok(&["check-ref-format", &format!("refs/heads/{name}")]);
        assert_eq!(
            tag::is_valid_name(name),
            git_says,
            "disagreed about {name:?} (git says {git_says})"
        );
    }
}

/// A rename that changes only letter case is refused before anything
/// runs. With the tag packed (the normal state after a clone), the
/// create+delete pair deletes BOTH names on a case-insensitive disk and
/// every command exits 0 (実測 2026-08-07 on NTFS).
#[tokio::test]
async fn a_case_only_tag_rename_is_refused_before_touching_anything() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    repo.git(&["tag", "v1.0"]);
    repo.git(&["pack-refs", "--all"]);
    let (exec, cancel) = env();

    let err = tag::rename(&exec, &repo.path, "v1.0", "V1.0", &cancel)
        .await
        .expect_err("case-only renames are refused");
    assert!(format!("{err}").contains("letter case"), "{err}");
    assert_eq!(repo.git(&["tag", "--list"]), "v1.0", "nothing was lost");
}
