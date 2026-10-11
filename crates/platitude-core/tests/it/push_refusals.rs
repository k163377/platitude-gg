//! What a push that did not go was, told apart for the screen to say in
//! its own words (デザイン規約 §答えの要らない報せ): the `pre-push` hook here,
//! and the far side's own rules and settings — each set up for real on a
//! bare repository over `file://`.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::sync::Arc;

use crate::support::TestRepo;
use crate::support::exec::{Said, env, logged, observed_env};
use crate::support::remote::origin_and_clone;
const NET: std::time::Duration = remote::DEFAULT_NETWORK_TIMEOUT;
use platitude_core::GitError;
use platitude_core::process::{CommandEnd, Kept};
use platitude_core::remote::{self, PushForce, PushSpec};
use platitude_core::report::{RemoteWhy, ReportKind};

/// `main` to `remote`'s `main`, forced or not.
async fn push_main(work: &TestRepo, remote: &str, force: PushForce) -> Result<(), GitError> {
    let (exec, cancel) = env();
    push_main_with(&exec, work, remote, force, &cancel).await
}

async fn push_main_with(
    exec: &platitude_core::GitExecutor,
    work: &TestRepo,
    remote: &str,
    force: PushForce,
    cancel: &tokio_util::sync::CancellationToken,
) -> Result<(), GitError> {
    let spec = PushSpec {
        remote: remote.into(),
        local: "main".into(),
        remote_branch: "main".into(),
        set_upstream: false,
        force,
    };
    remote::push(exec, &work.path, &spec, NET, cancel).await
}

fn kind_of(err: &GitError) -> Option<ReportKind> {
    err.report().map(|report| report.kind)
}

/// A `pre-push` hook that writes a line to each stream and says no.
const SAYS_NO: &str = "echo \"lint: out on stdout\"\necho \"lint: err on stderr\" >&2\nexit 1\n";

/// The hook here is what said no — a send and a delete alike — and its
/// words, the stdout half too, are under the push's row in the log.
#[tokio::test]
async fn a_push_the_pre_push_hook_turns_down_is_reported_as_the_hooks() {
    let (bare, mut work) = origin_and_clone();
    work.commit_file("b.txt", "ours\n", "work of our own");
    work.write_hook("pre-push", SAYS_NO);
    let said = Arc::new(Said::default());
    let (exec, cancel) = observed_env(said.clone(), Kept::Asked);

    let err = push_main_with(&exec, &work, "origin", PushForce::None, &cancel)
        .await
        .expect_err("the hook says no");
    let report = err.report().expect("the hook's no is a report");
    assert_eq!(report.kind, ReportKind::PrePush, "{err}");
    assert_eq!(
        (report.remote.as_str(), report.name.as_str()),
        ("origin", "main")
    );
    assert!(!err.is_outdated(), "nothing a fetch answers");
    let row = said
        .message_of(CommandEnd::Exited(1))
        .expect("the push's row failed");
    assert!(
        row.contains("lint: out on stdout") && row.contains("lint: err on stderr"),
        "both of the hook's streams are under the row: {row}"
    );

    let tip = work.git(&["rev-parse", "origin/main"]);
    let err = remote::delete_remote_branch(&exec, &work.path, "origin", "main", &tip, NET, &cancel)
        .await
        .expect_err("the hook says no to a delete too");
    assert_eq!(kind_of(&err), Some(ReportKind::PrePushDelete), "{err}");
    drop(bare);
}

/// Where `core.hooksPath` keeps the hook (husky's `.husky/_`), the hook
/// is found there.
#[tokio::test]
async fn a_hook_kept_elsewhere_by_hooks_path_is_found_there() {
    let (_bare, mut work) = origin_and_clone();
    work.commit_file("b.txt", "ours\n", "work of our own");
    work.write_hook("pre-push", SAYS_NO);
    std::fs::rename(work.path.join(".git/hooks"), work.path.join(".husky"))
        .expect("move the hooks");
    work.git(&["config", "core.hooksPath", ".husky"]);

    let err = push_main(&work, "origin", PushForce::None)
        .await
        .expect_err("the hook says no");
    assert_eq!(kind_of(&err), Some(ReportKind::PrePush), "{err}");
}

/// git's own answer to a refspec, before any hook runs, stays a failure
/// for the log — a hook being there says nothing of who said no.
#[tokio::test]
async fn a_refspec_git_turned_down_is_not_the_hooks() {
    let (_bare, work) = origin_and_clone();
    work.write_hook("pre-push", SAYS_NO);
    let (exec, cancel) = env();
    let spec = PushSpec {
        remote: "origin".into(),
        local: "no-such-branch".into(),
        remote_branch: "no-such-branch".into(),
        set_upstream: false,
        force: PushForce::None,
    };

    let err = remote::push(&exec, &work.path, &spec, NET, &cancel)
        .await
        .expect_err("nothing to send");
    assert!(
        matches!(err, GitError::Failed { .. }),
        "git's own refusal goes to the log: {err:?}"
    );
}

/// The same for a name git cannot tell apart — a branch and a tag both
/// called `main` (`src refspec main matches more than one`).
#[tokio::test]
async fn an_ambiguous_name_git_turned_down_is_not_the_hooks() {
    let (_bare, mut work) = origin_and_clone();
    work.commit_file("b.txt", "ours\n", "work of our own");
    work.git(&["tag", "main", "HEAD~1"]);
    work.write_hook("pre-push", SAYS_NO);

    let err = push_main(&work, "origin", PushForce::None)
        .await
        .expect_err("git cannot tell which main");
    assert!(
        matches!(err, GitError::Failed { .. }),
        "git's own refusal goes to the log: {err:?}"
    );
}

/// `receive.denyNonFastForwards` answers `[remote rejected]
/// (non-fast-forward)`: the far side's rule, which no fetch here answers —
/// not the push this end found out of date.
#[tokio::test]
async fn a_far_side_that_takes_no_rewrite_is_not_out_of_date() {
    let (mut bare, mut work) = origin_and_clone();
    bare.git(&["config", "receive.denyNonFastForwards", "true"]);
    work.git(&["commit", "--amend", "-m", "root, reworded"]);
    let leased = work.git(&["rev-parse", "origin/main"]);

    let err = push_main(
        &work,
        "origin",
        PushForce::WithLease {
            expect: Some(leased),
        },
    )
    .await
    .expect_err("the far side takes no rewrite");
    assert_eq!(
        kind_of(&err),
        Some(ReportKind::RemoteUpdate(RemoteWhy::Rewrite)),
        "{err}"
    );
    assert!(!err.is_outdated(), "no fetch puts this right");
}

/// The far side's own settings over a delete: no deletes at all, and the
/// branch its `HEAD` names.
#[tokio::test]
async fn a_far_side_that_keeps_a_branch_says_which_rule() {
    let (mut bare, mut work) = origin_and_clone();
    work.git(&["push", "origin", "main:side"]);
    work.git(&["fetch", "origin"]);
    let (exec, cancel) = env();

    let tip = work.git(&["rev-parse", "origin/main"]);
    let err = remote::delete_remote_branch(&exec, &work.path, "origin", "main", &tip, NET, &cancel)
        .await
        .expect_err("main is what its HEAD names");
    assert_eq!(
        kind_of(&err),
        Some(ReportKind::RemoteDelete(RemoteWhy::Current)),
        "{err}"
    );

    bare.git(&["config", "receive.denyDeletes", "true"]);
    let err = remote::delete_remote_branch(&exec, &work.path, "origin", "side", &tip, NET, &cancel)
        .await
        .expect_err("it deletes no branch");
    assert_eq!(
        kind_of(&err),
        Some(ReportKind::RemoteDelete(RemoteWhy::NoDeletes)),
        "{err}"
    );
}

/// More than `receive.maxInputSize`, and a ref another git holds the lock
/// of over there.
#[tokio::test]
async fn a_far_side_that_cannot_take_it_now_says_why() {
    let (mut bare, mut work) = origin_and_clone();
    bare.git(&["config", "receive.maxInputSize", "10"]);
    work.commit_file("big.txt", &"x".repeat(4096), "something big");
    let err = push_main(&work, "origin", PushForce::None)
        .await
        .expect_err("too large to unpack");
    assert_eq!(
        kind_of(&err),
        Some(ReportKind::RemoteUpdate(RemoteWhy::TooLarge)),
        "{err}"
    );

    bare.git(&["config", "--unset", "receive.maxInputSize"]);
    let lock = bare.path.join(".git/refs/heads/main.lock");
    std::fs::write(&lock, "").expect("lock");
    let err = push_main(&work, "origin", PushForce::None)
        .await
        .expect_err("another git holds the ref");
    std::fs::remove_file(&lock).expect("unlock");
    assert_eq!(
        kind_of(&err),
        Some(ReportKind::RemoteUpdate(RemoteWhy::Locked)),
        "{err}"
    );
}

/// A remote with a working tree of its own and `main` checked out in it.
#[tokio::test]
async fn a_far_side_with_the_branch_checked_out_says_so() {
    let (_bare, mut work) = origin_and_clone();
    let mut checked_out = TestRepo::init();
    checked_out.git(&["fetch", &work.file_url(), "main"]);
    checked_out.git(&["reset", "--hard", "FETCH_HEAD"]);
    work.git(&["remote", "add", "beside", &checked_out.file_url()]);
    work.git(&["fetch", "beside"]);
    work.commit_file("b.txt", "ours\n", "work of our own");

    let err = push_main(&work, "beside", PushForce::None)
        .await
        .expect_err("the branch is checked out over there");
    assert_eq!(
        kind_of(&err),
        Some(ReportKind::RemoteUpdate(RemoteWhy::CheckedOut)),
        "{err}"
    );
}

/// The hook's no is read off git's own record of the push: no other git
/// runs to tell it, whatever the log keeps.
#[tokio::test]
async fn naming_the_hook_runs_no_git_besides_the_push() {
    let (_bare, mut work) = origin_and_clone();
    work.commit_file("b.txt", "ours\n", "work of our own");
    work.write_hook("pre-push", SAYS_NO);
    let (exec, log, cancel) = logged();

    let err = push_main_with(&exec, &work, "origin", PushForce::None, &cancel)
        .await
        .expect_err("the hook says no");
    assert_eq!(kind_of(&err), Some(ReportKind::PrePush), "{err}");
    let rows = log.displays();
    assert!(
        rows.iter().all(|row| row.starts_with("git push")),
        "no git but the push: {rows:?}"
    );
}

/// git from 2.54 runs a hook the config names for the event
/// (`hook.<name>.event`) beside the hooks directory's — the same no as a
/// file's. The minimum reads no such hook and the push goes; whichever git
/// this is, the answer follows what it did.
#[tokio::test]
async fn a_pre_push_hook_the_config_names_is_the_hooks_too() {
    let (_bare, mut work) = origin_and_clone();
    work.commit_file("b.txt", "ours\n", "work of our own");
    work.git(&["config", "hook.lint.event", "pre-push"]);
    work.git(&[
        "config",
        "hook.lint.command",
        "echo \"lint: the config's hook says no\" >&2; exit 1",
    ]);
    let runs_it = work.git_ok(&["hook", "list", "pre-push"]);

    let pushed = push_main(&work, "origin", PushForce::None).await;
    match (runs_it, pushed) {
        (true, Err(err)) => assert_eq!(kind_of(&err), Some(ReportKind::PrePush), "{err}"),
        (false, Ok(())) => {}
        (runs_it, pushed) => panic!("git runs the config's hook: {runs_it}, the push: {pushed:?}"),
    }
}

/// `push.gpgSign=true` against a far side that takes no signed push (no
/// `receive.certNonceSeed` there): git dies before sending — a send and a
/// delete alike.
#[tokio::test]
async fn a_signed_push_the_far_side_does_not_take_says_so() {
    let (_bare, mut work) = origin_and_clone();
    work.commit_file("b.txt", "ours\n", "work of our own");
    work.git(&["config", "push.gpgSign", "true"]);
    let (exec, cancel) = env();

    let err = push_main_with(&exec, &work, "origin", PushForce::None, &cancel)
        .await
        .expect_err("the far side takes no signed push");
    assert_eq!(
        kind_of(&err),
        Some(ReportKind::SignedPushUntaken { deleting: false }),
        "{err}"
    );
    let tip = work.git(&["rev-parse", "origin/main"]);
    let err = remote::delete_remote_branch(&exec, &work.path, "origin", "main", &tip, NET, &cancel)
        .await
        .expect_err("nor a signed delete");
    assert_eq!(
        kind_of(&err),
        Some(ReportKind::SignedPushUntaken { deleting: true }),
        "{err}"
    );
}

/// The far side takes signed pushes, and the key to sign with is not
/// there: the certificate cannot be signed.
#[tokio::test]
async fn a_push_whose_certificate_cannot_be_signed_says_so() {
    let (mut bare, mut work) = origin_and_clone();
    bare.git(&["config", "receive.certNonceSeed", "a seed of our own"]);
    work.commit_file("b.txt", "ours\n", "work of our own");
    let key = work.path.join("no-such-key.pub");
    work.git(&["config", "push.gpgSign", "true"]);
    work.git(&["config", "gpg.format", "ssh"]);
    work.git(&["config", "user.signingkey", &key.to_string_lossy()]);

    let err = push_main(&work, "origin", PushForce::None)
        .await
        .expect_err("nothing signs the certificate");
    assert_eq!(
        kind_of(&err),
        Some(ReportKind::PushUnsigned { deleting: false }),
        "{err}"
    );
}

/// The same with an openpgp key, whose signing program never ran: git
/// names the certificate it could not sign.
#[tokio::test]
async fn a_push_whose_signing_program_fails_says_so() {
    let (mut bare, mut work) = origin_and_clone();
    bare.git(&["config", "receive.certNonceSeed", "a seed of our own"]);
    work.commit_file("b.txt", "ours\n", "work of our own");
    work.git(&["config", "push.gpgSign", "true"]);
    work.git(&["config", "user.signingkey", "0123456789ABCDEF"]);
    work.git(&["config", "gpg.program", "no-such-signing-program"]);

    let err = push_main(&work, "origin", PushForce::None)
        .await
        .expect_err("nothing signs the certificate");
    assert!(
        err.to_string()
            .contains("failed to sign the push certificate"),
        "the certificate's own words: {err}"
    );
    assert_eq!(
        kind_of(&err),
        Some(ReportKind::PushUnsigned { deleting: false }),
        "{err}"
    );
}
