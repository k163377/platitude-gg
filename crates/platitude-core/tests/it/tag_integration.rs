//! Making a tag, against real git.
//!
//! The refusal is half the point: `git tag` refuses a name that is taken,
//! and `--force` stays off — a release mark that moves without anybody
//! saying so is exactly what that refusal is there to stop (`tag::create`).
//! Sending one to a remote is `remote_tags_integration`, where the
//! readings it has to agree with are.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::report::ReportKind;
use platitude_core::tag;

#[tokio::test]
async fn a_tag_lands_on_the_commit_it_was_given() {
    let mut repo = TestRepo::init();
    let root = repo.commit_file_id("a.txt", "a\n", "root");
    repo.commit_file("b.txt", "b\n", "second");
    let (exec, cancel) = env();

    tag::create(&exec, &repo.path, "v1.0", &root, &cancel)
        .await
        .expect("create");

    assert_eq!(repo.git(&["tag", "--list"]), "v1.0");
    assert_eq!(
        repo.git(&["rev-parse", "v1.0"]),
        root,
        "the commit handed over, not the one HEAD is on"
    );
    assert_eq!(
        repo.git(&["cat-file", "-t", "v1.0"]),
        "commit",
        "lightweight: the name points straight at the commit"
    );
}

/// The menus hand over the row's own commit, but the argument is optional
/// the way git's is — and git's answer to leaving it off is HEAD.
#[tokio::test]
async fn no_commit_is_head() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    let head = repo.commit_file_id("b.txt", "b\n", "second");
    let (exec, cancel) = env();

    tag::create(&exec, &repo.path, "here", "", &cancel)
        .await
        .expect("create");

    assert_eq!(repo.git(&["rev-parse", "here"]), head);
}

/// **Made, or refused.** The refusal is git's, and it has to come back as
/// a refusal — a create that quietly moved somebody's release mark would
/// be indistinguishable from one that did nothing.
#[tokio::test]
async fn a_name_already_taken_is_refused_and_nothing_moves() {
    let mut repo = TestRepo::init();
    let root = repo.commit_file_id("a.txt", "a\n", "root");
    repo.git(&["tag", "-a", "v1.0", "-m", "first release"]);
    let head = repo.commit_file_id("b.txt", "b\n", "second");
    let (exec, cancel) = env();

    let refused = tag::create(&exec, &repo.path, "v1.0", &head, &cancel).await;

    assert!(refused.is_err(), "git will not take a name twice");
    assert_eq!(
        repo.git(&["rev-list", "-n", "1", "v1.0"]),
        root,
        "the tag is where it was"
    );
    assert_eq!(
        repo.git(&["cat-file", "-t", "v1.0"]),
        "tag",
        "and is still the annotated object it was"
    );
}

/// A name git will not take as a ref comes back as a refusal.
/// `tag::is_valid_name` is what keeps the box from getting this far; this
/// is the backstop under it.
#[tokio::test]
async fn a_name_git_refuses_makes_no_tag() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    let (exec, cancel) = env();

    let refused = tag::create(&exec, &repo.path, "has space", "", &cancel).await;

    assert!(refused.is_err());
    assert_eq!(repo.git(&["tag", "--list"]), "");
}

/// A name that starts with a dash reaches git as a name
/// (`--end-of-options`), and git turns it down on its own terms:
/// `fatal: '-dashed' is not a valid tag name.` (measured, git 2.55) —
/// **`check-ref-format` takes it, `git tag` does not**, so this is one
/// place where [`tag::is_valid_name`] says yes and the command still
/// refuses. Left to git: what a terminal can make, the box may offer
/// to make (デザイン規約 §左メニューの所作「規則は git のもの」),
/// and the refusal is git's own words.
#[tokio::test]
async fn a_leading_dash_reaches_git_as_a_name_and_git_refuses_it() {
    let mut repo = TestRepo::init();
    let head = repo.commit_file_id("a.txt", "a\n", "root");
    let (exec, cancel) = env();

    let refused = tag::create(&exec, &repo.path, "-dashed", &head, &cancel).await;

    let message = format!("{:?}", refused.expect_err("git refuses the name"));
    assert!(
        message.contains("not a valid tag name"),
        "git's own refusal, not an unrecognised option: {message}"
    );
    assert_eq!(repo.git(&["tag", "--list"]), "");
}

/// The first half of a rename: nothing has moved yet, so a refusal there
/// is one the box the name was typed into can still answer
/// (デザイン規約 §答えの要らない報せ). The second half — the new name made and the
/// old one still there — is [`platitude_core::report::half_renamed`], which
/// has no arrangement that reaches it from here.
#[tokio::test]
async fn a_name_a_tag_cannot_take_is_reported_under_the_old_one() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    repo.git(&["tag", "v1.0"]);
    repo.git(&["tag", "taken"]);
    let (exec, cancel) = env();

    let err = tag::rename(&exec, &repo.path, "v1.0", "taken", &cancel)
        .await
        .expect_err("git will not put a name that is taken on a second tag");
    let Some(report) = err.report() else {
        panic!("a name git would not take is a report: {err}");
    };
    assert_eq!(report.kind, ReportKind::RenameRefused);
    assert_eq!(report.name, "v1.0", "the tag still carries the old name");
    assert!(
        !report.reason.is_empty(),
        "git said why, and that goes under the box"
    );
    assert!(
        repo.git(&["tag", "--list"]).contains("v1.0"),
        "nothing moved"
    );
}
