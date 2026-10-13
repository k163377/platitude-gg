//! Creating and amending commits.
//!
//! Messages travel in a scratch file (`--file`): Windows bounds argument
//! length. `--cleanup=whitespace` is pinned so a repository's
//! `commit.cleanup` cannot rewrite what the editor showed (a typed `#` line
//! stays).
//!
//! Hooks always run: `--no-verify` is never passed.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::oid::Oid;
use crate::process::{GitCommand, GitExecutor};
use crate::repo::RepoInfo;
use crate::scratch::ScratchFile;

/// Knobs of one commit invocation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CommitOptions {
    /// Replace the current HEAD commit.
    pub amend: bool,
    /// Permit a commit that changes nothing (`--allow-empty`).
    pub allow_empty: bool,
    /// On amend, take over authorship (`--reset-author`).
    pub reset_author: bool,
}

/// Joins the two fields of a message editor into one commit message:
/// summary, blank line, description ([`split_message`] reads it back).
pub fn join_message(subject: &str, body: &str) -> String {
    let subject = subject.trim();
    let body = body.trim();
    if body.is_empty() {
        return subject.to_string();
    }
    if subject.is_empty() {
        return body.to_string();
    }
    format!("{subject}\n\n{body}")
}

/// Splits a commit message back into the editor's two fields: the first
/// line, and the rest without the separating blank line.
pub fn split_message(message: &str) -> (String, String) {
    let message = message.replace("\r\n", "\n");
    let (subject, rest) = match message.split_once('\n') {
        Some((s, r)) => (s, r),
        None => (message.as_str(), ""),
    };
    (
        subject.trim().to_string(),
        rest.trim_start_matches('\n').trim_end().to_string(),
    )
}

/// Commits the staged content.
///
/// An empty `message` is only valid together with [`CommitOptions::amend`],
/// where it means "keep the existing message" (`--no-edit`).
///
/// An amend of a commit that is itself empty (an `--allow-empty` marker)
/// goes with `--allow-empty`: git refuses to amend one into an empty
/// commit otherwise, words for a terminal and all, and its own `reword`
/// passes the same. An amend that would empty a commit with changes is
/// the screen's to refuse before the press ([`amend_would_empty`]).
///
/// Nothing is read back: a second command asking where HEAD landed could
/// fail on its own and report a made commit as failed. The refresh after
/// the write reads it.
pub async fn commit(
    executor: &GitExecutor,
    repo: &RepoInfo,
    message: &str,
    options: CommitOptions,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let allow_empty = options.allow_empty
        || (options.amend && head_is_empty(executor, &repo.workdir, cancel).await?);
    // Traced: a hook's no and an ssh signing program's are in git's record,
    // not its words ([`refused_by`]).
    let mut cmd = GitCommand::new().cwd(&repo.workdir).traced().arg("commit");
    if options.amend {
        cmd = cmd.arg("--amend");
    }
    if allow_empty {
        cmd = cmd.arg("--allow-empty");
    }
    if options.reset_author {
        cmd = cmd.arg("--reset-author");
    }

    // Held until the command finishes; dropping it removes the file.
    let _scratch;
    if message.trim().is_empty() {
        if !options.amend {
            return Err(GitError::UnexpectedOutput {
                command: "git commit".to_string(),
                message: "refusing to commit with an empty message".to_string(),
            });
        }
        cmd = cmd.arg("--no-edit");
    } else {
        let scratch =
            ScratchFile::create(&repo.git_dir, "COMMIT_MSG", normalized(message).as_bytes())
                .map_err(|source| GitError::Io {
                    command: "git commit".to_string(),
                    source,
                })?;
        cmd = cmd
            .args(["--cleanup=whitespace", "--file"])
            .arg(scratch.path());
        _scratch = scratch;
    }

    let command = cmd.describe();
    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code != 0 {
        return Err(refused(command, &out));
    }
    Ok(())
}

/// A commit that was not made, as something to report
/// (デザイン規約 §答えの要らない報せ): refusals this end could answer are
/// taken away before the button can be pressed, so a non-zero exit is
/// something outside saying no ([`refused_by`]).
fn refused(command: String, out: &crate::process::GitOutput) -> GitError {
    crate::report::reported(
        crate::report::WriteReport::local(refused_by(out)),
        command,
        out,
    )
}

/// The hooks whose no stops a commit (builtin/commit.c; `post-commit`'s end
/// changes nothing).
pub(crate) const COMMIT_HOOKS: [&str; 3] = ["pre-commit", "prepare-commit-msg", "commit-msg"];

/// Whether git could not sign: its own words name the signing (`LC_ALL=C`,
/// gpg-interface.c — gpg or gpgsm gave no signature, no ssh key is named or
/// none can be had, the ssh-keygen there cannot sign, the key or the
/// signature could not pass through the temporary file), or — for an ssh
/// signature, whose program's failure git passes on as that program's words
/// alone (`sign_buffer_ssh`) — the program ended non-zero in its trace.
pub(crate) fn signing_failed(said: &str, trace: &crate::process::Trace) -> bool {
    crate::relied_messages::SIGNING_FAILED.found_in(said) || trace.ssh_signer_failed()
}

/// Which of the outside's refusals a failed `git commit` was, as far as git
/// said or recorded it. A hook first, by its end in git's trace — git adds
/// nothing of its own to a hook's no, and the hook's own output, which may
/// carry a lock's or a signer's words, lands among git's: the index's lock
/// is taken before the hooks run, the signing and the ref's lock after them
/// all. Then a signing failure in git's own words (or an ssh signer's end,
/// [`signing_failed`]) and a lock git named. The rest is git's words to
/// explain.
fn refused_by(out: &crate::process::GitOutput) -> crate::report::ReportKind {
    use crate::report::ReportKind;
    let said = out.stderr_utf8();
    if out.trace.hook_failed(&COMMIT_HOOKS) {
        ReportKind::CommitHook
    } else if signing_failed(&said, &out.trace) {
        ReportKind::CommitUnsigned
    } else if crate::report::lock_taken(&said) {
        ReportKind::CommitLocked
    } else {
        ReportKind::Commit
    }
}

/// What an amend starts from: HEAD's message and the identity recorded
/// as its author.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeadCommit {
    pub message: String,
    pub author_name: String,
    pub author_email: String,
}

/// Reads HEAD's message and author, for prefilling an amend editor.
/// `None` on an unborn branch.
///
/// NUL-separated with the message last, since it spans lines.
pub async fn head_commit(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Option<HeadCommit>, GitError> {
    // Asked first, so a failure of the log read below stays a failure.
    let probe = GitCommand::new().cwd(workdir).answers_by_code(1).args([
        "rev-parse",
        "--verify",
        "--quiet",
        "HEAD",
    ]);
    if executor.run_unchecked(probe, cancel).await?.code != 0 {
        return Ok(None);
    }
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["log", "-1", "--format=%an%x00%ae%x00%B"]);
    let out = executor.run(cmd, cancel).await?;
    let text = out.stdout_utf8();
    let unexpected = || GitError::UnexpectedOutput {
        command: "git log -1".to_string(),
        message: "author and message are not NUL-separated".to_string(),
    };
    let (author_name, rest) = text.split_once('\0').ok_or_else(unexpected)?;
    let (author_email, message) = rest.split_once('\0').ok_or_else(unexpected)?;
    Ok(Some(HeadCommit {
        message: message.trim_end_matches('\n').to_string(),
        author_name: author_name.to_string(),
        author_email: author_email.to_string(),
    }))
}

/// Whether HEAD changes nothing against its first parent (the empty tree
/// for a root commit) — a marker made with `--allow-empty`. A merge
/// answers true too (`diff-tree` shows nothing for one unasked), which
/// costs nothing: git amends a merge whatever its tree.
async fn head_is_empty(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args([
            "diff-tree",
            "--quiet",
            "--no-commit-id",
            "--root",
            "-r",
            "HEAD",
        ])
        .answers_by_code(1);
    let out = executor.run_unchecked(cmd, cancel).await?;
    match out.code {
        0 => Ok(true),
        1 => Ok(false),
        code => Err(GitError::Failed {
            command: "git diff-tree --quiet HEAD".to_string(),
            code,
            stderr: out.stderr_utf8().into_owned(),
        }),
    }
}

/// Whether amending HEAD now would leave it changing nothing: what is
/// staged takes back everything HEAD changed, so the index's tree is
/// HEAD's first parent's. git refuses that amend, so the screen refuses
/// it first. A message-only amend of an empty HEAD is not this one
/// ([`commit`] lets it through). A root commit answers false: emptying
/// one means taking every file out of the index.
pub async fn amend_would_empty(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let parent = GitCommand::new().cwd(workdir).answers_by_code(1).args([
        "rev-parse",
        "--verify",
        "--quiet",
        "HEAD~1",
    ]);
    if executor.run_unchecked(parent, cancel).await?.code != 0 {
        return Ok(false);
    }
    if head_is_empty(executor, workdir, cancel).await? {
        return Ok(false);
    }
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["diff-index", "--cached", "--quiet", "HEAD~1", "--"])
        .answers_by_code(1);
    let out = executor.run_unchecked(cmd, cancel).await?;
    match out.code {
        0 => Ok(true),
        1 => Ok(false),
        code => Err(GitError::Failed {
            command: "git diff-index --cached --quiet HEAD~1".to_string(),
            code,
            stderr: out.stderr_utf8().into_owned(),
        }),
    }
}

/// True when HEAD exists and is a merge commit (amending one is a
/// different proposition and the UI warns about it).
pub async fn head_is_merge(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["log", "-1", "--format=%P"]);
    let out = executor.run(cmd, cancel).await?;
    Ok(out.stdout_utf8().split_whitespace().count() > 1)
}

/// The commit HEAD points at.
pub async fn head_oid(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Oid, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["rev-parse", "--verify", "HEAD"]);
    let out = executor.run(cmd, cancel).await?;
    Oid::from_hex(out.stdout_utf8().trim().as_bytes()).map_err(|_| GitError::UnexpectedOutput {
        command: "git rev-parse --verify HEAD".to_string(),
        message: out.stdout_utf8().trim().to_string(),
    })
}

/// Whether HEAD can reach `oid` — the commit is HEAD itself or something
/// it was built on; only this line of commits can be rewritten from where
/// the worktree stands.
pub async fn is_in_head_history(
    executor: &GitExecutor,
    workdir: &Path,
    oid: &Oid,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["merge-base", "--is-ancestor"])
        .arg(oid.to_hex())
        .arg("HEAD")
        .answers_by_code(1);
    let out = executor.run_unchecked(cmd, cancel).await?;
    match out.code {
        0 => Ok(true),
        1 => Ok(false),
        code => Err(GitError::Failed {
            command: format!("git merge-base --is-ancestor {} HEAD", oid.to_hex()),
            code,
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }),
    }
}

/// How many commits `oid` reaches that HEAD does not — what a force push
/// would take off the far side. Only asked when the commit is here to walk
/// (デザイン規約 §相手の履歴を置き換える).
pub async fn count_beyond_head(
    executor: &GitExecutor,
    workdir: &Path,
    oid: &Oid,
    cancel: &CancellationToken,
) -> Result<u32, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["rev-list", "--count"])
        .arg(format!("HEAD..{}", oid.to_hex()));
    let out = executor.run(cmd, cancel).await?;
    out.stdout_utf8()
        .trim()
        .parse()
        .map_err(|_| GitError::UnexpectedOutput {
            command: format!("git rev-list --count HEAD..{}", oid.to_hex()),
            message: "a count was expected".to_string(),
        })
}

/// Normalizes editor text for git: CRLF to LF and exactly one trailing
/// newline. `--cleanup=whitespace` handles the rest.
fn normalized(message: &str) -> String {
    let mut text = message.replace("\r\n", "\n");
    while text.ends_with('\n') {
        text.pop();
    }
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::refusing;

    /// The executor cannot run anything, so a commit that reached for git
    /// would fail on the spawn rather than come back as this refusal.
    #[tokio::test]
    async fn an_empty_message_is_refused_for_a_new_commit_without_asking_git() {
        let (exec, asked) = refusing::git();
        let repo = refusing::repo();

        let err = commit(
            &exec,
            &repo,
            "   \n\n",
            CommitOptions::default(),
            &CancellationToken::new(),
        )
        .await
        .expect_err("an empty message is refused");

        assert!(err.to_string().contains("empty message"), "{err}");
        assert_eq!(asked.count(), 0, "nothing was asked of git");
    }

    #[test]
    fn the_two_editor_fields_join_with_a_blank_line() {
        assert_eq!(join_message("subject", "body"), "subject\n\nbody");
        assert_eq!(join_message("  subject  ", "  "), "subject");
        assert_eq!(join_message("", "body only"), "body only");
        assert_eq!(join_message("", ""), "");
    }

    #[test]
    fn a_message_splits_back_into_the_fields_it_came_from() {
        assert_eq!(
            split_message("subject\n\nbody\nmore"),
            ("subject".to_string(), "body\nmore".to_string())
        );
        assert_eq!(
            split_message("subject only\n"),
            ("subject only".to_string(), String::new())
        );
        // git's own log output arrives with CRLF on Windows checkouts.
        assert_eq!(
            split_message("subject\r\n\r\nbody\r\n"),
            ("subject".to_string(), "body".to_string())
        );
        assert_eq!(
            split_message("subject\nrun-on"),
            ("subject".to_string(), "run-on".to_string())
        );
    }

    #[test]
    fn join_and_split_round_trip() {
        let (subject, body) = split_message(&join_message("s", "b1\nb2"));
        assert_eq!((subject.as_str(), body.as_str()), ("s", "b1\nb2"));
    }

    #[test]
    fn normalizes_line_endings_and_trailing_newline() {
        assert_eq!(
            normalized("subject\r\n\r\nbody\r\n\n\n"),
            "subject\n\nbody\n"
        );
        assert_eq!(normalized("subject"), "subject\n");
    }

    #[test]
    fn keeps_interior_blank_lines() {
        assert_eq!(normalized("a\n\n\nb"), "a\n\n\nb\n");
    }
}
