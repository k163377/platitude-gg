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
            author_avatar: 0,
            author_avatar_url: String::new(),
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
            remotes: Vec::new(),
            remote_urls: Vec::new(),
            remote_count: 0,
            default_remote: String::new(),
            push_default: String::new(),
            push_default_local: false,
            head_subject: String::new(),
            head_body: String::new(),
            head_author_name: String::new(),
            head_author_email: String::new(),
            head_author_differs: false,
            head_commit_seq: 0,
            last_write_op: String::new(),
            last_write_error: String::new(),
            last_write_stopped: false,
            write_seq: 0,
            move_ask_local: String::new(),
            move_ask_start: String::new(),
            move_ask_seq: 0,
            auto_fetch_running: false,
            auto_fetch_error: String::new(),
            fetch_failures: 0,
            fetch_log_raised: false,
            auto_fetch_suspended: false,
            feed: None,
        }
    }
}
impl RepoTab {
    /// Runs `f` with this tab's session, if the tab is still open.
    pub(super) fn with_session(&self, f: impl FnOnce(&Arc<platitude_core::session::RepoSession>)) {
        crate::hub::with_session(self.tab_id, f);
    }

    /// A fetch ended, whoever asked for it. Counts the ones that failed
    /// and stops the timer once there have been enough of them, so a
    /// machine that has lost the network stops reaching for it every
    /// interval. Anything that comes back clean clears the run.
    ///
    /// `announce` is whether this one may raise the command log. The
    /// fetch an opening fires may not: it still counts — the button
    /// speaks for fetching, not for who asked — but a machine that is
    /// offline would otherwise have the panel thrown up at it every time
    /// a tab opened (デザイン規約 §リモートから取り込む).
    pub(super) fn fetch_settled(&mut self, error: &str, announce: bool) {
        if error.is_empty() {
            self.fetch_failures = 0;
            self.fetch_log_raised = false;
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
        if announce && !self.fetch_log_raised {
            // The panel reads this; the ones after it are the same news.
            // Counted from the first failure that may speak rather than
            // from the first failure outright, so a quiet opening fetch
            // does not use up the run's one telling.
            self.fetch_log_raised = true;
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
        self.auto_fetch_suspended =
            crate::hub::from_session(self.tab_id, |s| s.suspend_auto_fetch()).unwrap_or(false);
    }

    /// Re-reads the face the configured identity wears: the identicon
    /// its name packs to, and the picture assigned to its address if
    /// there is one. Answers whether either moved, so the caller decides
    /// whether anyone has to be told — the feed says so once at the end
    /// of its own drain, an assignment arriving on its own has to say so
    /// itself.
    pub(super) fn read_author_avatar(&mut self) -> bool {
        let code = crate::encode::avatar_code(&self.author_name);
        let url = Hub::with(|hub| hub.avatar_url(&self.author_email)).unwrap_or_default();
        if code == self.author_avatar && url == self.author_avatar_url {
            return false;
        }
        self.author_avatar = code;
        self.author_avatar_url = url;
        true
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
    pub(super) fn move_head(&self, target: platitude_core::branch::CheckoutTarget, leaving: bool) {
        self.with_session(|s| {
            if leaving {
                s.checkout_leaving_operation(target.clone());
            } else {
                s.checkout(target.clone());
            }
        });
    }

    /// The commit slot's two fields, joined and optioned for core.
    pub(super) fn commit_from_fields(
        &mut self,
        subject: String,
        body: String,
        amend: bool,
        reset_author: bool,
    ) {
        let message = platitude_core::commit::join_message(&subject, &body);
        let options = platitude_core::commit::CommitOptions {
            amend,
            allow_empty: false,
            reset_author: amend && reset_author,
        };
        self.with_session(|s| s.commit(message.clone(), options));
    }

    /// The reset slot's mode word, turned into core's enum; an unknown
    /// word is a caller's bug and moves nothing.
    pub(super) fn reset_head(&mut self, rev: String, mode: String) {
        use platitude_core::branch::ResetMode;
        let mode = match mode.as_str() {
            "soft" => ResetMode::Soft,
            "mixed" => ResetMode::Mixed,
            "hard" => ResetMode::Hard,
            other => {
                tracing::warn!(mode = other, "unknown reset mode");
                return;
            }
        };
        self.with_session(|s| s.reset(rev.clone(), mode));
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
        tab.fetch_settled("", true);
        assert_eq!(tab.fetch_failures, 0);
        assert_eq!(tab.last_error, "");
        assert!(!tab.last_error_from_fetch);
    }

    #[test]
    fn a_fetch_that_lands_leaves_a_background_reads_line_standing() {
        let mut tab = RepoTab::default();
        tab.last_error = "fatal: bad revision".into();
        tab.last_error_from_fetch = false;
        tab.fetch_settled("", true);
        assert_eq!(tab.fetch_failures, 0);
        assert_eq!(tab.last_error, "fatal: bad revision");
    }

    #[test]
    fn the_fetch_an_opening_fires_fails_without_a_word_to_the_panel() {
        let mut tab = RepoTab::default();
        // Offline, and the tab has only just opened.
        tab.fetch_settled("fatal: unable to access", false);
        assert_eq!(tab.fetch_failures, 1, "it is a failed fetch like any other");
        assert_eq!(
            tab.last_error, "",
            "and the button is the only thing that says so"
        );
        assert!(!tab.last_error_from_fetch);
        assert!(
            !tab.fetch_log_raised,
            "the run's one telling is still there for a failure that may speak"
        );
    }

    #[test]
    fn later_failures_neither_rewrite_nor_reclaim_the_line() {
        let mut tab = RepoTab::default();
        // The first failure's line has been dismissed, and a background
        // read has written its own news since.
        tab.fetch_failures = 1;
        tab.fetch_log_raised = true;
        tab.last_error = "fatal: bad revision".into();
        tab.last_error_from_fetch = false;
        tab.fetch_settled("fatal: unable to access", true);
        assert_eq!(tab.fetch_failures, 2);
        // The second failure is the same news as the first: it does not
        // touch the line, so it cannot claim it either.
        assert_eq!(tab.last_error, "fatal: bad revision");
        assert!(!tab.last_error_from_fetch);
        // And the recovery that follows respects the standing owner.
        tab.fetch_settled("", true);
        assert_eq!(tab.last_error, "fatal: bad revision");
    }
}
