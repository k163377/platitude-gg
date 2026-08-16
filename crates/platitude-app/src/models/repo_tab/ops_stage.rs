//! Staging and discarding a chosen set of rows.

use super::*;

impl RepoTab {
    /// The gathered-path writes share one shape: take the set, skip an
    /// empty ask, hand the batch to the session.
    pub(super) fn drain_paths(
        &mut self,
        send: impl FnOnce(&std::sync::Arc<platitude_core::session::RepoSession>, Vec<String>),
    ) {
        let paths = std::mem::take(&mut self.pending_paths);
        if paths.is_empty() {
            return;
        }
        self.with_session(move |s| send(s, paths));
    }

    pub(super) fn stage_chosen(
        &mut self,
        kind: String,
        path: String,
        orig_path: String,
        hunk: i32,
        line: i32,
        fingerprint: String,
    ) {
        let Some(target) = crate::encode::worktree_target(&kind, &path, &orig_path) else {
            tracing::warn!(kind, "selection staging asked for a non-worktree diff");
            return;
        };
        let selects = crate::encode::hunk_selection(hunk, line);
        if selects.is_empty() {
            return;
        }
        // The fingerprint of the diff the indices were made on (hex, from
        // DiffModel). Without one the selection addresses nothing.
        let Ok(seen) = u64::from_str_radix(&fingerprint, 16) else {
            tracing::warn!(fingerprint, "selection staging without a diff fingerprint");
            return;
        };
        self.with_session(|s| s.apply_partial(target.clone(), selects.clone(), seen));
    }

    pub(super) fn discard_chosen(
        &mut self,
        kind: String,
        path: String,
        orig_path: String,
        hunk: i32,
        line: i32,
        fingerprint: String,
    ) {
        let Some(target) = crate::encode::worktree_target(&kind, &path, &orig_path) else {
            tracing::warn!(kind, "selection discard asked for a non-worktree diff");
            return;
        };
        let selects = crate::encode::hunk_selection(hunk, line);
        if selects.is_empty() {
            return;
        }
        let Ok(seen) = u64::from_str_radix(&fingerprint, 16) else {
            tracing::warn!(fingerprint, "selection discard without a diff fingerprint");
            return;
        };
        self.with_session(|s| s.discard_partial(target.clone(), selects.clone(), seen));
    }
}
