//! The stopped operation, read: what a merge is bringing in, why a
//! rebase stopped, how far a stepped operation has got, and the two
//! sides it is between.
//!
//! Refreshed on the same pass as the status it is read beside
//! ([`super::refresh`]), and told apart from it here because what it
//! costs is the several extra reads a stopped operation needs and an
//! untouched repository does not.

use super::joins::status_key;
use super::*;

/// What a standing operation adds to a status
/// ([`RepoSession::read_standing_op`]).
struct StandingOp {
    sides: conflict::Sides,
    /// The message a stopped merge is about to record; empty unless one
    /// is standing.
    op_message: String,
    /// The sides the pending merge commit will have as parents. `None` is
    /// "could not tell"; only nothing merging is an answer of no sides.
    incoming: Option<Vec<Oid>>,
}

impl RepoSession {
    /// Publishes what operation is standing and how far it has got, and
    /// nothing else.
    ///
    /// **The whole of it is a handful of file reads** — no process, no
    /// lock, no snapshot ([`opstate::detect_at`] /
    /// [`integrate::rebase_progress`]) — which is what lets the screen
    /// count the steps out rather than sample them: a replay moves the
    /// number about every eleven milliseconds, and the periodic re-read
    /// around it is ten seconds apart because it carries a whole `git
    /// status` (`ci/baseline/poll-cost-windows-x64.md`). Asking that one
    /// faster would have the reads competing with the replay they are
    /// about; asking this one faster costs nothing measurable.
    ///
    /// Not gated on anything: the caller ticks it while it knows a write
    /// that replays is out, and a repository with nothing standing
    /// answers exactly that.
    pub fn refresh_op_progress(self: &Arc<Self>) {
        let Some(git_dir) = self.git_dir() else {
            return;
        };
        self.sink.event(SessionEvent::OpProgress {
            op_state: opstate::detect_at(&git_dir),
            progress: integrate::rebase_progress(&git_dir),
        });
    }

    /// Loads status + op state and publishes them, returning whether the
    /// synthetic WIP row moved: the working tree turned dirty or clean, or
    /// a standing merge changed what it is bringing in.
    ///
    /// What the badge and the exit card read of a standing rebase — the
    /// counter and the stop — or the resting pair where none is standing.
    ///
    /// A read that could not tell keeps the stop it had, the way the
    /// merge's sides and the merge tool do: the tick that answered
    /// `editing: false` in the middle of an `edit` stop would hand the
    /// exit card's `--skip` back its plain click, and that click is not
    /// one the reader gets to take back (`Standing`). The counter is not
    /// held the same way — a stale N/M would be read as progress that
    /// happened, and the badge losing it for one tick costs nothing.
    async fn rebase_standing_held(
        &self,
        workdir: &std::path::Path,
        rebasing: bool,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> (Option<conflict::Progress>, integrate::RebaseStop) {
        if !rebasing {
            self.standing
                .set_rebase_stop(integrate::RebaseStop::default());
            return (None, integrate::RebaseStop::default());
        }
        match integrate::rebase_standing(&self.executor, workdir, cancel).await {
            Ok((progress, stop)) => {
                self.standing.set_rebase_stop(stop.clone());
                (progress, stop)
            }
            Err(_) => (None, self.standing.rebase_stop()),
        }
    }

    /// What a standing operation adds to a status. All three are the rare
    /// case, read only while something is stopped: the two sides only
    /// have names then, and only a stopped merge — the one operation
    /// finished from the commit box — has a message waiting to go in it
    /// and sides the pending commit will have as parents.
    async fn read_standing_op(
        &self,
        workdir: &std::path::Path,
        status: &WorkTreeStatus,
        op_state: &OpState,
        cancel: &CancellationToken,
    ) -> StandingOp {
        let sides = match integrate::InProgress::from_state(op_state) {
            Some(op) => conflict::sides(
                &self.executor,
                workdir,
                op,
                status.branch_head.as_deref(),
                cancel,
            )
            .await
            .unwrap_or_default(),
            None => conflict::Sides::default(),
        };
        let op_message = if op_state.merging {
            integrate::stopped_message(&self.executor, workdir, cancel).await
        } else {
            String::new()
        };
        let incoming = if op_state.merging {
            opstate::merge_heads(&self.executor, workdir, cancel).await
        } else {
            Some(Vec::new())
        };
        StandingOp {
            sides,
            op_message,
            incoming,
        }
    }

    /// Does not rebuild the graph itself: after a write the caller knows
    /// whether it needs one anyway, and rebuilding on both counts would do
    /// it twice.
    pub(super) async fn publish_status(self: &Arc<Self>) -> bool {
        let Some(workdir) = self.workdir() else {
            return false;
        };
        // Stamped before git is spawned, the way the refs read is: what
        // this status saw of HEAD is offered to the one record under it
        // (`Standing`).
        let looked = self.standing.stamp();
        let cancel = self.root_cancel.clone();
        let status = status::load(&self.executor, &workdir, &cancel).await;
        let op = opstate::detect(&self.executor, &workdir, &cancel).await;
        match (status, op) {
            (Ok(status), Ok(op_state)) => {
                // A read that looked before a write ended has nothing to
                // say for the repository after it — and nothing more to
                // spend on it either. Read again rather than lost: the
                // write behind it re-reads the tree only where it moved
                // the refs, so a status a fetch that brought nothing
                // fenced would otherwise wait for the next tick.
                if !self.standing.current(looked) {
                    self.read_status_from(self.status_read.stamp());
                    return false;
                }
                // Only a standing rebase has a counter to read or a stop
                // to explain, and the two ride one spawn — this runs every
                // tick for the life of a stop (`integrate::rebase_standing`).
                let (progress, stop) = self
                    .rebase_standing_held(&workdir, op_state.rebasing, &cancel)
                    .await;
                let StandingOp {
                    sides,
                    op_message,
                    incoming,
                } = self
                    .read_standing_op(&workdir, &status, &op_state, &cancel)
                    .await;
                // The tool is only worth naming where there is
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
                    self.standing.set_merge_tool(read.clone());
                    read
                } else {
                    // Not read this time, so repeat the last answer rather
                    // than replace it with nothing: a settings dialog left
                    // open would otherwise watch its value evaporate on
                    // the next tick, and a conflict resolved by the tool
                    // takes the name out of the pane it was just used in.
                    self.standing.merge_tool()
                };
                // Where the marks send a push — the branch's own, with the
                // repository's riding the same read. One short local `git
                // config` per tick, and only where there is a branch to
                // ask about — a detached HEAD marks nothing and has
                // nothing to push. Not gated on anything else: the
                // toolbar names its destination by these, so a mark moved
                // from a terminal has to turn up on the following tick
                // rather than at the next thing that happens to
                // invalidate a cache. The branch's half rides this status
                // event; the repository's is answered by the refs
                // snapshot, so one that moved sends the refs out to say
                // it again ([`RepoSession::note_push_default`]).
                let push_remote = match &status.branch_head {
                    Some(branch) => {
                        match remote::push_marks(&self.executor, &workdir, branch, &cancel).await {
                            Ok(marks) => {
                                self.note_push_default(&marks.push_default);
                                marks.push_remote.unwrap_or_default()
                            }
                            Err(_) => String::new(),
                        }
                    }
                    None => String::new(),
                };
                if !self.standing.current(looked) {
                    self.read_status_from(self.status_read.stamp());
                    return false;
                }
                // What this status saw of HEAD, into the one record every
                // consumer reads it from — before anything below is sent,
                // so the counts never arrive ahead of the branch they are
                // about. The number the record then stands at names the
                // report these counts belong beside.
                self.observe_head(looked, &status.head());
                let head_seq = self.standing.head_seq();
                // Whether the working-tree row stands, asked of the one
                // place that rule is written (`graph::wip_row_stands`):
                // the reader showing this status asks the same question of
                // the same function, so what the rows hold and what the
                // window expects them to hold cannot drift apart.
                let dirty = crate::graph::wip_row_stands(&status, &op_state);
                // Both halves are recorded whatever the other says: they
                // are what the next read compares against, and a `||` that
                // skipped the second would leave it behind.
                let dirt_flipped = self.standing.set_wip_dirty(dirty);
                // A read that could not tell keeps the sides it had: taking
                // them away would say the merge ended, and the graph would
                // be rebuilt without its dotted edges only to be rebuilt
                // again with them on the next tick.
                let merge_moved =
                    incoming.is_some_and(|sides| self.standing.set_merge_incoming(sides));
                let flipped = dirt_flipped || merge_moved;
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
                let moved = relock(&self.status_key).replace(key) != Some(key);
                let eol_marks = if stale || moved {
                    self.settle_eol_marks(&workdir, &status, &cancel).await
                } else {
                    self.eol_marks()
                };
                self.sink.event(SessionEvent::StatusLoaded {
                    status,
                    head_seq,
                    op_state,
                    progress,
                    sides,
                    op_message,
                    merge_tool,
                    push_remote,
                    eol_marks,
                    stop,
                });
                flipped
            }
            (Err(e), _) | (_, Err(e)) => {
                self.fail("status", e);
                false
            }
        }
    }
}
