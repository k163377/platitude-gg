//! What a write does to the files: stage / unstage / discard, commit,
//! and the two that move the whole tree — checkout and reset.
//!
//! What a write does to a *name* is [`super::ops_refs`], and the stash is
//! [`super::ops_stash`].

use super::build::{leave_operation, move_carrying};
use super::discard_record::{copy_for_reset, copy_of_choices, copy_of_partial};
use super::*;
use crate::discards;

impl RepoSession {
    /// `git add` for whole files.
    pub fn stage_paths(self: &Arc<Self>, paths: Vec<String>) -> Option<OperationId> {
        self.write(
            OperationKind::Stage,
            AfterWrite::Tree,
            move |exec, repo, cancel| async move {
                stage::stage_paths(&exec, &repo.workdir, &paths, &cancel).await
            },
        )
    }

    /// `git add --all` for the whole work tree, untracked included.
    pub fn stage_all(self: &Arc<Self>) -> Option<OperationId> {
        self.write(
            OperationKind::Stage,
            AfterWrite::Tree,
            move |exec, repo, cancel| async move {
                stage::stage_all(&exec, &repo.workdir, &cancel).await
            },
        )
    }

    /// `git add` over every path git reports as unmerged: the whole
    /// conflicted bucket marked resolved in one command.
    ///
    /// The set is read under the write lock, where the command runs: a
    /// list gathered in the UI predates the writes queued ahead, and a
    /// path that stopped being conflicted in between would be staged for
    /// real. The unstaged bucket beside it is not included.
    pub fn stage_conflicted(self: &Arc<Self>) -> Option<OperationId> {
        self.write(
            OperationKind::Stage,
            AfterWrite::Tree,
            move |exec, repo, cancel| async move {
                let paths: Vec<String> = status::load(&exec, &repo.workdir, &cancel)
                    .await?
                    .conflicted()
                    .map(|item| item.path().to_string())
                    .collect();
                // An empty set (the bucket emptied while queued) is
                // `stage_paths`'s to turn away.
                stage::stage_paths(&exec, &repo.workdir, &paths, &cancel).await
            },
        )
    }

    /// Empties the index back to HEAD.
    pub fn unstage_all(self: &Arc<Self>) -> Option<OperationId> {
        self.write(
            OperationKind::Unstage,
            AfterWrite::Tree,
            move |exec, repo, cancel| async move {
                stage::unstage_all(&exec, &repo.workdir, &cancel).await
            },
        )
    }

    /// Removes whole files from the index.
    pub fn unstage_paths(self: &Arc<Self>, paths: Vec<String>) -> Option<OperationId> {
        self.write(
            OperationKind::Unstage,
            AfterWrite::Tree,
            move |exec, repo, cancel| async move {
                stage::unstage_paths(&exec, &repo.workdir, &paths, &cancel).await
            },
        )
    }

    /// Discards a chosen set of rows as one queued write; a staged rename
    /// takes its source along, read from status inside the write
    /// ([`stage::with_old_names`]). What goes is copied first and put on
    /// the discard record once it went (破棄記録仕様.md §2.1).
    pub fn discard_chosen(
        self: &Arc<Self>,
        choices: Vec<(String, stage::DiscardSide)>,
    ) -> Option<OperationId> {
        let s = Arc::clone(self);
        self.write(
            OperationKind::Discard,
            AfterWrite::Tree,
            move |exec, repo, cancel| async move {
                let rows = stage::with_old_names(&exec, &repo.workdir, &choices, &cancel).await?;
                let copied = copy_of_choices(&exec, &repo, &rows, &cancel).await?;
                let done = stage::discard_rows(&exec, &repo.workdir, &rows, &cancel).await;
                s.record_copied(&exec, &repo, copied, done, &cancel).await
            },
        )
    }

    /// Throws away part of one file's unstaged diff (hunk / line level).
    /// The index keeps what is staged (see [`stage::discard_partial`]). The
    /// file is copied first, as [`Self::discard_chosen`] copies.
    pub fn discard_partial(
        self: &Arc<Self>,
        target: DiffTarget,
        selects: Vec<HunkSelect>,
        seen: u64,
    ) -> Option<OperationId> {
        let s = Arc::clone(self);
        self.write(
            OperationKind::Discard,
            AfterWrite::Tree,
            move |exec, repo, cancel| async move {
                let copied = copy_of_partial(&exec, &repo, &target, &selects, &cancel).await?;
                let done =
                    stage::discard_partial(&exec, &repo, &target, &selects, seen, &cancel).await;
                s.record_copied(&exec, &repo, copied, done, &cancel).await
            },
        )
    }

    /// Stages or unstages part of one file's diff (hunk / line level).
    /// `seen` is the fingerprint the selection's diff arrived with
    /// ([`SessionEvent::DiffLoaded`]).
    pub fn apply_partial(
        self: &Arc<Self>,
        target: DiffTarget,
        selects: Vec<HunkSelect>,
        seen: u64,
    ) -> Option<OperationId> {
        self.write(
            OperationKind::Stage,
            AfterWrite::Tree,
            move |exec, repo, cancel| async move {
                stage::apply_partial(&exec, &repo, &target, &selects, seen, &cancel).await
            },
        )
    }

    /// Commits the index (or amends HEAD — which moves the branch off the
    /// commit it replaces, told to the discard log where that went).
    pub fn commit(
        self: &Arc<Self>,
        message: String,
        options: CommitOptions,
    ) -> Option<OperationId> {
        let s = Arc::clone(self);
        self.write(
            OperationKind::Commit,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let before = if options.amend {
                    s.moving_off(&exec, &repo, &["HEAD".to_string()], &cancel)
                        .await?
                } else {
                    Vec::new()
                };
                commit::commit(&exec, &repo, &message, options, &cancel).await?;
                s.tell_if_left(&exec, &repo, &before, &cancel).await;
                Ok(())
            },
        )
    }

    /// Moves HEAD, taking uncommitted work along (デザイン規約
    /// §未コミット変更がある状態での移動).
    ///
    /// One command where git carries the changes; where it refuses
    /// (touching nothing), the long way — stash, move, put back. Nothing is
    /// asked first: the refusal proved the repository untouched, and every
    /// outcome of the long way is one the stash can undo.
    ///
    /// A move off a detached HEAD's own commits, or one that takes a branch
    /// off its commits (`ForceCreate`), is told to the discard log.
    pub fn checkout(self: &Arc<Self>, target: CheckoutTarget) -> Option<OperationId> {
        let s = Arc::clone(self);
        self.write(
            OperationKind::Checkout,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let moved = moved_by(&exec, &repo, &target, &cancel).await?;
                let before = s.moving_off(&exec, &repo, &moved, &cancel).await?;
                move_carrying(&exec, &repo, &target, &cancel).await?;
                s.tell_if_left(&exec, &repo, &before, &cancel).await;
                Ok(())
            },
        )
    }

    /// Puts the operation standing in the way down
    /// ([`leave_operation`](super::build::leave_operation)) and then moves.
    ///
    /// One write, so nothing can start the move mid-way and nobody can
    /// answer the question twice. Each command stands in the log on its
    /// own line (デザイン規約 §進行中の操作から出る).
    pub fn checkout_leaving_operation(
        self: &Arc<Self>,
        target: CheckoutTarget,
    ) -> Option<OperationId> {
        let s = Arc::clone(self);
        self.write(
            OperationKind::Checkout,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                leave_operation(&exec, &repo, &cancel).await?;
                // Read after the operation is put down: what it had made so
                // far is no part of the log (§1).
                let moved = moved_by(&exec, &repo, &target, &cancel).await?;
                let before = s.moving_off(&exec, &repo, &moved, &cancel).await?;
                move_carrying(&exec, &repo, &target, &cancel).await?;
                s.tell_if_left(&exec, &repo, &before, &cancel).await;
                Ok(())
            },
        )
    }

    /// Moves a local branch onto `start` and lands on it. A branch that
    /// fell behind is a fast-forward and simply moves; one holding commits
    /// `start` does not stops with [`SessionEvent::MoveNeedsAsk`], nothing
    /// touched.
    ///
    /// `leaving` is [`checkout_leaving_operation`](Self::checkout_leaving_operation)'s
    /// agreement, spent **after** the check: a move that has to ask leaves
    /// the operation standing, so a reader who walks away from `Move here?`
    /// still has their cherry-pick.
    pub fn checkout_moving_branch(
        self: &Arc<Self>,
        local: String,
        start: String,
        leaving: bool,
    ) -> Option<OperationId> {
        let session = Arc::clone(self);
        self.write(
            OperationKind::Checkout,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                if !branch::is_merged_into(&exec, &repo.workdir, &local, &start, &cancel).await? {
                    session
                        .sink
                        .event(SessionEvent::MoveNeedsAsk { local, start });
                    return Ok(());
                }
                if leaving {
                    leave_operation(&exec, &repo, &cancel).await?;
                }
                let target = CheckoutTarget::ForceCreate { local, start };
                let moved = moved_by(&exec, &repo, &target, &cancel).await?;
                let before = session.moving_off(&exec, &repo, &moved, &cancel).await?;
                move_carrying(&exec, &repo, &target, &cancel).await?;
                session.tell_if_left(&exec, &repo, &before, &cancel).await;
                Ok(())
            },
        )
    }

    /// Moves the current branch to `rev`, carrying the index and the
    /// working tree as far as `mode` says. `ResetMode::Hard` destroys
    /// uncommitted work, so the UI asks before sending that one — and what
    /// it destroys is copied first and put on the discard record with the
    /// move (破棄記録仕様.md §2).
    pub fn reset(self: &Arc<Self>, rev: String, mode: branch::ResetMode) -> Option<OperationId> {
        let s = Arc::clone(self);
        self.write(
            OperationKind::Reset,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let copied = match mode {
                    branch::ResetMode::Hard => copy_for_reset(&exec, &repo, &rev, &cancel).await?,
                    _ => None,
                };
                let before = s
                    .moving_off(&exec, &repo, &["HEAD".to_string()], &cancel)
                    .await?;
                let done = branch::reset(&exec, &repo.workdir, &rev, mode, &cancel).await;
                let done = s.record_copied(&exec, &repo, copied, done, &cancel).await;
                if done.is_ok() {
                    s.tell_if_left(&exec, &repo, &before, &cancel).await;
                }
                done
            },
        )
    }

    /// Makes a new working copy at `path` with `on` out in it (`git
    /// worktree add`). `name` is the copy as the screen names it, for a
    /// refusal's heading ([`crate::worktrees::add`]).
    ///
    /// The graph is read again behind it — a new branch is a new chip —
    /// and the worktree listing that follows every such write brings the
    /// copy's row.
    pub fn add_worktree(
        self: &Arc<Self>,
        path: String,
        on: crate::worktrees::CopyOn,
        name: String,
    ) -> Option<OperationId> {
        self.write(
            OperationKind::Worktree,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                crate::worktrees::add(&exec, &repo.workdir, &path, &on, &name, &cancel).await
            },
        )
    }

    /// Takes another working copy off the disk (`git worktree remove`),
    /// the branch it had out left where it is. `name` is the copy as the
    /// screen names it, for a refusal's heading
    /// ([`crate::worktrees::remove`]). Where it was and what it had out go
    /// on the discard record (破棄記録仕様.md §2) — also when the removal
    /// failed past git's point of no return: a folder it could not delete
    /// (one a program has open on Windows) fails it after git has taken the
    /// copy's own HEAD and reflog. A copy on no commit yet takes nothing.
    ///
    /// Moves no ref, so the refs are read first and the tree only where
    /// they moved; the listing behind it frees the branch, and drops a
    /// branchless copy's graph row with the one walk the write makes
    /// (`AfterWrite::Worktrees`).
    pub fn remove_worktree(self: &Arc<Self>, path: String, name: String) -> Option<OperationId> {
        let s = Arc::clone(self);
        self.write(
            OperationKind::Worktree,
            AfterWrite::Worktrees,
            move |exec, repo, cancel| async move {
                let listed = crate::worktrees::load(&exec, &repo.workdir, &cancel).await?;
                let before = listed
                    .into_iter()
                    .find(|entry| same_path_key(&entry.path) == same_path_key(&path));
                let done =
                    crate::worktrees::remove(&exec, &repo.workdir, &path, &name, &cancel).await;
                let gone = match &done {
                    Ok(()) => true,
                    Err(_) => crate::worktrees::load(&exec, &repo.workdir, &cancel)
                        .await
                        .is_ok_and(|listed| {
                            listed
                                .iter()
                                .all(|entry| same_path_key(&entry.path) != same_path_key(&path))
                        }),
                };
                let head = before
                    .as_ref()
                    .and_then(|entry| entry.head_hex.as_deref())
                    .filter(|hex| !Oid::hex_is_zero(hex))
                    .and_then(|hex| Oid::from_hex_str(hex).ok());
                if let (true, Some(head)) = (gone, head) {
                    let branch = before.as_ref().and_then(|entry| entry.branch.as_deref());
                    let written = discards::record_worktree_remove(
                        &exec,
                        &repo.workdir,
                        &repo.git_dir,
                        &path,
                        branch,
                        &head,
                        &cancel,
                    )
                    .await;
                    s.recorded(written);
                }
                done
            },
        )
    }
}

/// What a move can leave behind, the revs [`RepoSession::moving_off`]
/// reads: a detached HEAD's commit — a branch's stays with the branch — and
/// the branch a `ForceCreate` moves from under it.
async fn moved_by(
    exec: &GitExecutor,
    repo: &RepoInfo,
    target: &CheckoutTarget,
    cancel: &CancellationToken,
) -> Result<Vec<String>, GitError> {
    let mut revs = Vec::new();
    let cmd = GitCommand::new()
        .cwd(&repo.workdir)
        .args(["symbolic-ref", "--quiet", "HEAD"])
        .answers_by_code(1);
    if exec.run_unchecked(cmd, cancel).await?.code != 0 {
        revs.push("HEAD".to_string());
    }
    if let CheckoutTarget::ForceCreate { local, .. } = target {
        revs.push(format!("refs/heads/{local}"));
    }
    Ok(revs)
}
