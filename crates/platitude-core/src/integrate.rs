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
    // Exit 1 is this command answering rather than failing. Only 0 and 1
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
///
/// `--allow-empty` is about commits that were empty when they were made:
/// such a commit is exactly what was asked for, so it lands as it stands
/// instead of stopping to ask. Without the flag git stops on those too,
/// in the same words it uses for the commit that turns out to add
/// nothing — and those two are not the same answer (実測 2.55).
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
        .args(["cherry-pick", "--no-edit", "--allow-empty", "--"])
        .args(revs.iter().map(String::as_str))
        .answers_by_code();
    skip_past_empty_commits(
        executor,
        workdir,
        InProgress::CherryPick,
        cmd,
        revs.len(),
        cancel,
    )
    .await
}

/// `git revert <revs>`.
///
/// `--allow-empty` has no counterpart here (git rejects it outright), so
/// a revert of a commit whose undoing is already in the branch is the
/// one empty case, and it is walked past like the cherry-pick above.
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
        .args(revs.iter().map(String::as_str))
        .answers_by_code();
    skip_past_empty_commits(
        executor,
        workdir,
        InProgress::Revert,
        cmd,
        revs.len(),
        cancel,
    )
    .await
}

/// Runs a cherry-pick / revert to the end, taking git up on its own
/// `--skip` for every commit that leaves nothing to record.
///
/// A commit whose changes the branch already has writes no commit, and
/// both commands stop there rather than dropping it: exit 1, the
/// sequencer state left standing, and a message naming `--skip`. **That
/// stop asks nothing of the person who pressed the row** — no conflict
/// to resolve, the tree untouched, the branch already holding what was
/// to be copied — so it is answered here instead of arriving on screen
/// as a failed operation with a badge behind it
/// (デザイン規約 §履歴を合流させる). A `rebase` needs none of this: it
/// drops such commits by itself, and the `--empty=drop` that would say
/// so in one word only reached these two commands in git 2.45, past the
/// minimum this app supports
/// (internal-docs/git最低バージョン整合.md).
///
/// The number of commits bounds the loop: each `--skip` moves the
/// sequence on by one, so no more skips can be wanted than there were
/// commits to replay.
async fn skip_past_empty_commits(
    executor: &GitExecutor,
    workdir: &Path,
    op: InProgress,
    first: GitCommand,
    revs: usize,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let mut outcome = executor.run(first, cancel).await.map(drop);
    for _ in 0..revs {
        match &outcome {
            Err(error) if left_nothing_to_record(op, error) => {}
            _ => return outcome,
        }
        // A stop that left nothing standing has nothing to skip, and
        // `--skip` would answer "no revert in progress" with 128 — a red
        // row in the log for a repository that is perfectly in order.
        if !still_stepping(executor, workdir, op, cancel).await? {
            return Ok(());
        }
        let skip = GitCommand::new()
            .cwd(workdir)
            .args([op.command(), "--skip"])
            .answers_by_code();
        outcome = executor.run(skip, cancel).await.map(drop);
    }
    outcome
}

/// Whether git still holds this operation open, by either of the two
/// things a `--skip` needs: the marker it left behind, or a sequence
/// with steps still in it.
///
/// A revert that records nothing has neither when it was asked for one
/// commit (git refuses the commit before writing `REVERT_HEAD`, and a
/// single revert never opens a sequence at all), and only the sequence
/// when it was asked for several. A cherry-pick leaves its marker
/// either way (実測 2.55).
///
/// Another operation standing there is not this one: a rebase stopped
/// on a conflicting pick owns both the sequence and `CHERRY_PICK_HEAD`,
/// and it is not for a cherry-pick to step it on.
async fn still_stepping(
    executor: &GitExecutor,
    workdir: &Path,
    op: InProgress,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let state = opstate::detect(executor, workdir, cancel).await?;
    match InProgress::from_state(&state) {
        Some(current) => Ok(current == op),
        None => opstate::sequence_pending(executor, workdir, cancel).await,
    }
}

/// Whether git stopped because the commit it just replayed records
/// nothing — the branch has those changes already.
///
/// Classifies human-facing output under the same exception
/// [`work_is_in_the_way`] takes. `LC_ALL=C` pins the C-locale wording,
/// which names the command that stopped (実測 2.55, both wordings in
/// `integrate_integration`).
///
/// Anything unrecognised is `false` and travels on as the failure it
/// looks like: a reworded message costs the walk past, never
/// correctness.
fn left_nothing_to_record(op: InProgress, error: &GitError) -> bool {
    let GitError::Failed { stderr, .. } = error else {
        return false;
    };
    let said = stderr.to_ascii_lowercase();
    // cherry-pick weighs `--allow-empty` before writing, so it stops in
    // words of its own, naming the command and the `--skip` that leaves.
    if said.contains(&format!("the previous {} is now empty", op.command())) {
        return true;
    }
    // revert has no `--allow-empty` to weigh (git rejects the flag), so
    // it never reaches that message: the commit it was about to write is
    // refused by `git commit` itself, which says so on stdout and leaves
    // stderr empty — which is what [`crate::process::GitOutput::failure_message`]
    // passed through here. Every other way a revert stops writes to
    // stderr, conflicts included (実測 2.55).
    op == InProgress::Revert && said.contains("nothing to commit")
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

    fn stopped(text: &str) -> GitError {
        GitError::Failed {
            command: "git".to_string(),
            code: 1,
            stderr: text.to_string(),
        }
    }

    /// The two ways git says "that commit records nothing", word for
    /// word as 2.55 wrote them: cherry-pick weighs `--allow-empty` and
    /// names itself, revert leaves stderr empty and lets `git commit`
    /// answer on stdout. Both shapes are run for real in
    /// `integrate_integration`.
    #[test]
    fn the_two_stops_that_mean_the_branch_has_it_already() {
        assert!(left_nothing_to_record(
            InProgress::CherryPick,
            &stopped(
                "The previous cherry-pick is now empty, possibly due to \
                 conflict resolution.\nIf you wish to commit it anyway, use:\n\n    \
                 git commit --allow-empty\n\nOtherwise, please use \
                 'git cherry-pick --skip'"
            )
        ));
        assert!(left_nothing_to_record(
            InProgress::Revert,
            &stopped("On branch main\nnothing to commit, working tree clean")
        ));
    }

    /// Each command reads only its own stop. The bare commit refusal is
    /// a revert's alone — a cherry-pick reaching it has been stopped by
    /// something this does not know, and unknown stops travel on.
    #[test]
    fn a_command_does_not_walk_past_another_ones_stop() {
        assert!(!left_nothing_to_record(
            InProgress::Revert,
            &stopped("The previous cherry-pick is now empty, possibly due to conflict resolution.")
        ));
        assert!(!left_nothing_to_record(
            InProgress::CherryPick,
            &stopped("On branch main\nnothing to commit, working tree clean")
        ));
        assert!(!left_nothing_to_record(
            InProgress::CherryPick,
            &stopped("")
        ));
    }

    /// A conflict is a stop with work left in it, and a name git does
    /// not know is a plain failure. Both belong on screen.
    #[test]
    fn a_conflicted_stop_is_not_an_empty_one() {
        assert!(!left_nothing_to_record(
            InProgress::CherryPick,
            &stopped(
                "error: could not apply 36726c1... c3\n\
                 hint: Resolve all conflicts manually"
            )
        ));
        assert!(!left_nothing_to_record(
            InProgress::CherryPick,
            &stopped("fatal: bad revision 'nope'")
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
