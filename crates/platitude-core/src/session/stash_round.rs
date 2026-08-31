//! The stash out and back that both carries go through.
//!
//! A move and a rewrite are refused over a dirty working tree in different
//! words, but what gets them past it is the same three steps — empty the
//! tree, do the thing, put the work back — and these are the steps
//! ([`super::build`] is what puts them in order). They are one place
//! rather than two because the hard parts are shared: which entry is ours
//! to touch, and what a non-zero `pop` actually did.

use super::*;

/// The entry a [`stash_everything`] just made, for the moves that put it
/// back.
pub(super) const STASH_TOP: &str = "stash@{0}";

/// Stashes the whole working tree out of a move's way, and answers whether
/// an entry of ours was really made.
///
/// A clean tree stashes nothing while exiting 0 — the refusal that raised
/// the question can go stale when the tree is cleaned from a terminal in
/// between. With no entry of ours, [`STASH_TOP`] names somebody else's
/// work and must not be touched.
pub(super) async fn stash_everything(
    executor: &GitExecutor,
    repo: &RepoInfo,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let options = stash::PushOptions {
        include_untracked: true,
        keep_index: false,
        staged_only: false,
    };
    let before = stash::tip(executor, &repo.workdir, cancel).await?;
    stash::push(executor, &repo.workdir, "", options, &[], cancel).await?;
    Ok(stash::tip(executor, &repo.workdir, cancel).await? != before)
}

/// Puts the carried work back once the move or the rewrite has landed,
/// keeping the staged/unstaged split for as long as git will take it. The
/// last step of both carries, and the only one they share.
///
/// A conflicting restore exits non-zero while having done exactly what was
/// asked, so the exit code alone cannot judge it: the working tree decides.
/// Unmerged paths mean the merge landed and is waiting to be settled; a
/// clean tree means the restore did nothing, and then the split has to be
/// given up on (see below) or git's message goes through. Nothing was
/// unmerged when the carry began — the stash emptied the tree — so what is
/// found afterwards can only have come from the restore.
pub(super) async fn pop_back_split_first(
    executor: &GitExecutor,
    repo: &RepoInfo,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let kept_index = stash::pop_with_index(executor, &repo.workdir, STASH_TOP, cancel).await;
    if kept_index.is_ok() || conflicts_now(executor, repo, cancel).await? {
        return Ok(());
    }
    // git refuses `--index` outright when the staged half is what collides
    // ("conflicts in index. Try without --index.") and leaves everything
    // where it was. Its own advice is the fallback: restore without the
    // index, which brings the changes across merged and gives up only on
    // the staged/unstaged split.
    match stash::pop(executor, &repo.workdir, STASH_TOP, cancel).await {
        Ok(()) => Ok(()),
        Err(error) => {
            if conflicts_now(executor, repo, cancel).await? {
                Ok(())
            } else {
                Err(error)
            }
        }
    }
}

/// Best-effort restore after a move or a replay that failed outright (an
/// `Err`, not a `Blocked` refusal — a refusal git words in a way the
/// classifiers do not know arrives here). It did nothing, so the stash
/// was only the room it needed: put the work back before the caller
/// surfaces git's own error. If even the pop fails, that is logged and
/// the entry stays in the stash list, where the work is still
/// recoverable — the original error is the one worth showing.
pub(super) async fn pop_back_after_failure(
    executor: &GitExecutor,
    repo: &RepoInfo,
    cancel: &CancellationToken,
) {
    if let Err(error) = stash::pop(executor, &repo.workdir, STASH_TOP, cancel).await {
        tracing::warn!(
            %error,
            "the switch failed and the stashed work could not be popped back; \
             it remains in the stash list"
        );
    }
}

/// Whether the working tree has unmerged paths right now.
///
/// What a restore leaves behind is the only honest answer to "did that
/// non-zero exit do anything": `git stash pop` reports a conflict and a
/// refusal the same way.
pub(super) async fn conflicts_now(
    executor: &GitExecutor,
    repo: &RepoInfo,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    Ok(status::load(executor, &repo.workdir, cancel)
        .await?
        .has_conflicts())
}
