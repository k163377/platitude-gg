//! One section's feeds, drained into the rows and properties QML reads.
//!
//! Five feeds land here, and only the ones this section was attached to
//! ever hold anything — a list is wired to exactly one of them
//! (`attach::attach_section_feed` / `attach::attach_worktree_feed`).

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
        if let Some(feed) = self.refs_feed.clone()
            && let Some(snapshot) = feed.drain().pop()
        {
            settled = true;
            // The first snapshot is news whatever it holds: the default
            // selection is waiting on `refsLoaded`, and a section that is
            // legitimately empty would otherwise never say so.
            arrived |= !self.refs_loaded;
            self.refs_loaded = true;
            let fresh = !self
                .last_refs
                .as_ref()
                .is_some_and(|last| Arc::ptr_eq(last, &snapshot));
            if fresh {
                moved = true;
                self.last_refs = Some(Arc::clone(&snapshot));
                arrived |= match self.section.as_str() {
                    "branches" => {
                        let head = snapshot.locals.iter().find(|b| b.is_head);
                        self.head_name = head.map(|b| b.short.to_string()).unwrap_or_default();
                        self.head_oid = head.map(|b| b.oid.to_hex()).unwrap_or_default();
                        self.head_has_remote = head.is_some_and(|b| b.has_remote);
                        self.head_has_pr = head
                            .is_some_and(|b| crate::encode::pr_set().contains(b.short.as_str()));
                        self.take(Source::Locals(snapshot))
                    }
                    "remotes" => self.take(Source::Remotes(snapshot)),
                    _ => self.take(Source::Tags(snapshot)),
                };
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
            && let Some(stashes) = feed.drain().pop()
        {
            stashes_arrived = true;
            arrived |= self.take(Source::Stashes(stashes));
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
}
