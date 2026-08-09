//! Merge, rebase, cherry-pick and revert, plus the continue / abort / skip
//! routing they share.
//!
//! All four can stop halfway and leave the repository mid-operation; that is
//! normal, not an error. The caller reports git's message and re-reads
//! [`crate::opstate`] to find out where things stand.
//!
//! `--no-edit` is passed wherever git accepts it. The process layer already
//! pins `GIT_EDITOR=true` so nothing can hang on an editor, but being
//! explicit keeps the intent visible in the command line we log.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::opstate::{self, OpState};
use crate::process::{GitCommand, GitExecutor};

/// Knobs of `git merge`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MergeOptions {
    /// Always record a merge commit (`--no-ff`).
    pub no_ff: bool,
    /// Refuse anything but a fast-forward (`--ff-only`).
    pub ff_only: bool,
    /// Bring the changes in without committing (`--squash`).
    pub squash: bool,
    /// Replaces the generated merge message.
    pub message: Option<String>,
}

/// `git merge <rev>`.
pub async fn merge(
    executor: &GitExecutor,
    workdir: &Path,
    rev: &str,
    options: &MergeOptions,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let mut cmd = GitCommand::new().cwd(workdir).args(["merge", "--no-edit"]);
    if options.no_ff {
        cmd = cmd.arg("--no-ff");
    }
    if options.ff_only {
        cmd = cmd.arg("--ff-only");
    }
    if options.squash {
        cmd = cmd.arg("--squash");
    }
    if let Some(message) = &options.message {
        cmd = cmd.args(["-m", message]);
    }
    executor.run(cmd.args(["--", rev]), cancel).await.map(drop)
}

/// Knobs of `git rebase`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RebaseOptions {
    /// `--onto <newbase>`: transplant onto something other than `upstream`.
    pub onto: Option<String>,
    /// Which branch to rebase; HEAD when `None`.
    pub branch: Option<String>,
    /// Move refs that pointed into the rewritten range along with it
    /// (git 2.38+; the minimum supported version is well past that).
    pub update_refs: bool,
    /// `--root`: replay from the first commit, which has no parent to name
    /// as upstream. The `upstream` argument is left off entirely — editing
    /// the very first commit is impossible otherwise.
    pub root: bool,
}

/// What a rebase did — the plain one here and the driven one in
/// [`crate::sequencer`] answer alike, because git refuses both over a
/// dirty working tree in the very same words (実測).
#[derive(Debug)]
pub enum RebaseOutcome {
    /// git took the rebase through to the end. One that stopped part-way
    /// is *not* this: git exits non-zero and the error carries its
    /// message, with the sequencer state left for the UI to read.
    Done,
    /// git refused before touching anything, because uncommitted work is
    /// in the way, so the repository is exactly as it was. Carries the
    /// refusal itself: the caller goes round again through a stash
    /// (`RepoSession`'s carry), and a second refusal — the tree is empty
    /// by then, so something git cannot see past is holding it — has to
    /// say why it gave up, in git's own words.
    Blocked(GitError),
}

/// `git rebase <upstream>`.
///
/// `--autostash` is not passed, here or anywhere: it restores with a
/// plain `stash apply`, so everything that was staged comes back
/// unstaged, and no flag turns that off. A dirty tree is answered rather
/// than failed, and the caller carries the work across itself
/// (デザイン規約 §未コミット変更がある状態で履歴を書き換える).
pub async fn rebase(
    executor: &GitExecutor,
    workdir: &Path,
    upstream: &str,
    options: &RebaseOptions,
    cancel: &CancellationToken,
) -> Result<RebaseOutcome, GitError> {
    // Exit 1 is this command answering rather than failing, and both
    // answers have a landing of their own on screen: "your work is in the
    // way" sends the caller round through a stash, and a rebase that
    // stopped part-way raises the badge and the exit card. Only 0 and 1
    // are answers, so the 128 a name git does not know exits with still
    // reads as the failure it is (規約 §終了コードで答える問い合わせ).
    let cmd = rebase_command(workdir, upstream, options, None).answers_by_code();
    refusal_or(executor.run(cmd, cancel).await.map(drop))
}

/// Sorts a rebase's result into [`RebaseOutcome`], shared by the plain
/// rebase above and the driven one in [`crate::sequencer`].
pub(crate) fn refusal_or(result: Result<(), GitError>) -> Result<RebaseOutcome, GitError> {
    match result {
        Ok(()) => Ok(RebaseOutcome::Done),
        Err(GitError::Failed {
            command,
            code,
            stderr,
        }) if work_is_in_the_way(&stderr) => Ok(RebaseOutcome::Blocked(GitError::Failed {
            command,
            code,
            stderr,
        })),
        Err(other) => Err(other),
    }
}

/// Whether git's refusal is the "commit or stash them" one it gives
/// before touching anything, which a stash gets past.
///
/// Classifying human-facing output is otherwise off limits here, and this
/// earns the same exception [`crate::branch`] takes for `switch`: git
/// offers no machine-readable answer to "why will you not rebase", and
/// the answer decides whether the rebase is worth a second attempt. Every
/// invocation runs under `LC_ALL=C`, so the C-locale wording arrives.
///
/// The two wordings are the halves of git's own clean-tree check —
/// "cannot rebase: You have unstaged changes." and "cannot rebase: Your
/// index contains uncommitted changes." — and a plain rebase and an
/// interactive one word them identically (実測 2.55, both in
/// `integrate_integration`). Untracked files are not in the way at all: a
/// rebase over a tree holding only those goes straight through, so
/// nothing is stashed for them.
///
/// Anything unrecognised is `false` and travels on as an ordinary error:
/// a reworded message costs the retry, never correctness.
fn work_is_in_the_way(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    text.contains("cannot rebase:")
        && (text.contains("unstaged changes") || text.contains("uncommitted changes"))
}

/// Builds a rebase command; `todo_editor` turns it into an interactive one
/// driven by that `GIT_SEQUENCE_EDITOR` (see [`crate::sequencer`]).
pub(crate) fn rebase_command(
    workdir: &Path,
    upstream: &str,
    options: &RebaseOptions,
    todo_editor: Option<&str>,
) -> GitCommand {
    let mut cmd = GitCommand::new().cwd(workdir).arg("rebase");
    if options.update_refs {
        cmd = cmd.arg("--update-refs");
    }
    if let Some(editor) = todo_editor {
        cmd = cmd.arg("--interactive").env("GIT_SEQUENCE_EDITOR", editor);
    }
    if let Some(onto) = &options.onto {
        cmd = cmd.args(["--onto", onto]);
    }
    if options.root {
        cmd = cmd.arg("--root");
    } else {
        cmd = cmd.args(["--", upstream]);
    }
    if let Some(branch) = &options.branch {
        cmd = cmd.arg(branch);
    }
    cmd
}

/// `git cherry-pick <revs>`.
pub async fn cherry_pick(
    executor: &GitExecutor,
    workdir: &Path,
    revs: &[String],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    if revs.is_empty() {
        return Ok(());
    }
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["cherry-pick", "--no-edit", "--"])
        .args(revs.iter().map(String::as_str));
    executor.run(cmd, cancel).await.map(drop)
}

/// `git revert <revs>`.
pub async fn revert(
    executor: &GitExecutor,
    workdir: &Path,
    revs: &[String],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    if revs.is_empty() {
        return Ok(());
    }
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["revert", "--no-edit", "--"])
        .args(revs.iter().map(String::as_str));
    executor.run(cmd, cancel).await.map(drop)
}

/// How to leave an operation that stopped part-way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Continuation {
    /// Carry on with what is staged now.
    Continue,
    /// Undo everything and return to where the operation started.
    Abort,
    /// Drop the current commit and move to the next one.
    Skip,
    /// Stop stepping but keep the tree as it is (sequencer ops only).
    Quit,
}

impl Continuation {
    fn flag(self) -> &'static str {
        match self {
            Continuation::Continue => "--continue",
            Continuation::Abort => "--abort",
            Continuation::Skip => "--skip",
            Continuation::Quit => "--quit",
        }
    }
}

/// Which operation a continuation applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InProgress {
    Rebase,
    Merge,
    CherryPick,
    Revert,
}

impl InProgress {
    fn command(self) -> &'static str {
        match self {
            InProgress::Rebase => "rebase",
            InProgress::Merge => "merge",
            InProgress::CherryPick => "cherry-pick",
            InProgress::Revert => "revert",
        }
    }

    fn supports(self, continuation: Continuation) -> bool {
        match self {
            // `git merge` steps through nothing, so there is nothing to
            // skip or quit.
            InProgress::Merge => {
                matches!(continuation, Continuation::Continue | Continuation::Abort)
            }
            _ => true,
        }
    }

    /// The operation an [`OpState`] describes, if any.
    ///
    /// Order matters: a rebase that stops on a conflicting pick also writes
    /// `CHERRY_PICK_HEAD`, and the rebase is what the user must continue.
    pub fn from_state(state: &OpState) -> Option<Self> {
        if state.rebasing {
            Some(InProgress::Rebase)
        } else if state.merging {
            Some(InProgress::Merge)
        } else if state.cherry_picking {
            Some(InProgress::CherryPick)
        } else if state.reverting {
            Some(InProgress::Revert)
        } else {
            None
        }
    }
}

/// Continues, aborts or skips whatever is currently in progress.
///
/// Returns `Ok(false)` when nothing is in progress, so a UI button can be a
/// no-op instead of an error.
pub async fn resolve_current(
    executor: &GitExecutor,
    workdir: &Path,
    continuation: Continuation,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let state = opstate::detect(executor, workdir, cancel).await?;
    let Some(op) = InProgress::from_state(&state) else {
        return Ok(false);
    };
    resolve(executor, workdir, op, continuation, cancel).await?;
    Ok(true)
}

/// Continues, aborts or skips a named operation.
pub async fn resolve(
    executor: &GitExecutor,
    workdir: &Path,
    op: InProgress,
    continuation: Continuation,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    if !op.supports(continuation) {
        return Err(GitError::UnexpectedOutput {
            command: format!("git {} {}", op.command(), continuation.flag()),
            message: format!("{} does not support {}", op.command(), continuation.flag()),
        });
    }
    let mut cmd = GitCommand::new()
        .cwd(workdir)
        .args([op.command(), continuation.flag()]);
    // Continuing a sequencer op would otherwise open an editor for the
    // commit message it is about to write. `git merge --continue` and
    // `git rebase --continue` accept no arguments at all — they rely on
    // GIT_EDITOR, which the process layer pins to `true`.
    if continuation == Continuation::Continue
        && matches!(op, InProgress::CherryPick | InProgress::Revert)
    {
        cmd = cmd.arg("--no-edit");
    }
    executor.run(cmd, cancel).await.map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_supports_only_continue_and_abort() {
        assert!(InProgress::Merge.supports(Continuation::Continue));
        assert!(InProgress::Merge.supports(Continuation::Abort));
        assert!(!InProgress::Merge.supports(Continuation::Skip));
        assert!(InProgress::CherryPick.supports(Continuation::Skip));
    }

    #[test]
    fn rebase_wins_over_the_cherry_pick_head_it_leaves_behind() {
        let state = OpState {
            rebasing: true,
            cherry_picking: true,
            ..Default::default()
        };
        assert_eq!(InProgress::from_state(&state), Some(InProgress::Rebase));
        assert_eq!(InProgress::from_state(&OpState::default()), None);
    }

    /// Both halves of git's clean-tree check, word for word as 2.55 wrote
    /// them; the integration tests run the real thing, against a plain
    /// rebase and an interactive one alike.
    #[test]
    fn the_two_refusals_a_stash_gets_past() {
        assert!(work_is_in_the_way(
            "error: cannot rebase: You have unstaged changes.\n\
             error: Please commit or stash them."
        ));
        assert!(work_is_in_the_way(
            "error: cannot rebase: Your index contains uncommitted changes.\n\
             error: Please commit or stash them."
        ));
    }

    /// Anything else is an ordinary failure: a stash would not help, and
    /// retrying would rebase over a repository git has already touched.
    #[test]
    fn a_stopped_rebase_is_not_a_refusal() {
        assert!(!work_is_in_the_way(
            "error: could not apply 36726c1... c3\n\
             hint: Resolve all conflicts manually"
        ));
        assert!(!work_is_in_the_way(
            "fatal: It seems that there is already a rebase-merge directory"
        ));
        assert!(!work_is_in_the_way(
            "error: cannot rebase onto multiple branches"
        ));
        assert!(!work_is_in_the_way(""));
    }
}
