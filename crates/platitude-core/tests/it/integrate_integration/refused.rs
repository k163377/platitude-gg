//! A rewrite something here turned down after git started it: the
//! `pre-rebase` hook before anything moved, and a commit the replay remade
//! that could not be signed or that a hook said no to — put back where it
//! began. Each is a report, and the rebase's row in the log is a failure
//! with git's words under it, where the report sends the reader.

use std::sync::Arc;

use crate::support::exec::{isolated_global, observed_env};
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

/// A user's `trace2.eventBrief` would leave the times out of git's record,
/// and with them what tells the replay's own `git commit` from a git a hook
/// ran: the record is asked for whole all the same.
#[tokio::test]
async fn a_brief_trace_setting_still_names_a_reworded_commits_hook() {
    let (mut repo, middle) = three_commits();
    repo.write_hook("commit-msg", "exit 1\n");
    repo.git(&["config", "--global", "trace2.eventBrief", "true"]);

    let exec = isolated_global(repo.global_config());
    let cancel = tokio_util::sync::CancellationToken::new();
    let edit = sequencer::Edit::Reword("a subject the hook will not have\n".into());
    let plan = sequencer::plan_edit(&exec, &repo.path, &middle, edit, &cancel)
        .await
        .expect("plan");
    let err = sequencer::rebase_interactive(
        &exec,
        &info(&repo).await,
        &plan.upstream,
        &plan.steps,
        &plan.options(),
        &helper(),
        &cancel,
    )
    .await
    .expect_err("the hook says no");
    assert_eq!(kind_of(&err), Some(ReportKind::RewriteCommitHook), "{err}");
}

/// Another git holding a lock the replayed pick needs (`CHERRY_PICK_HEAD`'s,
/// which the abort does not take): the pick stops with the lock's words,
/// and the replay is put back — the reader presses again once it is gone.
#[tokio::test]
async fn a_replay_another_gits_lock_stops_is_put_back_and_reported() {
    let (mut repo, middle) = three_commits();
    let before = repo.git(&["rev-parse", "HEAD"]);
    let lock = repo.path.join(".git/CHERRY_PICK_HEAD.lock");
    std::fs::write(&lock, "").expect("lock");

    let said = Arc::new(Said::default());
    let err = run_plan(&repo, &middle, sequencer::Edit::Drop, &said).await;
    std::fs::remove_file(&lock).expect("unlock");
    assert_eq!(kind_of(&err), Some(ReportKind::RewriteLocked), "{err}");
    assert_eq!(current_op(&repo).await, None, "put back, nothing standing");
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), before, "where it began");
    assert!(
        failed_row(&said).contains("CHERRY_PICK_HEAD.lock': File exists"),
        "the lock's words are under the failed row"
    );
}

/// The same at the replay's first checkout (the index's lock): turned down
/// before anything moved, with nothing to put back.
#[tokio::test]
async fn a_replay_another_gits_lock_turns_down_at_the_start_is_reported() {
    let (mut repo, middle) = three_commits();
    let before = repo.git(&["rev-parse", "HEAD"]);
    let lock = repo.path.join(".git/index.lock");

    let said = Arc::new(Said::default());
    let (exec, cancel) = observed_env(said.clone(), Kept::Asked);
    let plan = sequencer::plan_edit(&exec, &repo.path, &middle, sequencer::Edit::Drop, &cancel)
        .await
        .expect("plan");
    std::fs::write(&lock, "").expect("lock");
    let err = sequencer::rebase_interactive(
        &exec,
        &info(&repo).await,
        &plan.upstream,
        &plan.steps,
        &plan.options(),
        &helper(),
        &cancel,
    )
    .await
    .expect_err("the index is another git's");
    std::fs::remove_file(&lock).expect("unlock");
    assert_eq!(kind_of(&err), Some(ReportKind::RewriteLocked), "{err}");
    assert_eq!(current_op(&repo).await, None, "nothing standing");
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), before, "nothing moved");
    assert!(
        failed_row(&said).contains("index.lock': File exists"),
        "the lock's words are under the failed row"
    );
}

/// A squash's commit turned down by `prepare-commit-msg`, the one hook it
/// runs (`-n`): git adds no words to the hook's, and the replay is put
/// back as for any other commit hook. The commit folded into is no root, so
/// the replay's first pick is the squash itself.
#[tokio::test]
async fn a_squash_its_prepare_commit_msg_hook_turns_down_is_put_back_and_reported() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "two\n", "second");
    let third = repo.commit_file_id("c.txt", "three\n", "third");
    repo.commit_file("d.txt", "four\n", "last");
    repo.write_hook(
        "prepare-commit-msg",
        "echo \"no squashes here\" >&2\nexit 1\n",
    );
    let before = repo.git(&["rev-parse", "HEAD"]);

    let said = Arc::new(Said::default());
    let err = run_plan(&repo, &third, sequencer::Edit::SquashIntoParent, &said).await;
    assert_eq!(kind_of(&err), Some(ReportKind::RewriteCommitHook), "{err}");
    assert_eq!(current_op(&repo).await, None, "put back, nothing standing");
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), before, "where it began");
    assert!(
        failed_row(&said).contains("no squashes here"),
        "the hook's words are under the failed row"
    );
}

/// An untracked file where the commit under the range has one — the
/// commit dropped deleted it — would be overwritten by the replay's first
/// checkout: git turns the rewrite down, nothing moved, and its words are
/// under the failed row. An ignored one is git's to write over, and not in
/// the way.
#[tokio::test]
async fn a_rewrite_whose_first_checkout_would_overwrite_untracked_files_is_turned_down() {
    let mut repo = TestRepo::init();
    repo.commit_file("old.txt", "kept once\n", "root");
    repo.git(&["rm", "-q", "old.txt"]);
    repo.commit_file("a.txt", "a\n", "drops old");
    let dropping = repo.git(&["rev-parse", "HEAD"]);
    repo.commit_file("b.txt", "b\n", "last");
    repo.write_file("old.txt", "untracked now\n");
    let before = repo.git(&["rev-parse", "HEAD"]);

    let said = Arc::new(Said::default());
    let err = run_plan(&repo, &dropping, sequencer::Edit::Drop, &said).await;
    assert_eq!(
        kind_of(&err),
        Some(ReportKind::RewriteUntrackedInTheWay),
        "{err}"
    );
    assert!(
        matches!(err, GitError::Reported { .. }),
        "git's own refusal: {err:?}"
    );
    assert!(
        failed_row(&said).contains("old.txt"),
        "git's words name the file under the failed row"
    );
    assert_eq!(current_op(&repo).await, None, "nothing standing");
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), before, "nothing moved");
    assert_eq!(
        std::fs::read_to_string(repo.path.join("old.txt")).expect("read"),
        "untracked now\n"
    );

    repo.write_file(".git/info/exclude", "old.txt\n");
    let (exec, cancel) = observed_env(Arc::new(Said::default()), Kept::Asked);
    let plan = sequencer::plan_edit(&exec, &repo.path, &dropping, sequencer::Edit::Drop, &cancel)
        .await
        .expect("plan");
    sequencer::rebase_interactive(
        &exec,
        &info(&repo).await,
        &plan.upstream,
        &plan.steps,
        &plan.options(),
        &helper(),
        &cancel,
    )
    .await
    .expect("an ignored file is not in the way");
}

/// Which commit a replay's first checkout lands on is git's to work out: a
/// reword's starts on the commit reworded, not the one under it
/// (sequencer.c `skip_unnecessary_picks`). That commit deleted the file
/// standing untracked here, so nothing writes over it and the reword goes
/// through.
#[tokio::test]
async fn a_reword_whose_commit_let_go_of_an_untracked_file_goes_through() {
    let mut repo = TestRepo::init();
    repo.commit_file("old.txt", "kept once\n", "root");
    repo.git(&["rm", "-q", "--cached", "old.txt"]);
    repo.commit_file("a.txt", "a\n", "stops tracking old");
    let letting_go = repo.git(&["rev-parse", "HEAD"]);
    repo.commit_file("b.txt", "b\n", "last");
    assert_eq!(
        std::fs::read_to_string(repo.path.join("old.txt")).expect("read"),
        "kept once\n",
        "the file stays, untracked"
    );

    let (exec, cancel) = observed_env(Arc::new(Said::default()), Kept::Asked);
    let edit = sequencer::Edit::Reword("let old.txt go\n".to_string());
    let plan = sequencer::plan_edit(&exec, &repo.path, &letting_go, edit, &cancel)
        .await
        .expect("plan");
    sequencer::rebase_interactive(
        &exec,
        &info(&repo).await,
        &plan.upstream,
        &plan.steps,
        &plan.options(),
        &helper(),
        &cancel,
    )
    .await
    .expect("nothing writes over the file");
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s", "HEAD~1"]),
        "let old.txt go"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path.join("old.txt")).expect("read"),
        "kept once\n"
    );
}

/// The plain rebase meets the same file at its first checkout: git turns
/// it down, and its row in the log is a failure with git's words under it.
#[tokio::test]
async fn a_rebase_untracked_files_turn_down_leaves_gits_words_on_its_row() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    repo.git(&["switch", "-q", "-c", "base"]);
    repo.commit_file("new.txt", "theirs\n", "base adds new");
    repo.git(&["switch", "-q", "main"]);
    repo.commit_file("b.txt", "b\n", "ours");
    repo.write_file("new.txt", "untracked here\n");

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
    .expect_err("the checkout would overwrite new.txt");
    assert!(matches!(err, GitError::Failed { .. }), "{err:?}");
    assert!(
        failed_row(&said).contains("would be overwritten by checkout"),
        "git's words are under the failed row"
    );
}
