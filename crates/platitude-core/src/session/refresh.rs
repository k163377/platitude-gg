//! Snapshot refreshes: refs, status (+ op state), author identity, head
//! reachability, stashes and worktrees.

use super::joins::{RefJoins, build_label_map, build_snapshot, join_key, refs_key, status_key};
use super::*;

impl RepoSession {
    /// Records what the refs read just saw of the branch tip, including
    /// the half of the reachability question the listing answers by
    /// itself: some other ref sitting exactly on the tip.
    ///
    /// That half is what makes tags count without paying for them. Tags
    /// are left out of the walk (`JetBrains/kotlin`: 45,846 of 53,672
    /// refs, 478ms of 501ms — ci/baseline/head-reach-windows-x64.md), and
    /// a tag on the tip is the shape that actually turns up; one strictly
    /// ahead of it is missed, which costs a hold mark on a row that could
    /// have been a click.
    fn remember_head_hold(&self, refs: &[RefEntry], head: &HeadState) {
        let hold = head.oid.map(|tip| HeadHold {
            tip,
            branch: head.branch.clone().unwrap_or_default(),
            on_a_ref: reachable::a_ref_sits_on_head(refs, head),
        });
        if let Ok(mut slot) = self.head_hold.lock() {
            *slot = hold;
        }
        // Beside the hold, and both are written before anything this
        // read wakes can look.
        if let Ok(mut slot) = self.head_tip.lock() {
            *slot = Some(head.oid);
        }
    }

    /// Where the last refs read left HEAD, for a caller that would
    /// otherwise spawn two processes to ask again.
    ///
    /// `None` means no read has landed and there is nothing to go on.
    pub(super) fn known_head_tip(&self) -> Option<Option<Oid>> {
        match self.head_tip.lock() {
            Ok(slot) => *slot,
            Err(e) => *e.into_inner(),
        }
    }

    /// Answers whether the branch HEAD is on is the only thing holding its
    /// tip, and sends the answer if it moved.
    ///
    /// Runs off the write queue and off the poll's two-process budget: it
    /// is started where the graph is rebuilt, because it describes the
    /// same picture — whether a rewrite here leaves the old commits drawn.
    pub(super) fn settle_head_reach(self: &Arc<Self>) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let Ok(Some(hold)) = self.head_hold.lock().map(|slot| slot.clone()) else {
            // No tip (an unborn branch): nothing to lose, nothing to ask.
            self.publish_head_reach(false);
            return;
        };
        if hold.on_a_ref {
            self.publish_head_reach(true);
            return;
        }
        let Ok(permit) = Arc::clone(&self.head_reach_slot).try_acquire_owned() else {
            tracing::trace!("head reach skipped: the previous walk has not finished");
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let _permit = permit;
            let cancel = s.root_cancel.clone();
            match reachable::reached_without_branch(
                &s.executor,
                &workdir,
                &hold.tip.to_hex(),
                &hold.branch,
                &cancel,
            )
            .await
            {
                Ok(reached) => s.publish_head_reach(reached),
                // A failed walk must not claim the tip is held: the mark
                // is the safe answer, and the next refresh asks again.
                Err(e) => {
                    tracing::warn!(error = %e, "could not tell whether the branch tip is held");
                    s.publish_head_reach(false);
                }
            }
        });
    }

    fn publish_head_reach(&self, reached_elsewhere: bool) {
        let moved = self
            .head_reach_seen
            .lock()
            .map(|mut slot| slot.replace(reached_elsewhere) != Some(reached_elsewhere))
            .unwrap_or(true);
        if moved {
            self.sink
                .event(SessionEvent::HeadReachChecked { reached_elsewhere });
        }
    }

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
                let previous = self
                    .refs_key
                    .lock()
                    .map(|mut slot| slot.replace(key))
                    .unwrap_or_default();
                // **The joins have a key of their own** (`join_key`), and
                // an unmoved repository does not build them at all. It
                // used to: a quiet tick sorted 53,724 refs into a snapshot
                // and 47,715 into a label map, then compared the result
                // with the last one to be told nothing had changed —
                // 43.4ms of a core, every tick, for an answer a hash
                // already had (ci/baseline/refs-join-windows-x64.md).
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
                let seen = self
                    .join_key
                    .lock()
                    .map(|mut slot| slot.replace(inputs))
                    .unwrap_or_default();
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
                snapshot.remote_names = remotes.into_iter().map(|r| r.name).collect();
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

    /// Loads status + op state and publishes them, returning whether
    /// working-tree dirtiness flipped.
    ///
    /// Does not rebuild the graph itself: after a write the caller knows
    /// whether it needs one anyway, and rebuilding on both counts would do
    /// it twice.
    pub(super) async fn publish_status(self: &Arc<Self>) -> bool {
        let Some(workdir) = self.workdir() else {
            return false;
        };
        let op_gen = self.status_gate.begin();
        let cancel = self.root_cancel.clone();
        let status = status::load(&self.executor, &workdir, &cancel).await;
        let op = opstate::detect(&self.executor, &workdir, &cancel).await;
        match (status, op) {
            (Ok(status), Ok(op_state)) => {
                // Only a stepping rebase has a counter to read, so the
                // common refresh costs nothing extra.
                let progress = if op_state.rebasing {
                    conflict::rebase_progress(&self.executor, &workdir, &cancel)
                        .await
                        .unwrap_or_default()
                } else {
                    None
                };
                // Likewise: the two sides only have names while something
                // is stopped, which is the rare case. Nothing stopped
                // means nothing read.
                let sides = match integrate::InProgress::from_state(&op_state) {
                    Some(op) => conflict::sides(&self.executor, &workdir, op, &cancel)
                        .await
                        .unwrap_or_default(),
                    None => conflict::Sides::default(),
                };
                // And again: the tool is only worth naming where there is
                // something to open with it, so a clean tree pays nothing
                // — unless the settings field asked, which it does once
                // per opening rather than once per poll.
                let asked = self.merge_tool_wanted.swap(false, Ordering::SeqCst);
                let merge_tool = if asked || status.conflicted().next().is_some() {
                    let read = conflict::configured_tool(&self.executor, &workdir, &cancel)
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or_default();
                    self.set_merge_tool_seen(read.clone());
                    read
                } else {
                    // Not read this time, so repeat the last answer rather
                    // than replace it with nothing: a settings dialog left
                    // open would otherwise watch its value evaporate on
                    // the next tick, and a conflict resolved by the tool
                    // takes the name out of the pane it was just used in.
                    self.merge_tool_seen()
                };
                if !self.status_gate.is_current(op_gen) {
                    return false;
                }
                let dirty = status.is_dirty();
                let flipped = self.wip_dirty.swap(dirty, Ordering::SeqCst) != dirty;
                // Reading the pending diffs is the one part of this that
                // scales with the change rather than with the tree, so it
                // does not run on every tick — only where the answer can
                // have moved. **What status reports is the test**, not
                // whether the tree turned dirty: the index cannot change
                // without status changing, including when it is another
                // git outside this window that changes it, and the index
                // is what a commit carries. A tick that reads the same
                // status reads no diffs.
                let stale = self.eol_marks_stale.swap(false, Ordering::SeqCst);
                let key = status_key(&status);
                let moved = self
                    .status_key
                    .lock()
                    .map(|mut slot| slot.replace(key) != Some(key))
                    .unwrap_or(true);
                let eol_marks = if stale || moved {
                    self.settle_eol_marks(&workdir, &status, &cancel).await
                } else {
                    self.eol_marks()
                };
                self.sink.event(SessionEvent::StatusLoaded {
                    status,
                    op_state,
                    progress,
                    sides,
                    merge_tool,
                    eol_marks,
                });
                flipped
            }
            (Err(e), _) | (_, Err(e)) => {
                self.fail("status", e);
                false
            }
        }
    }

    /// Re-reads the author identity and signing configuration.
    pub fn refresh_author(self: &Arc<Self>) {
        self.spawn_read("identity", |s, workdir, cancel| async move {
            let config = identity::load(&s.executor, &workdir, &cancel).await?;
            Ok(SessionEvent::AuthorLoaded { config })
        });
    }

    /// Records `user.name` / `user.email`.
    pub fn set_identity(
        self: &Arc<Self>,
        name: String,
        email: String,
        scope: identity::ConfigScope,
    ) {
        self.write(
            "identity",
            AfterWrite::Author,
            move |exec, repo, cancel| async move {
                let written =
                    identity::set_identity(&exec, &repo.workdir, &name, &email, scope, &cancel)
                        .await?;
                if written.is_saved() {
                    return Ok(());
                }
                // Half of an identity reads as a whole one everywhere it
                // is used, so a write that did not take is reported as a
                // failure even when git raised nothing against it.
                Err(GitError::Rejected {
                    message: if written.message.is_empty() {
                        "git still reports a different identity".to_string()
                    } else {
                        written.message
                    },
                })
            },
        );
    }

    /// Reads HEAD's message and author so an amend can start from them.
    ///
    /// On demand rather than with every refresh: only the amend path wants
    /// them, and a repository refresh already runs several commands.
    pub fn load_head_commit(self: &Arc<Self>) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            // An unborn branch has no HEAD to amend; that is a state, not a
            // failure worth an error banner.
            let head = commit::head_commit(&s.executor, &workdir, &cancel)
                .await
                .unwrap_or_default();
            s.sink.event(SessionEvent::HeadCommitLoaded { head });
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
