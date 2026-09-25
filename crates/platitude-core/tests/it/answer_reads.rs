//! Reads outside the configuration family (config_reads.rs) that answer
//! with their exit code (core.md「終了コードで答える問い合わせは」). The
//! two HEAD reads are pinned in `refs_integration`, where detached and
//! unborn are those exit codes.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::{assert_answered, logged};
use platitude_core::details::{self, DiffTarget};
use platitude_core::process::CommandEnd;
use platitude_core::{Oid, preview, stash};

#[track_caller]
fn assert_read_what_exists(ends: &[CommandEnd]) {
    assert!(
        ends.iter().all(|e| *e == CommandEnd::Exited(0)),
        "cat-file only runs against a side that is there: {ends:?}"
    );
}

/// `diff --no-index` exits 1 whenever the sides differ — against
/// `/dev/null`, every time.
#[tokio::test]
async fn diffing_an_untracked_file_answers_by_code() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("fresh.txt", "new\n");
    let (exec, log, cancel) = logged();

    let raw = details::file_diff_raw(
        &exec,
        &repo.path,
        &DiffTarget::Untracked {
            path: "fresh.txt".to_string(),
        },
        &cancel,
    )
    .await
    .expect("untracked diff");

    assert!(!raw.is_empty(), "the whole file is the diff");
    assert_eq!(log.ends_of(&["--no-index"]), vec![CommandEnd::Answered(1)]);
}

/// A preview's side is routinely missing (an added file's parent, `HEAD:`
/// when unborn, `:0:` for a staged deletion) and `cat-file` fails on it
/// with 128, so `preview::blob_side` probes with a read that answers and
/// runs `cat-file` only on a real side.
#[tokio::test]
async fn previewing_a_side_that_is_not_there_answers_by_code() {
    let mut repo = TestRepo::init();
    repo.commit_file("base.txt", "x\n", "base");
    repo.write_file("logo.png", "stands in for an image\n");
    repo.git(&["add", "--", "logo.png"]);
    repo.git(&["commit", "-m", "add image"]);
    let head = repo.git(&["rev-parse", "HEAD"]);
    let parent = repo.git(&["rev-parse", "HEAD^"]);
    let (exec, log, cancel) = logged();
    let scratch = tempfile::tempdir().expect("a temp dir");
    let files = preview::PreviewFiles::at(scratch.path().join("s"));

    let p = preview::file_preview(
        &exec,
        &repo.path,
        &DiffTarget::Commit {
            oid: Oid::from_hex_str(&head).expect("head"),
            parent: Some(Oid::from_hex_str(&parent).expect("parent")),
            path: "logo.png".to_string(),
            orig_path: None,
        },
        true,
        files.read(1),
        &cancel,
    )
    .await
    .expect("preview");

    assert!(p.old.is_none(), "the parent does not have the file yet");
    assert!(p.new.is_some(), "the commit that added it does");
    assert_read_what_exists(&log.ends_of(&["cat-file"]));
    assert_answered(&log.ends_of(&["rev-parse", "--verify"]), "the side probe");
}

/// A deleted file's new side is gone; the same probe keeps `cat-file` off
/// it before `preview::source_text` falls back to the old side.
#[tokio::test]
async fn colouring_a_deleted_file_answers_by_code() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.rs", "fn main() {}\n", "add");
    repo.git(&["rm", "--", "a.rs"]);
    repo.git(&["commit", "-m", "drop"]);
    let head = repo.git(&["rev-parse", "HEAD"]);
    let parent = repo.git(&["rev-parse", "HEAD^"]);
    let (exec, log, cancel) = logged();

    let source = preview::source_text(
        &exec,
        &repo.path,
        &DiffTarget::Commit {
            oid: Oid::from_hex_str(&head).expect("head"),
            parent: Some(Oid::from_hex_str(&parent).expect("parent")),
            path: "a.rs".to_string(),
            orig_path: None,
        },
        &cancel,
    )
    .await;

    assert_eq!(
        source.as_deref(),
        Some("fn main() {}\n"),
        "every row of the diff comes from the old side"
    );
    assert_read_what_exists(&log.ends_of(&["cat-file"]));
    assert_answered(&log.ends_of(&["rev-parse", "--verify"]), "the side probe");
}

/// Renaming probes the shifted entry before dropping the old one; 0 (the
/// store pushed it) and 1 (the refusing arm) are both answers.
#[tokio::test]
async fn renaming_a_stash_probes_by_code() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("a.txt", "two\n");
    repo.git(&["stash", "push", "-m", "before"]);
    let (exec, log, cancel) = logged();

    stash::rename(&exec, &repo.path, "stash@{0}", "after", &cancel)
        .await
        .expect("rename");

    assert_answered(
        &log.ends_of(&["rev-parse", "--verify"]),
        "the shifted-entry probe",
    );
}
