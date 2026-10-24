//! Everything that reaches a remote, and the reads before one.

use super::*;

impl RepoTab {
    /// Everything uncommitted, in one entry, untracked files included
    /// (デザイン規約 §変更を退避する「未追跡は必ず含める」).
    ///
    /// `message` is the summary standing in the commit box, or empty for
    /// git's own `WIP on …`; the press asks nothing.
    ///
    /// The press writes down its id (`ops::StashOut`): the reading that
    /// finds the tree empty comes long after the answer, when only the
    /// press can say whose doing it was.
    pub(super) fn stash_push(&mut self, message: String) {
        let options = platitude_core::stash::PushOptions {
            include_untracked: true,
            keep_index: false,
            staged_only: false,
        };
        let asked = self.ask_session(|s| s.stash_push(message.clone(), options, Vec::new()));
        self.stash_out.asked(asked);
    }

    /// The same over the gathered files only — it can empty the tree too,
    /// since they may be all there were.
    pub(super) fn stash_chosen_paths(&mut self, message: String) {
        let paths = std::mem::take(&mut self.pending_paths);
        if paths.is_empty() {
            return;
        }
        let options = platitude_core::stash::PushOptions {
            include_untracked: true,
            keep_index: false,
            staged_only: false,
        };
        let asked = self.ask_session(|s| s.stash_push(message.clone(), options, paths.clone()));
        self.stash_out.asked(asked);
    }

    pub(super) fn look_up_remote_branch(&mut self, remote: String, branch: String, local: String) {
        self.remote_branch_asked = Optional::none();
        self.remote_branch_revision = self.remote_branch_revision.wrapping_add(1);
        self.remote_branch_state = String::new();
        self.remote_branch_tip = String::new();
        self.remote_branch_theirs = 0;
        self.changed();
        self.with_session(|s| s.check_remote_branch(remote.clone(), branch.clone(), local.clone()));
    }

    pub(super) fn look_up_branch_delete(&mut self, branch: String) {
        self.branch_delete_asked = String::new();
        self.branch_delete_merged = String::new();
        self.changed();
        self.with_session(|s| s.check_branch_delete(branch.clone()));
    }

    /// `branch` pushed from its row's menu, where its marks and upstream
    /// send it (`RepoSession::push_named`, resolved inside the write as the
    /// toolbar's is). Answered by id through the toolbar's own owner
    /// (`ops::PushOut`): what git said is about that branch's standing, so a
    /// refusal is remembered for the branch whichever door sent it
    /// (デザイン規約 §リモートへ送る).
    pub(super) fn named_push(&mut self, branch: String, force: String, lease_expect: String) {
        let fallback = self.default_remote.clone();
        let force = Self::push_force(&force, &lease_expect);
        let asked =
            self.ask_session(|s| s.push_named(branch.clone(), fallback.clone(), force.clone()));
        self.push_out.asked(asked, branch);
    }

    /// The commit asked about is written down first, so an answer for a
    /// selection already left can be dropped (`signature_wanted`).
    pub(super) fn look_up_signature(&mut self, oid_hex: String) {
        let Ok(oid) = platitude_core::oid::Oid::from_hex_str(oid_hex.trim()) else {
            tracing::warn!(oid_hex, "invalid oid in signature check");
            return;
        };
        self.signature_wanted = oid.to_hex();
        self.with_session(|s| s.check_signature(oid));
    }
}
