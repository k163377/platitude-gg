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

    /// Answers whether a write went out. The pane arms a wait the write's
    /// own answer puts down, so a request turned away here — no worktree
    /// target, nothing selected, no fingerprint yet — must say so: armed
    /// with no answer coming, the wait would hold the marks for good.
    pub(super) fn stage_chosen(
        &mut self,
        kind: String,
        path: String,
        orig_path: String,
        hunk: i32,
        line: i32,
        fingerprint: String,
    ) -> bool {
        let Some(target) = crate::encode::worktree_target(&kind, &path, &orig_path) else {
            tracing::warn!(kind, "selection staging asked for a non-worktree diff");
            return false;
        };
        let selects = crate::encode::hunk_selection(hunk, line);
        if selects.is_empty() {
            return false;
        }
        // The fingerprint of the diff the indices were made on (hex, from
        // DiffModel). Without one the selection addresses nothing.
        let Ok(seen) = u64::from_str_radix(&fingerprint, 16) else {
            tracing::warn!(fingerprint, "selection staging without a diff fingerprint");
            return false;
        };
        let mut queued = false;
        self.with_session(|s| {
            s.apply_partial(target.clone(), selects.clone(), seen);
            queued = true;
        });
        queued
    }

    /// Answers whether a write went out, for the same reason as
    /// [`Self::stage_chosen`].
    pub(super) fn discard_chosen(
        &mut self,
        kind: String,
        path: String,
        orig_path: String,
        hunk: i32,
        line: i32,
        fingerprint: String,
    ) -> bool {
        let Some(target) = crate::encode::worktree_target(&kind, &path, &orig_path) else {
            tracing::warn!(kind, "selection discard asked for a non-worktree diff");
            return false;
        };
        let selects = crate::encode::hunk_selection(hunk, line);
        if selects.is_empty() {
            return false;
        }
        let Ok(seen) = u64::from_str_radix(&fingerprint, 16) else {
            tracing::warn!(fingerprint, "selection discard without a diff fingerprint");
            return false;
        };
        let mut queued = false;
        self.with_session(|s| {
            s.discard_partial(target.clone(), selects.clone(), seen);
            queued = true;
        });
        queued
    }
}
