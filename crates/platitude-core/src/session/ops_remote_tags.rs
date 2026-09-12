//! What each remote carries under `refs/tags/`, and the writes that move
//! it: the reading nothing local records, and sending, replacing or
//! taking away one tag.
//!
//! Apart from [`super::ops_remote`] because a tag is the one ref whose
//! whereabouts on a remote has to be asked for outright — every other row
//! of the sidebar reads its answer off `refs/remotes/` — and every write
//! here pays for that by reading again on its way out.

use super::*;

impl RepoSession {
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
        let remotes = match self.remotes(workdir, cancel).await {
            Ok(read) => read.list,
            Err(error) => {
                tracing::debug!(%error, "remote tags: the remotes could not be listed");
                return false;
            }
        };
        // Only the remotes that actually answered are replaced. One that
        // could not be reached keeps the readings it last gave, which is
        // what `refs/remotes/` does for branches on its own.
        let mut answered: Vec<crate::Name> = Vec::new();
        let mut fresh: Vec<(crate::Name, Oid, bool, crate::Name)> = Vec::new();
        for r in &remotes {
            if only.is_some_and(|wanted| wanted != r.name) {
                continue;
            }
            match remote::list_tags(exec, workdir, &r.name, timeout, cancel).await {
                Ok(tags) => {
                    let remote = crate::Name::from(r.name.as_str());
                    answered.push(remote.clone());
                    fresh.extend(
                        tags.into_iter()
                            .map(|t| (t.name, t.commit, t.annotated, remote.clone())),
                    );
                }
                Err(error) if error.is_cancelled() => return false,
                Err(error) => {
                    tracing::debug!(remote = %r.name, %error, "remote tags: unreadable");
                }
            }
        }
        self.remerge_remote_tags(&remotes, &answered, fresh)
    }

    /// Rebuilds the index with `fresh` in place of what `answered` said
    /// last, and says whether that changed anything. The only place the
    /// index is built: everything downstream shares the one it leaves.
    ///
    /// Reads what it keeps out of the index itself, so the readings are
    /// held once (see [`RemoteTagIndex::readings`]).
    fn remerge_remote_tags(
        &self,
        configured: &[remote::Remote],
        answered: &[crate::Name],
        fresh: Vec<(crate::Name, Oid, bool, crate::Name)>,
    ) -> bool {
        let current = self.remote_tag_index();
        let kept = current.readings().filter(|(_, _, _, remote)| {
            // A remote that is no longer configured stops answering for
            // names, and one that just answered is replaced rather than
            // added to.
            configured.iter().any(|r| r.name == remote.as_str())
                && !answered.iter().any(|a| a == remote)
        });
        let index = RemoteTagIndex::build(kept.chain(fresh.iter().cloned()));
        let mut slot = relock(&self.remote_tag_index);
        if **slot == index {
            return false;
        }
        *slot = Arc::new(index);
        self.remote_tag_gen.fetch_add(1, Ordering::SeqCst);
        true
    }

    /// The per-remote answers merged into the index the join reads.
    pub(super) fn remote_tag_index(&self) -> Arc<RemoteTagIndex> {
        Arc::clone(&relock(&self.remote_tag_index))
    }

    /// Sends one tag to one remote. `expect` pins a leased overwrite to
    /// the commit that remote was last seen holding the tag on; empty
    /// sends it plain (see [`remote::push_tag`]).
    ///
    /// **The badge is re-read before this write is done.** What the
    /// remotes carry under `refs/tags/` has no local record, so nothing
    /// else would notice that this push changed it — the refresh that
    /// follows reads `refs/`, and this tag was already there. Inside the
    /// write for the same reason [`Self::fetch_and_read_tags`] is: the
    /// press has already agreed to reach the network, which is what the
    /// unasked catch-up may not do (core.md タグのリモート状態). Only the
    /// remote that just moved is asked, and a push git refused moved
    /// nothing, so a failure leaves the last answer standing.
    ///
    /// **Nothing is refreshed from in here.** `AfterWrite::Graph` reads
    /// the refs once the closure returns, and a read asked for from
    /// inside would be a second pass over every ref — the longest read
    /// this application makes on a repository that has them.
    pub fn push_tag(
        self: &Arc<Self>,
        remote_name: String,
        tag: String,
        expect: String,
    ) -> Option<OperationId> {
        let timeout = self.network_timeout();
        let s = Arc::clone(self);
        self.write(
            OperationKind::Push,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                remote::push_tag(
                    &exec,
                    &repo.workdir,
                    &remote_name,
                    &tag,
                    &expect,
                    timeout,
                    &cancel,
                )
                .await?;
                s.read_remote_tags(&exec, &repo.workdir, Some(&remote_name), timeout, &cancel)
                    .await;
                Ok(())
            },
        )
    }

    /// Takes one tag off one remote, leaving whatever is here.
    ///
    /// Reads that remote's tags afterwards for the reason
    /// [`Self::push_tag`] does: nothing local records what a remote
    /// carries, so a delete that landed would otherwise keep its badge
    /// until the next timer tick.
    pub fn delete_remote_tag(
        self: &Arc<Self>,
        remote_name: String,
        tag: String,
    ) -> Option<OperationId> {
        let timeout = self.network_timeout();
        let s = Arc::clone(self);
        self.write(
            OperationKind::Push,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                remote::delete_remote_tag(
                    &exec,
                    &repo.workdir,
                    &remote_name,
                    &tag,
                    timeout,
                    &cancel,
                )
                .await?;
                s.read_remote_tags(&exec, &repo.workdir, Some(&remote_name), timeout, &cancel)
                    .await;
                Ok(())
            },
        )
    }

    /// Deletes a tag here and on the remote as one queued write.
    ///
    /// The local half goes first to match [`Self::delete_branch_everywhere`],
    /// though nothing about a tag refuses: what that ordering buys here is
    /// that a cancelled or failed pair never leaves the name gone from the
    /// remote while it still stands in the sidebar.
    pub fn delete_tag_everywhere(
        self: &Arc<Self>,
        tag: String,
        remote_name: String,
    ) -> Option<OperationId> {
        let timeout = self.network_timeout();
        let s = Arc::clone(self);
        // The far end paces the pair's second half and the read behind
        // it, so the pair is a kind of its own: it answers as a tag write
        // and runs on the remote lane ([`OperationKind::DeleteTagEverywhere`]).
        self.write(
            OperationKind::DeleteTagEverywhere,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                tag::delete(&exec, &repo.workdir, &tag, &cancel).await?;
                remote::delete_remote_tag(
                    &exec,
                    &repo.workdir,
                    &remote_name,
                    &tag,
                    timeout,
                    &cancel,
                )
                .await?;
                s.read_remote_tags(&exec, &repo.workdir, Some(&remote_name), timeout, &cancel)
                    .await;
                Ok(())
            },
        )
    }
}
