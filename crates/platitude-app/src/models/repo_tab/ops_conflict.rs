//! A merge, rebase or cherry-pick that stopped on conflicts.

use super::*;

impl RepoTab {
    pub(super) fn settle_operation(&mut self, how: String) {
        use platitude_core::integrate::Continuation;
        let continuation = match how.as_str() {
            "continue" => Continuation::Continue,
            "abort" => Continuation::Abort,
            "skip" => Continuation::Skip,
            "quit" => Continuation::Quit,
            other => {
                tracing::warn!(how = other, "unknown continuation");
                return;
            }
        };
        self.with_session(|s| s.resolve_current(continuation));
    }

    pub(super) fn take_side(&mut self, side: String) {
        use platitude_core::conflict::Side;
        let side = match side.as_str() {
            "ours" => Side::Ours,
            "theirs" => Side::Theirs,
            other => {
                tracing::warn!(side = other, "unknown conflict side");
                return;
            }
        };
        let paths = std::mem::take(&mut self.pending_paths);
        if paths.is_empty() {
            return;
        }
        self.with_session(|s| s.take_side(paths.clone(), side));
    }

    pub(super) fn run_merge_tool(&mut self) {
        let paths = std::mem::take(&mut self.pending_paths);
        if paths.is_empty() {
            return;
        }
        self.with_session(|s| s.mergetool(paths.clone()));
    }

    pub(super) fn list_merge_tools(&mut self) {
        if self.merge_tools_loading {
            return;
        }
        self.merge_tools_loading = true;
        self.with_session(|s| s.ask_merge_tools());
        self.changed();
    }
}
