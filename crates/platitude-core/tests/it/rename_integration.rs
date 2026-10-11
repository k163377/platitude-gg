//! Renaming the things git has no rename for: tags and stash entries.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::ReportKind;
use platitude_core::{branch, stash, tag};

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

    let err = stash::rename(&exec, &repo.path, "stash@{0}", &chosen, "renamed", &cancel)
        .await
        .expect_err("there is nothing left to rename");

    assert_eq!(
        err.report().map(|report| report.kind),
        Some(ReportKind::StashGone),
        "{err}"
    );
    assert_eq!(
        repo.git(&["stash", "list", "--format=%gs"]),
        "On main: kept"
    );
}

/// Another git holding the stash's lock turns the new entry down: nothing
/// moved, and the report says who.
#[tokio::test]
async fn a_stash_another_git_holds_the_lock_of_is_reported_as_locked() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    repo.write_file("a.txt", "one\n");
    repo.git(&["stash", "push", "-m", "chosen"]);
    let chosen = repo.git(&["rev-parse", "stash@{0}"]);
    let lock = repo.path.join(".git/refs/stash.lock");
    std::fs::write(&lock, "").expect("lock");
    let (exec, cancel) = env();

    let err = stash::rename(&exec, &repo.path, "stash@{0}", &chosen, "renamed", &cancel)
        .await
        .expect_err("git will not write a ref another process has locked");
    std::fs::remove_file(&lock).expect("unlock");
    assert_eq!(
        err.report().map(|report| report.kind),
        Some(ReportKind::StashRenameLocked),
        "{err}"
    );
    assert_eq!(
        repo.git(&["stash", "list", "--format=%gs"]),
        "On main: chosen"
    );
}

/// The new entry went in and the old one would not go — its log's lock
/// taken after the store: both stand, which is the rename half done, and
/// the lock is what stopped it.
#[tokio::test]
async fn a_stash_whose_old_entry_would_not_go_is_half_renamed() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    repo.write_file("a.txt", "one\n");
    repo.git(&["stash", "push", "-m", "chosen"]);
    let chosen = repo.git(&["rev-parse", "stash@{0}"]);
    repo.write_hook(
        "reference-transaction",
        "if [ \"$1\" = committed ]; then : > \"$(git rev-parse --git-dir)/logs/refs/stash.lock\"; fi\n",
    );
    let (exec, cancel) = env();

    let err = stash::rename(&exec, &repo.path, "stash@{0}", &chosen, "renamed", &cancel)
        .await
        .expect_err("the old entry stays");
    std::fs::remove_file(repo.path.join(".git/logs/refs/stash.lock")).expect("unlock");
    assert_eq!(
        err.report().map(|report| report.kind),
        Some(ReportKind::HalfRenamedLocked),
        "{err}"
    );
    assert_eq!(
        repo.git(&["stash", "list", "--format=%gs"])
            .lines()
            .collect::<Vec<_>>(),
        vec!["renamed", "On main: chosen"]
    );
}

/// A reftable store keeps one lock for every ref and names no file: a tag's
/// and a stash entry's refusal say `cannot lock references`, and are the
/// lock's report; a branch's rename says only `branch rename failed`, and
/// is the plain refusal — the store's lock file on disk is no word of git's.
/// Only where this git has the store.
#[tokio::test]
async fn a_reftable_stores_lock_is_reported_as_a_lock_where_git_names_it() {
    let Some(mut repo) = TestRepo::init_reftable() else {
        return;
    };
    repo.commit_file("a.txt", "a\n", "root");
    repo.git(&["branch", "feature"]);
    repo.git(&["tag", "v1.0"]);
    repo.write_file("a.txt", "one\n");
    repo.git(&["stash", "push", "-m", "chosen"]);
    let chosen = repo.git(&["rev-parse", "stash@{0}"]);
    let lock = repo.path.join(".git/reftable/tables.list.lock");
    std::fs::write(&lock, "").expect("lock");
    let (exec, cancel) = env();
    let kind = |err: platitude_core::GitError| err.report().map(|report| report.kind);

    let branch = branch::rename(&exec, &repo.path, "feature", "feat", false, &cancel)
        .await
        .expect_err("the store is locked");
    let tag = tag::rename(&exec, &repo.path, "v1.0", "v1.1", &cancel)
        .await
        .expect_err("the store is locked");
    let stashed = stash::rename(&exec, &repo.path, "stash@{0}", &chosen, "renamed", &cancel)
        .await
        .expect_err("the store is locked");
    std::fs::remove_file(&lock).expect("unlock");
    assert!(
        !branch.to_string().contains("lock"),
        "git names no lock over a branch: {branch}"
    );
    assert_eq!(kind(branch), Some(ReportKind::RenameRefused));
    assert_eq!(kind(tag), Some(ReportKind::RenameLocked));
    assert_eq!(kind(stashed), Some(ReportKind::StashRenameLocked));
    assert_eq!(repo.git(&["branch", "--list", "feat*"]), "feature");
}

/// The same lock taken between the new entry going in and the old one's
/// drop: git's drop says only that it could not, so the half-done rename is
/// git's refusal, not a lock's.
#[tokio::test]
async fn a_reftable_stash_whose_old_entry_would_not_go_is_half_renamed_by_git() {
    let Some(mut repo) = TestRepo::init_reftable() else {
        return;
    };
    repo.commit_file("a.txt", "a\n", "root");
    repo.write_file("a.txt", "one\n");
    repo.git(&["stash", "push", "-m", "chosen"]);
    let chosen = repo.git(&["rev-parse", "stash@{0}"]);
    repo.write_hook(
        "reference-transaction",
        "if [ \"$1\" = committed ]; then : > \"$(git rev-parse --git-dir)/reftable/tables.list.lock\"; fi\n",
    );
    let (exec, cancel) = env();

    let err = stash::rename(&exec, &repo.path, "stash@{0}", &chosen, "renamed", &cancel)
        .await
        .expect_err("the old entry stays");
    std::fs::remove_file(repo.path.join(".git/reftable/tables.list.lock")).expect("unlock");
    assert!(
        !err.to_string().contains("lock"),
        "git names no lock here: {err}"
    );
    assert_eq!(
        err.report().map(|report| report.kind),
        Some(ReportKind::HalfRenamedRefused),
        "{err}"
    );
}

/// What the name boxes refuse beside the names already there
/// (`RefsSnapshot::local_clash` / `tag_clash`): git keeps a ref as a
/// file, so `a` and `a/b` cannot both be, either way round, for a branch
/// or a tag. Letter case is the disk's to decide and is not held here.
#[test]
fn git_refuses_a_name_that_is_a_folder_of_another() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    repo.git(&["branch", "solo"]);
    repo.git(&["branch", "group/member"]);
    repo.git(&["tag", "v1"]);
    repo.git(&["tag", "rc/1"]);
    for (args, why) in [
        (["branch", "solo/inner"], "a branch under a branch"),
        (["branch", "group"], "a branch over a folder of branches"),
        (["tag", "v1/patch"], "a tag under a tag"),
        (["tag", "rc"], "a tag over a folder of tags"),
    ] {
        assert!(!repo.git_ok(&args), "{why}");
    }
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
            tag::ref_format_ok(name),
            git_says,
            "disagreed about {name:?} (git says {git_says})"
        );
    }
    // Past the format, the two names `git tag` itself turns down (2.55's
    // `check_tag_ref`; 2.43 still makes `refs/tags/HEAD`), as a branch's
    // box does (`branch::is_valid_name`).
    for name in ["-dash", "HEAD"] {
        assert!(!tag::is_valid_name(name), "{name:?}");
    }
    assert!(tag::is_valid_name("HEADS-UP"), "only the whole name");
}
