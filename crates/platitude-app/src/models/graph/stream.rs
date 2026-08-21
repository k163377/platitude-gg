//! What a walk's answers do to the rows already held: the in-place
//! replacement, and what the footer under them is left saying.

use super::*;

impl GraphModel {
    /// Replaces the whole list in place: unchanged rows stay untouched,
    /// contiguous runs of changed rows emit one ranged dataChanged, and
    /// only the length delta inserts or removes rows. No model reset —
    /// the view keeps its scroll position and never shows an empty list.
    #[expect(unsafe_code)]
    pub(super) fn splice_notified(&mut self, new_rows: Vec<GraphRowItem>) {
        let old_len = self.rows.len();
        let new_len = new_rows.len();
        let common = old_len.min(new_len);
        let mut head = new_rows;
        let extra = head.split_off(common);

        // In-place writes first (no Qt runs between here and the
        // notifications below — everything happens inside one slot).
        let mut ranges: Vec<(usize, usize)> = Vec::new();
        for (i, item) in head.into_iter().enumerate() {
            if self.rows[i] != item {
                self.rows[i] = item;
                push_run(&mut ranges, i);
            }
        }

        if old_len > new_len {
            if let Some(proxy) = self.try_get_rust_proxy_ptr() {
                // SAFETY: same pattern as QListModelBase::remove — the
                // proxy pointer stays valid while the QObject side is
                // attached, and we are on the Qt main thread in a slot.
                unsafe { &mut *proxy }.base_begin_remove_rows(
                    &mut *self,
                    &qtbridge::qtbridge_type_lib::QModelIndex::default(),
                    new_len as i32,
                    old_len as i32 - 1,
                );
                self.rows.truncate(new_len);
                // SAFETY: see above.
                unsafe { &mut *proxy }.base_end_remove_rows(&mut *self);
            } else {
                self.rows.truncate(new_len);
            }
        }

        self.notify_runs(ranges);

        if !extra.is_empty() {
            self.extend_notified(extra);
        }
    }

    /// What the footer under the graph says once a walk has stopped: the
    /// counts, the time it took, and — only where the walk was cut short —
    /// the lanes still running off the bottom of the last row.
    ///
    /// Whether this stream is the one being listened to stays with the
    /// caller: a chunked walk answers only for its own generation, and a
    /// replacement supersedes anything older. So does what only a
    /// replacement resets.
    pub(super) fn settle_footer(
        &mut self,
        row_total: i32,
        elapsed_ms: u64,
        walked: u32,
        truncated: bool,
    ) {
        self.loading = false;
        self.total_ms = elapsed_ms as i32;
        self.row_total = row_total;
        self.walked_total = walked as i32;
        self.truncated = truncated;
        self.finish_count += 1;
        self.tail_geometry = if truncated {
            self.rows
                .last()
                .map(|r| crate::encode::tail_lanes(&r.geometry))
                .unwrap_or_default()
        } else {
            String::new()
        };
    }
}
