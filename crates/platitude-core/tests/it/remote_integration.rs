//! Remote traffic, exercised entirely offline: the remote is a `file://`
//! URL of a second local repository (実装計画 §11.3).
//!
//! What git alone decides about that traffic is in [`periodic`].

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::env;
use crate::support::remote::origin_and_clone;
use crate::support::session::{opened, write_answer, write_result};
use platitude_core::GitError;
use platitude_core::OperationKind;
use platitude_core::commit;
use platitude_core::integrate::Landing;
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
async fn a_non_fast_forward_push_is_refused_until_forced() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

    let mut other = TestRepo::init();
    other.git(&["remote", "add", "origin", &bare.file_url()]);
    other.git(&["fetch", "origin"]);
    other.git(&["checkout", "-b", "main", "origin/main"]);
    other.commit_file("theirs.txt", "theirs\n", "their work");
    other.git(&["push", "origin", "main"]);

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

    // A stale lease is outdated too: this window is looking at an older remote.
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
    assert_eq!(
        err.report().map(|report| report.kind),
        Some(ReportKind::Moved),
        "a lease the remote left says the branch moved, not that it is ahead: {err}"
    );
    assert!(err.is_outdated(), "and is caught up with all the same");

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

/// git has no command for this: the name moves, its commit stays, and the
/// local branch that tracked it follows.
#[tokio::test]
async fn replacing_a_remote_branch_moves_the_name_and_the_tracking() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

    work.git(&["switch", "-c", "billing"]);
    work.commit_file("b.txt", "b\n", "billing work");
    work.git(&["push", "-u", "origin", "billing"]);
    let tip = work.git(&["rev-parse", "billing"]);
    // Replacing publishes nothing: this commit is only here.
    work.commit_file("b.txt", "b2\n", "not published");

    remote::replace_remote_branch(
        &exec,
        &work.path,
        "origin",
        "billing",
        "billing-v2",
        &tip,
        NET,
        &cancel,
    )
    .await
    .expect("replace remote branch");

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

#[tokio::test]
async fn a_replace_whose_push_fails_deletes_nothing() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

    work.git(&["switch", "-c", "billing"]);
    work.commit_file("b.txt", "b\n", "billing work");
    work.git(&["push", "-u", "origin", "billing"]);
    let tip = work.git(&["rev-parse", "origin/billing"]);
    // Unrelated work already under the new name: the push cannot fast-forward.
    work.git(&["switch", "-c", "someone-else", "main"]);
    work.commit_file("c.txt", "c\n", "not ours");
    work.git(&["push", "origin", "someone-else:billing-v2"]);
    work.git(&["switch", "billing"]);

    let error = remote::replace_remote_branch(
        &exec,
        &work.path,
        "origin",
        "billing",
        "billing-v2",
        &tip,
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

/// A branch of our own on the remote, pushed and tracked: its tip as the
/// screen shows it (the remote-tracking ref).
fn a_branch_over_there(work: &mut TestRepo, name: &str) -> String {
    work.git(&["switch", "-c", name]);
    work.commit_file(&format!("{name}.txt"), "ours\n", "our work");
    work.git(&["push", "-u", "origin", name]);
    work.git(&["switch", "main"]);
    work.git(&["rev-parse", &format!("origin/{name}")])
}

/// Someone else's push to `branch` after our last fetch. Returns the tip
/// it left over there, which this clone has never seen.
fn moved_over_there(bare: &TestRepo, branch: &str) -> String {
    let mut other = TestRepo::init();
    other.git(&["remote", "add", "origin", &bare.file_url()]);
    other.git(&["fetch", "origin"]);
    other.git(&["switch", "-c", branch, &format!("origin/{branch}")]);
    other.commit_file("theirs.txt", "theirs\n", "pushed since our fetch");
    other.git(&["push", "origin", branch]);
    other.git(&["rev-parse", "HEAD"])
}

/// The lease is on the commit the screen showed: a push since the last
/// fetch turns the delete into a refusal a fetch answers, and their
/// commit — never here — stays over there. Unmoved, the same delete goes.
#[tokio::test]
async fn a_remote_branch_that_moved_since_the_last_fetch_is_not_deleted() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();
    let shown = a_branch_over_there(&mut work, "topic");
    let theirs = moved_over_there(&bare, "topic");

    let err =
        remote::delete_remote_branch(&exec, &work.path, "origin", "topic", &shown, NET, &cancel)
            .await
            .expect_err("the remote is not where the screen showed it");
    let Some(report) = err.report() else {
        panic!("a stale lease reads as any outdated push: {err}");
    };
    assert_eq!(report.kind, ReportKind::MovedDelete);
    assert_eq!(
        (report.remote.as_str(), report.name.as_str()),
        ("origin", "topic")
    );
    assert_eq!(
        bare.git(&["rev-parse", "topic"]),
        theirs,
        "their commit is still over there"
    );

    work.git(&["fetch", "origin"]);
    let seen = work.git(&["rev-parse", "origin/topic"]);
    remote::delete_remote_branch(&exec, &work.path, "origin", "topic", &seen, NET, &cancel)
        .await
        .expect("leased to what is there, the delete goes");
    assert!(!bare.git_ok(&["rev-parse", "--verify", "refs/heads/topic"]));
}

/// The name goes qualified, so a branch the remote dropped since the last
/// fetch is the lease's refusal like any other move — a bare name fails
/// before the lease is weighed (`remote ref does not exist`) — and a tag
/// of the same name over there neither blocks the delete nor goes with it.
#[tokio::test]
async fn a_remote_branch_dropped_since_the_last_fetch_is_the_leases_refusal() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();
    let shown = a_branch_over_there(&mut work, "gone");
    bare.git(&["update-ref", "-d", "refs/heads/gone"]);

    let err =
        remote::delete_remote_branch(&exec, &work.path, "origin", "gone", &shown, NET, &cancel)
            .await
            .expect_err("nothing is there to hold the lease");
    assert_eq!(
        err.report().map(|report| report.kind),
        Some(ReportKind::MovedDelete),
        "{err}"
    );

    let tip = a_branch_over_there(&mut work, "twin");
    work.git(&["push", "origin", &format!("{tip}:refs/tags/twin")]);
    remote::delete_remote_branch(&exec, &work.path, "origin", "twin", &tip, NET, &cancel)
        .await
        .expect("the qualified name picks the branch");
    assert!(!bare.git_ok(&["rev-parse", "--verify", "refs/heads/twin"]));
    assert_eq!(
        bare.git(&["rev-parse", "refs/tags/twin"]),
        tip,
        "and the tag of the same name stays"
    );
}

/// Replace is a push and then the leased delete: refused, it stops with
/// both names over there and the tracking left on the old one.
#[tokio::test]
async fn a_replace_whose_delete_is_refused_leaves_both_names_and_the_tracking() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();
    let shown = a_branch_over_there(&mut work, "billing");
    let theirs = moved_over_there(&bare, "billing");

    let err = remote::replace_remote_branch(
        &exec,
        &work.path,
        "origin",
        "billing",
        "billing-v2",
        &shown,
        NET,
        &cancel,
    )
    .await
    .expect_err("the old name moved since the screen showed it");
    assert!(err.is_outdated(), "{err}");
    assert_eq!(
        bare.git(&["rev-parse", "billing"]),
        theirs,
        "the old name stays"
    );
    assert_eq!(
        bare.git(&["rev-parse", "billing-v2"]),
        shown,
        "beside the new one, which went up first"
    );
    assert_eq!(
        work.git(&["config", "branch.billing.merge"]),
        "refs/heads/billing",
        "and nothing was re-pointed"
    );
}

/// `Delete both` runs the local half first (`session::delete_branch_everywhere`
/// says why), so a refused lease stops it half-way: the branch is gone here
/// and the remote keeps theirs. The refusal is the outdated one a push
/// gives, and so is what follows it: a fetch, after which the commit the
/// screen showed is reached from the remote-tracking ref.
#[tokio::test(flavor = "multi_thread")]
async fn delete_both_refused_by_the_lease_keeps_the_commit_under_the_remote_tracking_ref() {
    let (mut bare, mut work) = origin_and_clone();
    let shown = a_branch_over_there(&mut work, "topic");
    let theirs = moved_over_there(&bare, "topic");
    let (sink, session) = opened(&work).await;

    let id = session
        .delete_branch_everywhere(
            "topic".into(),
            "origin".into(),
            "topic".into(),
            false,
            shown.clone(),
        )
        .expect("accepted");
    let refused = write_answer(&sink, id).await;

    assert!(refused.is_some(), "the remote half is refused");
    assert!(
        !work.git_ok(&["rev-parse", "--verify", "refs/heads/topic"]),
        "the local half had already run"
    );
    assert_eq!(bare.git(&["rev-parse", "topic"]), theirs);

    assert_eq!(
        write_result(&sink, OperationKind::Fetch).await,
        None,
        "the refusal is caught up with, as a push's is"
    );
    assert_eq!(work.git(&["rev-parse", "origin/topic"]), theirs);
    assert!(
        work.git_ok(&["merge-base", "--is-ancestor", &shown, "origin/topic"]),
        "and the commit the screen showed is still reached from it"
    );
}

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

/// Why the UI asks before a first push onto a taken name: git does not
/// refuse a fast-forward, so somebody else's branch quietly moves and we
/// end up tracking it.
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

/// A name taken by commits this history never had is refused outright, so
/// "taken" alone cannot decide the UI's move: the commit `branch_tip` hands
/// back tells the two cases apart before anything is sent.
#[tokio::test]
async fn a_taken_name_holding_commits_of_its_own_is_refused() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

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

    // Only a leased overwrite can land here; a lease on a commit that is no
    // longer there is refused.
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

/// Writes a `pre-receive` hook into a bare repository that turns every
/// push away, in the shape a forge writes its own refusals in.
///
/// Offline stand-in for any far-side refusal: a protected branch, a
/// repository rule and a hook all arrive as the same `[remote rejected]`,
/// differing only in the sentence underneath.
fn decline_every_push(bare: &TestRepo, said: &str) {
    decline_every_push_watched(bare, said, || {});
}

/// The same hook, telling `on_busy` if the install has to wait out a
/// neighbour holding the file open ([`TestRepo::write_hook_watched`]).
fn decline_every_push_watched(bare: &TestRepo, said: &str, on_busy: impl FnOnce()) {
    bare.write_hook_watched(
        "pre-receive",
        &format!("echo \"error: {said}\" >&2\nexit 1\n"),
        on_busy,
    );
}

#[tokio::test]
async fn a_branch_the_far_side_keeps_comes_back_as_a_refusal_with_its_words() {
    let (bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();
    decline_every_push(
        &bare,
        "GH006: Protected branch update failed for refs/heads/main.",
    );
    // Leased to what is there: the lease passes, so the hook is what says no.
    let tip = work.git(&["rev-parse", "origin/main"]);

    let err = remote::delete_remote_branch(&exec, &work.path, "origin", "main", &tip, NET, &cancel)
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
    // the prose: the far side's lines and git's own last word.
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

/// A tag refusal is a report like a branch's (デザイン規約 §答えの要らない報せ).
/// Both halves are checked: sending and deleting are different commands
/// (`push` / `push --delete`), so one being classified says nothing of the other.
#[tokio::test]
async fn a_tag_the_far_side_keeps_is_reported_the_way_a_branch_is() {
    let (bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();
    work.git(&["tag", "v1.0"]);
    remote::push_tag(&exec, &work.path, "origin", "v1.0", "", NET, &cancel)
        .await
        .expect("the tag goes over while nothing is standing over it");
    decline_every_push(&bare, "Tag protection rules prevent this.");
    let shown = work.git(&["rev-parse", "v1.0"]);

    let err = remote::delete_remote_tag(&exec, &work.path, "origin", "v1.0", &shown, NET, &cancel)
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

/// The same refusal, installed while a neighbour holds the hook open for
/// writing — the window a `fork()` in another test opens over a file this
/// one has just written. A hook git cannot execute declines silently, so
/// the install is what has to wait (`TestRepo::write_hook`).
///
/// Linux only: POSIX only says `execve` *may* refuse a file open for
/// writing, and Linux does.
#[tokio::test]
#[cfg(target_os = "linux")]
async fn a_hook_a_neighbour_holds_open_still_says_why_the_push_was_refused() {
    let (bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();
    work.git(&["tag", "v1.0"]);
    remote::push_tag(&exec, &work.path, "origin", "v1.0", "", NET, &cancel)
        .await
        .expect("the tag goes over while nothing is standing over it");
    decline_every_push(&bare, "Tag protection rules prevent this.");
    let shown = work.git(&["rev-parse", "v1.0"]);

    // The install below rewrites this same inode, so the handle stays on it.
    let hook = bare.path.join(".git").join("hooks").join("pre-receive");
    let handle = std::fs::OpenOptions::new()
        .write(true)
        .open(&hook)
        .expect("hold the hook open for writing");

    // Let go only once the install says it is waiting, inside the scope. An
    // install that never looked leaves the handle held, so the push below
    // meets the busy hook and loses the words.
    let (busy, saw_busy) = std::sync::mpsc::channel();
    let mut held = Some(handle);
    let installing = &bare;
    std::thread::scope(|scope| {
        // `move`, so the thread owns the sender: an install that never
        // finds the hook busy has to end the wait below by dropping it.
        scope.spawn(move || {
            decline_every_push_watched(
                installing,
                "Tag protection rules prevent this.",
                move || busy.send(()).expect("report ETXTBSY"),
            );
        });
        if saw_busy.recv().is_ok() {
            held = None;
        }
    });

    let err = remote::delete_remote_tag(&exec, &work.path, "origin", "v1.0", &shown, NET, &cancel)
        .await
        .expect_err("the far side keeps the tag");
    let Some(report) = err.report() else {
        panic!("a tag the far side keeps is a report, not a failure of ours: {err}");
    };
    assert_eq!(
        report.reason, "Tag protection rules prevent this.",
        "the hook's own words, not git's parenthetical over a hook it could not run: {err}"
    );
    drop(held);
}

/// git's own summary is what marks the refusal a fetch answers — the
/// report with a move behind it.
#[tokio::test]
async fn a_push_that_is_only_out_of_date_reports_gits_own_advice() {
    let (bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

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
        report.reason.is_empty(),
        "nobody over there saw this push, so there is nobody to quote: the line under the heading \
         is the screen's own (`Words.writeReportedWhy`), and git's advice — written for somebody \
         at a terminal — is read in the log with the command it came from. Carried: {}",
        report.reason
    );
}

/// A second working clone of the same bare `origin`, with a commit of
/// its own already pushed: the far side that has moved on.
fn origin_that_moved_on(bare: &TestRepo, file: &str, message: &str) -> TestRepo {
    let mut other = TestRepo::init();
    other.git(&["remote", "add", "origin", &bare.file_url()]);
    other.git(&["fetch", "origin"]);
    other.git(&["switch", "--force-create", "main", "origin/main"]);
    other.commit_file(file, "far side\n", message);
    other.git(&["push", "origin", "main"]);
    other
}

/// With `pull.rebase` set either way, a conflicting pull stops the way a
/// merge or rebase does — a landing, not a failure.
#[tokio::test]
async fn a_pull_that_conflicts_stops_with_the_operation_standing() {
    for (reconcile, marker) in [("false", "MERGE_HEAD"), ("true", "rebase-merge")] {
        let (bare, mut work) = origin_and_clone();
        let (exec, cancel) = env();
        work.git(&["branch", "--set-upstream-to=origin/main", "main"]);
        work.git(&["config", "pull.rebase", reconcile]);
        let _far = origin_that_moved_on(&bare, "a.txt", "their line");
        work.commit_file("a.txt", "our line\n", "ours");

        assert_eq!(
            remote::pull(&exec, &work.path, NET, &cancel)
                .await
                .expect("a conflict is an answer, not a failure"),
            Landing::Stopped,
            "pull.rebase={reconcile}"
        );
        assert!(
            work.path.join(".git").join(marker).exists(),
            "git left the {marker} standing to be finished"
        );
    }
}

/// A refusal stays a failure: no tracking information and a tree the pull
/// would write over both exit 1 with nothing standing, unlike the stop above.
#[tokio::test]
async fn a_pull_with_nothing_to_pull_from_is_a_failure() {
    let (_bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();
    work.git(&["switch", "--create", "solo"]);

    let err = remote::pull(&exec, &work.path, NET, &cancel)
        .await
        .expect_err("a branch with no tracking information has nowhere to go");
    assert!(
        matches!(err, GitError::Failed { code: 1, .. }),
        "git spends 1 on it: {err}"
    );
}

#[tokio::test]
async fn a_pull_brings_the_upstream_in_and_moves_the_branch() {
    let (bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();
    work.git(&["branch", "--set-upstream-to=origin/main", "main"]);
    let _far = origin_that_moved_on(&bare, "b.txt", "over there");

    assert_eq!(
        remote::pull(&exec, &work.path, NET, &cancel)
            .await
            .expect("pull"),
        Landing::Done
    );
    assert_eq!(
        work.git(&["log", "-1", "--format=%s", "main"]),
        "over there",
        "the branch is on what the far side holds"
    );
    assert_eq!(
        work.git(&["log", "-1", "--format=%s", "origin/main"]),
        "over there",
        "and the reading of the remote came down with it"
    );
}

/// What git itself does behind the fixed command lines these functions hand
/// it (the lines are unit-tested beside `remote::list` and `remote::fetch`).
/// What each reads of git's answer is held above and by `remote::refusal`'s
/// own tests, so only the full gate runs these.
mod periodic {
    use super::*;

    #[tokio::test]
    #[ignore = "git's own tracking-ref cleanup: not worth the pre-merge run"]
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
        let tip = work.git(&["rev-parse", "temp"]);

        remote::delete_remote_branch(&exec, &work.path, "origin", "temp", &tip, NET, &cancel)
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

    /// Why `Delete both` cannot run the remote half first: `push --delete`
    /// takes the remote-tracking ref with it, and a branch merged only into
    /// that upstream is then refused by the plain `branch --delete` —
    /// leaving the remote gone and the branch here.
    #[tokio::test]
    #[ignore = "git's own merged check behind a fixed order: not worth the pre-merge run"]
    async fn a_branch_whose_upstream_went_first_is_refused_as_not_merged() {
        let (_bare, mut work) = origin_and_clone();
        let (exec, cancel) = env();
        let shown = a_branch_over_there(&mut work, "topic");

        remote::delete_remote_branch(&exec, &work.path, "origin", "topic", &shown, NET, &cancel)
            .await
            .expect("the remote half goes");
        assert!(!work.git_ok(&["rev-parse", "--verify", "refs/remotes/origin/topic"]));

        let err = platitude_core::branch::delete(&exec, &work.path, "topic", false, &cancel)
            .await
            .expect_err("merged into an upstream that is no longer here");
        assert!(err.to_string().contains("not fully merged"), "{err}");
    }

    /// Adding a remote is bookkeeping: a URL that goes nowhere is
    /// accepted, which is why a failed push leaves the remote in place and
    /// `set-url` is the way back.
    #[tokio::test]
    #[ignore = "what git records behind a fixed command line: not worth the pre-merge run"]
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

        let spec = remote::plan_publish(&exec, &work.path, "fork", "main", "", &cancel)
            .await
            .expect("plan publish");
        remote::push(&exec, &work.path, &spec, NET, &cancel)
            .await
            .expect("push to the corrected remote");
    }

    /// The remote stays after its first push fails: undoing the add would
    /// throw away the only part of the answer worth keeping.
    #[tokio::test]
    #[ignore = "what git keeps after a failed push: not worth the pre-merge run"]
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
            // `--default` so an unset key is an empty answer; the exit
            // code would read to the harness as a broken command.
            work.git(&["config", "--default", "", "--get", "branch.main.remote"]),
            "",
            "and a push that never landed recorded no upstream"
        );
    }

    /// With `pull.rebase` and `pull.ff` unset, git refuses a divergence with
    /// exit 128 and nothing started; its own words, which name the setting,
    /// reach the reader (デザイン規約 §git が言ったことを読む場所).
    #[tokio::test]
    #[ignore = "git's own refusal text and exit code: not worth the pre-merge run"]
    async fn a_divergence_git_has_no_orders_for_is_a_failure() {
        let (bare, mut work) = origin_and_clone();
        let (exec, cancel) = env();
        work.git(&["branch", "--set-upstream-to=origin/main", "main"]);
        let _far = origin_that_moved_on(&bare, "a.txt", "their line");
        work.commit_file("a.txt", "our line\n", "ours");

        let err = remote::pull(&exec, &work.path, NET, &cancel)
            .await
            .expect_err("git will not choose between merging and rebasing");
        let GitError::Failed { code, stderr, .. } = &err else {
            panic!("the refusal is git's own: {err}");
        };
        assert_eq!(*code, 128);
        assert!(
            stderr.contains("divergent branches"),
            "git says what it needs: {stderr}"
        );
        assert!(
            !work.path.join(".git").join("MERGE_HEAD").exists(),
            "and nothing was started, so this is the failure it reads as"
        );
    }

    #[tokio::test]
    #[ignore = "git's own dirty-tree refusal: not worth the pre-merge run"]
    async fn a_pull_over_changes_it_would_write_on_is_a_failure() {
        let (bare, mut work) = origin_and_clone();
        let (exec, cancel) = env();
        work.git(&["branch", "--set-upstream-to=origin/main", "main"]);
        let _far = origin_that_moved_on(&bare, "a.txt", "their line");
        work.write_file("a.txt", "uncommitted\n");

        remote::pull(&exec, &work.path, NET, &cancel)
            .await
            .expect_err("git will not write over what is not committed");
        assert!(
            !work.git_ok(&["rev-parse", "--verify", "-q", "MERGE_HEAD"]),
            "and it left nothing standing, so the words are the whole answer"
        );
    }
}
