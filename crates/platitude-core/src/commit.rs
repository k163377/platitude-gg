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

/// Message of the current HEAD commit, for prefilling an amend editor.
pub async fn head_message(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<String, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["log", "-1", "--format=%B"]);
    let out = executor.run(cmd, cancel).await?;
    Ok(out.stdout_utf8().trim_end_matches('\n').to_string())
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

async fn head_oid(
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
