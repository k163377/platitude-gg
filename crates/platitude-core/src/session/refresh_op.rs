//! The status pass ([`RepoSession::publish_status`]) and what it reads
//! beside `git status`: the stopped operation — what a merge is bringing
//! in, why a rebase stopped, how far a stepped operation has got, and the
//! two sides it is between — and, where a mark sends the push away from
//! the upstream, the branch's counts against that destination.
//!
//! The stopped operation's reads are paid only while something is
//! stopped, the push counts only while the push goes elsewhere.

use super::joins::status_key;
use super::*;

/// What a standing operation adds to a status
/// ([`RepoSession::read_standing_op`]).
struct StandingOp {
    /// How far a standing rebase has got, and why it stopped — default
    /// where none stands.
    progress: Option<conflict::Progress>,
    stop: integrate::RebaseStop,
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

    /// Read only while something is stopped; the count and the stop only
    /// for a rebase, the message and the parents only for a stopped merge,
    /// the one operation finished from the commit box. Names cost a
    /// `name-rev`; the rest are file reads under the copy's own git
    /// directory (`RepoInfo::git_dir`).
    async fn read_standing_op(
        &self,
        paths: (&std::path::Path, &std::path::Path),
        status: &WorkingTreeStatus,
        op_state: &OpState,
        cancel: &CancellationToken,
    ) -> StandingOp {
        let (workdir, git_dir) = paths;
        let (progress, stop) = if op_state.rebasing {
            integrate::rebase_standing(git_dir)
        } else {
            Default::default()
        };
        let sides = match integrate::InProgress::from_state(op_state) {
            Some(op) => {
                conflict::sides(
                    &self.executor,
                    workdir,
                    git_dir,
                    op,
                    status.branch_head.as_deref(),
                    cancel,
                )
                .await
            }
            None => conflict::Sides::default(),
        };
        let op_message = if op_state.merging {
            integrate::stopped_message(git_dir)
        } else {
            String::new()
        };
        let incoming = if op_state.merging {
            opstate::merge_heads(git_dir)
        } else {
            Some(Vec::new())
        };
        StandingOp {
            progress,
            stop,
            sides,
            op_message,
            incoming,
        }
    }

    /// The branch's counts against where a mark sends its push, read only
    /// where that is not the upstream's remote ([`Self::push_goes_elsewhere`]):
    /// a process every tick, and nothing else reads them. A read that could
    /// not tell repeats the branch's last answer (`Standing`).
    async fn read_push_track(
        &self,
        workdir: &std::path::Path,
        branch: &str,
        status: &WorkingTreeStatus,
        marks: &remote::PushMarks,
        cancel: &CancellationToken,
    ) -> remote::PushTrack {
        let told = if self.push_goes_elsewhere(status, marks) {
            remote::push_track(&self.executor, workdir, branch, cancel).await
        } else {
            Ok(remote::PushTrack::default())
        };
        match told {
            Ok(track) => {
                self.standing.set_push_track(branch, track.clone());
                track
            }
            Err(_) => self.standing.push_track_of(branch),
        }
    }

    /// [`remote::pushes_elsewhere`] for a status whose upstream git can
    /// compare against — with none the push asks where it goes, and no
    /// count is read. The remote names are the kept ones, not read for
    /// this; before the first listing the cut falls back to the mark's
    /// exact prefix.
    fn push_goes_elsewhere(&self, status: &WorkingTreeStatus, marks: &remote::PushMarks) -> bool {
        let Some(upstream) = status
            .upstream
            .as_deref()
            .filter(|_| status.upstream_tracked)
        else {
            return false;
        };
        let push_remote = marks.push_remote.as_deref().unwrap_or_default();
        let push_default = marks
            .push_default
            .as_ref()
            .map_or("", |marked| marked.remote.as_str());
        let elsewhere = |names: &[remote::Remote]| {
            remote::pushes_elsewhere(
                upstream,
                push_remote,
                push_default,
                names.iter().map(|remote| remote.name.as_str()),
            )
        };
        self.remotes
            .peek(|remotes| elsewhere(&remotes.list))
            .unwrap_or_else(|| elsewhere(&[]))
    }

    /// Loads status + op state and publishes them. Answers whether the WIP
    /// row moved — the tree turned dirty or clean, or a standing merge
    /// changed what it brings in — or that nothing was published at all.
    ///
    /// The rebuild is the caller's: after a write it knows whether one is
    /// needed anyway, and rebuilding on both counts would do it twice.
    pub(super) async fn publish_status(self: &Arc<Self>) -> Reread {
        // Both paths from one reading of the record: the operation's
        // markers are the working copy's own, under its git directory.
        let Some(info) = self.repo_info() else {
            return Reread::Failed;
        };
        let (workdir, git_dir) = (info.workdir, info.git_dir);
        // Stamped before git is spawned, as in the refs read (`Standing`).
        let looked = self.standing.stamp();
        let cancel = self.root_cancel.clone();
        let status = status::load(&self.executor, &workdir, &cancel).await;
        match status {
            Ok(status) => {
                let op_state = opstate::detect_at(&git_dir);
                // Fenced by a write that ended after this looked: read
                // again, since the write re-reads the tree only where it
                // moved refs (a fetch that brought nothing would leave this
                // to the next tick).
                if !self.standing.current(looked) {
                    self.read_status_from(self.status_read.stamp());
                    return Reread::Same;
                }
                let StandingOp {
                    progress,
                    stop,
                    sides,
                    op_message,
                    incoming,
                } = self
                    .read_standing_op((&workdir, &git_dir), &status, &op_state, &cancel)
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
                let (push_remote, push_track) = match &status.branch_head {
                    Some(branch) => {
                        match remote::push_marks(&self.executor, &workdir, branch, &cancel).await {
                            Ok(marks) => {
                                self.note_push_default(&marks.push_default);
                                let track = self
                                    .read_push_track(&workdir, branch, &status, &marks, &cancel)
                                    .await;
                                (marks.push_remote.unwrap_or_default(), track)
                            }
                            Err(_) => Default::default(),
                        }
                    }
                    None => Default::default(),
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
                let (eol_marks, lfs) = tokio::join!(
                    async {
                        if stale || moved {
                            self.settle_eol_marks(&workdir, &status, &cancel).await
                        } else {
                            self.eol_marks()
                        }
                    },
                    self.lfs_needs(&workdir, &status, stale || moved, &cancel),
                );
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
                    push_track,
                    eol_marks,
                    lfs_needed: lfs.needed,
                    not_copied: lfs.not_copied,
                    stop,
                });
                if flipped { Reread::Moved } else { Reread::Same }
            }
            Err(e) => {
                self.fail(FollowUp::Status.label(), e);
                Reread::Failed
            }
        }
    }
}
