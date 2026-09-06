//! Everything that reaches a remote, and the reads before one.

use super::*;

impl RepoTab {
    /// Everything uncommitted, in one entry.
    ///
    /// Untracked files go with it: a stash that leaves new files behind is
    /// the surprise most often reported to other git GUIs, and the stash
    /// a switch makes on its own carries them for the same reason.
    ///
    /// `message` is the summary standing in the commit box, or empty —
    /// the press still asks nothing, so nothing is gathered before the
    /// write; what is already written down is used (デザイン規約
    /// §変更を退避する). Empty leaves git to write its own `WIP on …`.
    pub(super) fn stash_push(&mut self, message: String) {
        let options = platitude_core::stash::PushOptions {
            include_untracked: true,
            keep_index: false,
            staged_only: false,
        };
        self.with_session(|s| s.stash_push(message.clone(), options, Vec::new()));
    }

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
        self.with_session(|s| s.stash_push(message.clone(), options, paths.clone()));
    }

    pub(super) fn look_up_remote_branch(&mut self, remote: String, branch: String) {
        self.remote_branch_asked = String::new();
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

    /// `git branch --delete` (`-D` under `force`). The plain form is the
    /// one press whose menu stays up for git's answer, so which branch it
    /// was about is kept until that answer comes (`settle_write`); the
    /// forced form was already the answer to a refusal and stands for
    /// nothing.
    pub(super) fn branch_delete(&mut self, name: String, force: bool) {
        if self.arm_branch_delete(&name, force) {
            // QML is told the last answer is gone, so a delete of a
            // re-made branch of the same name reads its own answer as a
            // change rather than as the old one standing.
            self.changed();
        }
        self.with_session(|s| s.delete_branch(name.clone(), force));
    }

    /// Writes the plain delete down as the question the next branch
    /// answer is about, and takes the last answer down with it — the
    /// check's beat (`look_up_branch_delete`). Says whether anything was
    /// armed: the forced form arms nothing. Apart from the notify so a
    /// test can hold it against `settle_write`.
    pub(super) fn arm_branch_delete(&mut self, name: &str, force: bool) -> bool {
        if force {
            return false;
        }
        self.branch_delete_out = name.to_string();
        self.branch_delete_landed = String::new();
        self.branch_delete_refused = String::new();
        self.branch_delete_seq = 0;
        true
    }

    pub(super) fn branch_push(
        &mut self,
        remote: String,
        local: String,
        remote_branch: String,
        set_upstream: bool,
        force: String,
        lease_expect: String,
    ) {
        let force = Self::push_force(&force, &lease_expect);
        let spec = platitude_core::remote::PushSpec {
            remote,
            local,
            remote_branch,
            set_upstream,
            force,
        };
        self.with_session(|s| s.push(spec.clone()));
    }

    pub(super) fn branch_delete_everywhere(
        &mut self,
        branch: String,
        remote: String,
        remote_branch: String,
        force: bool,
    ) {
        self.with_session(|s| {
            s.delete_branch_everywhere(
                branch.clone(),
                remote.clone(),
                remote_branch.clone(),
                force,
            );
        });
    }

    /// Asked on every selection and answered by a git of its own. The
    /// commit asked about is written down first, so that an answer
    /// arriving for a selection already left behind can be told from the
    /// one the pane is waiting for (`signature_wanted`).
    pub(super) fn look_up_signature(&mut self, oid_hex: String) {
        let Ok(oid) = platitude_core::oid::Oid::from_hex_str(oid_hex.trim()) else {
            tracing::warn!(oid_hex, "invalid oid in signature check");
            return;
        };
        self.signature_wanted = oid.to_hex();
        self.with_session(|s| s.check_signature(oid));
    }
}
