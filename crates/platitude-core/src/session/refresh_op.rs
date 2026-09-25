//! The stopped operation, read: what a merge is bringing in, why a
//! rebase stopped, how far a stepped operation has got, and the two
//! sides it is between.
//!
//! Read on the status pass ([`super::refresh`]); kept apart because its
//! extra reads are paid only while something is stopped.

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
    /// Only file reads — no process, no lock, no snapshot
    /// ([`opstate::detect_at`] / [`integrate::rebase_progress`]) — so it
    /// can be ticked fast enough to count a replay's steps out, which the
    /// `git status` poll cannot (`ci/baseline/poll-cost-windows-x64.md`).
    ///
    /// Open to every caller: a repository with nothing standing answers
    /// exactly that.
    pub fn refresh_op_progress(self: &Arc<Self>) {
        let Some(git_dir) = self.git_dir() else {
            return;
        };
        let looked = self.standing.stamp();
        self.sink.event(SessionEvent::OpProgress {
            op_state: opstate::detect_at(&git_dir),
            progress: integrate::rebase_progress(&git_dir),
            looked,
        });
    }

    /// What the badge and the exit card read of a standing rebase — the
    /// counter and the stop — or the resting pair where none is standing.
    ///
    /// A read that could not tell keeps the stop it had: a tick answering
    /// `editing: false` mid-`edit` stop would hand the exit card's `--skip`
    /// back its plain click, which cannot be taken back (`Standing`). The
    /// counter is not kept — a stale N/M reads as progress that happened.
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

    /// Read only while something is stopped; the message and the parents
    /// only for a stopped merge, the one operation finished from the
    /// commit box.
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

    /// Loads status + op state and publishes them. Answers whether the WIP
    /// row moved — the tree turned dirty or clean, or a standing merge
    /// changed what it brings in — or that nothing was published at all.
    ///
    /// The rebuild is the caller's: after a write it knows whether one is
    /// needed anyway, and rebuilding on both counts would do it twice.
    pub(super) async fn publish_status(self: &Arc<Self>) -> Reread {
        let Some(workdir) = self.workdir() else {
            return Reread::Failed;
        };
        // Stamped before git is spawned, as in the refs read (`Standing`).
        let looked = self.standing.stamp();
        let cancel = self.root_cancel.clone();
        let status = status::load(&self.executor, &workdir, &cancel).await;
        let op = opstate::detect(&self.executor, &workdir, &cancel).await;
        match (status, op) {
            (Ok(status), Ok(op_state)) => {
                // Fenced by a write that ended after this looked: read
                // again, since the write re-reads the tree only where it
                // moved refs (a fetch that brought nothing would leave this
                // to the next tick).
                if !self.standing.current(looked) {
                    self.read_status_from(self.status_read.stamp());
                    return Reread::Same;
                }
                // Counter and stop ride one spawn: this runs every tick for
                // the life of a stop (`integrate::rebase_standing`).
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
                // Named only where there is a conflict to open, or the
                // settings field asked (once per opening).
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
                    // Repeat the last answer, or a settings dialog left
                    // open would watch its value vanish on the next tick.
                    self.standing.merge_tool()
                };
                // Where a push goes, read every tick so a mark moved from a
                // terminal shows on the next one; a detached HEAD has
                // nothing to push. The branch's mark rides this event, the
                // repository's the refs snapshot, so a moved default sends
                // the refs out again ([`RepoSession::note_push_default`]).
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
                    return Reread::Same;
                }
                // Into the HEAD record before anything below is sent, so the
                // counts never arrive ahead of their branch; `head_seq`
                // names the report they belong beside.
                self.observe_head(looked, &status.head());
                let head_seq = self.standing.head_seq();
                // The reader showing this status asks the same function, so
                // what the rows hold and what the window expects agree.
                let dirty = crate::graph::wip_row_stands(&status, &op_state);
                // Both halves are recorded whatever the other says: a `||`
                // would skip the second and leave its baseline behind.
                let dirt_flipped = self.standing.set_wip_dirty(dirty);
                // A read that could not tell keeps the sides it had; taking
                // them away would rebuild the graph without its dotted edges
                // for one tick.
                let merge_moved =
                    incoming.is_some_and(|sides| self.standing.set_merge_incoming(sides));
                let flipped = dirt_flipped || merge_moved;
                // The pending diffs scale with the change, so they are read
                // only when status moved (the index cannot change without
                // status changing, whoever changes it) or were marked stale.
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
                    looked,
                    op_state,
                    progress,
                    sides,
                    op_message,
                    merge_tool,
                    push_remote,
                    eol_marks,
                    stop,
                });
                if flipped { Reread::Moved } else { Reread::Same }
            }
            (Err(e), _) | (_, Err(e)) => {
                self.fail(FollowUp::Status.label(), e);
                Reread::Failed
            }
        }
    }
}
