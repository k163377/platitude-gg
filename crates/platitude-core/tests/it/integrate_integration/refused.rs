//! A rewrite something here turned down after git started it: the
//! `pre-rebase` hook before anything moved, and a commit the replay remade
//! that could not be signed or that a hook said no to — put back where it
//! began. Each is a report, and the rebase's row in the log is a failure
//! with git's words under it, where the report sends the reader.

use std::sync::Arc;

use crate::support::exec::observed_env;
use crate::support::integrate::{current_op, helper};
use crate::support::{Said, TestRepo, info};
use platitude_core::GitError;
use platitude_core::ReportKind;
use platitude_core::integrate::{self, RebaseOptions};
use platitude_core::process::{CommandEnd, Kept};
use platitude_core::sequencer;

/// The failed `rebase` row's words, whichever code it ended with — 1 at
/// 2.55, 128 for the `pre-rebase` refusal at the minimum.
fn failed_row(said: &Said) -> String {
    said.0
        .lock()
        .unwrap()
        .iter()
        .find(|(end, _)| matches!(end, CommandEnd::Exited(code) if *code != 0))
        .map(|(_, words)| words.clone())
        .expect("a failed row")
}

fn kind_of(err: &GitError) -> Option<ReportKind> {
    err.report().map(|report| report.kind)
}

/// Three commits on `main`; answers the middle one.
fn three_commits() -> (TestRepo, String) {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let middle = repo.commit_file_id("b.txt", "two\n", "middle");
    repo.commit_file("c.txt", "three\n", "last");
    (repo, middle)
}

/// Runs the plan for `edit` on `target`, watched.
async fn run_plan(
    repo: &TestRepo,
    target: &str,
    edit: sequencer::Edit,
    said: &Arc<Said>,
) -> GitError {
    let (exec, cancel) = observed_env(said.clone(), Kept::Asked);
    let plan = sequencer::plan_edit(&exec, &repo.path, target, edit, &cancel)
        .await
        .expect("plan");
    sequencer::rebase_interactive(
        &exec,
        &info(repo).await,
        &plan.upstream,
        &plan.steps,
        &plan.options(),
        &helper(),
        &cancel,
    )
    .await
    .expect_err("something here says no")
}

#[tokio::test]
async fn a_pre_rebase_hook_that_says_no_is_a_report() {
    let (mut repo, middle) = three_commits();
    // Somewhere to go that is not already under main: a rebase with
    // nothing to do ends before the hook runs.
    repo.git(&["switch", "-c", "base", "HEAD~2"]);
    repo.commit_file("x.txt", "elsewhere\n", "base moves on");
    repo.git(&["switch", "main"]);
    repo.write_hook("pre-rebase", "echo \"no rebases on main\" >&2\nexit 1\n");
    let before = repo.git(&["rev-parse", "HEAD"]);

    let said = Arc::new(Said::default());
    let (exec, cancel) = observed_env(said.clone(), Kept::Asked);
    let err = integrate::rebase(
        &exec,
        &repo.path,
        "base",
        &RebaseOptions::default(),
        &cancel,
    )
    .await
    .expect_err("the hook says no");
    assert_eq!(kind_of(&err), Some(ReportKind::RewritePreRebase), "{err}");
    assert!(
        failed_row(&said).contains("no rebases on main"),
        "the hook's words are under the failed row"
    );

    let said = Arc::new(Said::default());
    let err = run_plan(&repo, &middle, sequencer::Edit::Drop, &said).await;
    assert_eq!(kind_of(&err), Some(ReportKind::RewritePreRebase), "{err}");
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), before, "nothing moved");
    assert_eq!(current_op(&repo).await, None);
}

/// Dropping the middle commit remakes the last, which cannot be signed
/// with a key that is not there: the replay is put back.
#[tokio::test]
async fn a_replay_with_no_signing_key_named_is_put_back_and_reported() {
    let (mut repo, middle) = three_commits();
    repo.git(&["config", "commit.gpgsign", "true"]);
    repo.git(&["config", "gpg.format", "ssh"]);
    let before = repo.git(&["rev-parse", "HEAD"]);

    // git dies (128) inside the rebase's own process, the replay left
    // standing — not the exit 1 a stop answers with.
    let said = Arc::new(Said::default());
    let err = run_plan(&repo, &middle, sequencer::Edit::Drop, &said).await;
    assert_eq!(kind_of(&err), Some(ReportKind::RewriteUnsigned), "{err}");
    assert_eq!(current_op(&repo).await, None, "put back, nothing standing");
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), before, "where it began");
}

#[tokio::test]
async fn a_replay_that_could_not_sign_is_put_back_and_reported() {
    let (mut repo, middle) = three_commits();
    repo.git(&["config", "commit.gpgsign", "true"]);
    repo.git(&["config", "gpg.format", "ssh"]);
    let key = repo.path.join("no-such-key.pub");
    repo.git(&["config", "user.signingkey", &key.to_string_lossy()]);
    let before = repo.git(&["rev-parse", "HEAD"]);

    let said = Arc::new(Said::default());
    let err = run_plan(&repo, &middle, sequencer::Edit::Drop, &said).await;
    assert_eq!(kind_of(&err), Some(ReportKind::RewriteUnsigned), "{err}");
    assert_eq!(current_op(&repo).await, None, "put back, nothing standing");
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), before, "where it began");
    assert!(
        failed_row(&said).contains("failed to write commit object"),
        "git's words are under the failed row"
    );
}

/// A reword's new message turned down by the `commit-msg` hook — the
/// `exec` that writes it runs every commit hook.
#[tokio::test]
async fn a_reword_a_commit_hook_turns_down_is_put_back_and_reported() {
    let (mut repo, middle) = three_commits();
    repo.write_hook("commit-msg", "echo \"subject must say why\" >&2\nexit 1\n");
    let before = repo.git(&["rev-parse", "HEAD"]);

    let said = Arc::new(Said::default());
    let err = run_plan(
        &repo,
        &middle,
        sequencer::Edit::Reword("a subject the hook will not have\n".into()),
        &said,
    )
    .await;
    assert_eq!(kind_of(&err), Some(ReportKind::RewriteCommitHook), "{err}");
    assert_eq!(current_op(&repo).await, None, "put back, nothing standing");
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), before, "where it began");
    assert!(
        failed_row(&said).contains("subject must say why"),
        "the hook's words are under the failed row"
    );
}
