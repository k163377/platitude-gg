//! Stash listing and stash operations.
//!
//! Selectors (`stash@{0}`) come from a listing and are passed straight
//! back: the numbering shifts under every push and drop, so an invented
//! one could name someone else's entry. The one exception is [`rename`],
//! which checks its shifted selector by oid before dropping.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::oid::Oid;
use crate::process::{GitCommand, GitExecutor};

mod rename;

pub use rename::rename;

/// `--format=` for `stash list`: selector, commit, committer time,
/// subject, parents, and the `Stands-on:` trailer a discard's copy put in
/// the stash carries (破棄記録仕様.md §2.1). `-z` NUL-terminates records
/// (stash list forwards options to `git log`).
pub const STASH_FORMAT_ARG: &str =
    "--format=%gd%x00%H%x00%ct%x00%gs%x00%P%x00%(trailers:key=Stands-on,valueonly,separator=%x20)";

const STASH_FIELDS: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashEntry {
    /// Reflog selector (`stash@{0}`), the argument for later stash ops.
    pub name: String,
    pub oid: Oid,
    /// Committer time (unix seconds).
    pub time: i64,
    /// Reflog subject (`WIP on main: ...` or the custom message).
    pub message: String,
    /// Where it draws when its base is a commit a discard's copy made for
    /// it; `None` for a stash git made.
    pub stands: Option<crate::discards::Stands>,
}

/// What the working tree lets a stash do. Every refusal is git's own
/// (デザイン規約 §退避できるかどうかを、押す前に出す).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StashStanding {
    /// No commit yet ("You do not have the initial commit yet").
    Unborn,
    /// A file is unmerged ("needs merge").
    Conflicts,
    /// Nothing uncommitted to set aside.
    Clean,
    /// Anything staged, unstaged or untracked; git takes all three.
    Ready,
}

impl StashStanding {
    /// The word the UI branches on (the toolbar's stash button).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unborn => "unborn",
            Self::Conflicts => "conflicts",
            Self::Clean => "clean",
            Self::Ready => "ready",
        }
    }
}

/// The standing, off a status already in hand (no git runs).
/// `head_missing`: no commit yet (`WorkingTreeStatus::branch_oid` is `None`).
pub fn standing(head_missing: bool, counts: &crate::status::Counts) -> StashStanding {
    if head_missing {
        return StashStanding::Unborn;
    }
    if counts.conflicted > 0 {
        return StashStanding::Conflicts;
    }
    if counts.staged + counts.unstaged + counts.untracked == 0 {
        return StashStanding::Clean;
    }
    StashStanding::Ready
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("malformed stash list output")]
pub struct StashParseError;

/// Parses `stash list` output produced with [`STASH_FORMAT_ARG`] + `-z`.
pub fn parse_stashes(bytes: &[u8]) -> Result<Vec<StashEntry>, StashParseError> {
    let tokens: Vec<&[u8]> = bytes.split(|b| *b == 0).collect();
    // The empty token after the final NUL (the only one, for no output).
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
    for record in tokens.as_chunks::<STASH_FIELDS>().0 {
        let name = String::from_utf8_lossy(record[0]).into_owned();
        let oid = Oid::from_hex(record[1]).map_err(|_| StashParseError)?;
        let time = std::str::from_utf8(record[2])
            .map_err(|_| StashParseError)?
            .trim()
            .parse()
            .map_err(|_| StashParseError)?;
        let message = String::from_utf8_lossy(record[3]).into_owned();
        let parents: Vec<Oid> = String::from_utf8_lossy(record[4])
            .split(' ')
            .filter_map(|hex| Oid::from_hex_str(hex).ok())
            .collect();
        let stands = crate::discards::Stands::of(&parents, &String::from_utf8_lossy(record[5]));
        out.push(StashEntry {
            name,
            oid,
            time,
            message,
            stands,
        });
    }
    Ok(out)
}

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PushOptions {
    pub include_untracked: bool,
    pub keep_index: bool,
    pub staged_only: bool,
}

/// Commit `refs/stash` points at, or `None` with no stash at all.
///
/// Compared before and after a [`push`]: on a clean tree `git stash push`
/// exits 0 having created nothing, and `stash@{0}` is the older entry.
pub async fn tip(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Option<String>, GitError> {
    // Exit 1 is the answer "no stash here".
    let cmd = GitCommand::new().cwd(workdir).answers_by_code(1).args([
        "rev-parse",
        "--verify",
        "--quiet",
        "refs/stash",
    ]);
    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code != 0 {
        return Ok(None);
    }
    let text = out.stdout_utf8().trim().to_string();
    Ok((!text.is_empty()).then_some(text))
}

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
///
/// Answers by code: exit 1 covers both a conflicted restore and a no-op,
/// so the caller weighs it against the working tree, and the write it
/// belongs to says whether it failed.
pub async fn pop(
    executor: &GitExecutor,
    workdir: &Path,
    selector: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .answers_by_code(1)
        .args(["stash", "pop", selector]);
    executor.run(cmd, cancel).await.map(|_| ())
}

/// `git stash pop --index`: restores the entry *and* the split between
/// what was staged in it and what was not.
///
/// Exit 1 as in [`pop`]. On a conflict git gives up on the index part
/// ("Index was not unstashed"), leaves the markers and keeps the entry,
/// which is the way back.
pub async fn pop_with_index(
    executor: &GitExecutor,
    workdir: &Path,
    selector: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .answers_by_code(1)
        .args(["stash", "pop", "--index", selector]);
    executor.run(cmd, cancel).await.map(|_| ())
}

/// `git stash apply <selector>`: restores and keeps the entry. Exit 1 as
/// in [`pop`].
pub async fn apply(
    executor: &GitExecutor,
    workdir: &Path,
    selector: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .answers_by_code(1)
        .args(["stash", "apply", selector]);
    executor.run(cmd, cancel).await.map(|_| ())
}

/// Whether a string can be a stash's label. The reflog subject is free
/// text, so the only rules are the reflog's: one line (no control
/// characters), not blank.
pub fn is_valid_message(message: &str) -> bool {
    !message.trim().is_empty() && !message.chars().any(|c| c.is_control())
}

/// The label somebody gave a stash, out of its reflog subject; empty
/// where git wrote the whole subject itself.
///
/// git leaves three shapes:
/// - `WIP on <branch>: <abbrev> <subject>` — pushed with no message. It
///   names the commit the work stood on, not the work, so it answers empty.
/// - `On <branch>: <label>` — pushed with `--message`. Cut at the first
///   `": "`, since a detached HEAD puts `(no branch)` in that slot.
/// - `<label>` — put there by `stash store` ([`rename`]).
///
/// Nothing marks where a prefix ends, so a label that opens like one is
/// misread: `On second thought: …` loses its first words, and one opening
/// `WIP on ` answers empty (read by the caller as unnamed). Accepted.
#[must_use]
pub fn label_in(message: &str) -> &str {
    if message.starts_with("WIP on ") {
        return "";
    }
    if let Some((_, label)) = message
        .strip_prefix("On ")
        .and_then(|rest| rest.split_once(": "))
    {
        return label;
    }
    message
}

/// `git stash drop <selector>`: discards the entry. The caller confirms
/// first.
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
