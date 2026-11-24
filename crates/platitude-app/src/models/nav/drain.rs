//! One section's feeds, drained into the rows and properties QML reads.
//! A list is wired to exactly one of them (`attach`); the rest stay empty.

use super::*;

impl NavSectionModel {
    pub(super) fn drain(&mut self) {
        // Every push queues its own `drain`, so a call can find the queue
        // already emptied by the one before.
        let mut arrived = false;
        // Refs published at all, as against moved (`refs_settled`).
        let mut settled = false;
        let mut moved = false;
        if let Some(feed) = self.refs_feed.clone() {
            let mut head_moved = false;
            // A snapshot and the HEAD report can land in one drain; the
            // stand-in's marks are settled once both are in.
            for msg in feed.drain() {
                match msg {
                    RefsMsg::Snapshot { snapshot, looked } => {
                        settled = true;
                        // Said as this list applies it, with the read's
                        // stamp: a listing that saw a write's result has
                        // to be told from one already in flight, and the
                        // two arrive on separate feeds in no fixed order
                        // (`ops::StandIn`).
                        crate::hub::listing_applied(self.tab_id, &self.section, looked);
                        self.looked = looked;
                        // The first snapshot is news even when empty: the
                        // default selection waits on `refsLoaded`.
                        arrived |= !self.refs_loaded;
                        self.refs_loaded = true;
                        let fresh = !self
                            .last_refs
                            .as_ref()
                            .is_some_and(|last| Arc::ptr_eq(last, &snapshot));
                        if fresh {
                            moved = true;
                            self.refs_revision = self.refs_revision.wrapping_add(1);
                            self.last_refs = Some(Arc::clone(&snapshot));
                            arrived |= self.take(match self.section.as_str() {
                                "branches" => Source::Locals(snapshot),
                                "remotes" => Source::Remotes(snapshot),
                                _ => Source::Tags(snapshot),
                            });
                        }
                    }
                    // Only the branches section is handed this
                    // (`hub::sink`).
                    RefsMsg::Head(head) => {
                        let oid = crate::encode::oid_text(head.oid);
                        if self.head_name != head.branch || self.head_oid != oid {
                            self.head_name = head.branch;
                            self.head_oid = oid;
                            head_moved = true;
                        }
                    }
                }
            }
            if head_moved || moved {
                // A moved HEAD highlights another row: a rearrangement
                // of its own.
                arrived |= self.settle_head_marks() || head_moved;
            }
        }
        arrived |= self.take_status_feed();
        if let Some(feed) = self.carried_feed.clone()
            && let Some(CarriedStatusMsg { at, name, status }) = feed.drain().pop()
        {
            // No line-ending marks: they are about what this window's
            // next `git add` would record, and it adds nothing over there
            // (デザイン規約 §改行コードの警告).
            let moved = self.carried_at != at || self.carried_name != name;
            self.carried_at = at;
            self.carried_name = name;
            arrived |= self.take(Source::whole_files(status)) || moved;
        }
        let mut worktrees_arrived = false;
        if let Some(feed) = self.worktrees_feed.clone()
            && let Some(list) = feed.drain().pop()
        {
            let current = crate::hub::from_session(self.tab_id, |s| s.workdir())
                .flatten()
                .map(|p| platitude_core::session::same_path_key(&p.to_string_lossy()))
                .unwrap_or_default();
            worktrees_arrived = true;
            // The same stamp, for the listing a removed worktree waits on.
            crate::hub::listing_applied(self.tab_id, &self.section, list.looked);
            self.looked = list.looked;
            // A bare entry has no working tree to show.
            let list: Vec<_> = list.entries.into_iter().filter(|w| !w.bare).collect();
            arrived |= self.settle_home_branch(&list, &current);
            arrived |= self.take(Source::Worktrees { list, current });
        }
        let mut stashes_arrived = false;
        if let Some(feed) = self.stash_feed.clone()
            && let Some(list) = feed.drain().pop()
        {
            stashes_arrived = true;
            // The same stamp, for the listing a dropped stash waits on
            // (see the refs above).
            crate::hub::listing_applied(self.tab_id, &self.section, list.looked);
            self.looked = list.looked;
            arrived |= self.take(Source::Stashes(list.entries));
        }
        if arrived {
            self.show_arrival();
        }
        if settled {
            self.refs_settled();
        }
        if moved {
            self.refs_moved();
        }
        if worktrees_arrived {
            self.worktrees_settled();
        }
        if stashes_arrived {
            self.stashes_settled();
        }
        if crate::harness::memprobe::enabled() {
            self.note_footprint();
        }
    }

    /// The newest status of this window's tree, for a bucket list: noted as
    /// this reader's drawing (`ops::StageAhead::drawn`) and put on screen.
    /// Answers whether the rows or their marks moved.
    fn take_status_feed(&mut self) -> bool {
        let Some(feed) = self.status_feed.clone() else {
            return false;
        };
        let Some(StatusMsg {
            status,
            eol_marks,
            looked,
            ..
        }) = feed.drain().pop()
        else {
            return false;
        };
        self.looked = looked;
        if let Some(reader) = crate::ops::Reader::of_run(&self.run) {
            crate::hub::status_drawn(self.tab_id, reader, looked);
        }
        self.draw_status(status, eol_marks)
    }

    /// Puts `base` — a status as git reported it — on screen with the stage
    /// presses still out over it (`ops::StageAhead`), and answers whether
    /// the rows or their line-ending marks moved. The marks can move on their
    /// own: a line-ending answer arrives after the status it is about.
    fn draw_status(
        &mut self,
        base: Arc<platitude_core::status::WorktreeStatus>,
        marks: Arc<Vec<platitude_core::session::EolMark>>,
    ) -> bool {
        match crate::hub::status_picture(self.tab_id, self.looked, &base, &marks) {
            Some(picture) => {
                self.staged_from = Some(StatusBase {
                    status: base,
                    marks,
                });
                self.show_marks(Arc::new(picture.marks)) | self.take(Source::files(picture.status))
            }
            None => {
                self.staged_from = None;
                self.show_marks(marks) | self.take(Source::files(base))
            }
        }
    }

    /// Draws the status this list holds again with the stage presses as they
    /// stand now — a press moved rows, or an answer put them back
    /// (`restage`). Answers whether the rows moved; nothing for a list
    /// holding no status of this window's tree.
    pub(super) fn restage_files(&mut self) -> bool {
        if crate::ops::Reader::of_run(&self.run).is_none() {
            return false;
        }
        if let Some(base) = self.staged_from.take() {
            return self.draw_status(base.status, base.marks);
        }
        // The rows on screen are the status itself, so that is the one to
        // draw the presses over; it moves aside only where they move a row.
        let Source::Files { status, .. } = &self.all else {
            return false;
        };
        let Some(picture) =
            crate::hub::status_picture(self.tab_id, self.looked, status, &self.eol_marks)
        else {
            return false;
        };
        let Source::Files { status, .. } = std::mem::take(&mut self.all) else {
            return false;
        };
        self.staged_from = Some(StatusBase {
            status,
            marks: Arc::clone(&self.eol_marks),
        });
        self.show_marks(Arc::new(picture.marks));
        self.take(Source::files(picture.status));
        true
    }

    /// Answers whether the marks moved.
    fn show_marks(&mut self, marks: Arc<Vec<platitude_core::session::EolMark>>) -> bool {
        let marked = self.eol_marks != marks;
        self.eol_marks = marks;
        marked
    }

    /// Lays out the rows a read brought, its moves said apart from the
    /// reader's (`rows_arriving` … `rows_arrived`). `total` is settled by
    /// the arrange — the one place that knows how many rows are shown as
    /// gone.
    pub(super) fn show_arrival(&mut self) {
        self.rows_arriving();
        self.reshape();
        self.changed();
        self.rows_arrived();
    }

    /// What the current branch's own row wears — the marks and the counts,
    /// read off the snapshot by name so the stand-in draws what its row
    /// draws. Nothing while the branch is not in the snapshot yet (a HEAD
    /// reported ahead of the first listing, or a branch made since).
    /// Answers whether any of them moved.
    ///
    /// Off the snapshot, not the arranged rows (`view::arrange`): a filter
    /// or a fold leaves the branch no row, which is exactly when the
    /// stand-in takes a seat of its own (`HeadPinRow.seated`).
    pub(super) fn settle_head_marks(&mut self) -> bool {
        let branch = if self.head_name.is_empty() {
            None
        } else {
            self.all.branch_named(&self.head_name)
        };
        let has_remote = branch.is_some_and(|b| b.has_remote);
        let has_pr = branch.is_some() && crate::encode::pr_set().contains(self.head_name.as_str());
        let ahead = branch.map_or(0, |b| crate::encode::shown_count(b.counts.ahead));
        let behind = branch.map_or(0, |b| crate::encode::shown_count(b.counts.behind));
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

    /// Settles `home_branch` off a worktree listing: the branch the
    /// repository's own worktree keeps, where that worktree is not
    /// `current` (this window's, spelled as `drain` spells it).
    /// Answers whether it moved.
    pub(super) fn settle_home_branch(
        &mut self,
        list: &[platitude_core::worktrees::WorktreeEntry],
        current: &str,
    ) -> bool {
        let home = list
            .iter()
            .find(|w| w.main && platitude_core::session::same_path_key(&w.path) != current)
            .and_then(|w| {
                w.branch
                    .clone()
                    .or_else(|| w.held.as_ref().map(|held| held.branch.clone()))
            })
            .unwrap_or_default();
        let moved = self.home_branch != home;
        self.home_branch = home;
        moved
    }
}

#[cfg(test)]
mod tests;
