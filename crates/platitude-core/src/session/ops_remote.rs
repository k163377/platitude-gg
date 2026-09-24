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
    /// **What the press gives up**: with nothing brought down it does
    /// not double as a status poll, so a tree made dirty outside this
    /// window lands on the following tick.
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
    /// Refreshed as a write that moves history ([`AfterWrite::Graph`]):
    /// it is a fetch and an integration in one command, and the near half
    /// lands on this copy's branch, its index and its working tree.
    ///
    /// **The stop is not a failure**, the same as the merge's and the
    /// rebase's: git leaves the operation standing with its markers and
    /// the conflicted rows, and the way on is the exit card
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
    /// It is the one refusal with an answer: the fetch is what shows which
    /// commits the remote actually holds — the graph then draws both sides,
    /// so what an overwrite would remove can be seen —
    /// and it is also what re-arms the lease, which is pinned to a commit
    /// the remote has left and would be turned down again as it stands.
    ///
    /// Queued, so it reports and refreshes like any
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
    /// Resolving the target is part of the job: which remote a branch
    /// tracks lives in configuration, and a name like `origin/main`
    /// cannot be split back apart reliably.
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

    /// Marks a remote as this repository's origin, or clears the mark —
    /// both of its keys either way (`remote.pushDefault` and
    /// `checkout.defaultRemote`, [`remote::OriginMarks`]).
    ///
    /// An empty name clears it. **That reaches the repository's own
    /// config only** — a value set globally stays, and git has no local
    /// spelling for "not set" that would shadow the push's (measured: an
    /// empty local value means "no destination"). Moving the mark to
    /// another remote is what a repository has against a global one, and
    /// that is a set.
    ///
    /// Goes through the write queue for the refresh behind it: the mark is
    /// in the refs snapshot, and the sidebar reads it from there.
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
    /// else made". A read that reaches the network, which is why it is
    /// only asked while that question is on screen.
    ///
    /// One at a time: the name is asked about as it is typed, and a
    /// new ask cancels the round trip the last one started
    /// (`session::latest`). A cancelled ask answers nothing — the
    /// one that displaced it is the one the question is waiting
    /// on.
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
                        // walk that failed leaves `Unknown`, since
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
                // failure stays off the error surface: the question is
                // standing and about to say so itself, so opening the
                // command log would say the same thing twice
                // (the command is recorded either way).
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

    /// Points a branch at a remote branch and then sends it there — the
    /// answer to a rename here that takes nothing away over there
    /// (デザイン規約 §手元の改名の後のリモート).
    ///
    /// **The setting goes down first, and that order is the point.** git's
    /// own `push --set-upstream` records the pair only once the push has
    /// landed, so a push the far side turned down leaves the branch still
    /// measured against the name it was renamed away from. Written first,
    /// a refused push leaves a branch already pointing where it belongs
    /// and the toolbar's `push` is the retry ([`branch::set_upstream`]
    /// writes the pair straight for a name this repository has not
    /// fetched, which is exactly the name this pair is making).
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
    /// does as a push and a delete (see [`remote::replace_remote_branch`]).
    /// The UI asks first: the old name is destroyed.
    pub fn replace_remote_branch(
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
                remote::replace_remote_branch(
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
