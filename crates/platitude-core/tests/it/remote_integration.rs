//! Remote traffic, exercised entirely offline: the remote is a `file://`
//! URL of a second local repository (実装計画 §11.3).

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::env;
use crate::support::remote::origin_and_clone;
use platitude_core::GitError;
use platitude_core::commit;
use platitude_core::remote::{self, PushForce, PushSpec};
use platitude_core::report::ReportKind;

/// A `file://` remote answers instantly; the budget just has to exist.
const NET: std::time::Duration = remote::DEFAULT_NETWORK_TIMEOUT;

#[tokio::test]
async fn lists_remotes_from_config() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let (exec, cancel) = env();

    assert!(
        remote::list(&exec, &repo.path, &cancel)
            .await
            .expect("list")
            .is_empty(),
        "a fresh repository has no remotes"
    );

    repo.git(&["remote", "add", "origin", "file:///tmp/a"]);
    repo.git(&["remote", "add", "upstream", "file:///tmp/b"]);
    repo.git(&["remote", "set-url", "--push", "origin", "file:///tmp/push"]);

    let remotes = remote::list(&exec, &repo.path, &cancel)
        .await
        .expect("list");
    assert_eq!(remotes.len(), 2);
    assert_eq!(remotes[0].name, "origin");
    assert_eq!(remotes[0].fetch_url, "file:///tmp/a");
    assert_eq!(remotes[0].push_url, "file:///tmp/push");
    assert_eq!(remotes[1].name, "upstream");
}

#[tokio::test]
async fn push_then_fetch_moves_commits_between_repositories() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

    work.commit_file("b.txt", "two\n", "second");
    remote::push(
        &exec,
        &work.path,
        &PushSpec {
            remote: "origin".into(),
            local: "main".into(),
            remote_branch: "main".into(),
            set_upstream: true,
            force: PushForce::None,
        },
        NET,
        &cancel,
    )
    .await
    .expect("push");

    assert_eq!(
        bare.git(&["log", "-1", "--format=%s", "main"]),
        "second",
        "the remote advanced"
    );
    assert_eq!(
        work.git(&["rev-parse", "--abbrev-ref", "main@{upstream}"]),
        "origin/main",
        "--set-upstream recorded the tracking branch"
    );

    // A third repository fetches what was pushed.
    let mut other = TestRepo::init();
    other.git(&["remote", "add", "origin", &bare.file_url()]);
    remote::fetch(&exec, &other.path, Some("origin"), NET, &cancel)
        .await
        .expect("fetch");
    assert_eq!(
        other.git(&["log", "-1", "--format=%s", "origin/main"]),
        "second"
    );
}

#[tokio::test]
async fn a_non_fast_forward_push_is_refused_until_forced() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

    // Someone else publishes first.
    let mut other = TestRepo::init();
    other.git(&["remote", "add", "origin", &bare.file_url()]);
    other.git(&["fetch", "origin"]);
    other.git(&["checkout", "-b", "main", "origin/main"]);
    other.commit_file("theirs.txt", "theirs\n", "their work");
    other.git(&["push", "origin", "main"]);

    // Our own divergent history.
    work.git(&["commit", "--amend", "-m", "rewritten root"]);
    let spec = PushSpec {
        remote: "origin".into(),
        local: "main".into(),
        remote_branch: "main".into(),
        set_upstream: false,
        force: PushForce::None,
    };
    let err = remote::push(&exec, &work.path, &spec, NET, &cancel)
        .await
        .expect_err("non-fast-forward");
    assert!(
        err.to_string().contains("rejected") || err.to_string().contains("non-fast-forward"),
        "git's own rejection is passed through: {err}"
    );
    assert!(
        err.is_outdated(),
        "a refusal a fetch would answer is told apart from one it would not: {err}"
    );

    // A lease pinned to a commit the remote has moved past also fails, and
    // for the same reason: this window is looking at an older remote.
    let stale = work.git(&["rev-parse", "origin/main"]);
    let leased = PushSpec {
        force: PushForce::WithLease {
            expect: Some(stale),
        },
        ..spec.clone()
    };
    let err = remote::push(&exec, &work.path, &leased, NET, &cancel)
        .await
        .expect_err("stale lease");
    assert!(err.is_outdated(), "{err}");

    // Plain force wins.
    let forced = PushSpec {
        force: PushForce::Force,
        ..spec
    };
    remote::push(&exec, &work.path, &forced, NET, &cancel)
        .await
        .expect("forced push");
    assert_eq!(
        bare.git(&["log", "-1", "--format=%s", "main"]),
        "rewritten root"
    );
}

#[tokio::test]
async fn deleting_a_remote_branch_prunes_on_the_next_fetch() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

    work.git(&["checkout", "-b", "temp"]);
    work.commit_file("t.txt", "t\n", "temp work");
    remote::push(
        &exec,
        &work.path,
        &PushSpec {
            remote: "origin".into(),
            local: "temp".into(),
            remote_branch: "temp".into(),
            set_upstream: false,
            force: PushForce::None,
        },
        NET,
        &cancel,
    )
    .await
    .expect("push temp");
    assert!(bare.git(&["branch", "--list"]).contains("temp"));

    remote::delete_remote_branch(&exec, &work.path, "origin", "temp", NET, &cancel)
        .await
        .expect("delete remote branch");
    assert!(!bare.git(&["branch", "--list"]).contains("temp"));

    remote::fetch(&exec, &work.path, Some("origin"), NET, &cancel)
        .await
        .expect("fetch --prune");
    assert!(
        !work
            .git(&["branch", "-r", "--list"])
            .contains("origin/temp"),
        "--prune dropped the stale remote-tracking ref"
    );
}

/// The rename git has no command for: the name moves, what the branch
/// pointed at does not, and the local branch that tracked it comes along.
#[tokio::test]
async fn renaming_a_remote_branch_moves_the_name_and_the_tracking() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

    work.git(&["switch", "-c", "billing"]);
    work.commit_file("b.txt", "b\n", "billing work");
    work.git(&["push", "-u", "origin", "billing"]);
    let tip = work.git(&["rev-parse", "billing"]);
    // Renaming publishes nothing: this commit is only here.
    work.commit_file("b.txt", "b2\n", "not published");

    remote::rename_remote_branch(
        &exec,
        &work.path,
        "origin",
        "billing",
        "billing-v2",
        NET,
        &cancel,
    )
    .await
    .expect("rename remote branch");

    let listed = bare.git(&["branch", "--list"]);
    assert!(listed.contains("billing-v2"), "{listed}");
    assert!(
        !listed.contains("  billing\n"),
        "the old name is gone: {listed}"
    );
    assert_eq!(
        bare.git(&["rev-parse", "billing-v2"]),
        tip,
        "the new name points where the old one did, not at what is only here"
    );
    assert_eq!(
        work.git(&["config", "branch.billing.merge"]),
        "refs/heads/billing-v2",
        "the branch that tracked it follows the name"
    );
    assert!(
        !work
            .git(&["branch", "-r", "--list"])
            .contains("origin/billing\n"),
        "the tracking ref for the old name went with the delete"
    );
}

/// A push git turns down leaves the old name where it was: nothing is
/// deleted on the strength of a half-finished rename.
#[tokio::test]
async fn a_rename_whose_push_fails_deletes_nothing() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

    work.git(&["switch", "-c", "billing"]);
    work.commit_file("b.txt", "b\n", "billing work");
    work.git(&["push", "-u", "origin", "billing"]);
    // Somebody else's work already stands under the new name, on a line of
    // its own, so the push cannot be a fast-forward.
    work.git(&["switch", "-c", "someone-else", "main"]);
    work.commit_file("c.txt", "c\n", "not ours");
    work.git(&["push", "origin", "someone-else:billing-v2"]);
    work.git(&["switch", "billing"]);

    let error = remote::rename_remote_branch(
        &exec,
        &work.path,
        "origin",
        "billing",
        "billing-v2",
        NET,
        &cancel,
    )
    .await
    .expect_err("the push is refused");
    assert!(
        error.is_outdated() || matches!(error, GitError::Failed { .. }),
        "{error:?}"
    );
    assert!(
        bare.git(&["branch", "--list"]).contains("billing"),
        "the old name is still there"
    );
    assert_eq!(
        work.git(&["config", "branch.billing.merge"]),
        "refs/heads/billing",
        "and nothing was re-pointed"
    );
}

/// The first push of a branch goes where the question said, and records it
/// so the question is asked once.
#[tokio::test]
async fn publishing_sends_the_branch_and_records_the_upstream() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

    work.git(&["switch", "-c", "topic"]);
    work.commit_file("b.txt", "b\n", "topic work");

    let spec = remote::plan_publish(&exec, &work.path, "origin", "topic", "", &cancel)
        .await
        .expect("plan publish");
    assert!(spec.set_upstream, "the answer is recorded, not re-asked");
    remote::push(&exec, &work.path, &spec, NET, &cancel)
        .await
        .expect("publish");

    assert_eq!(
        bare.git(&["rev-parse", "topic"]),
        work.git(&["rev-parse", "topic"]),
        "the branch exists over there, at our tip"
    );
    assert_eq!(
        work.git(&["config", "branch.topic.merge"]),
        "refs/heads/topic"
    );
    assert_eq!(work.git(&["config", "branch.topic.remote"]), "origin");
}

/// The name over there is the question's to choose: it need not be the one
/// the branch has here.
#[tokio::test]
async fn publishing_can_use_a_different_name_on_the_remote() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

    work.git(&["switch", "-c", "topic"]);
    work.commit_file("b.txt", "b\n", "topic work");

    let spec = remote::plan_publish(&exec, &work.path, "origin", "feature/topic", "", &cancel)
        .await
        .expect("plan publish");
    remote::push(&exec, &work.path, &spec, NET, &cancel)
        .await
        .expect("publish");

    assert_eq!(
        bare.git(&["rev-parse", "feature/topic"]),
        work.git(&["rev-parse", "topic"])
    );
    assert_eq!(
        work.git(&["config", "branch.topic.merge"]),
        "refs/heads/feature/topic",
        "the upstream is the name over there"
    );
}

/// Why the UI has to ask before a first push lands on a name that is taken:
/// git does not refuse it. Whenever the push fast-forwards, somebody else's
/// branch quietly moves and we end up tracking it.
#[tokio::test]
async fn a_first_push_onto_a_taken_name_is_not_refused_when_it_fast_forwards() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

    // `shared` exists on the remote already, one commit behind us, and
    // nothing here records that it does.
    work.git(&["switch", "-c", "shared"]);
    work.commit_file("b.txt", "b\n", "theirs");
    work.git(&["push", "origin", "shared"]);
    let theirs = bare.git(&["rev-parse", "shared"]);
    work.commit_file("b.txt", "b2\n", "ours");

    let tip = remote::branch_tip(&exec, &work.path, "origin", "shared", NET, &cancel)
        .await
        .expect("ask")
        .expect("the question the UI puts to the remote before it pushes");
    assert!(
        commit::is_in_head_history(&exec, &work.path, &tip, &cancel)
            .await
            .expect("compare"),
        "what makes this the fast-forward case: their commit is one of ours"
    );

    let spec = remote::plan_publish(&exec, &work.path, "origin", "shared", "", &cancel)
        .await
        .expect("plan publish");
    remote::push(&exec, &work.path, &spec, NET, &cancel)
        .await
        .expect("git takes it — this is the hole the question covers");

    assert_ne!(
        bare.git(&["rev-parse", "shared"]),
        theirs,
        "their branch moved to our commit without git objecting"
    );
}

/// `ls-remote` matches a bare name against the tail of a ref, so the check
/// only means anything when it asks for the whole path.
#[tokio::test]
async fn the_remote_branch_check_answers_for_the_exact_name_only() {
    let (_bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

    work.git(&["switch", "-c", "feature/topic"]);
    work.commit_file("b.txt", "b\n", "topic work");
    work.git(&["push", "origin", "feature/topic"]);

    assert!(
        remote::branch_tip(&exec, &work.path, "origin", "feature/topic", NET, &cancel)
            .await
            .expect("ask")
            .is_some()
    );
    assert!(
        remote::branch_tip(&exec, &work.path, "origin", "topic", NET, &cancel)
            .await
            .expect("ask")
            .is_none(),
        "`topic` is not taken just because `feature/topic` is"
    );
    assert!(
        remote::branch_tip(&exec, &work.path, "origin", "feature", NET, &cancel)
            .await
            .expect("ask")
            .is_none()
    );
}

/// The other half of the same question. A name can be taken by commits
/// this history never had, and there git refuses the push outright — so
/// "taken" alone cannot decide what the UI should do about it. The commit
/// `branch_tip` hands back is what tells the two apart before anything is
/// sent.
#[tokio::test]
async fn a_taken_name_holding_commits_of_its_own_is_refused() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

    // Their branch leaves the trunk and takes a commit with it; ours
    // leaves from the same place and never gets that commit.
    work.git(&["switch", "-c", "theirs"]);
    work.commit_file("b.txt", "theirs\n", "theirs");
    work.git(&["push", "origin", "theirs"]);
    let theirs = bare.git(&["rev-parse", "theirs"]);
    work.git(&["switch", "-c", "ours", "HEAD~1"]);
    work.commit_file("c.txt", "ours\n", "ours");

    let tip = remote::branch_tip(&exec, &work.path, "origin", "theirs", NET, &cancel)
        .await
        .expect("ask")
        .expect("the name is taken");
    assert!(
        !commit::is_in_head_history(&exec, &work.path, &tip, &cancel)
            .await
            .expect("compare"),
        "their commit is not one of ours, which is what makes this refusable"
    );

    let spec = remote::plan_publish(&exec, &work.path, "origin", "theirs", "", &cancel)
        .await
        .expect("plan publish");
    let sent = remote::push(&exec, &work.path, &spec, NET, &cancel).await;
    assert!(
        sent.as_ref().is_err_and(GitError::is_outdated),
        "git turns a first push that is not a fast-forward down: {sent:?}"
    );
    assert_eq!(
        bare.git(&["rev-parse", "theirs"]),
        theirs,
        "and their branch is where it was"
    );

    // The only thing that can land here, and the lease is what makes it
    // offerable: pinned to a commit that is not there any more, git says no
    // rather than flattening a branch nobody looked at.
    let stale = remote::plan_publish(
        &exec,
        &work.path,
        "origin",
        "theirs",
        &"0".repeat(40),
        &cancel,
    )
    .await
    .expect("plan a leased publish");
    assert!(
        remote::push(&exec, &work.path, &stale, NET, &cancel)
            .await
            .is_err(),
        "a lease against a commit the remote does not hold is refused"
    );
    assert_eq!(bare.git(&["rev-parse", "theirs"]), theirs);

    let leased = remote::plan_publish(&exec, &work.path, "origin", "theirs", &theirs, &cancel)
        .await
        .expect("plan a leased publish");
    remote::push(&exec, &work.path, &leased, NET, &cancel)
        .await
        .expect("the overwrite the question offers, pinned to what it showed");
    assert_ne!(
        bare.git(&["rev-parse", "theirs"]),
        theirs,
        "their branch now carries ours instead"
    );
}

/// Adding a remote is bookkeeping, not a connection: a URL that goes
/// nowhere is accepted, which is why a failed push leaves the remote in
/// place and `set-url` is the way back.
#[tokio::test]
async fn adding_a_remote_records_the_url_without_reaching_it() {
    let (bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

    let nowhere = "file:///nowhere/there-is-no-such-repository.git";
    remote::add(&exec, &work.path, "fork", nowhere, &cancel)
        .await
        .expect("add a remote nothing answers for");
    assert_eq!(work.git(&["remote", "get-url", "fork"]), nowhere);

    let again = remote::add(&exec, &work.path, "fork", nowhere, &cancel)
        .await
        .expect_err("git keeps its own names unique");
    assert!(matches!(again, GitError::Failed { .. }), "{again:?}");

    remote::set_url(&exec, &work.path, "fork", &bare.file_url(), &cancel)
        .await
        .expect("correct the URL");
    assert_eq!(work.git(&["remote", "get-url", "fork"]), bare.file_url());

    // And the corrected remote is usable, which is the whole point of
    // keeping it rather than undoing the add.
    let spec = remote::plan_publish(&exec, &work.path, "fork", "main", "", &cancel)
        .await
        .expect("plan publish");
    remote::push(&exec, &work.path, &spec, NET, &cancel)
        .await
        .expect("push to the corrected remote");
}

/// The first push to a remote made by the same answer: when the URL turns
/// out to go nowhere, the push fails and the remote stays. Undoing the add
/// would throw away the only part of the answer that was worth keeping.
#[tokio::test]
async fn a_push_to_a_remote_that_goes_nowhere_leaves_the_remote_behind() {
    let (_bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

    let nowhere = "file:///nowhere/there-is-no-such-repository.git";
    remote::add(&exec, &work.path, "fork", nowhere, &cancel)
        .await
        .expect("add");
    let spec = remote::plan_publish(&exec, &work.path, "fork", "main", "", &cancel)
        .await
        .expect("plan publish");
    let error = remote::push(&exec, &work.path, &spec, NET, &cancel)
        .await
        .expect_err("nothing answers there");
    assert!(matches!(error, GitError::Failed { .. }), "{error:?}");

    assert_eq!(
        work.git(&["remote", "get-url", "fork"]),
        nowhere,
        "the remote is still here to be corrected"
    );
    assert_eq!(
        // `--default` so an unset key is an empty answer rather than an
        // exit code the harness reads as a broken command.
        work.git(&["config", "--default", "", "--get", "branch.main.remote"]),
        "",
        "and a push that never landed recorded no upstream"
    );
}

/// Writes a `pre-receive` hook into a bare repository that turns every
/// push away, in the shape a forge writes its own refusals in.
///
/// The hook is how a refusal the far side decides for itself can be had
/// offline at all: a protected branch, a repository rule and a hook all
/// reach this end as the same `[remote rejected]`, and only the sentence
/// underneath differs.
fn decline_every_push(bare: &TestRepo, said: &str) {
    bare.write_hook(
        "pre-receive",
        &format!("echo \"error: {said}\" >&2\nexit 1\n"),
    );
}

#[tokio::test]
async fn a_branch_the_far_side_keeps_comes_back_as_a_refusal_with_its_words() {
    let (bare, work) = origin_and_clone();
    let (exec, cancel) = env();
    decline_every_push(
        &bare,
        "GH006: Protected branch update failed for refs/heads/main.",
    );

    let err = remote::delete_remote_branch(&exec, &work.path, "origin", "main", NET, &cancel)
        .await
        .expect_err("the far side keeps main");
    let Some(report) = err.report() else {
        panic!("a refusal the far side made is told apart from a failure of ours: {err}");
    };
    assert_eq!(report.remote, "origin");
    assert_eq!(report.name, "main");
    assert_eq!(
        report.kind,
        ReportKind::RemoteDelete,
        "what was asked for was the removal"
    );
    assert_eq!(
        report.reason, "GH006: Protected branch update failed for refs/heads/main.",
        "the words are the far side's own, without git's framing"
    );
    // Under `--porcelain` the per-ref result is on stdout and stderr keeps
    // the prose, so what the log carries is what a push already carried:
    // the far side's lines, and git's own last word about the refs.
    assert!(
        err.to_string().contains("GH006") && err.to_string().contains("failed to push"),
        "git's whole message is still there for the log: {err}"
    );
}

#[tokio::test]
async fn a_push_the_far_side_turns_down_is_not_one_a_fetch_would_answer() {
    let (bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();
    decline_every_push(&bare, "Changes must be made through a pull request.");
    work.commit_file("b.txt", "ours\n", "work of our own");

    let err = remote::push(
        &exec,
        &work.path,
        &PushSpec {
            remote: "origin".into(),
            local: "main".into(),
            remote_branch: "main".into(),
            set_upstream: false,
            force: PushForce::None,
        },
        NET,
        &cancel,
    )
    .await
    .expect_err("the hook declines");
    let Some(report) = err.report() else {
        panic!("a fast-forward the far side declined is neither outdated nor ours: {err}");
    };
    assert_eq!(
        report.kind,
        ReportKind::RemoteUpdate,
        "this one was sending, not removing"
    );
    assert_eq!(
        report.reason,
        "Changes must be made through a pull request."
    );
}

/// The same refusal over a tag, which used to reach the screen as a plain
/// failure while the branch beside it came down as a report
/// (デザイン規約 §答えの要らない報せ). **Both halves of a tag's traffic are read
/// the same way** — sending one and taking one off the far side are
/// different commands here (`push` against `push --delete`), so one being
/// classified says nothing about the other.
#[tokio::test]
async fn a_tag_the_far_side_keeps_is_reported_the_way_a_branch_is() {
    let (bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();
    work.git(&["tag", "v1.0"]);
    remote::push_tag(&exec, &work.path, "origin", "v1.0", "", NET, &cancel)
        .await
        .expect("the tag goes over while nothing is standing over it");
    decline_every_push(&bare, "Tag protection rules prevent this.");

    let err = remote::delete_remote_tag(&exec, &work.path, "origin", "v1.0", NET, &cancel)
        .await
        .expect_err("the far side keeps the tag");
    let Some(report) = err.report() else {
        panic!("a tag the far side keeps is a report, not a failure of ours: {err}");
    };
    assert_eq!(report.kind, ReportKind::RemoteDelete);
    assert_eq!(report.remote, "origin");
    assert_eq!(report.name, "v1.0", "the tag names itself, not a branch");
    assert_eq!(report.reason, "Tag protection rules prevent this.");

    work.commit_file("c.txt", "more\n", "something to tag");
    work.git(&["tag", "v1.1"]);
    let err = remote::push_tag(&exec, &work.path, "origin", "v1.1", "", NET, &cancel)
        .await
        .expect_err("the far side keeps its tags");
    let Some(report) = err.report() else {
        panic!("a tag the far side would not take is a report as well: {err}");
    };
    assert_eq!(report.kind, ReportKind::RemoteUpdate);
    assert_eq!(report.name, "v1.1");
    assert_eq!(report.reason, "Tag protection rules prevent this.");
    assert!(
        !err.is_outdated(),
        "nothing about a tag is answered by fetching, whatever the refusal was"
    );
}

/// The refusal a fetch answers says so in git's own words, which is what
/// separates a report with a move behind it from one without.
#[tokio::test]
async fn a_push_that_is_only_out_of_date_reports_gits_own_advice() {
    let (bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

    // Somebody else pushes while this end is not looking.
    let mut other = TestRepo::init();
    other.git(&["remote", "add", "origin", &bare.file_url()]);
    other.git(&["fetch", "origin"]);
    other.git(&["checkout", "-b", "main", "origin/main"]);
    other.commit_file("theirs.txt", "theirs\n", "pushed while you slept");
    other.git(&["push", "origin", "main"]);
    work.commit_file("ours.txt", "ours\n", "work of our own");

    let err = remote::push(
        &exec,
        &work.path,
        &PushSpec {
            remote: "origin".into(),
            local: "main".into(),
            remote_branch: "main".into(),
            set_upstream: false,
            force: PushForce::None,
        },
        NET,
        &cancel,
    )
    .await
    .expect_err("the remote holds what we have not got");
    let Some(report) = err.report() else {
        panic!("being out of date is a report of its own: {err}");
    };
    assert_eq!(report.kind, ReportKind::Outdated);
    assert_eq!(report.remote, "origin");
    assert_eq!(report.name, "main");
    assert!(
        !report.reason.is_empty(),
        "git says why in advice of its own, and that is what goes under the heading"
    );
    assert!(
        !report.reason.contains("hint:"),
        "git's framing comes off the way the far side's does: {}",
        report.reason
    );
}
