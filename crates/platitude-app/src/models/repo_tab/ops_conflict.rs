//! Bringing work in — a merge, a rebase, a pull — and a merge, rebase or
//! cherry-pick that stopped on conflicts.

use super::*;

impl RepoTab {
    /// Behind `merge`: the slot's flags as core's options; a message of
    /// blanks is none.
    pub(super) fn merge_into_head(
        &mut self,
        rev: String,
        no_ff: bool,
        ff_only: bool,
        message: String,
    ) {
        let options = platitude_core::integrate::MergeOptions {
            no_ff,
            ff_only,
            squash: false,
            message: (!message.trim().is_empty()).then_some(message),
        };
        self.ask_session(|s| s.merge(rev, options));
    }

    /// Behind `rebase`: onto `onto` where one is given.
    pub(super) fn rebase_head(&mut self, upstream: String, onto: String) {
        let options = platitude_core::integrate::RebaseOptions {
            onto: crate::encode::optional_text(onto),
            ..Default::default()
        };
        self.ask_session(|s| s.rebase(upstream, options));
    }

    /// Behind `pull`: the worktree's own branch where `branch` is empty, a
    /// fast-forward of the branch named otherwise.
    pub(super) fn pull_or_fast_forward(&mut self, branch: String) {
        if branch.is_empty() {
            self.ask_session(platitude_core::session::RepoSession::pull);
        } else {
            self.ask_session(|s| s.fast_forward(branch));
        }
    }

    /// `--continue` commits the index git holds: **not while a stage or
    /// unstage press stands** ([`Self::staging_turns_away`]). The ways out
    /// that take no index stay open.
    pub(super) fn settle_operation(&mut self, how: String) {
        use platitude_core::integrate::Continuation;
        let continuation = match how.as_str() {
            "continue" if self.staging_turns_away("continue") => return,
            "continue" => Continuation::Continue,
            "abort" => Continuation::Abort,
            "skip" => Continuation::Skip,
            "quit" => Continuation::Quit,
            other => {
                tracing::warn!(how = other, "unknown continuation");
                return;
            }
        };
        self.ask_session(|s| s.resolve_current(continuation));
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
        self.ask_session(|s| s.take_side(paths, side));
    }

    pub(super) fn run_merge_tool(&mut self) {
        let paths = std::mem::take(&mut self.pending_paths);
        if paths.is_empty() {
            return;
        }
        self.ask_session(|s| s.mergetool(paths));
    }

    pub(super) fn list_merge_tools(&mut self) {
        if self.merge_tools_loading {
            return;
        }
        // Marked only once the ask is out: with no session no answer
        // comes, and an uncleared mark would refuse every later ask.
        if crate::hub::from_session(self.tab_id, |s| s.ask_merge_tools()).is_some() {
            self.merge_tools_loading = true;
            self.changed();
        }
    }
}
