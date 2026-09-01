//! Snapshot refreshes: refs, status, stashes and worktrees. The stopped
//! operation read beside the status is [`super::refresh_op`], the author
//! identity reads are [`super::author`], and the head-reachability record
//! and walk [`super::head_reach`].

use super::joins::{RefJoins, build_label_map, build_snapshot, join_key, refs_key};
use super::*;

impl RepoSession {
    pub fn refresh_refs(self: &Arc<Self>) {
        if !self.refs_read.claim() {
            return;
        }
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let mut held = SlotHeld::new(&s.refs_read);
            loop {
                if s.publish_refs(true).await {
                    s.refresh_log();
                }
                if !held.finish() {
                    break;
                }
            }
        });
    }

    /// Reads refs and HEAD, publishes the snapshot and the label diff, and
    /// reports whether the ref layout moved since the last read.
    ///
    /// Chips alone are applied without rebuilding, but a moved ref means
    /// commits the graph has never seen (an external commit, a fetch, a
    /// switch), and those only appear if the walk runs again.
    ///
    /// Does not rebuild the graph itself: a caller that reads status in the
    /// same pass rebuilds once for both (see [`RepoSession::refresh_poll`]).
    pub(super) async fn publish_refs(self: &Arc<Self>, forget_remotes_on_move: bool) -> bool {
        let Some(workdir) = self.workdir() else {
            return false;
        };
        let op_gen = self.refs_gate.begin();
        let cancel = self.root_cancel.clone();
        let refs = refs::load(&self.executor, &workdir, &cancel).await;
        // The listing marks the branch HEAD is on, so the ordinary case is
        // already answered and the two processes that ask again are not
        // spawned at all. Detached and unborn have no marked ref, and only
        // those pay (`refs::head_in`).
        let head = match refs.as_ref().ok().and_then(|refs| refs::head_in(refs)) {
            Some(head) => Ok(head),
            None => refs::head_state(&self.executor, &workdir, &cancel).await,
        };
        // A repository with no remotes is normal, and so is a failure
        // to read the list; neither is a reason to lose the refs.
        let remotes = self.remotes(&workdir, &cancel).await.unwrap_or_default();
        match (refs, head) {
            (Ok(refs), Ok(head)) => {
                if !self.refs_gate.is_current(op_gen) {
                    return false;
                }
                // First, and before the joins: the walk asks git where
                // HEAD is only while nothing has told it, so the answer
                // goes down as early as this read can put it there.
                //
                // It does not save the two walks `open` starts. Measured
                // on `JetBrains/kotlin`: the listing above is 300ms on its
                // own, and both of them are past this point before it
                // returns. What it settles is every rebuild after — a
                // commit, a fetch, a poll tick that found a ref moved.
                self.remember_head_hold(&refs, &head);
                let remote_tags = self.remote_tag_index();
                let key = refs_key(&refs, &head);
                let previous = relock(&self.refs_key).replace(key);
                // **The joins have a key of their own** (`join_key`), and
                // an unmoved repository does not build them at all —
                // sorting tens of thousands of refs into a snapshot only
                // to compare it equal is measurable work on every quiet
                // tick (ci/baseline/refs-join-windows-x64.md).
                //
                // The snapshot still goes out. A consumer that attached
                // after the last one is waiting for it, and it is the one
                // already published, so the sidebar reads it by pointer
                // and rebuilds nothing (`share_snapshot`).
                let inputs = join_key(
                    key,
                    self.remote_tag_gen.load(Ordering::SeqCst),
                    self.worktree_gen.load(Ordering::SeqCst),
                    &remotes,
                );
                let seen = relock(&self.join_key).replace(inputs);
                if seen == Some(inputs)
                    && let Some(held) = self.published_snapshot()
                {
                    self.sink.event(SessionEvent::RefsLoaded { snapshot: held });
                    // The refs are part of `inputs`, so they are where
                    // they were: nothing to walk, nothing to re-ask.
                    return false;
                }
                // One index and one set of joins for both halves: the
                // sidebar snapshot and the row chips read the same
                // listing, and building either twice is one whole join
                // thrown away.
                let held = self.worktree_holders();
                let joins = RefJoins::new(&refs, &held);
                let mut snapshot = build_snapshot(&refs, &head, &remote_tags, &joins);
                (snapshot.remote_names, snapshot.remote_urls) = remotes
                    .list
                    .into_iter()
                    .map(|r| (r.name, r.fetch_url))
                    .unzip();
                snapshot.push_default = remotes.push_default;
                let label_map = build_label_map(&refs, &head, &remote_tags, &joins);
                self.sink.event(SessionEvent::RefsLoaded {
                    snapshot: self.share_snapshot(snapshot),
                });
                // Last, and not before the snapshot: the chip diff is read
                // and sent under the graph lock (see `apply_refs`), and
                // nothing else may run inside it.
                self.apply_refs(label_map);
                let refs_moved = previous.is_some_and(|previous| previous != key);
                // A tip that nothing else holds is a property of where the
                // refs point, so it is re-asked when they move — and on
                // the first read, which has nothing to compare against.
                if previous != Some(key) {
                    self.settle_head_reach();
                    // HEAD moving swaps out the checked-out files, and with
                    // them whatever the neighbours of a path looked like.
                    self.forget_eol_derived();
                }
                // The first read already used the current remote list. An
                // external ref move is different: it may have arrived with
                // a config edit, so the following read has to ask again.
                if refs_moved && forget_remotes_on_move {
                    self.remotes.forget();
                }
                refs_moved
            }
            (Err(e), _) | (_, Err(e)) => {
                self.fail("refs", e);
                false
            }
        }
    }

    pub fn refresh_status(self: &Arc<Self>) {
        if !self.status_read.claim() {
            return;
        }
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let mut held = SlotHeld::new(&s.status_read);
            loop {
                // An external change (another tool, the terminal) can make
                // the tree dirty or clean, which adds or removes the WIP row.
                if s.publish_status().await {
                    s.refresh_log();
                }
                if !held.finish() {
                    break;
                }
            }
        });
    }

    /// One gated snapshot read: at most one in flight with a single repeat
    /// booked behind it ([`ReadSlot`]), and an answer only from the pass
    /// that is still the current one ([`OpGate`]).
    ///
    /// The slot and the gate are reached through accessors because the
    /// spawned task outlives this call and each snapshot has its own pair.
    ///
    /// Not what refs and status do: those publish through a shared path
    /// and answer their caller whether the graph has to be walked again.
    fn refresh_gated<F, Fut>(
        self: &Arc<Self>,
        op: &'static str,
        slot: fn(&Self) -> &ReadSlot,
        gate: fn(&Self) -> &OpGate,
        read: F,
    ) where
        F: Fn(Arc<Self>, PathBuf, CancellationToken) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<SessionEvent, GitError>> + Send,
    {
        let Some(workdir) = self.workdir() else {
            return;
        };
        if !slot(self).claim() {
            return;
        }
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let mut held = SlotHeld::new(slot(&s));
            loop {
                let op_gen = gate(&s).begin();
                let cancel = s.root_cancel.clone();
                match read(Arc::clone(&s), workdir.clone(), cancel).await {
                    Ok(event) => {
                        if gate(&s).is_current(op_gen) {
                            s.sink.event(event);
                        }
                    }
                    Err(e) => s.fail(op, e),
                }
                if !held.finish() {
                    break;
                }
            }
        });
    }

    pub fn refresh_stashes(self: &Arc<Self>) {
        self.refresh_gated(
            "stash",
            |s| &s.stash_read,
            |s| &s.stash_gate,
            |s, workdir, cancel| async move {
                let stashes = stash::load(&s.executor, &workdir, &cancel).await?;
                Ok(SessionEvent::StashesLoaded { stashes })
            },
        );
    }

    pub fn refresh_worktrees(self: &Arc<Self>) {
        self.refresh_gated(
            "worktrees",
            |s| &s.worktrees_read,
            |s| &s.worktrees_gate,
            |s, workdir, cancel| async move {
                let worktrees = crate::worktrees::load(&s.executor, &workdir, &cancel).await?;
                // A working copy taken or given back moves no ref, so the
                // join that marks the rows has to be asked for by name.
                if s.note_worktree_holders(&worktrees, &workdir) {
                    s.refresh_refs();
                }
                Ok(SessionEvent::WorktreesLoaded { worktrees })
            },
        );
    }
}
