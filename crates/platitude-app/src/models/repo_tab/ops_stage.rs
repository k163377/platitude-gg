//! Staging and discarding a chosen set of rows.

use super::*;

impl RepoTab {
    pub(super) fn stage_chosen_lines(
        &mut self,
        kind: String,
        path: String,
        orig_path: String,
        fingerprint: String,
    ) {
        let lines = std::mem::take(&mut self.pending_lines);
        let Some(target) = crate::encode::worktree_target(&kind, &path, &orig_path) else {
            tracing::warn!(kind, "line staging asked for a non-worktree diff");
            return;
        };
        let selects = crate::encode::line_selection(&lines);
        if selects.is_empty() {
            return;
        }
        let Ok(seen) = u64::from_str_radix(&fingerprint, 16) else {
            tracing::warn!(fingerprint, "line staging without a diff fingerprint");
            return;
        };
        self.with_session(|s| s.apply_partial(target.clone(), selects.clone(), seen));
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
