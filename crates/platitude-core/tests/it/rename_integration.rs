//! Renaming the things git has no rename for: tags and stash entries.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::{stash, tag};

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
async fn renaming_a_stash_keeps_its_contents_and_says_so_everywhere() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    repo.write_file("a.txt", "changed\n");
    repo.write_file("new.txt", "untracked\n");
    repo.git(&["stash", "push", "-u", "-m", "half of the work"]);
    let oid = repo.git(&["rev-parse", "stash@{0}"]);
    let (exec, cancel) = env();

    stash::rename(
        &exec,
        &repo.path,
        "stash@{0}",
        &oid,
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
    let oid = repo.git(&["rev-parse", "stash@{1}"]);
    let (exec, cancel) = env();

    stash::rename(
        &exec,
        &repo.path,
        "stash@{1}",
        &oid,
        "renamed first",
        &cancel,
    )
    .await
    .expect("rename");

    let list = repo.git(&["stash", "list", "--format=%gs"]);
    assert_eq!(
        list.lines().collect::<Vec<_>>(),
        vec!["renamed first", "On main: second"],
        "the renamed entry is at the top; the reflog only grows at the front"
    );
}

/// A stash pushed elsewhere between the selection and the press moves
/// every number down: the entry chosen is the one renamed, wherever it
/// stands now, and the one that took its number keeps its name.
#[tokio::test]
async fn a_stash_pushed_since_the_selection_is_not_the_one_renamed() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    repo.write_file("a.txt", "one\n");
    repo.git(&["stash", "push", "-m", "chosen"]);
    let chosen = repo.git(&["rev-parse", "stash@{0}"]);
    repo.write_file("a.txt", "two\n");
    repo.git(&["stash", "push", "-m", "pushed since"]);
    let (exec, cancel) = env();

    stash::rename(&exec, &repo.path, "stash@{0}", &chosen, "renamed", &cancel)
        .await
        .expect("the chosen entry is still there to rename");

    let list = repo.git(&["stash", "list", "--format=%gs"]);
    assert_eq!(
        list.lines().collect::<Vec<_>>(),
        vec!["renamed", "On main: pushed since"]
    );
}

/// An entry above the chosen one dropped elsewhere since: the selection's
/// number is now past the end of the list, and the chosen entry is one up.
#[tokio::test]
async fn a_stash_dropped_above_the_selection_is_renamed_where_it_stands_now() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    repo.write_file("a.txt", "one\n");
    repo.git(&["stash", "push", "-m", "chosen"]);
    repo.write_file("a.txt", "two\n");
    repo.git(&["stash", "push", "-m", "dropped since"]);
    let chosen = repo.git(&["rev-parse", "stash@{1}"]);
    repo.git(&["stash", "drop", "stash@{0}"]);
    let (exec, cancel) = env();

    stash::rename(&exec, &repo.path, "stash@{1}", &chosen, "renamed", &cancel)
        .await
        .expect("the chosen entry is still there to rename");

    assert_eq!(repo.git(&["stash", "list", "--format=%gs"]), "renamed");
}

/// The entry chosen has left the list since (popped or dropped elsewhere):
/// nothing is written, and no other entry is touched.
#[tokio::test]
async fn a_stash_gone_since_the_selection_is_turned_down() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    repo.write_file("a.txt", "one\n");
    repo.git(&["stash", "push", "-m", "kept"]);
    repo.write_file("a.txt", "two\n");
    repo.git(&["stash", "push", "-m", "chosen"]);
    let chosen = repo.git(&["rev-parse", "stash@{0}"]);
    repo.git(&["stash", "drop", "stash@{0}"]);
    let (exec, cancel) = env();

    stash::rename(&exec, &repo.path, "stash@{0}", &chosen, "renamed", &cancel)
        .await
        .expect_err("there is nothing left to rename");

    assert_eq!(
        repo.git(&["stash", "list", "--format=%gs"]),
        "On main: kept"
    );
}

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
