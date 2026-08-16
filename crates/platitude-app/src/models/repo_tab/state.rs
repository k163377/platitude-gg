use super::*;

impl Default for RepoTab {
    fn default() -> Self {
        Self {
            tab_id: 0,
            state: String::new(),
            title: String::new(),
            repo_path: String::new(),
            picker_folder_url: String::new(),
            error: String::new(),
            error_kind: String::new(),
            error_path: String::new(),
            last_error: String::new(),
            last_error_from_fetch: false,
            // Mirrors core LogOptions::default().
            tags_shown: true,
            busy_count: 0,
            busy_op: String::new(),
            merge_tools: String::new(),
            merge_tools_loading: false,
            remote_names: String::new(),
            remote_branch_asked: String::new(),
            remote_branch_state: String::new(),
            remote_branch_tip: String::new(),
            remote_branch_theirs: 0,
            branch_delete_asked: String::new(),
            branch_delete_merged: true,
            publish_range: String::new(),
            publish_total: 0,
            publish_published: 0,
            history_oid: String::new(),
            history_in: false,
            head_reached_elsewhere: false,
            author_name: String::new(),
            author_email: String::new(),
            // Assumed fine until the check says otherwise, so nothing
            // flashes a warning during startup.
            identity_ready: true,
            signs_commits: false,
            signing_format: String::new(),
            signature_oid: String::new(),
            signature_kind: String::new(),
            signature_code: String::new(),
            signature_signer: String::new(),
            pending_paths: Vec::new(),
            pending_lines: Vec::new(),
            remotes: Vec::new(),
            remote_count: 0,
            default_remote: String::new(),
            head_subject: String::new(),
            head_body: String::new(),
            head_author_name: String::new(),
            head_author_email: String::new(),
            head_author_differs: false,
            head_commit_seq: 0,
            last_write_op: String::new(),
            last_write_error: String::new(),
            write_seq: 0,
            move_ask_local: String::new(),
            move_ask_start: String::new(),
            move_ask_seq: 0,
            auto_fetch_running: false,
            auto_fetch_error: String::new(),
            fetch_failures: 0,
            auto_fetch_suspended: false,
            feed: None,
        }
    }
}
impl RepoTab {
    /// Runs `f` with this tab's session, if the tab is still open.
    pub(super) fn with_session(&self, f: impl FnOnce(&Arc<platitude_core::session::RepoSession>)) {
        if let Some(Some(session)) = Hub::with(|hub| hub.session(self.tab_id)) {
            f(&session);
        }
    }

    /// A fetch ended, whoever asked for it. Counts the ones that failed
    /// and stops the timer once there have been enough of them, so a
    /// machine that has lost the network stops reaching for it every
    /// interval. Anything that comes back clean clears the run.
    pub(super) fn fetch_settled(&mut self, error: &str) {
        if error.is_empty() {
            self.fetch_failures = 0;
            self.auto_fetch_error = String::new();
            // The header line is state, not history: a fetch that has
            // just landed makes "fetch cannot reach the remote" untrue,
            // and red kept up over that would contradict the button that
            // is already back to normal (デザイン規約 §リモートから取り込む
            // 「成功が 1 回入れば数は 0 に戻る」— its command-log side).
            // Rows are left alone: history stays until a reader clears it.
            if self.last_error_from_fetch {
                self.last_error = String::new();
                self.last_error_from_fetch = false;
            }
            return;
        }
        self.fetch_failures += 1;
        if self.fetch_failures == 1 {
            // The panel reads this; the ones after it are the same news.
            self.last_error = error.to_string();
            self.last_error_from_fetch = true;
            self.fetch_first_failed();
        }
        if self.fetch_failures < FETCH_FAILURES_BEFORE_STOP || self.auto_fetch_suspended {
            return;
        }
        // Whether there was a timer to stop is the answer to "is this a
        // repository that fetches on its own at all": one that does not
        // has nothing suspended, and nothing to be told about it.
        let mut stopped = false;
        if let Some(Some(session)) = Hub::with(|hub| hub.session(self.tab_id)) {
            stopped = session.suspend_auto_fetch();
        }
        self.auto_fetch_suspended = stopped;
    }

    /// Whether HEAD carries someone else's name — the only case where an
    /// amend has authorship to take over (`--reset-author`).
    ///
    /// git refuses to commit with an empty `user.name`, so a HEAD that is
    /// really there always has one: an empty name is "no HEAD read yet"
    /// rather than an identity to compare against.
    pub(super) fn compare_head_author(&mut self) {
        self.head_author_differs = !self.head_author_name.is_empty()
            && (self.head_author_name != self.author_name
                || self.head_author_email != self.author_email);
    }

    /// Decodes the push-force pair QML sends.
    pub(super) fn push_force(force: &str, lease_expect: &str) -> platitude_core::remote::PushForce {
        use platitude_core::remote::PushForce;
        match force {
            "lease" => PushForce::WithLease {
                expect: (!lease_expect.is_empty()).then(|| lease_expect.to_string()),
            },
            "force" => PushForce::Force,
            _ => PushForce::None,
        }
    }

    /// Moves HEAD, taking uncommitted work along — through a stash when
    /// git will not carry it itself, which needs nothing asked here.
    pub(super) fn move_head(&self, target: platitude_core::branch::CheckoutTarget) {
        self.with_session(|s| s.checkout(target.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // These stay off the first-failure branch: that one raises a Qt
    // signal, which wants an attached object. The fetch-recover verb
    // walks it in the real window instead.

    #[test]
    fn a_fetch_that_lands_takes_down_the_line_a_fetch_put_up() {
        let mut tab = RepoTab::default();
        tab.fetch_failures = 2;
        tab.last_error = "fatal: unable to access".into();
        tab.last_error_from_fetch = true;
        tab.fetch_settled("");
        assert_eq!(tab.fetch_failures, 0);
        assert_eq!(tab.last_error, "");
        assert!(!tab.last_error_from_fetch);
    }

    #[test]
    fn a_fetch_that_lands_leaves_a_background_reads_line_standing() {
        let mut tab = RepoTab::default();
        tab.last_error = "fatal: bad revision".into();
        tab.last_error_from_fetch = false;
        tab.fetch_settled("");
        assert_eq!(tab.fetch_failures, 0);
        assert_eq!(tab.last_error, "fatal: bad revision");
    }

    #[test]
    fn later_failures_neither_rewrite_nor_reclaim_the_line() {
        let mut tab = RepoTab::default();
        // The first failure's line has been dismissed, and a background
        // read has written its own news since.
        tab.fetch_failures = 1;
        tab.last_error = "fatal: bad revision".into();
        tab.last_error_from_fetch = false;
        tab.fetch_settled("fatal: unable to access");
        assert_eq!(tab.fetch_failures, 2);
        // The second failure is the same news as the first: it does not
        // touch the line, so it cannot claim it either.
        assert_eq!(tab.last_error, "fatal: bad revision");
        assert!(!tab.last_error_from_fetch);
        // And the recovery that follows respects the standing owner.
        tab.fetch_settled("");
        assert_eq!(tab.last_error, "fatal: bad revision");
    }
}
