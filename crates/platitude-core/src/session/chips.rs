//! What a refs read leaves for the graph: the snapshot it hands out,
//! and the chips it puts on the rows it already numbered.

use super::*;

impl RepoSession {
    /// The snapshot to publish: the one already on screen when this read
    /// found it unchanged, so the sidebar tells "the same" by pointer and
    /// rebuilds nothing. Published even so: withheld, a consumer attached
    /// since the last one would sit empty until something moved.
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
        // Kept only while this session keeps what it reads
        // (`RepoSession::keeps_what_it_reads`).
        if self.keeps_what_it_reads() {
            *slot = Some(Arc::clone(&shared));
        }
        shared
    }

    /// Installs a new label map into the join and sends the rows whose
    /// chips changed.
    ///
    /// Read and send happen under the lock a log pass installs and
    /// announces its graph under, so a diff can neither be invalidated
    /// between the two nor arrive ahead of its rows. The generation covers
    /// a superseded pass that installs late, whose row numbers point at
    /// other commits in the consumer's graph.
    pub(super) fn apply_refs(&self, label_map: LabelIndex) {
        let tags = self.tags_shown();
        let Some(mut shared) = self.store_shared() else {
            return;
        };
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
