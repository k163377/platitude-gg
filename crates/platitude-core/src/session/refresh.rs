//! Snapshot refreshes: refs, status, stashes and worktrees. The stopped
//! operation read beside the status is [`super::refresh_op`], the author
//! identity reads are [`super::author`], and the head-reachability record
//! and walk [`super::head_reach`].

use super::joins::{RefJoins, build_label_map, build_snapshot, join_key, refs_key};
use super::*;

impl RepoSession {
    pub fn refresh_refs(self: &Arc<Self>) {
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            // **The rebuild is asked for from inside the pass**, so that a
            // caller closing the reads (`wait_for_snapshot_reads`) has
            // closed the ask as well: a boundary that let the reader out
            // first would have the reader's rebuild land on top of the one
            // the caller took afterwards and cancel it. A caller answered
            // by somebody else's pass is covered by the same rule, since
            // that pass asked before it could answer anyone.
            s.refs_read
                .run(|| async {
                    let moved = s.publish_refs().await;
                    if moved {
                        s.refresh_log();
                    }
                    moved
                })
                .await;
        });
    }

    /// [`RepoSession::publish_refs`] behind the single flight every refs
    /// reader shares ([`ReadFlight`]). **The way in for all of them** —
    /// the periodic tick and a write's own settling included, which is
    /// what keeps two `for-each-ref` over tens of thousands of refs from
    /// running at once.
    pub(super) async fn read_refs(self: &Arc<Self>) -> bool {
        self.refs_read.run(|| self.publish_refs()).await
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
    async fn publish_refs(self: &Arc<Self>) -> bool {
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
        // Whether the list below is repeated rather than read. It decides
        // what a ref move means for it further down, and it has to be
        // asked before the read that would settle it either way.
        let remotes_repeated = self.remotes.peek(|_| ()).is_some();
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
                // A ref move may have arrived with a config edit, so the
                // following read has to ask again — unless this pass read
                // the list itself, in which case what it holds is already
                // newer than the move and dropping it would only buy the
                // same answer twice (a write drops it on the way in, so
                // the read that settles the write is exactly that case).
                //
                // **Read from the pass, not from its caller.** One pass
                // answers every reader sharing this flight ([`ReadFlight`]),
                // so a caller that named its own case would be naming it
                // for readers that are not in it.
                if refs_moved && remotes_repeated {
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
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            // Asked for from inside the pass, for the reason
            // [`RepoSession::refresh_refs`] gives.
            s.status_read
                .run(|| async {
                    // An external change (another tool, the terminal) can
                    // make the tree dirty or clean, which adds or removes
                    // the WIP row.
                    let flipped = s.publish_status().await;
                    if flipped {
                        s.refresh_log();
                    }
                    flipped
                })
                .await;
        });
    }

    /// [`RepoSession::publish_status`] behind the single flight every
    /// status reader shares ([`ReadFlight`]). **The way in for all of
    /// them** — the periodic tick and a write's own settling included,
    /// which is what keeps two `status -uall` from lstat'ing every tracked
    /// and ignored file at the same time. `publish_status` lives in
    /// [`super::refresh_op`] and so cannot be made private the way
    /// [`RepoSession::publish_refs`] is; what keeps a fourth caller off it
    /// is this doc and
    /// `session_integration::the_ways_in_to_a_status_read_never_run_two_at_once`.
    pub(super) async fn read_status(self: &Arc<Self>) -> bool {
        self.status_read.run(|| self.publish_status()).await
    }

    /// One snapshot read behind its own flight ([`ReadFlight`]), and an
    /// answer only from the pass that is still the current one
    /// ([`OpGate`]).
    ///
    /// The flight and the gate are reached through accessors because the
    /// spawned task outlives this call and each snapshot has its own pair.
    ///
    /// Not what refs and status do: those publish through a shared path
    /// and answer their caller whether the graph has to be walked again,
    /// so their flight is entered from more than this one place.
    fn refresh_gated<F, Fut>(
        self: &Arc<Self>,
        op: &'static str,
        flight: fn(&Self) -> &ReadFlight,
        gate: fn(&Self) -> &OpGate,
        read: F,
    ) where
        F: FnOnce(Arc<Self>, PathBuf, CancellationToken) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<SessionEvent, GitError>> + Send,
    {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let session = Arc::clone(&s);
            flight(&s)
                .run(move || async move {
                    let op_gen = gate(&session).begin();
                    let cancel = session.root_cancel.clone();
                    match read(Arc::clone(&session), workdir, cancel).await {
                        Ok(event) => {
                            if gate(&session).is_current(op_gen) {
                                session.sink.event(event);
                            }
                        }
                        Err(e) => session.fail(op, e),
                    }
                    // Nothing here rebuilds the graph, so there is nothing
                    // to answer the callers sharing this pass.
                    false
                })
                .await;
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
