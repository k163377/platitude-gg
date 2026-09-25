//! Snapshot refreshes: refs, status, stashes and worktrees. The stopped
//! operation read beside the status is [`super::refresh_op`], the author
//! identity reads are [`super::author`], and the head-reachability record
//! and walk [`super::head_reach`].

use super::joins::{RefJoins, build_label_map, build_snapshot, join_key, refs_keys};
use super::*;

impl RepoSession {
    pub fn refresh_refs(self: &Arc<Self>) {
        self.read_refs_from(self.refs_read.stamp());
    }

    /// The refs pass for a caller whose place in the flight is already
    /// taken (`ReadFlight::stamp`) — the way the fenced read asks again
    /// from inside its own pass.
    pub(super) fn read_refs_from(self: &Arc<Self>, stamp: Stamp) {
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            // The rebuild is asked for inside the pass, so a caller closing
            // the reads (`wait_for_snapshot_reads`) has closed the ask too;
            // asked after, it would land on top of the caller's own rebuild
            // and cancel it.
            s.refs_read
                .run_from(stamp, || async {
                    let reread = s.publish_refs().await;
                    if reread == Reread::Moved {
                        s.refresh_log();
                    }
                    reread
                })
                .await;
        });
    }

    /// [`RepoSession::publish_refs`] behind the refs readers' single
    /// flight ([`ReadFlight`]) — the way in for all of them, the tick and
    /// a write's own settling included.
    pub(super) async fn read_refs(self: &Arc<Self>) -> Reread {
        self.refs_read.run(|| self.publish_refs()).await
    }

    /// Reads refs and HEAD, publishes the snapshot and the label diff, and
    /// reports whether the ref layout moved since the last read — or that
    /// nothing was published at all.
    ///
    /// A moved ref means commits the graph has never seen, which only a
    /// walk brings. The rebuild is the caller's, so one that reads status
    /// in the same pass rebuilds once for both
    /// ([`RepoSession::refresh_poll`]).
    async fn publish_refs(self: &Arc<Self>) -> Reread {
        let Some(workdir) = self.workdir() else {
            return Reread::Failed;
        };
        // Stamped before git is spawned: the stamp orders when the
        // repository was looked at (`Standing`).
        let looked = self.standing.stamp();
        let cancel = self.root_cancel.clone();
        let refs = refs::load(&self.executor, &workdir, &cancel).await;
        // Only detached and unborn have no marked ref in the listing and
        // pay for asking again (`refs::head_in`).
        let head = match refs.as_ref().ok().and_then(|refs| refs::head_in(refs)) {
            Some(head) => Ok(head),
            None => refs::head_state(&self.executor, &workdir, &cancel).await,
        };
        // Asked before the read below settles it: whether that read is a
        // repeat decides what a ref move means further down.
        let remotes_repeated = self.remotes.peek(|_| ()).is_some();
        // No remotes, or a failed read of them, is no reason to lose the
        // refs.
        let remotes = self.remotes(&workdir, &cancel).await.unwrap_or_default();
        match (refs, head) {
            (Ok(refs), Ok(head)) => {
                // Fenced by a write that ended after this looked
                // (`Standing::current`): read again, since an index-only
                // write reads no refs behind itself. Stamped inside this
                // pass, so a read the write already has waiting on the gate
                // answers it.
                if !self.standing.current(looked) {
                    self.read_refs_from(self.refs_read.stamp());
                    return Reread::Same;
                }
                // Before the joins: the walk asks git for HEAD only while
                // nothing has told it. Too late for the two walks `open`
                // starts; it saves every rebuild after.
                self.record_head_from_refs(looked, &refs, &head);
                let remote_tags = self.remote_tag_index();
                let keys = refs_keys(&refs, &head);
                let key = keys.walk;
                let previous = relock(&self.refs_key).replace(key);
                // The joins have a key of their own (`join_key`): an
                // unmoved repository does not build them, since sorting
                // every ref into a snapshot only to compare it equal is
                // work on every quiet tick
                // (ci/baseline/refs-join-windows-x64.md). The held
                // snapshot still goes out, for a consumer that attached
                // since; the sidebar reads it by pointer (`share_snapshot`).
                let inputs = join_key(
                    keys.listing,
                    self.remote_tag_gen.load(Ordering::SeqCst),
                    self.worktree_gen.load(Ordering::SeqCst),
                    &remotes,
                );
                let seen = relock(&self.join_key).replace(inputs);
                if seen == Some(inputs)
                    && let Some(held) = self.published_snapshot()
                {
                    self.sink.event(SessionEvent::RefsLoaded {
                        snapshot: held,
                        looked,
                    });
                    // The refs are part of `inputs`: nothing moved.
                    return Reread::Same;
                }
                let held = self.worktree_holders();
                let joins = RefJoins::new(&refs, &held);
                let mut snapshot = build_snapshot(&refs, &head, &remote_tags, &joins);
                (snapshot.remote_names, snapshot.remote_urls) = remotes
                    .list
                    .into_iter()
                    .map(|r| (r.name, r.fetch_url))
                    .unzip();
                snapshot.push_default = remotes.push_default;
                snapshot.checkout_default = remotes.checkout_default;
                let label_map = build_label_map(&refs, &head, &remote_tags, &joins);
                self.sink.event(SessionEvent::RefsLoaded {
                    snapshot: self.share_snapshot(snapshot),
                    looked,
                });
                // Last, after the snapshot: the chip diff is read and
                // sent under the graph lock (see `apply_refs`), and
                // nothing else may run inside it.
                self.apply_refs(label_map);
                let refs_moved = previous.is_some_and(|previous| previous != key);
                // Whether a tip is held by nothing else depends on where the
                // refs point: re-asked when they move, and on the first read.
                if previous != Some(key) {
                    self.settle_head_reach();
                    // A HEAD move swaps the checked-out files, and with them
                    // the line-ending neighbours.
                    self.forget_eol_derived();
                }
                // A ref move may have come with a config edit, so the next
                // read asks again — unless this pass read the list itself
                // and already holds something newer than the move. Decided
                // from the pass, not the caller: one pass answers every
                // reader in the flight ([`ReadFlight`]).
                if refs_moved && remotes_repeated {
                    self.remotes.forget();
                }
                if refs_moved {
                    Reread::Moved
                } else {
                    Reread::Same
                }
            }
            (Err(e), _) | (_, Err(e)) => {
                self.fail(FollowUp::Refs.label(), e);
                Reread::Failed
            }
        }
    }

    pub fn refresh_status(self: &Arc<Self>) {
        self.read_status_from(self.status_read.stamp());
    }

    /// [`RepoSession::read_refs_from`] for the status pass.
    pub(super) fn read_status_from(self: &Arc<Self>, stamp: Stamp) {
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            // The rebuild is asked for inside the pass, as in
            // [`RepoSession::read_refs_from`].
            s.status_read
                .run_from(stamp, || async {
                    let reread = s.publish_status().await;
                    if reread == Reread::Moved {
                        s.refresh_log();
                    }
                    reread
                })
                .await;
        });
    }

    /// [`RepoSession::publish_status`] behind the status readers' single
    /// flight ([`ReadFlight`]) — the way in for all of them.
    /// `publish_status` lives in [`super::refresh_op`] and so cannot be
    /// private the way [`RepoSession::publish_refs`] is; nothing else may
    /// call it
    /// (`session_integration::the_ways_in_to_a_status_read_never_run_two_at_once`).
    pub(super) async fn read_status(self: &Arc<Self>) -> Reread {
        self.status_read.run(|| self.publish_status()).await
    }

    /// Reads the stash list again, behind its own flight ([`ReadFlight`]).
    ///
    /// Answers with the read's task, done once the listing has published
    /// — what the write queue waits on before it says a write is settled
    /// (`session::write`) — and the task answers the reads that failed:
    /// empty where the listing landed. `None` where no repository is open.
    pub fn refresh_stashes(self: &Arc<Self>) -> Option<tokio::task::JoinHandle<Vec<FollowUp>>> {
        let workdir = self.workdir()?;
        let s = Arc::clone(self);
        Some(self.runtime.spawn(async move {
            let session = Arc::clone(&s);
            let published = s
                .stash_read
                .run(move || async move {
                    // Stamped before git is spawned, as in the refs pass
                    // (`Standing::stamp`).
                    let looked = session.standing.stamp();
                    let cancel = session.root_cancel.clone();
                    match stash::load(&session.executor, &workdir, &cancel).await {
                        Ok(stashes) => {
                            session
                                .sink
                                .event(SessionEvent::StashesLoaded { stashes, looked });
                            true
                        }
                        Err(e) => {
                            session.fail(FollowUp::Stashes.label(), e);
                            false
                        }
                    }
                })
                .await;
            if published {
                Vec::new()
            } else {
                vec![FollowUp::Stashes]
            }
        }))
    }

    /// Reads the worktree list again, behind its own flight as
    /// [`Self::refresh_stashes`] does, and then the reads it asks for: a
    /// working copy taken or given back moves no ref, so the joins are
    /// asked for by name, and a copy on no branch is a row only the walk
    /// can put there.
    ///
    /// Those reads are awaited after the flight is let go: the task is
    /// what the write queue waits on to call a write settled, and letting
    /// the walk run on would put that boundary before its row. Every
    /// caller the pass answers gets the same news ([`WorktreeRead`]) and
    /// waits for the same reads. The task answers the reads that did not
    /// land, the listing's own included; empty where all did.
    pub fn refresh_worktrees(self: &Arc<Self>) -> Option<tokio::task::JoinHandle<Vec<FollowUp>>> {
        let workdir = self.workdir()?;
        let s = Arc::clone(self);
        Some(self.runtime.spawn(async move {
            let session = Arc::clone(&s);
            let read = s
                .worktrees_read
                .run(move || async move {
                    let cancel = session.root_cancel.clone();
                    match crate::worktrees::load(&session.executor, &workdir, &cancel).await {
                        Ok(worktrees) => {
                            let news = session.note_worktree_holders(&worktrees, &workdir);
                            session
                                .sink
                                .event(SessionEvent::WorktreesLoaded { worktrees });
                            WorktreeRead {
                                published: true,
                                news,
                            }
                        }
                        Err(e) => {
                            session.fail(FollowUp::Worktrees.label(), e);
                            WorktreeRead::default()
                        }
                    }
                })
                .await;
            let mut failed = Vec::new();
            if !read.published {
                failed.push(FollowUp::Worktrees);
            }
            let mut walk = read.news.walk;
            if read.news.joins {
                match s.read_refs().await {
                    Reread::Moved => walk = true,
                    Reread::Same => {}
                    Reread::Failed => failed.push(FollowUp::Refs),
                }
            }
            if walk && !s.settle_graph().await.landed() {
                failed.push(FollowUp::Graph);
            }
            failed
        }))
    }
}
