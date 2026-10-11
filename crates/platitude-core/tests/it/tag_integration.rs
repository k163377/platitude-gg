//! Making a tag, against real git; git's refusals of a taken or invalid
//! name are in [`periodic`]. Sending one to a remote is
//! `remote_tags_integration`.

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

/// `check-ref-format` takes a leading dash but `git tag` does not. The
/// boxes turn it down first ([`tag::is_valid_name`]); this is git's
/// refusal under them, should one get through.
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

/// The first half of a rename: nothing has moved yet, so the refusal goes
/// back under the box the name was typed into (デザイン規約 §答えの要らない報せ).
/// The second half — new name made, old one still there — is
/// [`platitude_core::report::half_renamed`], unreachable from here.
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
    assert_eq!(
        report.fact, "",
        "git's words are the log's; the screen's line points there"
    );
    assert!(
        err.to_string().contains("already exists"),
        "git said why, and the log keeps it: {err}"
    );
    assert!(
        repo.git(&["tag", "--list"]).contains("v1.0"),
        "nothing moved"
    );
}

/// The new name's lock held by another git: its own report, nothing made.
#[tokio::test]
async fn a_tag_rename_another_git_holds_the_lock_for_is_reported_as_locked() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    repo.git(&["tag", "v1.0"]);
    std::fs::create_dir_all(repo.path.join(".git/refs/tags")).expect("tags folder");
    std::fs::write(repo.path.join(".git/refs/tags/v1.1.lock"), "").expect("lock");
    let (exec, cancel) = env();

    let err = tag::rename(&exec, &repo.path, "v1.0", "v1.1", &cancel)
        .await
        .expect_err("git will not write a ref another process has locked");
    assert_eq!(
        err.report().map(|report| report.kind),
        Some(ReportKind::RenameLocked),
        "{err}"
    );
    assert_eq!(repo.git(&["tag", "--list"]), "v1.0", "nothing moved");
}

/// A tag deleted elsewhere between the menu and the press, with a branch
/// of the same name still standing: a short name would resolve to the
/// branch, and the new tag would mark the branch's commit. The old name
/// is read as the tag it was, so git refuses and nothing is made.
#[tokio::test]
async fn a_tag_gone_before_the_press_is_not_taken_from_a_branch_of_its_name() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    repo.git(&["tag", "v1.0"]);
    repo.git(&["branch", "v1.0"]);
    repo.commit_file("b.txt", "b\n", "second");
    repo.git(&["branch", "--force", "v1.0"]);
    repo.git(&["tag", "--delete", "v1.0"]);
    let (exec, cancel) = env();

    let err = tag::rename(&exec, &repo.path, "v1.0", "v1.1", &cancel)
        .await
        .expect_err("there is no tag to rename");
    assert_eq!(
        err.report().map(|report| report.kind),
        Some(ReportKind::RenameRefused),
        "nothing moved, so the box can still answer it: {err}"
    );
    assert_eq!(repo.git(&["tag", "--list"]), "", "no tag was made");
}

/// What the pre-merge run leaves out: git refusing a tag name that is
/// taken or that no ref can have. `tag::create` hands it over on a fixed
/// command line with no `--force`, so the refusal is git's; what the box
/// turns down first is `tag::is_valid_name`, held to git pre-merge by
/// `rename_integration::the_name_rules_are_the_ones_git_applies`.
mod periodic {
    use super::*;

    /// The refusal has to come back as one: a create that quietly moved a
    /// release mark would look like one that did nothing.
    #[tokio::test]
    #[ignore = "git's refusal of a taken tag name: not worth the pre-merge run"]
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

    /// The backstop under `tag::is_valid_name`.
    #[tokio::test]
    #[ignore = "git's refusal of a name no ref can have: not worth the pre-merge run"]
    async fn a_name_git_refuses_makes_no_tag() {
        let mut repo = TestRepo::init();
        repo.commit_file("a.txt", "a\n", "root");
        let (exec, cancel) = env();

        let refused = tag::create(&exec, &repo.path, "has space", "", &cancel).await;

        assert!(refused.is_err());
        assert_eq!(repo.git(&["tag", "--list"]), "");
    }
}
