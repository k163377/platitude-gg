//! Free helpers of the session: the carry-across moves and rewrites.

use super::*;

/// Everything one `git rebase --interactive` needs, held together so the
/// two attempts a carry makes are the same command twice over.
pub(super) struct Replay<'a> {
    upstream: &'a str,
    steps: &'a [sequencer::RebaseStep],
    options: integrate::RebaseOptions,
    /// The todo-editor binary shipped beside the application.
    helper: PathBuf,
}

impl Replay<'_> {
    /// Locates the helper, which packaging must keep beside the app.
    pub(super) fn of<'a>(
        upstream: &'a str,
        steps: &'a [sequencer::RebaseStep],
        options: integrate::RebaseOptions,
    ) -> Result<Replay<'a>, GitError> {
        let helper = sequencer::helper_path().map_err(|source| GitError::Io {
            command: "git rebase --interactive".to_string(),
            source,
        })?;
        Ok(Replay {
            upstream,
            steps,
            options,
            helper,
        })
    }

    async fn run(
        &self,
        executor: &GitExecutor,
        repo: &RepoInfo,
        cancel: &CancellationToken,
    ) -> Result<integrate::RebaseOutcome, GitError> {
        sequencer::rebase_interactive(
            executor,
            repo,
            self.upstream,
            self.steps,
            &self.options,
            &self.helper,
            cancel,
        )
        .await
    }
}

/// The two shapes of history rewrite this application runs, held together
/// because git refuses both over a dirty working tree in the very same
/// words — so both go round through the same stash, and the person who
/// staged half of their work gets it back staged either way
/// (デザイン規約 §未コミット変更がある状態で履歴を書き換える).
pub(super) enum Rewrite<'a> {
    /// `git rebase <upstream>`: a whole branch onto a new base.
    Onto {
        upstream: &'a str,
        options: &'a integrate::RebaseOptions,
    },
    /// `git rebase --interactive`: a plan assembled here, for the edits
    /// that touch one commit (`squash` / reword / drop).
    Replay(&'a Replay<'a>),
}

impl Rewrite<'_> {
    async fn run(
        &self,
        executor: &GitExecutor,
        repo: &RepoInfo,
        cancel: &CancellationToken,
    ) -> Result<integrate::RebaseOutcome, GitError> {
        match self {
            Rewrite::Onto { upstream, options } => {
                integrate::rebase(executor, &repo.workdir, upstream, options, cancel).await
            }
            Rewrite::Replay(replay) => replay.run(executor, repo, cancel).await,
        }
    }
}

/// Replays a one-commit edit plan through `git rebase --interactive`.
pub(super) async fn run_plan(
    executor: &GitExecutor,
    repo: &RepoInfo,
    plan: &sequencer::EditPlan,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let replay = Replay::of(&plan.upstream, &plan.steps, plan.options())?;
    rewrite_carrying(executor, repo, &Rewrite::Replay(&replay), cancel).await
}

/// Runs `rewrite`, going round through a stash when the working tree is
/// in the way (デザイン規約 §未コミット変更がある状態で履歴を書き換える).
pub(super) async fn rewrite_carrying(
    executor: &GitExecutor,
    repo: &RepoInfo,
    rewrite: &Rewrite<'_>,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    match rewrite.run(executor, repo, cancel).await? {
        integrate::RebaseOutcome::Done => Ok(()),
        integrate::RebaseOutcome::Blocked(refusal) => {
            tracing::info!(%refusal, "rebase refused: going round through a stash");
            carry_across_rewrite(executor, repo, rewrite, refusal, cancel).await
        }
    }
}

/// Stash, rewrite, put back — the same three steps [`carry_across`] takes
/// around a move, with the rewrite in the middle. `--autostash` is not
/// what runs them, for two measured reasons: it restores with a plain
/// `stash apply`, so **everything that was staged comes back unstaged**,
/// and when the rebase stops part-way it parks the work in
/// `.git/rebase-merge/autostash`, where `stash list` cannot see it and
/// neither can the graph.
///
/// The restore is skipped when the rebase stopped part-way, and that is
/// the whole difference from a move: git will not write into an index
/// that already holds unmerged paths, so a `pop` there does nothing at
/// all while reporting the conflict it walked into (実測 — 規約 §`stash
/// pop` の非ゼロを conflict と読んでよいのは). The entry stays in the
/// stash list, drawn as its own row in the graph, and the person settling
/// the conflict puts it back when the operation is over — which is where
/// the same three commands typed by hand would leave it.
async fn carry_across_rewrite(
    executor: &GitExecutor,
    repo: &RepoInfo,
    rewrite: &Rewrite<'_>,
    refusal: GitError,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    if !stash_everything(executor, repo, cancel).await? {
        // The tree was cleaned between the refusal and now, so there is
        // nothing of ours to carry and nothing of anybody else's to
        // touch: the rebase that was refused goes through as it stands.
        return match rewrite.run(executor, repo, cancel).await? {
            integrate::RebaseOutcome::Done => Ok(()),
            integrate::RebaseOutcome::Blocked(again) => Err(again),
        };
    }
    match rewrite.run(executor, repo, cancel).await {
        Ok(integrate::RebaseOutcome::Done) => {}
        Ok(integrate::RebaseOutcome::Blocked(_)) => {
            // Nothing should stand in the way of a tree that was just
            // emptied, so whatever is holding this one is not something a
            // stash gets past. Put the work back and let git's first
            // refusal say why — a failure to put it back is the more
            // urgent news and goes through instead.
            stash::pop(executor, &repo.workdir, STASH_TOP, cancel).await?;
            return Err(refusal);
        }
        Err(error) => {
            // A rebase that stopped part-way is holding the tree; the
            // work stays in the stash until the operation is over. One
            // that failed without starting leaves the emptied tree, and
            // then the stash was only the room it needed.
            if !opstate::detect(executor, &repo.workdir, cancel)
                .await?
                .any()
            {
                pop_back_after_failure(executor, repo, cancel).await;
            }
            return Err(error);
        }
    }

    pop_back_split_first(executor, repo, cancel).await
}

/// The entry a [`stash_everything`] just made, for the moves that put it
/// back.
const STASH_TOP: &str = "stash@{0}";

/// Moves HEAD to `target`, going round through a stash when the working
/// tree is in the way (デザイン規約 §未コミット変更がある状態での移動).
pub(super) async fn move_carrying(
    executor: &GitExecutor,
    repo: &RepoInfo,
    target: &CheckoutTarget,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    match branch::checkout(executor, &repo.workdir, target, cancel).await? {
        branch::CheckoutOutcome::Moved => Ok(()),
        branch::CheckoutOutcome::Blocked(refusal) => {
            tracing::info!(%refusal, "move refused: going round through a stash");
            carry_across(executor, repo, target, refusal, cancel).await
        }
    }
}

/// Stash, move, put back — what a person would type when git will not
/// carry the work itself. `refusal` is what git said the first time, kept
/// for the dead ends that have nothing better to report.
///
/// The restore is a merge, so the changes land on top of what the target
/// has and only the parts git cannot combine need settling. Going through
/// a stash rather than `switch --merge` buys two things that flag cannot
/// give — **the staged/unstaged split survives** (`--index`), and a
/// conflict **keeps the stash entry**, so the work still exists somewhere
/// other than a marked-up file.
async fn carry_across(
    executor: &GitExecutor,
    repo: &RepoInfo,
    target: &CheckoutTarget,
    refusal: GitError,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    if !stash_everything(executor, repo, cancel).await? {
        // The tree was cleaned between the refusal and now, so there is
        // nothing of ours to carry and nothing of anybody else's to
        // touch: the move that was refused goes through as it stands.
        return match branch::checkout(executor, &repo.workdir, target, cancel).await? {
            branch::CheckoutOutcome::Moved => Ok(()),
            branch::CheckoutOutcome::Blocked(again) => Err(again),
        };
    }
    let outcome = match branch::checkout(executor, &repo.workdir, target, cancel).await {
        Ok(outcome) => outcome,
        Err(error) => {
            pop_back_after_failure(executor, repo, cancel).await;
            return Err(error);
        }
    };
    if let branch::CheckoutOutcome::Blocked(_) = outcome {
        // Nothing should stand in the way of a tree that was just
        // emptied, so whatever is holding this one is not something a
        // stash gets past (a `--skip-worktree` file, say). Put the work
        // back and let git's first refusal say why — a failure to put it
        // back is the more urgent news and goes through instead.
        stash::pop(executor, &repo.workdir, STASH_TOP, cancel).await?;
        return Err(refusal);
    }

    pop_back_split_first(executor, repo, cancel).await
}

/// Stashes the whole working tree out of a move's way, and answers whether
/// an entry of ours was really made.
///
/// A clean tree stashes nothing while exiting 0 — the refusal that raised
/// the question can go stale when the tree is cleaned from a terminal in
/// between. With no entry of ours, [`STASH_TOP`] names somebody else's
/// work and must not be touched.
async fn stash_everything(
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
async fn pop_back_split_first(
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
async fn pop_back_after_failure(
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
