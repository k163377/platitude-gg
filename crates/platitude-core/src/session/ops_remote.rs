//! Remote-facing operations: fetch, the push variants, remote
//! configuration, and what each remote advertises under `refs/tags/`.

use super::*;

impl RepoSession {
    /// `git fetch --prune`; `None` fetches every remote.
    pub fn fetch(self: &Arc<Self>, remote: Option<String>) {
        let timeout = self.network_timeout();
        let s = Arc::clone(self);
        self.write(
            "fetch",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                s.fetch_and_read_tags(&exec, &repo.workdir, remote.as_deref(), timeout, &cancel)
                    .await
            },
        );
    }

    /// The fetch, and then the one thing it cannot leave behind: where the
    /// remotes keep their tags.
    ///
    /// A fetched tag lands in `refs/tags/` beside the ones made here, so
    /// afterwards nothing local says which is which. Asking costs a second
    /// round trip, and this is where it belongs — the user has already
    /// agreed to reach the network, and a poll never should.
    pub(super) async fn fetch_and_read_tags(
        self: &Arc<Self>,
        exec: &GitExecutor,
        workdir: &Path,
        remote_name: Option<&str>,
        timeout: std::time::Duration,
        cancel: &CancellationToken,
    ) -> Result<(), GitError> {
        remote::fetch(exec, workdir, remote_name, timeout, cancel).await?;
        self.read_remote_tags(exec, workdir, remote_name, timeout, cancel)
            .await;
        Ok(())
    }

    /// Records what each remote advertises under `refs/tags/`.
    ///
    /// Reports nothing upwards. A badge is not worth failing a fetch that
    /// worked, and a remote that could not be reached keeps the answer it
    /// last gave instead of dropping every cloud it accounted for — which
    /// is what `refs/remotes/` does for branches on its own.
    /// Answers whether what the remotes carry moved — the caller's reason
    /// to republish, and nobody else's business.
    pub(super) async fn read_remote_tags(
        &self,
        exec: &GitExecutor,
        workdir: &Path,
        only: Option<&str>,
        timeout: std::time::Duration,
        cancel: &CancellationToken,
    ) -> bool {
        let remotes = match remote::list(exec, workdir, cancel).await {
            Ok(list) => list,
            Err(error) => {
                tracing::debug!(%error, "remote tags: the remotes could not be listed");
                return false;
            }
        };
        // A remote that is no longer configured stops answering for names.
        self.lock_remote_tags()
            .retain(|name, _| remotes.iter().any(|r| r.name == *name));
        for r in remotes {
            if only.is_some_and(|wanted| wanted != r.name) {
                continue;
            }
            match remote::list_tags(exec, workdir, &r.name, timeout, cancel).await {
                Ok(tags) => {
                    self.lock_remote_tags().insert(r.name, tags);
                }
                Err(error) if error.is_cancelled() => return false,
                Err(error) => {
                    tracing::debug!(remote = %r.name, %error, "remote tags: unreadable");
                }
            }
        }
        self.remerge_remote_tags()
    }

    fn lock_remote_tags(&self) -> std::sync::MutexGuard<'_, RemoteTagsByRemote> {
        match self.remote_tags.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        }
    }

    /// Merges the per-remote answers into the index the joins read, and
    /// says whether that changed anything. The only place the index is
    /// built: everything downstream shares the one it leaves behind.
    fn remerge_remote_tags(&self) -> bool {
        let mut index = RemoteTagIndex::new();
        for (remote, tags) in self.lock_remote_tags().iter() {
            for tag in tags {
                let place: &mut RemoteTagPlace = index
                    .entry(tag.name.clone())
                    .or_default()
                    .entry(tag.commit)
                    .or_default();
                place.annotated |= tag.annotated;
                place.remotes.push(remote.clone());
            }
        }
        let mut slot = match self.remote_tag_index.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        if **slot == index {
            return false;
        }
        *slot = Arc::new(index);
        true
    }

    /// The per-remote answers merged into the index the join reads.
    pub(super) fn remote_tag_index(&self) -> Arc<RemoteTagIndex> {
        match self.remote_tag_index.lock() {
            Ok(g) => Arc::clone(&g),
            Err(e) => Arc::clone(&e.into_inner()),
        }
    }

    /// `git push` for one branch.
    pub fn push(self: &Arc<Self>, spec: remote::PushSpec) {
        let timeout = self.network_timeout();
        let s = Arc::clone(self);
        self.write(
            "push",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let target = spec.remote.clone();
                let result = remote::push(&exec, &repo.workdir, &spec, timeout, &cancel).await;
                s.catch_up_after(&result, target);
                result
            },
        );
    }

    /// Fetches when a push was refused for knowing the remote only as it
    /// used to be, and does nothing otherwise.
    ///
    /// It is the one refusal with an answer: the fetch is what shows which
    /// commits the remote actually holds — the graph then draws both sides,
    /// so what an overwrite would remove can be seen rather than described —
    /// and it is also what re-arms the lease, which is pinned to a commit
    /// the remote has left and would be turned down again as it stands.
    ///
    /// Queued rather than run here, so it reports and refreshes like any
    /// other fetch. The push still fails: nothing is retried, and the next
    /// move is whoever is looking at it to make.
    fn catch_up_after(self: &Arc<Self>, result: &Result<(), GitError>, remote: String) {
        if matches!(result, Err(GitError::PushOutdated { .. })) {
            self.fetch(Some(remote));
        }
    }

    /// Pushes the branch that is checked out to wherever it belongs.
    ///
    /// Resolving the target is part of the job rather than something the
    /// UI works out: which remote a branch tracks lives in configuration,
    /// and a name like `origin/main` cannot be split back apart reliably.
    pub fn push_current(self: &Arc<Self>, fallback_remote: String, force: remote::PushForce) {
        let timeout = self.network_timeout();
        let s = Arc::clone(self);
        self.write(
            "push",
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
        );
    }

    /// The first push of a branch, to the target the user just named.
    ///
    /// Separate from [`Self::push_current`] because nothing local knows
    /// where this goes: both halves come from the question, and the answer
    /// becomes the upstream so the question is asked once per branch.
    pub fn publish_current(
        self: &Arc<Self>,
        remote_name: String,
        remote_branch: String,
        expect: String,
    ) {
        let timeout = self.network_timeout();
        let s = Arc::clone(self);
        self.write(
            "push",
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
        );
    }

    /// `git remote add <name> <url>`.
    ///
    /// Nothing is contacted, so this succeeds on a URL that goes nowhere;
    /// the push that follows is what finds out. The remote is left in place
    /// when that happens — [`Self::set_remote_url`] is the way back.
    pub fn add_remote(self: &Arc<Self>, name: String, url: String) {
        self.write(
            "remote",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                remote::add(&exec, &repo.workdir, &name, &url, &cancel).await
            },
        );
    }

    /// `git remote set-url <name> <url>` — correcting a URL typed wrong.
    pub fn set_remote_url(self: &Arc<Self>, name: String, url: String) {
        self.write(
            "remote",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                remote::set_url(&exec, &repo.workdir, &name, &url, &cancel).await
            },
        );
    }

    /// Asks whether a remote already carries a branch name, so a first push
    /// can tell "this creates a branch" from "this advances one somebody
    /// else made". A read, not a write — but one that reaches the network,
    /// which is why it is only asked while that question is on screen.
    pub fn check_remote_branch(self: &Arc<Self>, remote_name: String, branch: String) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let timeout = self.network_timeout();
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
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
                // The name is taken, so what matters now is whether git
                // would take the push: it does when this history already
                // contains what the remote holds, and refuses otherwise
                // (実測). The commit may not be in this repository at all —
                // a branch never fetched — and then neither answer is
                // ours to give.
                Ok(Some(over_there)) => {
                    tip = over_there.to_hex();
                    match commit::is_in_head_history(&s.executor, &workdir, &over_there, &cancel)
                        .await
                    {
                        Ok(true) => remote::RemoteBranchState::FastForward,
                        // Only an overwrite can land here, and an overwrite
                        // has to say what it takes off — the commit is in
                        // this repository (that is how the comparison was
                        // answered at all), so the walk can count them.
                        Ok(false) => {
                            theirs = commit::count_beyond_head(
                                &s.executor,
                                &workdir,
                                &over_there,
                                &cancel,
                            )
                            .await
                            .unwrap_or(0);
                            remote::RemoteBranchState::Refused
                        }
                        Err(_) => remote::RemoteBranchState::Unknown,
                    }
                }
                // A remote that cannot be reached answers too, and the
                // failure is not raised as one: the question is standing
                // and about to say so itself, so opening the command log
                // over it would say the same thing twice (the command is
                // recorded either way).
                Err(_) => remote::RemoteBranchState::Unreachable,
            };
            s.sink.event(SessionEvent::RemoteBranchChecked {
                remote: remote_name,
                branch,
                state,
                tip,
                theirs,
            });
        });
    }

    /// `git push <remote> --delete <branch>`.
    pub fn delete_remote_branch(self: &Arc<Self>, remote_name: String, branch_name: String) {
        let timeout = self.network_timeout();
        self.write(
            "push",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                remote::delete_remote_branch(
                    &exec,
                    &repo.workdir,
                    &remote_name,
                    &branch_name,
                    timeout,
                    &cancel,
                )
                .await
            },
        );
    }

    /// Renames a branch on a remote, which git does as a push and a delete
    /// (see [`remote::rename_remote_branch`]). The UI asks first: the old
    /// name is destroyed, not moved.
    pub fn rename_remote_branch(self: &Arc<Self>, remote_name: String, from: String, to: String) {
        let timeout = self.network_timeout();
        self.write(
            "push",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                remote::rename_remote_branch(
                    &exec,
                    &repo.workdir,
                    &remote_name,
                    &from,
                    &to,
                    timeout,
                    &cancel,
                )
                .await
            },
        );
    }
}
