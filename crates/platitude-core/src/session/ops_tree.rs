//! Working-tree and local-ref writes: stage / unstage / discard, commit,
//! checkout, reset, and the branch / tag / stash commands.

use super::build::{conflicts_now, move_carrying};
use super::*;

impl RepoSession {
    /// `git add` for whole files.
    pub fn stage_paths(self: &Arc<Self>, paths: Vec<String>) {
        self.write(
            "stage",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                stage::stage_paths(&exec, &repo.workdir, &paths, &cancel).await
            },
        );
    }

    /// `git add --all` for the whole work tree, untracked included.
    pub fn stage_all(self: &Arc<Self>) {
        self.write(
            "stage",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                stage::stage_all(&exec, &repo.workdir, &cancel).await
            },
        );
    }

    /// Empties the index back to HEAD.
    pub fn unstage_all(self: &Arc<Self>) {
        self.write(
            "unstage",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                stage::unstage_all(&exec, &repo.workdir, &cancel).await
            },
        );
    }

    /// Removes whole files from the index.
    pub fn unstage_paths(self: &Arc<Self>, paths: Vec<String>) {
        self.write(
            "unstage",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                stage::unstage_paths(&exec, &repo.workdir, &paths, &cancel).await
            },
        );
    }

    /// Throws away unstaged modifications of tracked files.
    pub fn discard_paths(self: &Arc<Self>, paths: Vec<String>) {
        self.write(
            "discard",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                stage::discard_worktree(&exec, &repo.workdir, &paths, &cancel).await
            },
        );
    }

    /// Throws away both sides of a path at once, back to HEAD: what is
    /// staged and what is on disk. A rename must be given both of its
    /// names (see [`stage::discard_to_head`]).
    pub fn discard_paths_to_head(self: &Arc<Self>, paths: Vec<String>) {
        self.write(
            "discard",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                stage::discard_to_head(&exec, &repo.workdir, &paths, &cancel).await
            },
        );
    }

    /// Deletes untracked files.
    pub fn remove_untracked(self: &Arc<Self>, paths: Vec<String>) {
        self.write(
            "clean",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                stage::remove_untracked(&exec, &repo.workdir, &paths, &cancel).await
            },
        );
    }

    /// Throws away part of one file's unstaged diff (hunk / line level).
    /// The index keeps what is staged (see [`stage::discard_partial`]).
    pub fn discard_partial(
        self: &Arc<Self>,
        target: DiffTarget,
        selects: Vec<HunkSelect>,
        seen: u64,
    ) {
        self.write(
            "discard",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                stage::discard_partial(&exec, &repo, &target, &selects, seen, &cancel).await
            },
        );
    }

    /// Stages or unstages part of one file's diff (hunk / line level).
    /// `seen` is the fingerprint the selection's diff arrived with
    /// ([`SessionEvent::DiffLoaded`]).
    pub fn apply_partial(
        self: &Arc<Self>,
        target: DiffTarget,
        selects: Vec<HunkSelect>,
        seen: u64,
    ) {
        self.write(
            "stage",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                stage::apply_partial(&exec, &repo, &target, &selects, seen, &cancel).await
            },
        );
    }

    /// Commits the index (or amends HEAD).
    pub fn commit(self: &Arc<Self>, message: String, options: CommitOptions) {
        self.write(
            "commit",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                commit::commit(&exec, &repo, &message, options, &cancel)
                    .await
                    .map(drop)
            },
        );
    }

    /// Moves HEAD, taking uncommitted work along (デザイン規約
    /// §未コミット変更がある状態での移動).
    ///
    /// The everyday case is one command: git carries the changes wherever
    /// they do not stand in the way. Where they do it refuses and touches
    /// nothing, and this goes round the long way instead — stash, move,
    /// put back — which is the sequence a person would type. Nothing is
    /// asked first: the refusal itself proved the repository is untouched,
    /// and every outcome of the long way is one the working tree can show
    /// and the stash can undo.
    pub fn checkout(self: &Arc<Self>, target: CheckoutTarget) {
        self.write(
            "checkout",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                move_carrying(&exec, &repo, &target, &cancel).await
            },
        );
    }

    /// Moves a local branch onto `start` and lands on it, asking first only
    /// when there is something to ask about.
    ///
    /// Landing on a branch that has fallen behind is the everyday case and
    /// loses nothing: every commit it has is already reachable from where
    /// it is going, so the move is a fast-forward and simply happens. Only
    /// where the branch holds commits `start` does not — the case the
    /// question's own words describe — does this stop and emit
    /// [`SessionEvent::MoveNeedsAsk`] without touching anything.
    ///
    /// The check is a read, so a refusal here has nothing to undo.
    pub fn checkout_moving_branch(self: &Arc<Self>, local: String, start: String) {
        let session = Arc::clone(self);
        self.write(
            "checkout",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                if !branch::is_merged_into(&exec, &repo.workdir, &local, &start, &cancel).await? {
                    session
                        .sink
                        .event(SessionEvent::MoveNeedsAsk { local, start });
                    return Ok(());
                }
                let target = CheckoutTarget::ForceCreate { local, start };
                move_carrying(&exec, &repo, &target, &cancel).await
            },
        );
    }

    /// Moves the current branch to `rev`, carrying the index and the
    /// working tree as far as `mode` says. `ResetMode::Hard` destroys
    /// uncommitted work, so the UI asks before sending that one.
    pub fn reset(self: &Arc<Self>, rev: String, mode: branch::ResetMode) {
        self.write(
            "reset",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                branch::reset(&exec, &repo.workdir, &rev, mode, &cancel).await
            },
        );
    }

    /// Creates a branch, optionally switching to it.
    pub fn create_branch(
        self: &Arc<Self>,
        name: String,
        start_point: Option<String>,
        switch_to: bool,
    ) {
        self.write(
            "branch",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                branch::create(
                    &exec,
                    &repo.workdir,
                    &name,
                    start_point.as_deref(),
                    switch_to,
                    &cancel,
                )
                .await
            },
        );
    }

    pub fn delete_branch(self: &Arc<Self>, name: String, force: bool) {
        self.write(
            "branch",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                branch::delete(&exec, &repo.workdir, &name, force, &cancel).await
            },
        );
    }

    /// Asks whether `branch --delete` would go through for this branch:
    /// merged into its reference point it deletes quietly, unmerged git
    /// refuses. Asked when a menu opens over the branch, so its delete
    /// row can wear `-D` from the start instead of only after a refused
    /// try. Both halves of the answer are git's own — the upstream from
    /// `for-each-ref`, the reachability from `merge-base` — and the only
    /// rule copied here is which reference point wins. A read, not a
    /// write, so it skips the queue the way the other checks do.
    pub fn check_branch_delete(self: &Arc<Self>, branch: String) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            let reference = match branch::upstream_of(&s.executor, &workdir, &branch, &cancel).await
            {
                Ok(Some(upstream)) => upstream,
                Ok(None) => "HEAD".to_string(),
                Err(_) => return,
            };
            let rev = format!("refs/heads/{branch}");
            if let Ok(merged) =
                branch::is_merged_into(&s.executor, &workdir, &rev, &reference, &cancel).await
            {
                s.sink
                    .event(SessionEvent::BranchDeleteChecked { branch, merged });
            }
        });
    }

    pub fn rename_branch(self: &Arc<Self>, from: String, to: String, force: bool) {
        self.write(
            "branch",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                branch::rename(&exec, &repo.workdir, &from, &to, force, &cancel).await
            },
        );
    }

    /// Renames a tag: a new name on the same object, then the old name
    /// dropped (git has no rename of its own — see [`crate::tag`]).
    pub fn rename_tag(self: &Arc<Self>, from: String, to: String) {
        self.write(
            "tag",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                tag::rename(&exec, &repo.workdir, &from, &to, &cancel).await
            },
        );
    }

    /// Deletes a tag. Destructive in one way only: what it marked may
    /// have nothing else reaching it, so the UI asks first.
    pub fn delete_tag(self: &Arc<Self>, name: String) {
        self.write(
            "tag",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                tag::delete(&exec, &repo.workdir, &name, &cancel).await
            },
        );
    }

    /// Renames a stash entry — stored again under the new label, old entry
    /// dropped (git has no rename for one — see [`crate::stash::rename`]).
    pub fn rename_stash(self: &Arc<Self>, selector: String, message: String) {
        self.write(
            "stash",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                stash::rename(&exec, &repo.workdir, &selector, &message, &cancel).await
            },
        );
    }

    /// `git stash push`, over the whole working tree or only `paths`.
    pub fn stash_push(
        self: &Arc<Self>,
        message: String,
        options: stash::PushOptions,
        paths: Vec<String>,
    ) {
        self.write(
            "stash",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                stash::push(&exec, &repo.workdir, &message, options, &paths, &cancel).await
            },
        );
    }

    /// `git stash pop <selector>` (drops the stash on success).
    ///
    /// A restore that conflicts is not a failure: the work is across,
    /// waiting to be settled, and git keeps the entry in that case — so a
    /// conflicting pop lands exactly where an apply would have, and the
    /// way back is still in the list (デザイン規約 §変更を退避する).
    /// The exit code cannot tell that apart from a refusal that did
    /// nothing, so the working tree decides (`conflicts_now`).
    ///
    /// Only when the tree was settled to begin with, though: git will not
    /// restore onto an index that already has unmerged paths — it refuses
    /// outright and changes nothing (measured) — and the conflicts still
    /// standing there afterwards are the old ones, not proof of anything.
    pub fn stash_pop(self: &Arc<Self>, selector: String) {
        self.write(
            "stash",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let settled_first = !conflicts_now(&exec, &repo, &cancel).await?;
                match stash::pop(&exec, &repo.workdir, &selector, &cancel).await {
                    Ok(()) => Ok(()),
                    Err(error) => {
                        if settled_first && conflicts_now(&exec, &repo, &cancel).await? {
                            Ok(())
                        } else {
                            Err(error)
                        }
                    }
                }
            },
        );
    }

    /// `git stash apply <selector>` (keeps the stash).
    pub fn stash_apply(self: &Arc<Self>, selector: String) {
        self.write(
            "stash",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                stash::apply(&exec, &repo.workdir, &selector, &cancel).await
            },
        );
    }

    /// `git stash drop <selector>`.
    pub fn stash_drop(self: &Arc<Self>, selector: String) {
        self.write(
            "stash",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                stash::drop(&exec, &repo.workdir, &selector, &cancel).await
            },
        );
    }
}
