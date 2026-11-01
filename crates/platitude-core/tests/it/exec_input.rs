//! What a command is handed on stdin (`GitCommand::input`): all of it and
//! then its end, sent while git's output is read so neither waits on the
//! other; an end git comes to before it has read it all is git's answer
//! where that is a no and a failure where it is a yes; a cancel takes the
//! send down with git; and the log row holds the command, not what it was
//! handed.

use std::sync::Arc;

use platitude_core::commit::{self, CommitOptions};
use platitude_core::process::{CommandEnd, GitCommand, Live};
use platitude_core::{GitError, ReportKind};
use tokio_util::sync::CancellationToken;

use crate::support::exec::{env, isolated, logged};
use crate::support::wait::{bounded, poll_once};
use crate::support::{TestRepo, barrier_hook, info};

/// Past what any stdin pipe the suite runs on holds (1 MiB on Linux, where
/// the process layer widens it for an input past that —
/// `process::child::PIPE_WIDTH`; 64 KiB or less otherwise): a send git does
/// not read cannot finish.
const PAST_THE_PIPE: usize = 8 * 1024 * 1024;

/// Linux's widened stdin pipe, the widest the suite runs on; an input no
/// larger is sent through the pipe as made.
const WIDEST_PIPE: usize = 1024 * 1024;

/// The bytes reach git whole: the id is that of `hello\n`, where a stdin
/// that never got them would hash nothing.
#[tokio::test]
async fn a_small_input_reaches_git_whole() {
    let repo = TestRepo::init();
    let (exec, cancel) = env();
    let hashed = GitCommand::new()
        .cwd(&repo.path)
        .args(["hash-object", "--stdin"])
        .input(b"hello\n".to_vec());
    let out = bounded("the hash", exec.run(hashed, &cancel))
        .await
        .expect("git hashes it");
    assert_eq!(
        out.stdout_utf8().trim(),
        "ce013625030ba8dba906f756967f9e9ca394464a"
    );
}

/// `cat-file --batch` answers each line before it reads the next, and both
/// sides here are past what a pipe holds: a run that sent first and read
/// after would stall, git on its full output and the send on git.
#[tokio::test]
async fn an_input_and_an_output_past_the_pipe_go_through_together() {
    let mut repo = TestRepo::init();
    let content = format!("{}\n", "x".repeat(99));
    repo.commit_file("blob.txt", &content, "one blob");
    let blob = repo.git(&["rev-parse", "HEAD:blob.txt"]);
    const ASKS: usize = 40_000;
    let asked = format!("{blob}\n").repeat(ASKS);
    assert!(
        asked.len() > WIDEST_PIPE + WIDEST_PIPE / 2,
        "past the pipe: {}",
        asked.len()
    );
    let (exec, cancel) = env();
    let batch = GitCommand::new()
        .cwd(&repo.path)
        .args(["cat-file", "--batch"])
        .input(asked);
    let out = bounded("the batch", exec.run(batch, &cancel))
        .await
        .expect("git answers every line");
    let one = format!("{blob} blob 100\n{content}\n");
    assert_eq!(out.stdout.len(), one.len() * ASKS);
    assert!(out.stdout.starts_with(one.as_bytes()) && out.stdout.ends_with(one.as_bytes()));
}

/// A git that reads no stdin ends with the send not through, the pipe
/// broken under it. Its no is the answer, by its own code; its yes is a
/// failure, since it said yes without what it was handed — and the row
/// says so. The yes is asked twice: with [`WIDEST_PIPE`] — past the pipe as
/// made, too small to be widened, and within the 2 MiB Windows' writes hand
/// one thread (tokio `io::blocking`), where only the flush meets the broken
/// pipe — and past several of those, through Linux's widened pipe.
#[tokio::test]
async fn an_end_before_the_input_is_read_is_gits_no_and_never_its_yes() {
    let repo = TestRepo::init();
    let (exec, log, cancel) = logged();
    let past = vec![b'x'; PAST_THE_PIPE];

    let missing = GitCommand::new()
        .cwd(&repo.path)
        .args(["rev-parse", "--verify", "--quiet", "refs/heads/nothing"])
        .input(past.clone());
    let err = bounded("the no", exec.run(missing, &cancel))
        .await
        .expect_err("nothing is there");
    assert!(matches!(err, GitError::Failed { code: 1, .. }), "{err:?}");

    let sizes = [WIDEST_PIPE, PAST_THE_PIPE];
    for size in sizes {
        let unread = GitCommand::new()
            .cwd(&repo.path)
            .args(["rev-parse", "--git-dir"])
            .input(vec![b'x'; size]);
        let err = bounded("the yes", exec.run(unread, &cancel))
            .await
            .expect_err("a yes over unread input");
        match &err {
            GitError::Io { source, .. } => {
                assert_eq!(
                    source.kind(),
                    std::io::ErrorKind::BrokenPipe,
                    "{size}: {err}"
                );
                assert!(
                    source.to_string().contains("all of its input"),
                    "{size}: {err}"
                );
            }
            other => panic!("{size}: not the input's failure: {other:?}"),
        }
    }
    let mut ends = vec![CommandEnd::Exited(1)];
    ends.extend(sizes.iter().map(|_| CommandEnd::Failed));
    assert_eq!(log.ends_of(&["rev-parse"]), ends);
}

/// A commit hook that says no before git reads the message (the hook runs
/// first) leaves the send broken under a message past the pipe; the commit
/// is reported as the hook's no, which git's trace names.
#[tokio::test]
async fn a_hooks_no_before_the_message_is_read_is_the_hooks_no() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    repo.write_hook("pre-commit", "echo 'not today' >&2\nexit 1\n");
    let repo_info = info(&repo).await;
    let (exec, cancel) = env();
    let message = "m".repeat(PAST_THE_PIPE);
    let options = CommitOptions {
        allow_empty: true,
        ..CommitOptions::default()
    };
    let err = bounded(
        "the refused commit",
        commit::commit(&exec, &repo_info, &message, options, &cancel),
    )
    .await
    .expect_err("the hook says no");
    assert_eq!(
        err.report().map(|report| report.kind),
        Some(ReportKind::CommitHook),
        "{err}"
    );
}

/// What git is handed stays out of the log: the row is the command alone,
/// in both its forms.
#[tokio::test]
async fn what_git_is_handed_stays_out_of_the_log() {
    let repo = TestRepo::init();
    let (exec, log, cancel) = logged();
    let hashed = GitCommand::new()
        .cwd(&repo.path)
        .args(["hash-object", "--stdin"])
        .input(b"a word for git alone\n".to_vec());
    bounded("the hash", exec.run(hashed, &cancel))
        .await
        .expect("git hashes it");
    assert_eq!(log.displays(), ["git hash-object --stdin"]);
    let described = format!("{:?}", GitCommand::new().input(b"a word".to_vec()));
    assert!(!described.contains("word"), "{described}");
}

/// A cancel while the send is under way — git held by its `pre-commit`
/// hook, which runs before git reads the message, and the message past
/// what the pipe holds, so the send cannot have finished — ends the run as
/// cancelled, the send with it, and everything git started is gone before
/// it answers. Nothing is committed.
#[tokio::test]
async fn a_cancel_mid_send_ends_the_send_with_the_run() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    let release = repo.path.join("release");
    repo.write_hook("pre-commit", &barrier_hook(&release));
    let head = repo.git(&["rev-parse", "HEAD"]);
    let repo_info = info(&repo).await;
    let live = Arc::new(Live::default());
    let exec = isolated().counted(Arc::clone(&live));
    let cancel = CancellationToken::new();
    let message = "m".repeat(PAST_THE_PIPE);
    let options = CommitOptions {
        allow_empty: true,
        ..CommitOptions::default()
    };
    let mut run = Box::pin(commit::commit(
        &exec, &repo_info, &message, options, &cancel,
    ));
    assert!(poll_once(&mut run).is_pending(), "the send waits on git");
    assert_eq!(live.count(), 1, "spawned, and sending since the same poll");

    cancel.cancel();
    let err = bounded("the cancelled commit", run)
        .await
        .expect_err("cancelled");
    assert!(err.is_cancelled(), "{err:?}");
    bounded("every git gone", live.idle())
        .await
        .expect("every end told");
    // Off Windows git alone is ended; its hook leaves once let go.
    std::fs::write(&release, b"").expect("let the hook go");
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), head, "nothing committed");
}
