//! What a write does to a name: creating, renaming and taking away
//! branches and tags, and the upstream a branch follows.
//!
//! Beside [`super::ops_tree`], which is what a write does to the files.

use super::*;

impl RepoSession {
    /// Creates a branch, optionally switching to it.
    pub fn create_branch(
        self: &Arc<Self>,
        name: String,
        start_point: Option<String>,
        switch_to: bool,
    ) -> Option<OperationId> {
        self.write(
            OperationKind::Branch,
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
        )
    }

    pub fn delete_branch(self: &Arc<Self>, name: String, force: bool) -> Option<OperationId> {
        self.write(
            OperationKind::Branch,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                branch::delete(&exec, &repo.workdir, &name, force, &cancel).await
            },
        )
    }

    /// Asks whether `branch --delete` would go through for this branch
    /// (merged into its reference point) or be refused, for a menu opened
    /// over a branch the drawn rows could not answer for
    /// (`GraphModel.branchDeleteMerged`), so its delete row can wear `-D`
    /// from the start. The reference point is git's own rule, read off the
    /// snapshot (`BranchItem::upstream_oid`: the upstream where the listing
    /// resolves it, HEAD otherwise); reachability is `merge-base`'s. A
    /// read, so it skips the queue.
    ///
    /// **Every ask is answered**, a failed read included
    /// ([`SessionEvent::BranchDeleteChecked::merged`]), so a harness
    /// waiting for the echo sees a run whose reads fell over.
    pub fn check_branch_delete(self: &Arc<Self>, branch: String) {
        let Some(workdir) = self.workdir() else {
            self.sink.event(SessionEvent::BranchDeleteChecked {
                branch,
                merged: None,
            });
            return;
        };
        let snapshot = self.published_snapshot();
        let reference = snapshot
            .as_deref()
            .and_then(|snapshot| snapshot.local_named(&branch))
            .and_then(|item| item.upstream_oid)
            .map_or_else(|| "HEAD".to_string(), |oid| oid.to_hex());
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            let rev = format!("refs/heads/{branch}");
            let merged = branch::is_merged_into(&s.executor, &workdir, &rev, &reference, &cancel)
                .await
                .ok();
            s.sink
                .event(SessionEvent::BranchDeleteChecked { branch, merged });
        });
    }

    /// Points a local branch at the remote branch it is measured against.
    ///
    /// `remote` and `remote_branch` arrive apart, as the question was
    /// answered — a pair cannot be read two ways where `origin/x` can
    /// ([`branch::set_upstream`]). Queued: the counts beside the branch,
    /// the delete's reference point and a push's target all come off this
    /// setting, and the refresh behind the write redraws them.
    pub fn set_upstream(
        self: &Arc<Self>,
        branch: String,
        remote: String,
        remote_branch: String,
    ) -> Option<OperationId> {
        self.write(
            OperationKind::Branch,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                branch::set_upstream(
                    &exec,
                    &repo.workdir,
                    &branch,
                    &remote,
                    &remote_branch,
                    &cancel,
                )
                .await
            },
        )
    }

    pub fn rename_branch(
        self: &Arc<Self>,
        from: String,
        to: String,
        force: bool,
    ) -> Option<OperationId> {
        self.write(
            OperationKind::Branch,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                branch::rename(&exec, &repo.workdir, &from, &to, force, &cancel).await
            },
        )
    }

    /// Puts a tag on a commit. Lightweight and unforced: an existing
    /// name is git's to refuse (see [`crate::tag::create`]).
    pub fn create_tag(self: &Arc<Self>, name: String, commit: String) -> Option<OperationId> {
        self.write(
            OperationKind::Tag,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                tag::create(&exec, &repo.workdir, &name, &commit, &cancel).await
            },
        )
    }

    /// Renames a tag: a new name on the same object, then the old name
    /// dropped (git has no rename of its own — see [`crate::tag`]).
    pub fn rename_tag(self: &Arc<Self>, from: String, to: String) -> Option<OperationId> {
        self.write(
            OperationKind::Tag,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                tag::rename(&exec, &repo.workdir, &from, &to, &cancel).await
            },
        )
    }

    /// Deletes a tag. What it marked may have nothing else reaching it, so
    /// the UI asks first.
    pub fn delete_tag(self: &Arc<Self>, name: String) -> Option<OperationId> {
        self.write(
            OperationKind::Tag,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                tag::delete(&exec, &repo.workdir, &name, &cancel).await
            },
        )
    }
}
