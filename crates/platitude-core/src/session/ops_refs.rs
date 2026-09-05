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
    /// refuses. Asked when a menu opens over a branch the drawn rows
    /// could not answer for (`GraphModel.branchDeleteMerged`), so its
    /// delete row can wear `-D` from the start instead of only after a
    /// refused try. The reference point is read off the snapshot, which
    /// already holds git's own rule for it (`BranchItem::upstream_oid`:
    /// the upstream where the listing resolves it, HEAD otherwise — a
    /// configured name that resolves to nothing is not the measure), and
    /// the reachability is `merge-base`'s. A read, not a write, so it
    /// skips the queue the way the other checks do.
    pub fn check_branch_delete(self: &Arc<Self>, branch: String) {
        let Some(workdir) = self.workdir() else {
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
            if let Ok(merged) =
                branch::is_merged_into(&s.executor, &workdir, &rev, &reference, &cancel).await
            {
                s.sink
                    .event(SessionEvent::BranchDeleteChecked { branch, merged });
            }
        });
    }

    /// Points a local branch at the remote branch it is measured against.
    ///
    /// `upstream` is the full remote-tracking refname the question was
    /// answered with — the one spelling that cannot be read two ways
    /// ([`branch::set_upstream`]). Through the queue like every other
    /// write: the counts beside the branch, the delete's reference point
    /// and where a push goes all come off this setting, so the reads
    /// behind it are the ones that put the new answer on screen.
    pub fn set_upstream(self: &Arc<Self>, branch: String, upstream: String) {
        self.write(
            "branch",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                branch::set_upstream(&exec, &repo.workdir, &branch, &upstream, &cancel).await
            },
        );
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

    /// Puts a tag on a commit. Lightweight, and never forced: an existing
    /// name is git's to refuse (see [`crate::tag::create`]).
    pub fn create_tag(self: &Arc<Self>, name: String, commit: String) {
        self.write(
            "tag",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                tag::create(&exec, &repo.workdir, &name, &commit, &cancel).await
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
}
