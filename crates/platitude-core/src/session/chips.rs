//! What a refs read leaves for the graph: the snapshot it hands out,
//! and the chips it puts on the rows it already numbered.

use super::*;

impl RepoSession {
    /// The snapshot to publish: the one already on screen when this read
    /// found it unchanged, so the sidebar can tell "the same" from "equal"
    /// by pointer and rebuild nothing for it.
    ///
    /// Still published either way. Withholding the event instead would
    /// save the same work, but a consumer that attached after the last one
    /// went out would then sit empty until something moved, and "nothing
    /// changed" is the state that lasts longest.
    pub(super) fn published_snapshot(&self) -> Option<Arc<RefsSnapshot>> {
        relock(&self.last_snapshot).as_ref().map(Arc::clone)
    }

    pub(super) fn share_snapshot(&self, fresh: RefsSnapshot) -> Arc<RefsSnapshot> {
        let mut slot = relock(&self.last_snapshot);
        if let Some(previous) = slot.as_ref()
            && **previous == fresh
        {
            return Arc::clone(previous);
        }
        let shared = Arc::new(fresh);
        *slot = Some(Arc::clone(&shared));
        shared
    }

    /// Installs a new label map into the join and sends the rows whose
    /// chips changed.
    ///
    /// Read and send happen under one lock, and the row numbers travel
    /// with the graph they were read from: a log pass installs its own
    /// state and announces it under the same lock, so a diff can neither
    /// be invalidated between the two nor arrive ahead of the rows it
    /// numbers. The generation covers what the lock cannot — a superseded
    /// pass that installs late leaves `shared` describing a graph the
    /// consumer already dropped, and its row numbers point at other
    /// commits there.
    pub(super) fn apply_refs(&self, label_map: LabelIndex) {
        let tags = self.tags_shown();
        let mut shared = self.lock_shared();
        shared.label_map = label_map;

        let mut fresh: HashMap<u32, Vec<RefLabel>> = HashMap::new();
        for (oid, labels) in shared.label_map.commits(tags) {
            if let Some(row) = shared.builder.row_of(&oid) {
                fresh.insert(row, labels.to_vec());
            }
        }
        let mut changed: Vec<(u32, Vec<RefLabel>)> = Vec::new();
        for (row, labels) in &fresh {
            if shared.applied.get(row) != Some(labels) {
                changed.push((*row, labels.clone()));
            }
        }
        for row in shared.applied.keys() {
            if !fresh.contains_key(row) {
                changed.push((*row, Vec::new()));
            }
        }
        shared.applied = fresh;
        changed.sort_by_key(|(row, _)| *row);
        // Mirror the chip change into the delivered-rows record, or the
        // next background rebuild would see a phantom difference and swap
        // an identical graph.
        for (row, labels) in &changed {
            if let Some(sent) = shared.sent_rows.get_mut(*row as usize) {
                sent.labels = RowPrint::labels_of(labels);
            }
        }
        if changed.is_empty() {
            return;
        }
        self.sink.event(SessionEvent::LabelsChanged {
            generation: shared.generation,
            rows: changed,
        });
    }
}
