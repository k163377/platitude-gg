//! What a tab lets go of when it is stood in another working copy of
//! the repository it is showing (`RepoTab::restand`).

use super::*;

impl RepoTab {
    /// Back to before the copy being left was read.
    ///
    /// What stays is what the repository answered, which a linked copy
    /// answers the same: who this tab is and what it is called, identity
    /// and signing settings, remotes, merge tools, the eye on the tags and
    /// a commit's signature. Everything else goes, and a field added to
    /// this model belongs here — it is the copy's, or an answer the
    /// retired sink never delivers (`Hub::let_go_of_session`).
    /// `write_seq` and `move_ask_seq` stay too: the staying page reads
    /// them as rises (`RepoPage.seenWriteSeq`).
    ///
    /// Written out, not `*self = Self::default()`: dropping the value
    /// deletes its QObject, and the next slot call aborts ("No proxy").
    pub(super) fn forget_the_copy(&mut self) {
        self.error = String::new();
        self.error_kind = String::new();
        self.error_path = String::new();
        self.last_error = String::new();
        self.last_error_from_fetch = false;
        self.busy_op = String::new();
        self.replaying = false;
        self.merge_tools_loading = false;
        self.remote_branch_asked = Optional::none();
        self.remote_branch_revision = 0;
        self.remote_branch_state = String::new();
        self.remote_branch_tip = String::new();
        self.remote_branch_theirs = 0;
        self.branch_delete_asked = String::new();
        self.branch_delete_merged = String::new();
        self.branch_delete_out = crate::ops::BranchDeleteOut::default();
        self.branch_delete_landed = String::new();
        self.branch_delete_refused = String::new();
        self.branch_delete_answer = -1;
        self.gone_branch = String::new();
        self.gone_remote = String::new();
        self.gone_tag = String::new();
        self.gone_stash = String::new();
        self.gone_worktree = String::new();
        // The ask goes and the answer stays: a read still out answers
        // nowhere, and the page asks again (`RepoPage.standSettled`).
        self.signature_wanted = String::new();
        self.pending_paths = Vec::new();
        self.discard_count = 0;
        self.discard_only = String::new();
        self.head_subject = String::new();
        self.head_body = String::new();
        self.head_author_name = String::new();
        self.head_author_email = String::new();
        self.head_author_differs = false;
        self.head_commit_seq = 0;
        self.last_write_error = String::new();
        self.last_write_stopped = false;
        self.write_watch = WriteWatch::default();
        self.write_refused = false;
        self.write_stale_diff = false;
        self.write_moved_head = false;
        self.write_reworded = false;
        self.write_branch_op = false;
        self.write_fetched = false;
        self.write_answers = Vec::new();
        self.commit_out = crate::ops::Press::default();
        self.commit_answer = -1;
        self.diff_reread = crate::ops::DiffReread::default();
        self.tree_seen = 0;
        self.tree_emptied = false;
        self.stash_out = crate::ops::StashOut::default();
        self.stash_answer = -1;
        self.push_out = crate::ops::PushOut::default();
        self.push_answer = -1;
        self.push_answer_branch = String::new();
        self.ref_push_out = crate::ops::PushOut::default();
        self.ref_push_answer = -1;
        self.ref_push_target = String::new();
        self.write_report_kind = String::new();
        self.write_report_remote = String::new();
        self.write_report_name = String::new();
        self.write_report_reason = String::new();
        self.move_ask_local = String::new();
        self.move_ask_start = String::new();
        self.auto_fetch_running = false;
        self.fetch_failures = 0;
        self.fetch_log_raised = false;
        self.auto_fetch_suspended = false;
        // Last, the one thing this leaves standing: one command in flight
        // until the new session says where it is (`RepoTab::standing`).
        self.busy_count = 1;
        self.standing = true;
    }
}
