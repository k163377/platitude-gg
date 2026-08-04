//! Stash listing and stash operations.
//!
//! Selectors (`stash@{0}`) come from a listing and are passed straight
//! back; nothing here builds one from an index, because the numbering
//! shifts under every push and drop.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::oid::Oid;
use crate::process::{GitCommand, GitExecutor};

/// `--format=` for `stash list`; fields: reflog selector, commit id,
/// committer time, reflog subject. Records are NUL-terminated via `-z`
/// (stash list forwards options to `git log`).
pub const STASH_FORMAT_ARG: &str = "--format=%gd%x00%H%x00%ct%x00%gs";

const STASH_FIELDS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashEntry {
    /// Reflog selector (`stash@{0}`), the argument for later stash ops.
    pub name: String,
    pub oid: Oid,
    /// Committer time (unix seconds).
    pub time: i64,
    /// Reflog subject (`WIP on main: ...` or the custom message).
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("malformed stash list output")]
pub struct StashParseError;

/// Parses `stash list` output produced with [`STASH_FORMAT_ARG`] + `-z`.
pub fn parse_stashes(bytes: &[u8]) -> Result<Vec<StashEntry>, StashParseError> {
    let tokens: Vec<&[u8]> = bytes.split(|b| *b == 0).collect();
    // Trailing empty token after the final NUL (or a single empty token
    // for empty output).
    let tokens = match tokens.split_last() {
        Some((last, rest)) if last.iter().all(|b| b.is_ascii_whitespace()) => rest,
        _ => &tokens[..],
    };
    if tokens.is_empty() {
        return Ok(Vec::new());
    }
    if tokens.len() % STASH_FIELDS != 0 {
        return Err(StashParseError);
    }
    let mut out = Vec::with_capacity(tokens.len() / STASH_FIELDS);
    for record in tokens.chunks_exact(STASH_FIELDS) {
        let name = String::from_utf8_lossy(record[0]).into_owned();
        let oid = Oid::from_hex(record[1]).map_err(|_| StashParseError)?;
        let time = std::str::from_utf8(record[2])
            .map_err(|_| StashParseError)?
            .trim()
            .parse()
            .map_err(|_| StashParseError)?;
        let message = String::from_utf8_lossy(record[3]).into_owned();
        out.push(StashEntry {
            name,
            oid,
            time,
            message,
        });
    }
    Ok(out)
}

/// Loads the stash list.
pub async fn load(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Vec<StashEntry>, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["stash", "list", "-z", STASH_FORMAT_ARG]);
    let out = executor.run(cmd, cancel).await?;
    parse_stashes(&out.stdout).map_err(|e| GitError::UnexpectedOutput {
        command: "git stash list".to_string(),
        message: e.to_string(),
    })
}

/// Knobs of `git stash push`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PushOptions {
    /// Stash untracked files too (`--include-untracked`).
    pub include_untracked: bool,
    /// Leave the index as it is (`--keep-index`).
    pub keep_index: bool,
    /// Stash only what is staged (`--staged`).
    pub staged_only: bool,
}

/// `git stash push`: saves the working tree, optionally limited to `paths`.
pub async fn push(
    executor: &GitExecutor,
    workdir: &Path,
    message: &str,
    options: PushOptions,
    paths: &[String],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let mut cmd = GitCommand::new().cwd(workdir).args(["stash", "push"]);
    if options.include_untracked {
        cmd = cmd.arg("--include-untracked");
    }
    if options.keep_index {
        cmd = cmd.arg("--keep-index");
    }
    if options.staged_only {
        cmd = cmd.arg("--staged");
    }
    if !message.trim().is_empty() {
        cmd = cmd.args(["--message", message]);
    }
    if !paths.is_empty() {
        cmd = cmd
            .arg("--")
            .args(paths.iter().map(|p| crate::process::literal_pathspec(p)));
    }
    executor.run(cmd, cancel).await.map(|_| ())
}

/// `git stash pop <selector>`: restores and removes the entry.
pub async fn pop(
    executor: &GitExecutor,
    workdir: &Path,
    selector: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    run_selector(executor, workdir, "pop", selector, cancel).await
}

/// `git stash pop --index`: restores the entry *and* the split between
/// what was staged in it and what was not.
///
/// Non-zero exit does not mean nothing happened. When the restore
/// conflicts git gives up on the index part ("Index was not unstashed"),
/// leaves the markers in the files and **keeps the entry** — which is what
/// leaves a way back. Callers decide what that is worth by looking at the
/// working tree afterwards, not at the exit code.
pub async fn pop_with_index(
    executor: &GitExecutor,
    workdir: &Path,
    selector: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["stash", "pop", "--index", selector]);
    executor.run(cmd, cancel).await.map(|_| ())
}

/// `git stash apply <selector>`: restores and keeps the entry.
pub async fn apply(
    executor: &GitExecutor,
    workdir: &Path,
    selector: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    run_selector(executor, workdir, "apply", selector, cancel).await
}

/// `git stash drop <selector>`: discards the entry. Destructive — the
/// caller confirms first.
pub async fn drop(
    executor: &GitExecutor,
    workdir: &Path,
    selector: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    run_selector(executor, workdir, "drop", selector, cancel).await
}

async fn run_selector(
    executor: &GitExecutor,
    workdir: &Path,
    op: &str,
    selector: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new().cwd(workdir).args(["stash", op, selector]);
    executor.run(cmd, cancel).await.map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn z(tokens: &[&str]) -> Vec<u8> {
        let mut v = Vec::new();
        for t in tokens {
            v.extend_from_slice(t.as_bytes());
            v.push(0);
        }
        v
    }

    #[test]
    fn parses_entries() {
        let bytes = z(&[
            "stash@{0}",
            SHA,
            "1700000000",
            "WIP on main: 1234567 subject",
            "stash@{1}",
            SHA,
            "1699999999",
            "On feature: custom message",
        ]);
        let stashes = parse_stashes(&bytes).unwrap();
        assert_eq!(stashes.len(), 2);
        assert_eq!(stashes[0].name, "stash@{0}");
        assert_eq!(stashes[0].time, 1_700_000_000);
        assert_eq!(stashes[1].message, "On feature: custom message");
    }

    #[test]
    fn empty_output_means_no_stashes() {
        assert!(parse_stashes(b"").unwrap().is_empty());
        assert!(parse_stashes(b"\n").unwrap().is_empty());
    }

    #[test]
    fn wrong_arity_is_an_error() {
        let bytes = z(&["stash@{0}", SHA]);
        assert!(parse_stashes(&bytes).is_err());
    }
}
