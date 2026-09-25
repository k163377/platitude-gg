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
        self.stash_out
            .asked(asked.map(platitude_core::OperationId::as_u64));
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
        self.stash_out
            .asked(asked.map(platitude_core::OperationId::as_u64));
    }

    pub(super) fn look_up_remote_branch(&mut self, remote: String, branch: String) {
        self.remote_branch_asked = Optional::none();
        self.remote_branch_revision = self.remote_branch_revision.wrapping_add(1);
        self.remote_branch_state = String::new();
        self.remote_branch_tip = String::new();
        self.remote_branch_theirs = 0;
        self.changed();
        self.with_session(|s| s.check_remote_branch(remote.clone(), branch.clone()));
    }

    pub(super) fn look_up_branch_delete(&mut self, branch: String) {
        self.branch_delete_asked = String::new();
        self.branch_delete_merged = String::new();
        self.changed();
        self.with_session(|s| s.check_branch_delete(branch.clone()));
    }

    /// `git push <remote> <local>:<remote_branch>`, keyed to a ref row.
    /// Nothing in `ui/` calls `pushBranch` yet; it is answered by id anyway
    /// (`ops::PushOut`), so a door added to it gets its own refusal.
    pub(super) fn branch_push(
        &mut self,
        remote: String,
        local: String,
        remote_branch: String,
        set_upstream: bool,
        force: String,
        lease_expect: String,
    ) {
        let row = format!("{remote}/{remote_branch}");
        let force = Self::push_force(&force, &lease_expect);
        let spec = platitude_core::remote::PushSpec {
            remote,
            local,
            remote_branch,
            set_upstream,
            force,
        };
        let asked = self.ask_session(|s| s.push(spec.clone()));
        self.ref_push_asked(&row, asked.map(platitude_core::OperationId::as_u64));
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
