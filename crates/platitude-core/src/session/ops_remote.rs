//! Remote-facing operations: fetch, the push variants, remote
//! configuration, and what each remote advertises under `refs/tags/`.

use super::*;

impl RepoSession {
    /// `git fetch --prune`; `None` fetches every remote.
    ///
    /// Refreshed as a refs-only write, the same as the two nobody asks
    /// for: a fetch writes `refs/remotes/*` and `FETCH_HEAD` and gets no
    /// nearer the working tree than that ([`AfterWrite::Refs`]).
    ///
    /// **What the press gives up**: with nothing brought down it no
    /// longer doubles as a status poll, so a tree made dirty outside this
    /// window lands on the following tick rather than on the button.
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
    /// It is the one refusal with an answer: the fetch is what shows which
    /// commits the remote actually holds — the graph then draws both sides,
    /// so what an overwrite would remove can be seen rather than described —
    /// and it is also what re-arms the lease, which is pinned to a commit
    /// the remote has left and would be turned down again as it stands.
    ///
    /// Queued rather than run here, so it reports and refreshes like any
    /// other fetch — under an id of its own, which nobody holds: the
    /// push's answer is the push's. The push still fails: nothing is
    /// retried, and the next move is whoever is looking at it to make.
    fn catch_up_after(self: &Arc<Self>, result: &Result<(), GitError>, remote: String) {
        if result.as_ref().is_err_and(|error| error.is_outdated())
            && self.fetch(Some(remote)).is_none()
        {
            tracing::debug!("the catch-up fetch was not accepted: the session is closed");
        }
    }

    /// Pushes the branch that is checked out to wherever it belongs.
    ///
    /// Resolving the target is part of the job rather than something the
    /// UI works out: which remote a branch tracks lives in configuration,
    /// and a name like `origin/main` cannot be split back apart reliably.
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
    /// Separate from [`Self::push_current`] because nothing local knows
    /// where this goes: both halves come from the question, and the answer
    /// becomes the upstream so the question is asked once per branch.
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
    /// Nothing is contacted, so this succeeds on a URL that goes nowhere;
    /// the push that follows is what finds out. The remote is left in place
    /// when that happens — [`Self::set_remote_url`] is the way back.
    pub fn add_remote(self: &Arc<Self>, name: String, url: String) -> Option<OperationId> {
        self.write(
            OperationKind::Remote,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                remote::add(&exec, &repo.workdir, &name, &url, &cancel).await
            },
        )
    }

    /// Marks where a push goes when no branch says otherwise, or clears the
    /// mark (`remote.pushDefault`).
    ///
    /// An empty name clears it. **That reaches the repository's own config
    /// only** — a value set globally stays, and git has no local spelling
    /// for "not set" that would shadow it (measured: an empty local value is
    /// "no destination", not "unset"). Moving the mark to another remote is
    /// what a repository has against a global one, and that is a set rather
    /// than a clear.
    ///
    /// Goes through the write queue for the refresh behind it: the mark is
    /// in the refs snapshot, and the sidebar reads it from there.
    pub fn set_push_default(self: &Arc<Self>, name: String) -> Option<OperationId> {
        self.write(
            OperationKind::Remote,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                if name.is_empty() {
                    remote::clear_push_default(&exec, &repo.workdir, &cancel).await
                } else {
                    remote::set_push_default(&exec, &repo.workdir, &name, &cancel).await
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
    /// else made". A read, not a write — but one that reaches the network,
    /// which is why it is only asked while that question is on screen.
    ///
    /// One at a time: the name is asked about as it is typed, and a new
    /// ask cancels the round trip the last one started rather than
    /// letting two race for the far side (`session::latest`). A cancelled
    /// ask answers nothing — the one that displaced it is the one the
    /// question is waiting on.
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
                // The name is taken, so what matters now is whether git
                // would take the push: it does when this history already
                // contains what the remote holds, and refuses otherwise
                // (measured). The commit may not be in this repository at all —
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
                        // answered at all), so the walk can count them. A
                        // walk that failed anyway must not answer "nothing":
                        // `theirs` is only trusted under `Refused`.
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
                // A newer ask took the question over: nothing to answer.
                Err(e) if e.is_cancelled() => return,
                // A remote that cannot be reached answers too, and the
                // failure is not raised as one: the question is standing
                // and about to say so itself, so opening the command log
                // over it would say the same thing twice (the command is
                // recorded either way).
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

    /// `git push <remote> --delete <branch>`.
    pub fn delete_remote_branch(
        self: &Arc<Self>,
        remote_name: String,
        branch_name: String,
    ) -> Option<OperationId> {
        let timeout = self.network_timeout();
        self.write(
            OperationKind::Push,
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
        )
    }

    /// Deletes a branch here and on its remote as one write. The local
    /// half goes first because it is the half that can refuse: a refusal
    /// stops the pair with nothing touched anywhere, and the menu row
    /// that asked morphs the way the plain delete's does. The remote
    /// half reaches the network, which is why the pair sits on the write
    /// queue as one command with one answer (合成は 1 手目が失敗したら
    /// 止める — core.md) — and why the pair is a kind of its own
    /// ([`OperationKind::DeleteBranchEverywhere`]): it answers as a
    /// branch write, but the far end paces its second half.
    pub fn delete_branch_everywhere(
        self: &Arc<Self>,
        branch: String,
        remote_name: String,
        remote_branch: String,
        force: bool,
    ) -> Option<OperationId> {
        let timeout = self.network_timeout();
        self.write(
            OperationKind::DeleteBranchEverywhere,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                branch::delete(&exec, &repo.workdir, &branch, force, &cancel).await?;
                remote::delete_remote_branch(
                    &exec,
                    &repo.workdir,
                    &remote_name,
                    &remote_branch,
                    timeout,
                    &cancel,
                )
                .await
            },
        )
    }

    /// Renames a branch on a remote, which git does as a push and a delete
    /// (see [`remote::rename_remote_branch`]). The UI asks first: the old
    /// name is destroyed, not moved.
    pub fn rename_remote_branch(
        self: &Arc<Self>,
        remote_name: String,
        from: String,
        to: String,
    ) -> Option<OperationId> {
        let timeout = self.network_timeout();
        self.write(
            OperationKind::Push,
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
        )
    }
}
