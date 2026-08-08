//! Checkout and local branch management.
//!
//! Uses `switch` rather than `checkout` throughout. `checkout` doubles as a
//! file-restoring command, so a branch whose name collides with a path is
//! ambiguous; `switch` only ever moves HEAD and says so in its errors. The
//! same split is why unstaging uses `restore` (see [`crate::stage`]).

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// What a checkout should land on.
///
/// Every variant lands on a branch. Nothing here detaches HEAD: a branch
/// is what the next commit needs somewhere to go, and the UI offers to
/// make one wherever a bare commit is what was pointed at (デザイン規約
/// §ブランチ・コミットへの移動). A HEAD already detached — left by git
/// itself, or by the command line — is read and worked from as normal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckoutTarget {
    /// An existing local branch.
    Branch { name: String },
    /// A remote-tracking branch: creates `local` tracking it and switches.
    Track { remote_ref: String, local: String },
    /// An existing local branch, moved to `start` before landing on it.
    ///
    /// Commits only that branch had are left unreferenced, so the UI asks
    /// before running this one. The move and the landing are one command
    /// (`switch --force-create`): git either does both or neither, and a
    /// working tree in the way still refuses the whole thing.
    ForceCreate { local: String, start: String },
}

/// What a move did.
#[derive(Debug)]
pub enum CheckoutOutcome {
    /// HEAD moved, carrying whatever uncommitted work did not stand in
    /// the way.
    Moved,
    /// git refused because uncommitted work stands in the way, and
    /// aborted before touching anything, so the repository is exactly as
    /// it was. Carries the refusal itself: the caller goes round again
    /// through a stash (`RepoSession::checkout`), and a second refusal —
    /// the tree is empty by then, so something git cannot see past is
    /// holding it — has to say why it gave up, in git's own words.
    Blocked(GitError),
}

/// Whether git's refusal is the everyday "your work is in the way" one,
/// which a stash gets past.
///
/// Classifying human-facing output is otherwise off limits here, and this
/// is the one place that earns the exception: git offers no
/// machine-readable answer to "why can I not move", and the answer decides
/// whether the move is worth a second attempt. Every invocation runs under
/// `LC_ALL=C`, so the C-locale wording is what arrives.
///
/// Anything unrecognised is `false` and travels on as an ordinary error: a
/// reworded message costs the retry, never correctness.
fn work_is_in_the_way(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    // "The following untracked working tree files would be overwritten by
    // checkout:", the singular "Untracked working tree file 'x' would be
    // overwritten by merge." a restore runs into, and "Your local changes
    // to the following files would be overwritten by checkout:".
    text.contains("untracked working tree file")
        || text.contains("would lose untracked files")
        || text.contains("would be overwritten by checkout")
}

/// Moves HEAD to `target`, taking uncommitted work along where git will
/// have it.
///
/// Deliberately never `--merge`: that flag would three-way merge the
/// changes in, but it reports a conflicted result as a *success* with no
/// merge left to abort, and refuses to run at all while anything is
/// staged. Carrying changes over a collision is done by stashing across
/// the move instead (`RepoSession::checkout`), which keeps both the
/// staged/unstaged split and a way back.
pub async fn checkout(
    executor: &GitExecutor,
    workdir: &Path,
    target: &CheckoutTarget,
    cancel: &CancellationToken,
) -> Result<CheckoutOutcome, GitError> {
    let cmd = GitCommand::new().cwd(workdir).arg("switch");
    let cmd = match target {
        CheckoutTarget::Branch { name } => cmd.args(["--", name.as_str()]),
        CheckoutTarget::Track { remote_ref, local } => {
            cmd.args(["--create", local, "--track", remote_ref])
        }
        // No `--track`: a branch that already exists keeps whatever
        // upstream it was given, and moving it says nothing about that.
        CheckoutTarget::ForceCreate { local, start } => cmd.args(["--force-create", local, start]),
    };
    match executor.run(cmd, cancel).await {
        Ok(_) => Ok(CheckoutOutcome::Moved),
        Err(GitError::Failed {
            command,
            code,
            stderr,
        }) => {
            let refusal = GitError::Failed {
                command,
                code,
                stderr,
            };
            match &refusal {
                GitError::Failed { stderr, .. } if work_is_in_the_way(stderr) => {
                    Ok(CheckoutOutcome::Blocked(refusal))
                }
                _ => Err(refusal),
            }
        }
        Err(other) => Err(other),
    }
}

/// What a reset does to the index and the working tree once the branch
/// itself has moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetMode {
    /// Branch only. Whatever the commits left behind changed stays in the
    /// index, on top of anything staged already.
    Soft,
    /// Branch and index. Every file keeps the content it has on disk;
    /// none of it is staged any more.
    Mixed,
    /// Branch, index and working tree. Uncommitted work is destroyed —
    /// git kept no copy of it to restore.
    Hard,
}

impl ResetMode {
    fn flag(self) -> &'static str {
        match self {
            ResetMode::Soft => "--soft",
            ResetMode::Mixed => "--mixed",
            ResetMode::Hard => "--hard",
        }
    }
}

/// Moves the ref HEAD is on (the current branch) to `rev`.
///
/// `rev` is a commit, never a path, so it goes *before* any `--`: to
/// `git reset` a `--` opens the pathspec form, which takes no mode flag
/// at all. `--end-of-options` does the guarding a `--` does elsewhere.
///
/// Not a way out of an operation in progress, and the UI does not offer
/// it as one: mid-merge, `Soft` refuses outright ("Cannot do a soft reset
/// in the middle of a merge") while the other two drop `MERGE_HEAD`
/// without a word, abandoning the merge as a side effect (実測).
pub async fn reset(
    executor: &GitExecutor,
    workdir: &Path,
    rev: &str,
    mode: ResetMode,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new().cwd(workdir).args([
        "reset",
        "--quiet",
        mode.flag(),
        "--end-of-options",
        rev,
    ]);
    executor.run(cmd, cancel).await.map(drop)
}

/// Creates a branch at `start_point` (HEAD when `None`), optionally
/// switching to it.
pub async fn create(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    start_point: Option<&str>,
    switch_to: bool,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new().cwd(workdir);
    let cmd = if switch_to {
        cmd.args(["switch", "--create", name])
    } else {
        cmd.args(["branch", "--", name])
    };
    let cmd = match start_point {
        Some(start) => cmd.arg(start),
        None => cmd,
    };
    executor.run(cmd, cancel).await.map(drop)
}

/// Deletes a local branch. `force` maps to `-D` (drops unmerged work);
/// without it git refuses to delete an unmerged branch itself.
pub async fn delete(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    force: bool,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let flag = if force { "-D" } else { "-d" };
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["branch", flag, "--", name]);
    executor.run(cmd, cancel).await.map(drop)
}

/// Renames a local branch. `force` allows overwriting an existing name.
pub async fn rename(
    executor: &GitExecutor,
    workdir: &Path,
    from: &str,
    to: &str,
    force: bool,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let flag = if force { "-M" } else { "-m" };
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["branch", flag, "--", from, to]);
    executor.run(cmd, cancel).await.map(drop)
}

/// True when every commit of `rev` is already reachable from `into`.
///
/// This is what makes deleting a branch safe; the UI asks before offering
/// the forced delete.
pub async fn is_merged_into(
    executor: &GitExecutor,
    workdir: &Path,
    rev: &str,
    into: &str,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["merge-base", "--is-ancestor", rev, into])
        // Exit 1 here means "no", which is half of what this asks. Left
        // unmarked, the command log would read it as a failure and raise
        // itself over an answer.
        .answers_by_code();
    let out = executor.run_unchecked(cmd, cancel).await?;
    match out.code {
        0 => Ok(true),
        1 => Ok(false),
        code => Err(GitError::Failed {
            command: format!("git merge-base --is-ancestor {rev} {into}"),
            code,
            stderr: out.stderr_utf8().trim().to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The messages git actually prints under LC_ALL=C. The integration
    // tests prove the installed git still says them; these pin down which
    // ones are worth a second attempt through a stash.

    #[test]
    fn tracked_collisions_are_worth_another_go() {
        let stderr = "error: Your local changes to the following files would \
                      be overwritten by checkout:\n\ta.txt\nPlease commit your \
                      changes or stash them before you switch branches.\nAborting";
        assert!(work_is_in_the_way(stderr));
    }

    #[test]
    fn untracked_collisions_are_too() {
        let plain = "error: The following untracked working tree files would \
                     be overwritten by checkout:\n\tc.txt\nPlease move or remove \
                     them before you switch branches.\nAborting";
        let merging = "error: Untracked working tree file 'c.txt' would be \
                       overwritten by merge.";
        assert!(work_is_in_the_way(plain));
        assert!(
            work_is_in_the_way(merging),
            "the merging path names one file at a time"
        );
    }

    #[test]
    fn every_other_failure_stays_an_error() {
        for stderr in [
            "fatal: invalid reference: nope",
            "error: you need to resolve your current index first\na.txt: needs merge",
            "fatal: cannot continue with staged changes in the following files:\na.txt",
        ] {
            assert!(!work_is_in_the_way(stderr), "{stderr}");
        }
    }
}
