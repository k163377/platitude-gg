//! What a push that did not go was, told apart for the screen to say in
//! its own words (デザイン規約 §答えの要らない報せ): a remote that moved on,
//! the `pre-push` hook here, and the far side's own rules and settings —
//! each set up for real on a bare repository over `file://`.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::sync::Arc;

use crate::support::TestRepo;
use crate::support::exec::{Said, env, logged, observed_env};
use crate::support::remote::{far, moved_over_there, origin_and_clone};
use platitude_core::GitError;
use platitude_core::process::{CommandEnd, Kept};
use platitude_core::remote::{self, PushForce, PushSpec};
use platitude_core::report::{RemoteWhy, ReportKind};

/// A `file://` remote answers instantly; the budget just has to exist.
const NET: std::time::Duration = remote::DEFAULT_NETWORK_TIMEOUT;

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
        target: far(remote, "main"),
        local: "main".into(),
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
    let err = remote::delete_remote_branch(
        &exec,
        &work.path,
        &far("origin", "main"),
        &tip,
        NET,
        &cancel,
    )
    .await
    .expect_err("the hook says no to a delete too");
    assert_eq!(kind_of(&err), Some(ReportKind::PrePushDelete), "{err}");
    drop(bare);
}

/// git's own answer to a refspec, before any hook runs, stays a failure
/// for the log — a hook being there says nothing of who said no.
#[tokio::test]
async fn a_refspec_git_turned_down_is_not_the_hooks() {
    let (_bare, work) = origin_and_clone();
    work.write_hook("pre-push", SAYS_NO);
    let (exec, cancel) = env();
    let spec = PushSpec {
        target: far("origin", "no-such-branch"),
        local: "no-such-branch".into(),
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
    let err = remote::delete_remote_branch(
        &exec,
        &work.path,
        &far("origin", "main"),
        &tip,
        NET,
        &cancel,
    )
    .await
    .expect_err("main is what its HEAD names");
    assert_eq!(
        kind_of(&err),
        Some(ReportKind::RemoteDelete(RemoteWhy::Current)),
        "{err}"
    );

    bare.git(&["config", "receive.denyDeletes", "true"]);
    let err = remote::delete_remote_branch(
        &exec,
        &work.path,
        &far("origin", "side"),
        &tip,
        NET,
        &cancel,
    )
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

#[tokio::test]
async fn a_non_fast_forward_push_is_refused_until_forced() {
    let (mut bare, mut work) = origin_and_clone();
    moved_over_there(&bare, "main", "theirs.txt", "their work");

    work.git(&["commit", "--amend", "-m", "rewritten root"]);
    let err = push_main(&work, "origin", PushForce::None)
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
    let leased = PushForce::WithLease {
        expect: Some(stale),
    };
    let err = push_main(&work, "origin", leased)
        .await
        .expect_err("stale lease");
    assert_eq!(
        kind_of(&err),
        Some(ReportKind::Moved),
        "a lease the remote left says the branch moved, not that it is ahead: {err}"
    );
    assert!(err.is_outdated(), "and is caught up with all the same");

    push_main(&work, "origin", PushForce::Force)
        .await
        .expect("forced push");
    assert_eq!(
        bare.git(&["log", "-1", "--format=%s", "main"]),
        "rewritten root"
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

    let err = remote::delete_remote_branch(
        &exec,
        &work.path,
        &far("origin", "main"),
        &tip,
        NET,
        &cancel,
    )
    .await
    .expect_err("the far side keeps main");
    let Some(report) = err.report() else {
        panic!("a refusal the far side made is told apart from a failure of ours: {err}");
    };
    assert_eq!(report.remote, "origin");
    assert_eq!(report.name, "main");
    assert_eq!(
        report.kind,
        ReportKind::RemoteDelete(RemoteWhy::Rule),
        "what was asked for was the removal, and a hook over there said no"
    );
    assert_eq!(report.fact, "", "the screen says it in its own words");
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
    decline_every_push(&bare, "Changes must be made through a pull request.");
    work.commit_file("b.txt", "ours\n", "work of our own");

    let err = push_main(&work, "origin", PushForce::None)
        .await
        .expect_err("the hook declines");
    let Some(report) = err.report() else {
        panic!("a fast-forward the far side declined is neither outdated nor ours: {err}");
    };
    assert_eq!(
        report.kind,
        ReportKind::RemoteUpdate(RemoteWhy::Rule),
        "this one was sending, not removing"
    );
    assert!(
        err.to_string()
            .contains("Changes must be made through a pull request."),
        "the far side's words are the log's: {err}"
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
    assert_eq!(report.kind, ReportKind::RemoteDelete(RemoteWhy::Rule));
    assert_eq!(report.remote, "origin");
    assert_eq!(report.name, "v1.0", "the tag names itself, not a branch");

    work.commit_file("c.txt", "more\n", "something to tag");
    work.git(&["tag", "v1.1"]);
    let err = remote::push_tag(&exec, &work.path, "origin", "v1.1", "", NET, &cancel)
        .await
        .expect_err("the far side keeps its tags");
    let Some(report) = err.report() else {
        panic!("a tag the far side would not take is a report as well: {err}");
    };
    assert_eq!(report.kind, ReportKind::RemoteUpdate(RemoteWhy::Rule));
    assert_eq!(report.name, "v1.1");
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
    assert!(
        err.report().is_some(),
        "a tag the far side keeps is a report, not a failure of ours: {err}"
    );
    assert!(
        err.to_string()
            .contains("Tag protection rules prevent this."),
        "the hook ran and said its words, which the log keeps — not a hook git could not run: {err}"
    );
    drop(held);
}

/// git's own summary is what marks the refusal a fetch answers — the
/// report with a move behind it.
#[tokio::test]
async fn a_push_that_is_only_out_of_date_reports_gits_own_advice() {
    let (bare, mut work) = origin_and_clone();
    moved_over_there(&bare, "main", "theirs.txt", "pushed while you slept");
    work.commit_file("ours.txt", "ours\n", "work of our own");

    let err = push_main(&work, "origin", PushForce::None)
        .await
        .expect_err("the remote holds what we have not got");
    let Some(report) = err.report() else {
        panic!("being out of date is a report of its own: {err}");
    };
    assert_eq!(report.kind, ReportKind::Outdated);
    assert_eq!(report.remote, "origin");
    assert_eq!(report.name, "main");
    assert!(
        report.fact.is_empty(),
        "the line under the heading is the screen's own (`Words.writeReportedWhy`), and git's \
         advice — written for somebody at a terminal — is read in the log with the command it \
         came from. Carried: {}",
        report.fact
    );
}

/// What git itself does behind the fixed command lines the tests above
/// pass — left to the full gate (rules-refs/core.md `periodic`).
mod periodic {
    use super::*;

    /// Where `core.hooksPath` keeps the hook (husky's `.husky/_`), the hook
    /// is found there.
    #[tokio::test]
    #[ignore = "git finding the hook under core.hooksPath: not worth the pre-merge run"]
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

    /// git from 2.54 runs a hook the config names for the event
    /// (`hook.<name>.event`) beside the hooks directory's — the same no as a
    /// file's. The minimum reads no such hook and the push goes; whichever git
    /// this is, the answer follows what it did.
    #[tokio::test]
    #[ignore = "git 2.54's config-named hooks, by whichever git this machine has: not worth the pre-merge run"]
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
            (runs_it, pushed) => {
                panic!("git runs the config's hook: {runs_it}, the push: {pushed:?}")
            }
        }
    }
}
