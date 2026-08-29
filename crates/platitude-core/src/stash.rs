//! Stash listing and stash operations.
//!
//! Selectors (`stash@{0}`) come from a listing and are passed straight
//! back — the numbering shifts under every push and drop, so a selector
//! this module invents could name someone else's entry. The one place
//! that must invent one is [`rename`], whose own `store` pushed the list
//! down by one; it shifts the selector and then proves it still names
//! the same commit (by oid) before dropping anything.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::oid::Oid;
use crate::process::{GitCommand, GitExecutor};

mod rename;

pub use rename::rename;

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

/// What the working tree lets a stash do. Every refusal here is git's
/// own, measured rather than guessed (rules-refs/app-ui.md §stash):
/// before the first commit git turns the write down flat however dirty
/// the folder is; with a file unmerged it refuses the whole write, not
/// the unmerged path.
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

/// The standing itself, off a status already in hand — no git runs.
/// `head_missing` is "the repository has no commit yet"
/// (`WorkTreeStatus::branch_oid` is `None`).
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

/// Commit `refs/stash` points at, or `None` with no stash at all.
///
/// Taken before and after a [`push`] to tell whether an entry was really
/// made: on a clean tree `git stash push` exits 0 having created nothing,
/// and `stash@{0}` then names whatever entry was already there.
pub async fn tip(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Option<String>, GitError> {
    // Exit 1 is the answer "there is no stash here", which every
    // repository that has never had one gives. Left unmarked it reads as
    // a failed command and raises the log over a question nobody asked.
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
///
/// Answers by code for the same reason [`pop_with_index`] does: exit 1
/// covers both a restore that landed conflicted and one that did nothing,
/// so it is data for the caller to weigh against the working tree, not a
/// failure to report on sight. Whether it turns out to be one is said by
/// the write it belongs to.
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
        .answers_by_code(1)
        .args(["stash", "pop", "--index", selector]);
    executor.run(cmd, cancel).await.map(|_| ())
}

/// `git stash apply <selector>`: restores and keeps the entry.
///
/// Answers by code for the same reason [`pop`] does: exit 1 covers both a
/// restore that landed conflicted and one that did nothing, so it is data
/// for the caller to weigh against the working tree, not a failure to
/// report on sight.
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

/// Whether a string can be a stash's label.
///
/// Not a ref name — a stash is named by its reflog subject, which is free
/// text — so the only rules are the reflog's own: one line, and something
/// on it.
pub fn is_valid_message(message: &str) -> bool {
    !message.trim().is_empty() && !message.chars().any(|c| c.is_control())
}

/// The label somebody gave a stash, out of the reflog subject a listing
/// carries — empty where git wrote the whole subject itself.
///
/// git leaves three shapes on the reflog (実測):
/// - `WIP on <branch>: <abbrev> <subject>` — its own, for an entry pushed
///   with no message. That names the commit the work was standing on, not
///   the work, so nobody gave this one a label and it answers empty.
/// - `On <branch>: <label>` — an entry pushed with `--message`. The prefix
///   is cut at the first `": "` and not at the first space: a detached
///   HEAD puts `(no branch)` in that slot.
/// - `<label>` — an entry put on the reflog by `stash store`, which is
///   what [`rename`] is built out of; there is no branch for it to name.
///
/// The reflog holds one line with nothing marking where a prefix ends, so
/// git cannot tell a label that opens like one of its own from the prefix
/// it writes, and neither can this: `On second thought: …` loses its first
/// words, and a label opening `WIP on ` answers empty. Both are recorded
/// rather than fixed — the caller reads an empty answer as "nobody named
/// this one", which is what an entry pushed without a message is.
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

    #[test]
    fn a_label_is_one_line_with_something_on_it() {
        assert!(is_valid_message("half of the refactor"));
        assert!(is_valid_message("日本語のラベル"));
        assert!(!is_valid_message(""));
        assert!(!is_valid_message("   "));
        assert!(!is_valid_message("two\nlines"));
        assert!(!is_valid_message("tab\there"));
    }

    /// The three shapes measured out of git, and the one it cannot tell
    /// apart from a prefix it wrote.
    #[test]
    fn a_label_is_read_out_of_the_prefix_git_put_on_it() {
        assert_eq!(
            label_in("On main: half of the refactor"),
            "half of the refactor"
        );
        assert_eq!(
            label_in("On (no branch): on a detached head"),
            "on a detached head"
        );
        // A summary written for a commit has its own colon in it, and the
        // cut is at the first `": "` — the one git made.
        assert_eq!(
            label_in("On main: feat: write the summary"),
            "feat: write the summary"
        );
        // git's own, naming the commit the work stood on.
        assert_eq!(label_in("WIP on main: 1234567 subject"), "");
        assert_eq!(label_in("WIP on (no branch): 1234567 subject"), "");
        // Put on the reflog by `stash store` (a rename), which has no
        // branch to name.
        assert_eq!(label_in("a plain label"), "a plain label");
        assert_eq!(
            label_in("On its own with no colon"),
            "On its own with no colon"
        );
        // The two ambiguities, recorded rather than fixed: a label that
        // opens the way one of git's prefixes does is read as one.
        assert_eq!(label_in("On second thought: revert it"), "revert it");
        assert_eq!(label_in("WIP on the parser"), "");
    }

    #[test]
    fn the_standing_names_the_one_refusal_that_applies() {
        use crate::status::Counts;
        let counts = |staged, unstaged, untracked, conflicted| Counts {
            staged,
            unstaged,
            untracked,
            conflicted,
            partially_staged: 0,
        };
        // Unborn outranks everything: git refuses however dirty the
        // folder is.
        assert_eq!(standing(true, &counts(1, 2, 3, 0)).as_str(), "unborn");
        assert_eq!(standing(false, &counts(1, 0, 0, 2)).as_str(), "conflicts");
        assert_eq!(standing(false, &counts(0, 0, 0, 0)).as_str(), "clean");
        assert_eq!(standing(false, &counts(0, 0, 1, 0)).as_str(), "ready");
        assert_eq!(standing(false, &counts(2, 1, 0, 0)).as_str(), "ready");
    }
}
