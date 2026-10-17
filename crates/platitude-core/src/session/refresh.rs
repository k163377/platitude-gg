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

    /// [`Self::read_refs`] for a caller whose place in the flight was
    /// taken when it was asked ([`ReadFlight::stamp`]).
    pub(super) async fn read_refs_at(self: &Arc<Self>, stamp: Stamp) -> Reread {
        self.refs_read.run_from(stamp, || self.publish_refs()).await
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
                let keys = refs_keys(&refs, &head);
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
                // Whether the held snapshot still answers, decided once, ahead
                // of what a rebuilt one waits for: past the fence below
                // nothing may wait, or a write ending meanwhile would see a
                // reading from before it published. One pass at a time
                // (`refs_read`), so nothing moves the key in between.
                let unmoved = if *relock(&self.join_key) == Some(inputs) {
                    self.published_snapshot()
                } else {
                    None
                };
                // What the menus' `push` rows count with.
                let push_tracks = if unmoved.is_none() {
                    self.read_push_tracks(&workdir, &refs, &remotes, &cancel)
                        .await
                } else {
                    None
                };
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
                let key = keys.walk;
                let previous = relock(&self.refs_key).replace(key);
                *relock(&self.join_key) = Some(inputs);
                if let Some(held) = unmoved {
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
                // Not the one HEAD is on: the status tick reads and keeps its
                // mark (`StatusLoaded::push_remote`).
                snapshot.branch_push_remotes = remotes
                    .branch_push_remotes
                    .into_iter()
                    .filter(|mark| head.branch.as_deref() != Some(mark.branch.as_str()))
                    .collect();
                // A read that could not tell repeats the last answer, as the
                // status tick's does (`Standing::push_track_of`): a diverged
                // row falling to the plain push would lose its overwrite.
                snapshot.push_tracks = push_tracks.unwrap_or_else(|| {
                    self.published_snapshot()
                        .map(|held| held.push_tracks.clone())
                        .unwrap_or_default()
                });
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

    /// The counts of the branches a mark sends away from their upstream's
    /// remote, against where it sends them ([`remote::push_tracks`]) — what
    /// a menu's `push` row on such a branch weighs
    /// ([`RefsSnapshot::branch_push`]). Asked where the snapshot looks to be
    /// rebuilt — a pass the fence then turns back has read it for nothing —
    /// and only where some branch goes elsewhere: one process for a
    /// repository with marks, none (and no walk over the refs) for one
    /// without. `None` where the read could not tell.
    async fn read_push_tracks(
        &self,
        workdir: &Path,
        refs: &[RefEntry],
        remotes: &remote::Remotes,
        cancel: &CancellationToken,
    ) -> Option<Vec<(String, remote::PushTrack)>> {
        if remotes.push_default.is_none() && remotes.branch_push_remotes.is_empty() {
            return Some(Vec::new());
        }
        let push_default = remotes
            .push_default
            .as_ref()
            .map_or("", |marked| marked.remote.as_str());
        let mark_of = |branch: &str| {
            remotes
                .branch_push_remotes
                .binary_search_by(|mark| mark.branch.as_str().cmp(branch))
                .map_or("", |at| remotes.branch_push_remotes[at].remote.as_str())
        };
        let index = refs::RemoteBranches::index(refs);
        // A branch with an upstream: with none, the push asks where it goes
        // and nothing is counted. One measured against a branch here goes
        // to a remote by a mark alone. Not the one HEAD is on, which the
        // status tick counts.
        let elsewhere: std::collections::HashSet<&str> = refs
            .iter()
            .filter(|entry| entry.kind == RefKind::LocalBranch && !entry.is_head)
            .filter(|local| {
                let upstream = match index.spoken_for(local) {
                    Some(upstream) => remote::Upstream::Remote(upstream.short.as_str()),
                    None if local.upstream_is_here() => remote::Upstream::Here,
                    None => return false,
                };
                remote::pushes_elsewhere(
                    upstream,
                    mark_of(local.short.as_str()),
                    push_default,
                    remotes.list.iter().map(|r| r.name.as_str()),
                )
            })
            .map(|local| local.short.as_str())
            .collect();
        if elsewhere.is_empty() {
            return Some(Vec::new());
        }
        match remote::push_tracks(&self.executor, workdir, cancel).await {
            Ok(mut tracks) => {
                tracks.retain(|(branch, _)| elsewhere.contains(branch.as_str()));
                Some(tracks)
            }
            Err(error) => {
                // A close cancels it, which says nothing about the repository.
                if !error.is_cancelled() {
                    tracing::warn!(%error, "the counts against where marks send pushes");
                }
                None
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

    /// [`Self::read_status`] for a place taken when the caller was asked.
    pub(super) async fn read_status_at(self: &Arc<Self>, stamp: Stamp) -> Reread {
        self.status_read
            .run_from(stamp, || self.publish_status())
            .await
    }

    /// Reads the stash list behind its own flight ([`ReadFlight`]) and
    /// publishes it, answering with what it published ([`StashRead`]) —
    /// no list where the read failed (the failure is on the error surface).
    ///
    /// The one reader of the list: the listing and the walk, which draws
    /// each stash as a row, both ask here, so one pass answers whichever of
    /// them asked before it began, and a walk handed the listing's answer
    /// reads nothing while that answer still stands
    /// ([`Self::stashes_standing`]).
    pub(super) async fn read_stashes(self: &Arc<Self>) -> StashRead {
        let Some(workdir) = self.workdir() else {
            return StashRead::default();
        };
        self.stash_read
            .run(|| async {
                // Stamped before git is spawned, as in the refs pass
                // (`Standing::stamp`).
                let looked = self.standing.stamp();
                let cancel = self.root_cancel.clone();
                match stash::load(&self.executor, &workdir, &cancel).await {
                    Ok(stashes) => {
                        let list = Arc::new(stashes);
                        let (listing, moved) = relock(&self.stash_listings).record(&list);
                        self.sink.event(SessionEvent::StashesLoaded {
                            stashes: list.as_ref().clone(),
                            looked,
                        });
                        StashRead {
                            list: Some(list),
                            looked,
                            listing,
                            moved,
                        }
                    }
                    Err(e) => {
                        self.fail(FollowUp::Stashes.label(), e);
                        StashRead {
                            looked,
                            ..StashRead::default()
                        }
                    }
                }
            })
            .await
    }

    /// The list `read` published, while it still speaks for the
    /// repository: no write has ended since it looked, and no listing has
    /// published since. Held past either, it would hand a walk a stash that
    /// is gone — the walk names each stash's commit on its command line, so
    /// a dropped one is drawn back while its object lasts — and that walk,
    /// asked last, would replace the one that drew the drop.
    pub(super) fn stashes_standing(&self, read: StashRead) -> Option<Arc<Vec<StashEntry>>> {
        let list = read.list?;
        let newest = relock(&self.stash_listings).newest();
        (read.listing == newest && self.standing.current(read.looked)).then_some(list)
    }

    /// The worktree listing behind its own flight, published, and the
    /// joins it asks for — not the walk, which the caller folds into the
    /// one it makes for everything it read ([`Listed`]). Every caller the
    /// pass answers gets the same news ([`WorktreeRead`]) and waits for the
    /// same joins.
    pub(super) async fn read_worktrees(self: &Arc<Self>) -> Listed {
        let Some(workdir) = self.workdir() else {
            return Listed {
                failed: vec![FollowUp::Worktrees],
                ..Listed::default()
            };
        };
        let read = self
            .worktrees_read
            .run(|| async {
                // Stamped before git is spawned, as the stashes are.
                let looked = self.standing.stamp();
                let cancel = self.root_cancel.clone();
                match crate::worktrees::load(&self.executor, &workdir, &cancel).await {
                    Ok(listed) => {
                        let worktrees = super::removals::left_out(listed.clone(), self);
                        self.note_worktree_heads(&worktrees, &workdir);
                        // The holds only from a listing no write has ended
                        // behind (`Standing::current`): one that looked before
                        // a worktree's removal answered would note it back,
                        // and the refs read that follows would draw its marks
                        // again. A write that moves the holds reads a listing
                        // behind itself (`AfterWrite::Worktrees`).
                        let news = if self.standing.current(looked) {
                            *relock(&self.worktrees_listed) = Arc::new(listed);
                            self.note_worktree_holders(&worktrees, &workdir)
                        } else {
                            super::joins::WorktreeNews::default()
                        };
                        self.sink
                            .event(SessionEvent::WorktreesLoaded { worktrees, looked });
                        WorktreeRead {
                            published: true,
                            news,
                        }
                    }
                    Err(e) => {
                        self.fail(FollowUp::Worktrees.label(), e);
                        WorktreeRead::default()
                    }
                }
            })
            .await;
        let mut listed = Listed {
            walk: read.news.walk,
            ..Listed::default()
        };
        if !read.published {
            listed.failed.push(FollowUp::Worktrees);
        }
        if read.news.joins {
            match self.read_refs().await {
                Reread::Moved => listed.walk = true,
                Reread::Same => {}
                Reread::Failed => listed.failed.push(FollowUp::Refs),
            }
        }
        listed
    }

    /// The stash and worktree listings together, for a caller that walks
    /// once behind both.
    pub(super) async fn read_listings(self: &Arc<Self>) -> (StashRead, Listed) {
        let (stashes, mut listed) = tokio::join!(self.read_stashes(), self.read_worktrees());
        if stashes.list.is_none() {
            listed.failed.push(FollowUp::Stashes);
        }
        (stashes, listed)
    }
}

/// What a listing read behind a write, a refresh or this tree's paced
/// read left to its caller:
/// whether the history has to be walked for it, and the reads that did not
/// land.
#[derive(Debug, Default)]
pub(super) struct Listed {
    pub(super) walk: bool,
    pub(super) failed: Vec<FollowUp>,
}
