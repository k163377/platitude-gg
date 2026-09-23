//! One section's feeds, drained into the rows and properties QML reads.
//!
//! Six feeds land here, and only the one this section was attached to
//! ever holds anything — a list is wired to exactly one of them
//! (`attach::attach_section_feed` / `attach::attach_worktree_feed` /
//! `attach::attach_carried_feed`).

use super::*;

impl NavSectionModel {
    pub(super) fn take_feeds(&mut self) {
        // Every push queues its own `drain`, so a second call can find the
        // queue already emptied by the first. Nothing arrived means
        // nothing to rebuild.
        let mut arrived = false;
        // Whether refs were published at all, which is a different
        // question from whether they moved (see `refs_settled`).
        let mut settled = false;
        let mut moved = false;
        if let Some(feed) = self.refs_feed.clone() {
            let mut head_moved = false;
            // In the order the session said them: a snapshot and the
            // report of where HEAD stands can land in one drain, and
            // the row the stand-in follows is worked out once both are
            // in.
            for msg in feed.drain() {
                match msg {
                    RefsMsg::Snapshot { snapshot, looked } => {
                        settled = true;
                        // **Said at the moment this list applies it**,
                        // and said with the stamp of the read: whoever
                        // is holding rows off the screen for a write has
                        // to tell a listing that saw what the write left
                        // from one that was already in flight when it
                        // ended, and the two reach here down separate
                        // feeds in no fixed order
                        // (`ops::StandIn`).
                        crate::hub::listing_applied(self.tab_id, &self.section, looked);
                        // The first snapshot is news whatever it holds:
                        // the default selection is waiting on
                        // `refsLoaded`, and a section that is legitimately
                        // empty would otherwise never say so.
                        arrived |= !self.refs_loaded;
                        self.refs_loaded = true;
                        let fresh = !self
                            .last_refs
                            .as_ref()
                            .is_some_and(|last| Arc::ptr_eq(last, &snapshot));
                        if fresh {
                            moved = true;
                            self.last_refs = Some(Arc::clone(&snapshot));
                            arrived |= self.take(match self.section.as_str() {
                                "branches" => Source::Locals(snapshot),
                                "remotes" => Source::Remotes(snapshot),
                                _ => Source::Tags(snapshot),
                            });
                        }
                    }
                    // Only the branches section is handed this
                    // (`hub::sink`): the row it highlights and
                    // the stand-in above it are the
                    // record's.
                    RefsMsg::Head(head) => {
                        if self.head_name != head.branch || self.head_oid != head.oid_hex {
                            self.head_name = head.branch;
                            self.head_oid = head.oid_hex;
                            head_moved = true;
                        }
                    }
                }
            }
            if head_moved || moved {
                // A HEAD that moved is another row highlighted, which is
                // an arrangement of its own; the marks the stand-in wears
                // follow either side moving.
                arrived |= self.settle_head_marks() || head_moved;
            }
        }
        if let Some(feed) = self.status_feed.clone()
            && let Some(StatusMsg {
                status, eol_marks, ..
            }) = feed.drain().pop()
        {
            // The marks are the other half of what a file row shows, and
            // they can move on their own — a line-ending answer arrives
            // after the status it is about.
            let marked = self.eol_marks != eol_marks;
            self.eol_marks = eol_marks;
            arrived |= self.take(Source::files(status)) || marked;
        }
        if let Some(feed) = self.carried_feed.clone()
            && let Some(CarriedStatusMsg { at, name, status }) = feed.drain().pop()
        {
            // No line-ending marks come with these. They are about what
            // the next `git add` in this window would record, and this
            // window adds nothing over there (規約 §行末の改行コード).
            let moved = self.carried_at != at || self.carried_name != name;
            self.carried_at = at;
            self.carried_name = name;
            arrived |= self.take(Source::whole_files(status)) || moved;
        }
        if let Some(feed) = self.worktrees_feed.clone()
            && let Some(list) = feed.drain().pop()
        {
            let current = crate::hub::from_session(self.tab_id, |s| s.workdir())
                .flatten()
                .map(|p| p.to_string_lossy().replace('\\', "/").to_lowercase())
                .unwrap_or_default();
            // A bare entry has no working copy to show.
            let list = list.into_iter().filter(|w| !w.bare).collect();
            arrived |= self.take(Source::Worktrees { list, current });
        }
        let mut stashes_arrived = false;
        if let Some(feed) = self.stash_feed.clone()
            && let Some(list) = feed.drain().pop()
        {
            stashes_arrived = true;
            // The same, for the listing a dropped row waits on — its own
            // read, with its own stamp (see the refs above).
            crate::hub::listing_applied(self.tab_id, &self.section, list.looked);
            arrived |= self.take(Source::Stashes(list.entries));
        }
        if arrived {
            // `total` is settled by the arrange below, which is the one
            // place that knows how many rows are being shown as gone.
            self.reshape();
            self.changed();
        }
        if settled {
            self.refs_settled();
        }
        if moved {
            self.refs_moved();
        }
        if stashes_arrived {
            self.stashes_settled();
        }
        if crate::harness::memprobe::enabled() {
            self.note_footprint();
        }
    }

    /// What the current branch's own row wears — the marks and the
    /// counts, read off the snapshot by name so the stand-in draws what
    /// the row it stands for draws. Nothing while the branch is not in
    /// the snapshot yet (a HEAD reported ahead of the first listing, or a
    /// branch made since it). Answers whether any of them moved.
    ///
    /// **The counts are read here** (`view::arrange`, where
    /// `head_depth` comes from): a filter or a folded folder leaves the
    /// branch with no row at all, and that is exactly when the stand-in
    /// takes a seat of its own (`HeadPinRow.seated`) and still has its
    /// pair to draw.
    pub(super) fn settle_head_marks(&mut self) -> bool {
        let branch = if self.head_name.is_empty() {
            None
        } else {
            self.all.branch_named(&self.head_name)
        };
        let has_remote = branch.is_some_and(|b| b.has_remote);
        let has_pr = branch.is_some() && crate::encode::pr_set().contains(self.head_name.as_str());
        let ahead = branch.map_or(0, |b| field::counted(b.ahead));
        let behind = branch.map_or(0, |b| field::counted(b.behind));
        let gone = branch.map_or("", |b| b.upstream_gone.as_str());
        let changed = (has_remote, has_pr, ahead, behind, gone)
            != (
                self.head_has_remote,
                self.head_has_pr,
                self.head_ahead,
                self.head_behind,
                self.head_upstream_gone.as_str(),
            );
        self.head_has_remote = has_remote;
        self.head_has_pr = has_pr;
        self.head_ahead = ahead;
        self.head_behind = behind;
        self.head_upstream_gone = gone.to_string();
        changed
    }
}
