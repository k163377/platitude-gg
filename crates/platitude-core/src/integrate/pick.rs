//! `git cherry-pick` and `git revert`, and the commits they walk past.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use super::{InProgress, Landing, opstate};
use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// `git cherry-pick <revs>`.
///
/// `--allow-empty` lets a commit that was empty when made land as it
/// stands; without it git stops on it in the same words as on one that
/// became empty, and only the latter is walked past. A conflict is
/// [`Landing::Stopped`].
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
/// No `--allow-empty` (git rejects it for revert): the one empty case is a
/// revert already in the branch, walked past like a cherry-pick's.
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
/// A commit the branch already has stops both commands (exit 1, sequence
/// standing, a message naming `--skip`) though it asks nothing of the
/// person, so it is answered here (デザイン規約 §履歴を合流させる).
/// `--empty=drop` would do it but is past the minimum git version
/// (internal-docs/git最低バージョン整合.md).
///
/// The number of commits bounds the loop: each `--skip` moves the
/// sequence on by one.
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
        // Nothing standing = nothing to skip; `--skip` would exit 128 ("no
        // revert in progress") as a red log row.
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

/// Where the sequence came to rest: done, stopped on a conflict, or failed
/// (the empty stops are answered by the loop before this).
///
/// The code is read as well as the marker: a cherry-pick or revert asked
/// for while one is already standing exits 128 with its own marker there.
/// A read that fails answers "no", so git's reason reaches the screen
/// instead of a `rev-parse` error.
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
    // The operation standing has to be this one: a rebase stopped on a
    // conflicting pick owns `CHERRY_PICK_HEAD` too, and a cherry-pick git
    // refuses over a stopped merge finds the merge's marker.
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

/// Whether git still holds this operation open by either thing a `--skip`
/// needs: its marker, or a sequence with steps left.
///
/// An empty single revert has neither (git refuses before writing
/// `REVERT_HEAD`), several reverts leave only the sequence, and a
/// cherry-pick leaves its marker either way. Another operation standing is
/// not this one: a rebase stopped on a conflicting pick owns both the
/// sequence and `CHERRY_PICK_HEAD`.
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

/// Whether git stopped because the replayed commit records nothing (the
/// branch has it already).
///
/// Reads human-facing output under the exception `rebase::work_is_in_the_way`
/// takes (`LC_ALL=C` wording). Anything unrecognised is `false` and travels
/// on as a failure: a reworded message costs only the walk past.
fn left_nothing_to_record(op: InProgress, error: &GitError) -> bool {
    let GitError::Failed { stderr, .. } = error else {
        return false;
    };
    let said = stderr.to_ascii_lowercase();
    // cherry-pick weighs `--allow-empty` before writing, so it stops in
    // words of its own.
    if said.contains(&format!("the previous {} is now empty", op.command())) {
        return true;
    }
    // revert never reaches that message: `git commit` refuses the commit on
    // stdout with stderr empty, and `GitOutput::failure_message` passes
    // stdout through here. Every other revert stop writes to stderr.
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

    /// Word for word as git writes them; `integrate_integration` runs both
    /// for real.
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

    /// The bare commit refusal is a revert's alone — a cherry-pick reaching
    /// it was stopped by something unknown, which travels on.
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
