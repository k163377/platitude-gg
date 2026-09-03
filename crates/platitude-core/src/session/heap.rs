//! The session's own memory report -- what it is holding on to.

use super::*;

impl RepoSession {
    /// What this session is holding on to, part by part.
    ///
    /// Everything named here outlives the operation that filled it: it is
    /// still there when the window is idle, which is what the memory budget
    /// is about. `refs-snapshot` is an `Arc` the sidebar models hold too —
    /// counted in full on both sides, and the app's report says so.
    pub fn heap_report(&self) -> Vec<crate::mem::Part> {
        use crate::mem::{Footprint as _, Part};
        let shared = self.lock_shared();
        let mut parts = vec![
            Part::of("sent-rows", &shared.sent_rows),
            Part::new(
                "label-map",
                shared.label_map.heap_bytes(),
                shared.label_map.len(),
            ),
            Part::new("applied", shared.applied.heap_bytes(), shared.applied.len()),
            Part::new(
                "graph-builder",
                shared.builder.heap_bytes(),
                shared.builder.tracked_oids(),
            ),
            Part::new(
                "publish-marks",
                shared.publish_marks.heap_bytes(),
                shared.publish_marks.tracked_oids(),
            ),
        ];
        drop(shared);

        let snapshot = relock(&self.last_snapshot).clone();
        let (snap_bytes, snap_refs) = match &snapshot {
            Some(s) => (
                s.heap_bytes(),
                s.locals.len() + s.remotes.len() + s.tags.len(),
            ),
            None => (0, 0),
        };
        parts.push(Part::new("refs-snapshot", snap_bytes, snap_refs));

        let index = self.remote_tag_index();
        parts.push(Part::new(
            "remote-tag-index",
            index.heap_bytes(),
            index.len(),
        ));

        let marks = Arc::clone(&relock(&self.eol_marks));
        parts.push(Part::new("eol-marks", marks.heap_bytes(), marks.len()));

        let (base_bytes, base_count) = {
            let g = relock(&self.eol_baselines);
            (g.heap_bytes(), g.len())
        };
        parts.push(Part::new("eol-baselines", base_bytes, base_count));
        parts
    }
}
