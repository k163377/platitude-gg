//! Creating and amending commits.
//!
//! Messages travel in a scratch file (`-F`), never on the command line:
//! argument length is bounded on Windows and a message is arbitrary user
//! text. `--cleanup=whitespace` is pinned so a repository's `commit.cleanup`
//! cannot silently rewrite what the editor showed — in particular, a line
//! the user typed starting with `#` stays in the message.
//!
//! Hooks are git's business and run normally: `--no-verify` is never passed.

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
    /// Replace the current HEAD commit instead of adding one.
    pub amend: bool,
    /// Permit a commit that changes nothing (`--allow-empty`).
    pub allow_empty: bool,
    /// On amend, take over authorship (`--reset-author`).
    pub reset_author: bool,
}

/// Joins the two fields of a message editor into one commit message.
///
/// The convention lives here rather than in the UI: a summary line, a
/// blank line, then the description — the shape every git tool expects,
/// and the shape [`split_message`] reads back.
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

/// Splits a commit message back into the editor's two fields.
///
/// The summary is the first line; the description is what follows once the
/// blank line separating them is gone.
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

/// Commits the staged content and returns the resulting commit id.
///
/// An empty `message` is only valid together with [`CommitOptions::amend`],
/// where it means "keep the existing message" (`--no-edit`).
pub async fn commit(
    executor: &GitExecutor,
    repo: &RepoInfo,
    message: &str,
    options: CommitOptions,
    cancel: &CancellationToken,
) -> Result<Oid, GitError> {
    let mut cmd = GitCommand::new().cwd(&repo.workdir).arg("commit");
    if options.amend {
        cmd = cmd.arg("--amend");
    }
    if options.allow_empty {
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

    executor.run(cmd, cancel).await?;
    head_oid(executor, &repo.workdir, cancel).await
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
///
/// One command for both, NUL-separated: a message spans lines, so it has
/// to come last and no printable separator would be safe in front of it.
pub async fn head_commit(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<HeadCommit, GitError> {
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
    Ok(HeadCommit {
        message: message.trim_end_matches('\n').to_string(),
        author_name: author_name.to_string(),
        author_email: author_email.to_string(),
    })
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
/// it was built on.
///
/// What history a rewrite may touch: only this line of commits can be
/// amended or replayed from where the working tree stands. `--is-ancestor`
/// answers by exit code — 0 and 1 are the two answers; anything else
/// (an unreadable repository, a vanished object) is a failure, not a "no".
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

/// How many commits `oid` reaches that HEAD does not.
///
/// What an overwrite would take off the far side: those commits stay in the
/// repository that holds them, but nothing on that branch points at them
/// afterwards. Only asked when the commit is here to walk — the count is
/// the one thing the question can put a number on (§相手の履歴を置き換える).
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
        // A message with no blank line keeps everything after line one.
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
