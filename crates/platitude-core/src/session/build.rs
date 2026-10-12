//! The carry-across moves and rewrites: the order their steps go in. The
//! stash round both share is [`super::stash_round`].

use super::stash_round::{
    STASH_TOP, pop_back_after_failure, pop_back_split_first, stash_everything,
};
use super::*;

/// Everything one `git rebase --interactive` needs, so the carry's two
/// attempts are the same command, tip check included ([`Replay::run`]).
pub(super) struct Replay<'a> {
    upstream: &'a str,
    steps: &'a [sequencer::RebaseStep],
    options: integrate::RebaseOptions,
    /// The tip `steps` was written against, full hex. There is no
    /// unpinned replay: an empty one refuses ([`Replay::tip_still_stands`]).
    expect_head: &'a str,
    /// The todo-editor binary.
    helper: PathBuf,
}

impl Replay<'_> {
    /// Locates the helper, which packaging must keep beside the app.
    pub(super) fn of<'a>(
        upstream: &'a str,
        steps: &'a [sequencer::RebaseStep],
        options: integrate::RebaseOptions,
        expect_head: &'a str,
    ) -> Result<Replay<'a>, GitError> {
        let helper = sequencer::helper_path().map_err(|source| GitError::Io {
            command: "git rebase --interactive".to_string(),
            source,
        })?;
        Ok(Replay {
            upstream,
            steps,
            options,
            expect_head,
            helper,
        })
    }

    async fn run(
        &self,
        executor: &GitExecutor,
        repo: &RepoInfo,
        cancel: &CancellationToken,
    ) -> Result<integrate::RebaseOutcome, GitError> {
        self.tip_still_stands(executor, repo, cancel).await?;
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

    /// Refuses the replay when HEAD is no longer the commit the todo was
    /// written for: the helper replaces git's todo whole, so a commit that
    /// landed since is not in it, and a rebase silently drops what the todo
    /// leaves out.
    ///
    /// Inside the replay, so before each spawn: the carry runs it twice,
    /// with a `detect` and a stash between, and a check before the first
    /// alone leaves that stretch open. The window only narrows — git takes
    /// a todo but not the tip it was written for.
    async fn tip_still_stands(
        &self,
        executor: &GitExecutor,
        repo: &RepoInfo,
        cancel: &CancellationToken,
    ) -> Result<(), GitError> {
        let head = commit::head_oid(executor, &repo.workdir, cancel).await?;
        if head.to_hex() == self.expect_head {
            return Ok(());
        }
        Err(report::rewrite_tip_moved())
    }
}

/// The two history rewrites, which git refuses over a dirty tree in the
/// same words, so both go round through the same stash
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

/// Replays a one-commit edit plan through `git rebase --interactive`,
/// pinned to `EditPlan::tip` ([`Replay::tip_still_stands`]): the rows the
/// todo is made of already end at HEAD, so the pin costs no spawn.
pub(super) async fn run_plan(
    executor: &GitExecutor,
    repo: &RepoInfo,
    plan: &sequencer::EditPlan,
    cancel: &CancellationToken,
) -> Result<integrate::Landing, GitError> {
    let replay = Replay::of(&plan.upstream, &plan.steps, plan.options(), &plan.tip)?;
    rewrite_carrying(executor, repo, &Rewrite::Replay(&replay), cancel).await
}

/// Runs `rewrite`, going round through a stash when the working tree is
/// in the way (デザイン規約 §未コミット変更がある状態で履歴を書き換える).
///
/// Answers a [`Landing`](integrate::Landing): the dirty-tree refusal is
/// what the carry handles, so it never reaches a caller.
pub(super) async fn rewrite_carrying(
    executor: &GitExecutor,
    repo: &RepoInfo,
    rewrite: &Rewrite<'_>,
    cancel: &CancellationToken,
) -> Result<integrate::Landing, GitError> {
    match rewrite.run(executor, repo, cancel).await? {
        integrate::RebaseOutcome::Done => Ok(integrate::Landing::Done),
        integrate::RebaseOutcome::Stopped => Ok(integrate::Landing::Stopped),
        integrate::RebaseOutcome::Blocked(refusal) => {
            tracing::info!(%refusal, "rebase refused: going round through a stash");
            carry_across(executor, repo, &Carried::Rewrite(rewrite), refusal, cancel).await
        }
    }
}

/// What one attempt at a carried command came to ([`Carried::attempt`]).
enum Attempt {
    Done,
    /// Stopped on a conflict, git holding the tree.
    Stopped,
    /// Refused over uncommitted work in the way — the one refusal a stash
    /// gets past ([`carry_across`]).
    Blocked(GitError),
}

/// The commands git refuses over uncommitted work in their way, which a
/// stash gets past: a move, a rewrite, a merge or a pull. Each tells its
/// own refusal apart ([`Carried::attempt`]); the round through the stash
/// is one ([`carry_across`]).
enum Carried<'a> {
    Move(&'a CheckoutTarget),
    Rewrite(&'a Rewrite<'a>),
    Integrate(&'a Integrate<'a>),
}

impl Carried<'_> {
    async fn attempt(
        &self,
        executor: &GitExecutor,
        repo: &RepoInfo,
        cancel: &CancellationToken,
    ) -> Result<Attempt, GitError> {
        match self {
            Carried::Move(target) => {
                match branch::checkout(executor, &repo.workdir, target, cancel).await? {
                    branch::CheckoutOutcome::Moved => Ok(Attempt::Done),
                    branch::CheckoutOutcome::Blocked(refusal) => Ok(Attempt::Blocked(refusal)),
                }
            }
            Carried::Rewrite(rewrite) => match rewrite.run(executor, repo, cancel).await? {
                integrate::RebaseOutcome::Done => Ok(Attempt::Done),
                integrate::RebaseOutcome::Stopped => Ok(Attempt::Stopped),
                integrate::RebaseOutcome::Blocked(refusal) => Ok(Attempt::Blocked(refusal)),
            },
            Carried::Integrate(what) => match what.run(executor, repo, cancel).await {
                Ok(integrate::Landing::Done) => Ok(Attempt::Done),
                Ok(integrate::Landing::Stopped) => Ok(Attempt::Stopped),
                Err(error) if integrate::refused_over_work(&error) => Ok(Attempt::Blocked(error)),
                Err(error) => Err(error),
            },
        }
    }

    /// The answer when an operation stands as the carry begins: `git
    /// stash push` resets the tree and takes the operation's marker
    /// (`MERGE_HEAD`, `REVERT_HEAD` …) down with it, so nothing is carried
    /// under one. A rewrite says so in a report of its own; a merge or a
    /// pull keeps git's refusal — the screen offers neither under a
    /// standing operation, and git's own check before a merge reads
    /// `MERGE_HEAD` and `CHERRY_PICK_HEAD` but not `REVERT_HEAD`
    /// (builtin/merge.c, the minimum to 2.55), so this is the backstop. A
    /// move cannot get here: git refuses every move under a standing
    /// operation in words of its own
    /// (rules-refs/core.md「立っている操作の上への rewrite は断る」).
    fn under_standing(&self, state: &OpState, refusal: GitError) -> GitError {
        match self {
            Carried::Rewrite(_) => report::rewrite_while_standing(standing_name(state)),
            Carried::Move(_) | Carried::Integrate(_) => refusal,
        }
    }
}

/// What a merge and a pull bring in: git turns both down over uncommitted
/// work its fast-forward or merge would write over, in the same words
/// ([`integrate::refused_over_work`]), so both go round through the same
/// stash (デザイン規約 §履歴を合流させる).
pub(super) enum Integrate<'a> {
    /// `git merge <rev>`.
    Merge {
        rev: &'a str,
        options: &'a integrate::MergeOptions,
    },
    /// `git pull --ff-only --no-rebase`, which lands or is refused and
    /// never stops.
    Pull { timeout: std::time::Duration },
}

impl Integrate<'_> {
    async fn run(
        &self,
        executor: &GitExecutor,
        repo: &RepoInfo,
        cancel: &CancellationToken,
    ) -> Result<integrate::Landing, GitError> {
        match self {
            Integrate::Merge { rev, options } => {
                integrate::merge(executor, &repo.workdir, rev, options, cancel).await
            }
            Integrate::Pull { timeout } => remote::pull(executor, &repo.workdir, *timeout, cancel)
                .await
                .map(|()| integrate::Landing::Done),
        }
    }
}

/// Runs `what`, going round through a stash when git refuses it over work
/// in the way — the one refusal a stash gets past; any other comes back as
/// git said it.
pub(super) async fn integrate_carrying(
    executor: &GitExecutor,
    repo: &RepoInfo,
    what: &Integrate<'_>,
    cancel: &CancellationToken,
) -> Result<integrate::Landing, GitError> {
    match what.run(executor, repo, cancel).await {
        Err(refusal) if integrate::refused_over_work(&refusal) => {
            tracing::info!(%refusal, "refused over work in the way: going round through a stash");
            carry_across(executor, repo, &Carried::Integrate(what), refusal, cancel).await
        }
        other => other,
    }
}

/// The standing operation's name for a refusal: git's verb where
/// [`integrate::InProgress`] knows one, else bisect — the only thing
/// `OpState::any` counts and `from_state` does not.
pub(super) fn standing_name(state: &OpState) -> &'static str {
    integrate::InProgress::from_state(state)
        .map(integrate::InProgress::command)
        .unwrap_or("bisect")
}

/// Puts the operation standing in a move's way down, and the tree it left
/// behind with it, so `git switch` has nothing left to refuse (git refuses
/// every move while a merge / rebase / cherry-pick / revert stands,
/// whatever the tree).
///
/// - **cherry-pick / revert / merge**: `--quit`, which keeps what earlier
///   steps committed. The unmerged index it leaves cannot be stashed
///   (`error: could not write index`), so conflicted paths are marked
///   resolved first (markers become content, as `Mark all resolved`
///   does), and the tree goes into a stash that **stays there**: markers
///   belong to the branch they were made on.
/// - **rebase**: `--abort` — `--quit` would leave HEAD detached on the
///   half-rewritten line with its copies unreferenced.
///
/// With nothing standing, an unmerged index alone (what the exit card's
/// `--quit` row leaves) is cleared by the same two steps. A clean tree
/// stashes nothing (exit 0), so no entry is left behind.
pub(super) async fn leave_operation(
    executor: &GitExecutor,
    repo: &RepoInfo,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let state = opstate::detect(executor, &repo.workdir, cancel).await?;
    let standing = integrate::InProgress::from_state(&state);
    if standing == Some(integrate::InProgress::Rebase) {
        return integrate::resolve(
            executor,
            &repo.workdir,
            integrate::InProgress::Rebase,
            integrate::Continuation::Abort,
            cancel,
        )
        .await;
    }
    if let Some(op) = standing {
        integrate::resolve(
            executor,
            &repo.workdir,
            op,
            integrate::Continuation::Quit,
            cancel,
        )
        .await?;
    }
    // Read here, not handed in: the unmerged set is only true now.
    let paths: Vec<String> = status::load(executor, &repo.workdir, cancel)
        .await?
        .conflicted()
        .map(|item| item.path().to_string())
        .collect();
    // Nothing in the way: stashing a tree nobody asked about would take
    // work from a reader only switching branches.
    if standing.is_none() && paths.is_empty() {
        return Ok(());
    }
    stage::stage_paths(executor, &repo.workdir, &paths, cancel).await?;
    stash_everything(executor, repo, cancel).await.map(drop)
}

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
            carry_across(executor, repo, &Carried::Move(target), refusal, cancel)
                .await
                .map(drop)
        }
    }
}

/// Stash, run `what`, put back — with the staged/unstaged split kept
/// (`--index`), so the work exists somewhere other than a marked-up file.
/// `refusal` is what git said first, kept for the dead ends with nothing
/// better to report.
///
/// Not `--autostash`: it restores with a plain `stash apply` (everything
/// staged comes back unstaged), and a stopped rebase or merge parks the
/// work in `.git/rebase-merge/autostash` / `MERGE_AUTOSTASH`, out of
/// `stash list` and the graph.
///
/// The restore is skipped when the command stops part-way: git will not
/// write into an index holding unmerged paths, so a `pop` does nothing
/// while reporting the conflict it walked into
/// (rules-refs/core.md「`stash pop` の非ゼロを conflict と読めるのは」).
/// The entry stays in the stash list for the person settling the
/// conflict, where the same commands typed by hand would leave it.
async fn carry_across(
    executor: &GitExecutor,
    repo: &RepoInfo,
    what: &Carried<'_>,
    refusal: GitError,
    cancel: &CancellationToken,
) -> Result<integrate::Landing, GitError> {
    // Asked before the stash, whatever is carried: one `rev-parse` on a
    // path already four processes long (`Carried::under_standing`). A
    // bisect carries — it is no `InProgress`.
    let state = opstate::detect(executor, &repo.workdir, cancel).await?;
    if integrate::InProgress::from_state(&state).is_some() {
        return Err(what.under_standing(&state, refusal));
    }
    if !stash_everything(executor, repo, cancel).await? {
        // Cleaned since the refusal: nothing to carry, so just rerun.
        return match what.attempt(executor, repo, cancel).await? {
            Attempt::Done => Ok(integrate::Landing::Done),
            Attempt::Stopped => Ok(integrate::Landing::Stopped),
            Attempt::Blocked(again) => Err(again),
        };
    }
    match what.attempt(executor, repo, cancel).await {
        Ok(Attempt::Done) => {}
        // No pop: the work stays in the stash (see above,
        // デザイン規約 §未コミット変更がある状態で履歴を書き換える).
        Ok(Attempt::Stopped) => return Ok(integrate::Landing::Stopped),
        Ok(Attempt::Blocked(_)) => {
            // Not something a stash gets past (a `--skip-worktree` file,
            // say): put the work back and report git's first refusal,
            // unless the pop fails.
            stash::pop(executor, &repo.workdir, STASH_TOP, cancel).await?;
            return Err(refusal);
        }
        Err(error) => {
            // A failure that left an operation standing keeps the work in
            // the stash until that is dealt with; one that failed without
            // starting gets it back.
            if !opstate::detect(executor, &repo.workdir, cancel)
                .await?
                .any()
            {
                pop_back_after_failure(executor, repo, cancel).await;
            }
            return Err(error);
        }
    }
    pop_back_split_first(executor, repo, cancel).await?;
    Ok(integrate::Landing::Done)
}
