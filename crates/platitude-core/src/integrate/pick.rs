//! `git cherry-pick` and `git revert`, and the commits they walk past.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use super::{InProgress, Landing, opstate};
use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// `git cherry-pick <revs>`.
///
/// `--allow-empty` is about commits that were empty when they were made:
/// such a commit is exactly what was asked for, so it lands as it stands
/// instead of stopping to ask. Without the flag git stops on those too,
/// in the same words it uses for the commit that turns out to add
/// nothing — and those two are not the same answer (実測 2.55).
///
/// A conflict is [`Landing::Stopped`], not a failure: copying a commit
/// onto a branch that has moved on ends there as ordinarily as merging
/// one does (デザイン規約 §進行中の操作から出る).
pub async fn cherry_pick(
    executor: &GitExecutor,
    workdir: &Path,
    revs: &[String],
    cancel: &CancellationToken,
) -> Result<Landing, GitError> {
    if revs.is_empty() {
        return Ok(Landing::Done);
    }
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["cherry-pick", "--no-edit", "--allow-empty", "--"])
        .args(revs.iter().map(String::as_str))
        .answers_by_code(1);
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
/// one empty case, and it is walked past like the cherry-pick above. A
/// conflict lands the same way that one's does.
pub async fn revert(
    executor: &GitExecutor,
    workdir: &Path,
    revs: &[String],
    cancel: &CancellationToken,
) -> Result<Landing, GitError> {
    if revs.is_empty() {
        return Ok(Landing::Done);
    }
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["revert", "--no-edit", "--"])
        .args(revs.iter().map(String::as_str))
        .answers_by_code(1);
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
) -> Result<Landing, GitError> {
    let mut outcome = executor.run(first, cancel).await.map(drop);
    for _ in 0..revs {
        match &outcome {
            Err(error) if left_nothing_to_record(op, error) => {}
            _ => return landed(executor, workdir, op, outcome, cancel).await,
        }
        // A stop that left nothing standing has nothing to skip, and
        // `--skip` would answer "no revert in progress" with 128 — a red
        // row in the log for a repository that is perfectly in order.
        if !still_stepping(executor, workdir, op, cancel).await? {
            return Ok(Landing::Done);
        }
        let skip = GitCommand::new()
            .cwd(workdir)
            .args([op.command(), "--skip"])
            .answers_by_code(1);
        outcome = executor.run(skip, cancel).await.map(drop);
    }
    landed(executor, workdir, op, outcome, cancel).await
}

/// Where the sequence came to rest: through to the end, standing there
/// for someone to finish, or failed.
///
/// The empty stops are already behind this — the loop above answers those
/// itself — so what arrives here is a conflict, a name git could not
/// read, or a tree it would not write over. Only the first leaves the
/// operation standing (実測 2.55: the other two exit 128 with no marker
/// of any kind), and that one is the landing the working tree is the
/// answer to (デザイン規約 §進行中の操作から出る).
///
/// **The code is read as well as the marker**, for the reason rebase's
/// is: a cherry-pick or a revert asked for while one is *already*
/// standing exits 128 with its own marker right there (実測 2.55), and
/// the marker alone would call that a stop.
///
/// A read that fails answers "no", for the reason merge's does: this is a
/// question *about* the failure, and letting it replace the answer would
/// report a `rev-parse` where git said why it would not copy the commit.
async fn landed(
    executor: &GitExecutor,
    workdir: &Path,
    op: InProgress,
    outcome: Result<(), GitError>,
    cancel: &CancellationToken,
) -> Result<Landing, GitError> {
    let Err(error) = outcome else {
        return Ok(Landing::Done);
    };
    if !matches!(error, GitError::Failed { code: 1, .. }) {
        return Err(error);
    }
    // The operation standing has to be *this* one: a rebase stopped on a
    // conflicting pick owns `CHERRY_PICK_HEAD` too, and a cherry-pick
    // asked for over a stopped merge is refused by git rather than
    // stopped — the marker on disk then belongs to the merge (実測 2.55).
    let standing = opstate::detect(executor, workdir, cancel)
        .await
        .ok()
        .and_then(|state| InProgress::from_state(&state));
    if standing == Some(op) {
        Ok(Landing::Stopped)
    } else {
        Err(error)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
