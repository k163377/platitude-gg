//! Remote-facing operations: fetch, the push variants, remote
//! configuration, and what each remote advertises under `refs/tags/`.

use super::*;

impl RepoSession {
    /// `git fetch --prune`; `None` fetches every remote.
    ///
    /// Refreshed as refs-only ([`AfterWrite::Refs`]): a fetch writes
    /// `refs/remotes/*` and `FETCH_HEAD`, nothing nearer the working tree.
    /// So it is no status poll — a tree made dirty outside this window
    /// lands on the following tick.
    pub fn fetch(self: &Arc<Self>, remote: Option<String>) -> Option<OperationId> {
        let timeout = self.network_timeout();
        let s = Arc::clone(self);
        self.write(
            OperationKind::Fetch,
            AfterWrite::Refs,
            move |exec, repo, cancel| async move {
                s.fetch_and_read_tags(&exec, &repo.workdir, remote.as_deref(), timeout, &cancel)
                    .await
            },
        )
    }

    /// `git pull`: the branch the working tree is on and its upstream.
    ///
    /// Refreshed as a history move ([`AfterWrite::Graph`]): the
    /// integrating half lands on this branch, its index and working tree.
    /// A stop is not a failure, as with merge and rebase
    /// (デザイン規約 §進行中の操作から出る).
    pub fn pull(self: &Arc<Self>) -> Option<OperationId> {
        let timeout = self.network_timeout();
        let session = Arc::clone(self);
        self.write(
            OperationKind::Pull,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let landing = remote::pull(&exec, &repo.workdir, timeout, &cancel).await?;
                session.note_landing(landing);
                Ok(())
            },
        )
    }

    /// `git push` for one branch.
    pub fn push(self: &Arc<Self>, spec: remote::PushSpec) -> Option<OperationId> {
        let timeout = self.network_timeout();
        let s = Arc::clone(self);
        self.write(
            OperationKind::Push,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let target = spec.remote.clone();
                let result = remote::push(&exec, &repo.workdir, &spec, timeout, &cancel).await;
                s.catch_up_after(&result, target);
                result
            },
        )
    }

    /// Fetches when a push was refused for knowing the remote only as it
    /// used to be, and does nothing otherwise.
    ///
    /// The fetch shows which commits the remote holds (the graph then
    /// draws what an overwrite would remove) and re-arms the lease, which
    /// is pinned to a commit the remote has left.
    ///
    /// Queued under an id of its own that nobody holds: the push's answer
    /// is the push's. The push still fails; nothing is retried.
    fn catch_up_after(self: &Arc<Self>, result: &Result<(), GitError>, remote: String) {
        if result.as_ref().is_err_and(|error| error.is_outdated())
            && self.fetch(Some(remote)).is_none()
        {
            tracing::debug!("the catch-up fetch was not accepted: the session is closed");
        }
    }

    /// Pushes the branch that is checked out to wherever it belongs.
    ///
    /// The target is resolved inside the write: which remote a branch
    /// tracks lives in configuration, and `origin/main` cannot be split
    /// back apart reliably.
    pub fn push_current(
        self: &Arc<Self>,
        fallback_remote: String,
        force: remote::PushForce,
    ) -> Option<OperationId> {
        let timeout = self.network_timeout();
        let s = Arc::clone(self);
        self.write(
            OperationKind::Push,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let spec = remote::plan_current_push(
                    &exec,
                    &repo.workdir,
                    &fallback_remote,
                    force,
                    &cancel,
                )
                .await?;
                let target = spec.remote.clone();
                let result = remote::push(&exec, &repo.workdir, &spec, timeout, &cancel).await;
                s.catch_up_after(&result, target);
                result
            },
        )
    }

    /// The first push of a branch, to the target the user just named.
    ///
    /// Apart from [`Self::push_current`] because nothing local knows where
    /// this goes; the answer becomes the upstream, so it is asked once per
    /// branch.
    pub fn publish_current(
        self: &Arc<Self>,
        remote_name: String,
        remote_branch: String,
        expect: String,
    ) -> Option<OperationId> {
        let timeout = self.network_timeout();
        let s = Arc::clone(self);
        self.write(
            OperationKind::Push,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let spec = remote::plan_publish(
                    &exec,
                    &repo.workdir,
                    &remote_name,
                    &remote_branch,
                    &expect,
                    &cancel,
                )
                .await?;
                let target = spec.remote.clone();
                let result = remote::push(&exec, &repo.workdir, &spec, timeout, &cancel).await;
                s.catch_up_after(&result, target);
                result
            },
        )
    }

    /// `git remote add <name> <url>`.
    ///
    /// Nothing is contacted, so a URL that goes nowhere succeeds and stays
    /// until a push finds out; [`Self::set_remote_url`] is the way back.
    pub fn add_remote(self: &Arc<Self>, name: String, url: String) -> Option<OperationId> {
        self.write(
            OperationKind::Remote,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                remote::add(&exec, &repo.workdir, &name, &url, &cancel).await
            },
        )
    }

    /// Marks a remote as this repository's origin, or clears the mark
    /// (empty name) — both keys either way (`remote.pushDefault` and
    /// `checkout.defaultRemote`, [`remote::OriginMarks`]).
    ///
    /// **Clearing reaches the repository's own config only**: a global
    /// value stays, and git has no local spelling for "not set" (an empty
    /// local value means "no destination"). Against a global mark, move it
    /// to another remote.
    ///
    /// Queued for the refresh behind it: the sidebar reads the mark off the
    /// refs snapshot.
    pub fn mark_origin(self: &Arc<Self>, name: String) -> Option<OperationId> {
        self.write(
            OperationKind::Remote,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                if name.is_empty() {
                    remote::clear_origin(&exec, &repo.workdir, &cancel).await
                } else {
                    remote::mark_origin(&exec, &repo.workdir, &name, &cancel).await
                }
            },
        )
    }

    /// `git remote set-url <name> <url>` — correcting a URL typed wrong.
    pub fn set_remote_url(self: &Arc<Self>, name: String, url: String) -> Option<OperationId> {
        self.write(
            OperationKind::Remote,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                remote::set_url(&exec, &repo.workdir, &name, &url, &cancel).await
            },
        )
    }

    /// Asks whether a remote already carries a branch name, so a first push
    /// can tell "this creates a branch" from "this advances one somebody
    /// else made". Reaches the network, so it is only asked while that
    /// question is on screen.
    ///
    /// One at a time (`session::latest`): the name is asked as it is typed,
    /// a new ask cancels the last one's round trip, and a cancelled ask
    /// answers nothing.
    pub fn check_remote_branch(self: &Arc<Self>, remote_name: String, branch: String) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let timeout = self.network_timeout();
        let cancel = self.remote_branch_read.begin(&self.root_cancel);
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let mut tip = String::new();
            let mut theirs = 0;
            let state = match remote::branch_tip(
                &s.executor,
                &workdir,
                &remote_name,
                &branch,
                timeout,
                &cancel,
            )
            .await
            {
                Ok(None) => remote::RemoteBranchState::Free,
                // Taken: git takes the push only when this history already
                // contains the remote's tip. A tip never fetched here leaves
                // neither answer ours to give.
                Ok(Some(over_there)) => {
                    tip = over_there.to_hex();
                    match commit::is_in_head_history(&s.executor, &workdir, &over_there, &cancel)
                        .await
                    {
                        Ok(true) => remote::RemoteBranchState::FastForward,
                        // Only an overwrite lands here, and it has to say
                        // what it takes off; the tip is here (the comparison
                        // answered), so the walk can count. `theirs` is only
                        // trusted under `Refused`, so a failed walk is
                        // `Unknown`.
                        Ok(false) => {
                            match commit::count_beyond_head(
                                &s.executor,
                                &workdir,
                                &over_there,
                                &cancel,
                            )
                            .await
                            {
                                Ok(count) => {
                                    theirs = count;
                                    remote::RemoteBranchState::Refused
                                }
                                Err(_) => remote::RemoteBranchState::Unknown,
                            }
                        }
                        Err(_) => remote::RemoteBranchState::Unknown,
                    }
                }
                // A newer ask took the question over.
                Err(e) if e.is_cancelled() => return,
                // Unreachable answers too, off the error surface: the
                // question says so itself (the command is logged either
                // way).
                Err(_) => remote::RemoteBranchState::Unreachable,
            };
            if cancel.is_cancelled() {
                return;
            }
            s.sink.event(SessionEvent::RemoteBranchChecked {
                remote: remote_name,
                branch,
                state,
                tip,
                theirs,
            });
        });
    }

    /// `git push <remote> --delete <branch>`, leased to `expect`, the
    /// commit the screen showed it on ([`remote::delete_remote_branch`]).
    /// A remote that moved since is caught up with, as a push is
    /// ([`Self::catch_up_after`]). The commits only the remote branch held
    /// leave the graph at once (`session::leaving`).
    pub fn delete_remote_branch(
        self: &Arc<Self>,
        remote_name: String,
        branch_name: String,
        expect: String,
    ) -> Option<OperationId> {
        let timeout = self.network_timeout();
        let status = self.measures_head(None, Some((&remote_name, &branch_name)));
        let (remote, branch) = (remote_name.clone(), branch_name.clone());
        let s = Arc::clone(self);
        self.write_taking(
            OperationKind::Push,
            AfterWrite::Name { status },
            &[LeavingRef::Remote(&remote, &branch)],
            move |exec, repo, cancel| async move {
                let result = remote::delete_remote_branch(
                    &exec,
                    &repo.workdir,
                    &remote_name,
                    &branch_name,
                    &expect,
                    timeout,
                    &cancel,
                )
                .await;
                s.catch_up_after(&result, remote_name);
                result
            },
        )
    }

    /// Deletes a branch here and on its remote as one write, the remote
    /// half leased to `expect` as in [`Self::delete_remote_branch`].
    ///
    /// The local half goes first because it can refuse, stopping the pair
    /// with nothing touched (core.md「複合操作は 1 手目が失敗したら止める」) —
    /// and it has to: `push --delete` takes the remote-tracking ref with
    /// it, and a plain `branch --delete` merged only into that upstream is
    /// then refused as not merged, leaving the remote gone and the branch
    /// here. A refused lease stops the pair half-way instead, with the
    /// branch gone here and its commit reachable from the remote-tracking
    /// ref — the catch-up fetch moves that ref on to where the remote went.
    ///
    /// A kind of its own ([`OperationKind::DeleteBranchEverywhere`]): it
    /// answers as a branch write, but the far end paces its second half.
    ///
    /// Plain or forced, both names leave the graph at once, and the commits
    /// only they held with them (`session::leaving`): what a plain delete
    /// was merged into may be the remote branch going too.
    pub fn delete_branch_everywhere(
        self: &Arc<Self>,
        branch: String,
        remote_name: String,
        remote_branch: String,
        force: bool,
        expect: String,
    ) -> Option<OperationId> {
        let timeout = self.network_timeout();
        let status = self.measures_head(Some(&branch), Some((&remote_name, &remote_branch)));
        let names = (branch.clone(), remote_name.clone(), remote_branch.clone());
        let s = Arc::clone(self);
        self.write_taking(
            OperationKind::DeleteBranchEverywhere,
            AfterWrite::Name { status },
            &[
                LeavingRef::Branch(&names.0),
                LeavingRef::Remote(&names.1, &names.2),
            ],
            move |exec, repo, cancel| async move {
                branch::delete(&exec, &repo.workdir, &branch, force, &cancel).await?;
                s.own_config_rewrite();
                let result = remote::delete_remote_branch(
                    &exec,
                    &repo.workdir,
                    &remote_name,
                    &remote_branch,
                    &expect,
                    timeout,
                    &cancel,
                )
                .await;
                s.catch_up_after(&result, remote_name);
                result
            },
        )
    }

    /// Points a branch at a remote branch and then sends it there — the
    /// answer to a rename here that takes nothing away over there
    /// (デザイン規約 §手元の改名の後のリモート).
    ///
    /// **The setting goes down first.** `push --set-upstream` records the
    /// pair only once the push landed, so a refused push would leave the
    /// branch measured against the name it was renamed away from; written
    /// first, the toolbar's `push` is the retry ([`branch::set_upstream`]
    /// writes the pair straight for a name not yet fetched).
    pub fn point_upstream_and_push(
        self: &Arc<Self>,
        branch: String,
        remote_name: String,
        remote_branch: String,
    ) -> Option<OperationId> {
        let timeout = self.network_timeout();
        self.write(
            OperationKind::Push,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                branch::set_upstream(
                    &exec,
                    &repo.workdir,
                    &branch,
                    &remote_name,
                    &remote_branch,
                    &cancel,
                )
                .await?;
                let spec = remote::PushSpec {
                    remote: remote_name,
                    local: format!("refs/heads/{branch}"),
                    remote_branch,
                    set_upstream: false,
                    force: remote::PushForce::None,
                };
                remote::push(&exec, &repo.workdir, &spec, timeout, &cancel).await
            },
        )
    }

    /// Replaces a branch on a remote with one under a new name, which git
    /// does as a push and a delete (see [`remote::replace_remote_branch`]),
    /// the delete leased to `expect`, and caught up with as a push is. The
    /// UI asks first: the old name is destroyed.
    pub fn replace_remote_branch(
        self: &Arc<Self>,
        remote_name: String,
        from: String,
        to: String,
        expect: String,
    ) -> Option<OperationId> {
        let timeout = self.network_timeout();
        let s = Arc::clone(self);
        self.write(
            OperationKind::Push,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let result = remote::replace_remote_branch(
                    &exec,
                    &repo.workdir,
                    &remote_name,
                    &from,
                    &to,
                    &expect,
                    timeout,
                    &cancel,
                )
                .await;
                s.catch_up_after(&result, remote_name);
                result
            },
        )
    }
}
