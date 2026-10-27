//! What a tab lets go of when it is stood in another worktree of
//! the repository it is showing (`RepoTab::restand`).

use super::*;

impl RepoTab {
    /// Back to before the worktree being left was read.
    ///
    /// What stays is what the repository answered, which a linked worktree
    /// answers the same: who this tab is and what it is called, identity
    /// and signing settings, remotes, merge tools, the eye on the tags and
    /// a commit's signature. Everything else goes, and a field added to
    /// this model belongs here — it is the worktree's, or an answer the
    /// retired sink never delivers (`Hub::let_go_of_session`).
    /// `write_seq`, `move_ask_seq` and `paced_seq` stay too: the staying
    /// page reads them as rises (`RepoPage.seenWriteSeq`).
    ///
    /// Written out, not `*self = Self::default()`: dropping the value
    /// deletes its QObject, and the next slot call aborts ("No proxy").
    pub(super) fn forget_the_worktree(&mut self) {
        self.error = String::new();
        self.error_kind = String::new();
        self.error_path = String::new();
        self.take_down_last_error();
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
        // What a delete took away stays gone until the next session's lists
        // say (`ops::StandIn::session_gone`): the staying page would draw it
        // back from the listing it holds.
        self.read_stand_in();
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
        self.amend_empties = false;
        self.last_write_error = String::new();
        self.last_write_stopped = false;
        self.write_watch = WriteWatch::default();
        self.clear_write_group();
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
        self.worktree_out = crate::ops::Press::default();
        self.worktree_answer = -1;
        self.worktree_answer_path = String::new();
        self.worktree_move_out = crate::ops::Press::default();
        self.worktree_move_answer = -1;
        self.worktree_move_from = String::new();
        self.worktree_move_to = String::new();
        self.worktree_move_standing = false;
        self.move_ask_local = String::new();
        self.move_ask_start = String::new();
        self.move_ask_lost = 0;
        self.paced_worktrees = Vec::new();
        self.auto_fetch_running = false;
        self.auto_fetch_seq = 0;
        self.fetch_failures = 0;
        self.fetch_log_raised = false;
        self.auto_fetch_suspended = false;
        // The retired sink never delivers the settles still owed.
        self.unsettled.clear();
        self.settling = false;
        // Last, the one thing this leaves standing: one command in flight
        // until the new session says where it is (`RepoTab::standing`).
        self.busy_count = 1;
        self.standing = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standing_elsewhere_puts_the_whole_write_group_at_rest() {
        let mut tab = RepoTab {
            write_refused: true,
            write_stale_diff: true,
            write_moved_head: true,
            write_reworded: true,
            write_branch_op: true,
            write_tag_op: true,
            write_fetched: true,
            write_report_kind: "update-rule".into(),
            write_report_remote: "origin".into(),
            write_report_name: "main".into(),
            write_report_fact: "main".into(),
            ..RepoTab::default()
        };
        tab.forget_the_worktree();
        assert_eq!(
            [
                tab.write_refused,
                tab.write_stale_diff,
                tab.write_moved_head,
                tab.write_reworded,
                tab.write_branch_op,
                tab.write_tag_op,
                tab.write_fetched,
            ],
            [false; 7]
        );
        assert_eq!(
            [
                tab.write_report_kind.as_str(),
                tab.write_report_remote.as_str(),
                tab.write_report_name.as_str(),
                tab.write_report_fact.as_str(),
            ],
            [""; 4]
        );
    }
}
