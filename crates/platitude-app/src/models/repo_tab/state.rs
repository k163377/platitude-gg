use super::*;

impl Default for RepoTab {
    // One line per field of the QObject: the length is the struct's width.
    // `qproperty!` binds a direct field, so a narrower body means splitting
    // the tab's QObject itself.
    #[expect(clippy::too_many_lines)]
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
            last_error_why: String::new(),
            last_error_from_fetch: false,
            // Placeholder: the page's restore writes the saved flag
            // (`PageLayout.applySavedLayout`) and the session reads the
            // same one at open (`Hub::ensure_open`).
            tags_shown: true,
            busy_count: 0,
            busy_op: String::new(),
            unsettled: Vec::new(),
            settling: false,
            standing: false,
            replaying: false,
            merge_tools: Vec::new(),
            merge_tools_loading: false,
            remote_names: Vec::new(),
            remote_branch_asked: Optional::none(),
            remote_branch_revision: 0,
            remote_branch_state: String::new(),
            remote_branch_tip: String::new(),
            remote_branch_theirs: 0,
            branch_delete_asked: String::new(),
            branch_delete_merged: String::new(),
            branch_delete_out: crate::ops::BranchDeleteOut::default(),
            branch_delete_landed: String::new(),
            branch_delete_refused: String::new(),
            branch_delete_answer: -1,
            gone_branch: String::new(),
            gone_remote: String::new(),
            gone_tag: String::new(),
            gone_stash: String::new(),
            gone_worktree: String::new(),
            gone_worktree_chip: String::new(),
            gone_worktree_branch: String::new(),
            signature_wanted: String::new(),
            author_name: String::new(),
            author_email: String::new(),
            author_avatar: 0,
            author_avatar_url: String::new(),
            // Assumed fine until the check says otherwise, so startup
            // stays quiet.
            identity_ready: true,
            identity_read: false,
            identity_unread: false,
            signs_commits: false,
            signing_format: String::new(),
            signature_oid: String::new(),
            signature_kind: String::new(),
            signature_code: String::new(),
            signature_signer: String::new(),
            pending_paths: Vec::new(),
            discard_count: 0,
            discard_only: String::new(),
            remote_urls: Vec::new(),
            remote_count: 0,
            default_remote: String::new(),
            push_default: String::new(),
            push_default_local: false,
            marked_origin: String::new(),
            head_subject: String::new(),
            head_body: String::new(),
            head_author_name: String::new(),
            head_author_email: String::new(),
            head_author_differs: false,
            head_commit_seq: 0,
            amend_empties: false,
            amend_watched: false,
            last_write_error: String::new(),
            last_write_stopped: false,
            write_seq: 0,
            discard_seq: 0,
            taken_seq: 0,
            restore_seq: 0,
            restore_how: String::new(),
            paced_seq: 0,
            paced_worktrees: Vec::new(),
            write_watch: crate::models::repo_tab::WriteWatch::default(),
            write_refused: false,
            write_stale_diff: false,
            write_moved_head: false,
            write_reworded: false,
            write_branch_op: false,
            write_tag_op: false,
            write_fetched: false,
            write_answers: Vec::new(),
            commit_out: crate::ops::Press::default(),
            commit_answer: -1,
            stash_out: crate::ops::TreeOut::default(),
            stash_answer: -1,
            push_out: crate::ops::PushOut::default(),
            push_answer: -1,
            push_answer_branch: String::new(),
            ref_push_out: crate::ops::PushOut::default(),
            ref_push_answer: -1,
            ref_push_target: String::new(),
            worktree_out: crate::ops::Press::default(),
            worktree_answer: -1,
            worktree_answer_path: String::new(),
            worktree_move_out: crate::ops::Press::default(),
            worktree_move_answer: -1,
            worktree_move_from: String::new(),
            worktree_move_to: String::new(),
            worktree_move_standing: false,
            diff_reread: crate::ops::DiffReread::default(),
            tree_seen: 0,
            tree_emptied: false,
            write_report_kind: String::new(),
            write_report_remote: String::new(),
            write_report_name: String::new(),
            write_report_fact: String::new(),
            move_ask_local: String::new(),
            move_ask_start: String::new(),
            move_ask_lost: 0,
            move_ask_seq: 0,
            auto_fetch_running: false,
            auto_fetch_seq: 0,
            fetch_failures: 0,
            fetch_log_raised: false,
            auto_fetch_suspended: false,
            feed: None,
        }
    }
}
impl RepoTab {
    /// Runs `f` with this tab's session, if the tab is still open.
    /// Reads only — a write sent from here is one nothing can wait for;
    /// writes go through [`Self::ask_session`].
    pub(super) fn with_session<R>(
        &self,
        f: impl FnOnce(&Arc<platitude_core::session::RepoSession>) -> R,
    ) {
        crate::hub::with_session(self.tab_id, f);
    }

    /// The one door a write leaves this tab by. Answers with the id the
    /// session accepted it under — nothing with no session or a closed one.
    ///
    /// Every waiter on a write keys on this id (`RepoPage.pendingPopId`,
    /// `ops::StandIn`, `write_watch`); the watch takes it inside this
    /// call, the only moment the id is anybody's in particular.
    pub(super) fn ask_session(
        &mut self,
        f: impl FnOnce(
            &Arc<platitude_core::session::RepoSession>,
        ) -> Option<platitude_core::OperationId>,
    ) -> Option<u64> {
        let asked = crate::hub::from_session(self.tab_id, f)
            .flatten()
            .map(platitude_core::OperationId::as_u64);
        self.write_watch.asked(asked);
        // From here, not from its start: a write queued behind another
        // starts only once that one has settled.
        if let Some(id) = asked
            && self.unsettle(id)
        {
            self.settling_changed();
        }
        asked
    }

    /// Counts a write of this tab's out until what it left has been read
    /// again (`settling`). Says whether the tab was settled before.
    pub(super) fn unsettle(&mut self, id: u64) -> bool {
        if !self.unsettled.contains(&id) {
            self.unsettled.push(id);
        }
        let was_settled = !self.settling;
        self.settling = true;
        was_settled
    }

    /// The session of the worktree this tab was stood in has answered (open
    /// or not), so the count `RepoTab::restand` took goes back. Guarded:
    /// every opening calls it, and one nobody stood for holds no count.
    pub(super) fn stood(&mut self) {
        if !self.standing {
            return;
        }
        self.standing = false;
        self.busy_count = (self.busy_count - 1).max(0);
    }

    /// The page has put a status on screen beside HEAD report `seen`;
    /// `emptied` is whether it left the worktree empty.
    ///
    /// Written down before anything asks, because a status and its write
    /// answer are drained apart and either can arrive first.
    pub(super) fn tree_was_read(&mut self, seen: u64, emptied: bool) {
        if seen < self.tree_seen {
            return;
        }
        self.tree_seen = seen;
        self.tree_emptied = emptied;
    }

    /// Behind `takeStashLanding` (`ops::TreeOut`).
    pub(super) fn stash_landing_taken(&mut self) -> bool {
        self.stash_out.taken(self.tree_seen, self.tree_emptied)
    }

    /// Behind `takeDiffRead` (`ops::DiffReread`).
    pub(super) fn diff_reread_taken(&mut self) -> bool {
        self.diff_reread.taken(self.tree_seen)
    }

    /// Behind `noteDiffRead`: nothing is armed where the status the last
    /// answer publishes has already been read.
    pub(super) fn read_diff_for_answers(&mut self) {
        let after = self.write_answers.last().map_or(0, |a| a.head_seq);
        self.diff_reread.read_after(after, self.tree_seen);
    }

    /// Takes the header line down, whoever put it up.
    pub(super) fn take_down_last_error(&mut self) {
        self.last_error = String::new();
        self.last_error_why = String::new();
        self.last_error_from_fetch = false;
    }

    /// A fetch ended, whoever asked for it: counts consecutive failures
    /// and stops the timer after enough (デザイン規約 §リモートから取り込む).
    ///
    /// `announce` is whether this one may raise the command log: only a
    /// fetch the user pressed may. The timer's and the opening's count but
    /// say nothing beyond the button — a machine that slept or went offline
    /// fails them as a matter of course, and the panel would be thrown up
    /// at every interval and every tab opened (デザイン規約
    /// §リモートから取り込む, the one exception to a failure raising the log).
    pub(super) fn fetch_settled(&mut self, error: &str, announce: bool) {
        if error.is_empty() {
            self.fetch_failures = 0;
            self.fetch_log_raised = false;
            // The header line is state, and a landed fetch makes it untrue
            // (デザイン規約 §リモートから取り込む「成功が 1 回入れば数は 0 に戻る」).
            // Rows are history and stay.
            if self.last_error_from_fetch {
                self.take_down_last_error();
            }
            return;
        }
        self.fetch_failures += 1;
        if announce && !self.fetch_log_raised {
            // Once per run — later failures are the same news; a quiet
            // unasked fetch does not spend it.
            self.fetch_log_raised = true;
            self.last_error = error.to_string();
            self.last_error_why = crate::hub::TabMsg::unreached_word(error);
            self.last_error_from_fetch = true;
            self.fetch_first_failed();
        }
        if self.fetch_failures < FETCH_FAILURES_BEFORE_STOP || self.auto_fetch_suspended {
            return;
        }
        // No timer to stop means nothing suspended and nothing to tell.
        self.auto_fetch_suspended =
            crate::hub::from_session(self.tab_id, |s| s.suspend_auto_fetch()).unwrap_or(false);
    }

    /// Re-reads the configured identity's identicon and assigned picture.
    /// Answers whether either moved; the caller does the telling (the
    /// feed once at the end of its drain).
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
    /// amend has authorship to take over (`--reset-author`). An empty
    /// name is "no HEAD read yet": git refuses an empty `user.name`.
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

    /// Moves HEAD, taking uncommitted work along (`RepoSession::checkout`).
    pub(super) fn move_head(
        &mut self,
        target: platitude_core::branch::CheckoutTarget,
        leaving: bool,
    ) {
        self.ask_session(|s| {
            if leaving {
                s.checkout_leaving_operation(target)
            } else {
                s.checkout(target)
            }
        });
    }

    /// The commit slot's two fields, joined and optioned for core.
    ///
    /// The press records the id it was accepted under in the same breath:
    /// the answer cannot say whose press it was, and only its own answer
    /// empties the editor (`ops::Press`).
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
        let asked = self.ask_session(|s| s.commit(message, options));
        self.commit_out.asked(asked);
    }

    /// The add slot's mode word, turned into what core stands the new worktree
    /// on, and the press written down with its folder so the answer can
    /// say where to go. An unknown word is a caller's bug and makes
    /// nothing.
    pub(super) fn worktree_add(
        &mut self,
        path: String,
        mode: &str,
        branch: String,
        start: String,
        name: String,
    ) {
        use platitude_core::worktrees::WorktreeOn;
        let on = match mode {
            "new" => WorktreeOn::NewBranch {
                name: branch,
                start,
            },
            "branch" => WorktreeOn::Branch(branch),
            "track" => WorktreeOn::Tracking {
                local: branch,
                remote_ref: start,
            },
            other => {
                tracing::warn!(mode = other, "unknown worktree add mode");
                return;
            }
        };
        let asked = self.ask_session(|s| s.add_worktree(path.clone(), on, name));
        self.worktree_out.asked(asked);
        self.worktree_answer_path = path;
    }

    /// `git worktree move`: the folder at `from` renamed to `to`, beside it.
    /// Whether this tab stands in it is written down at the press — the
    /// answer decides where the tab stands next (`worktreeMoveStanding`).
    /// Says whether the queue took it: only then does an answer come.
    pub(super) fn worktree_move(&mut self, from: String, to: String, name: String) -> bool {
        let asked = self.ask_session(|s| s.move_worktree(from.clone(), to.clone(), name));
        self.worktree_move_out.asked(asked);
        self.worktree_move_standing = platitude_core::session::same_path_key(&self.repo_path)
            == platitude_core::session::same_path_key(&from);
        self.worktree_move_from = from;
        self.worktree_move_to = to;
        asked.is_some()
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
        self.ask_session(|s| s.reset(rev, mode));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // These stay off the first-failure branch: its Qt signal wants an
    // attached object (the fetch-recover verb walks it).

    #[test]
    fn a_fetch_that_lands_takes_down_the_line_a_fetch_put_up() {
        let mut tab = RepoTab {
            fetch_failures: 2,
            last_error: "fatal: unable to access".into(),
            last_error_from_fetch: true,
            ..RepoTab::default()
        };
        tab.fetch_settled("", true);
        assert_eq!(tab.fetch_failures, 0);
        assert_eq!(tab.last_error, "");
        assert!(!tab.last_error_from_fetch);
    }

    #[test]
    fn a_fetch_that_lands_leaves_a_background_reads_line_standing() {
        let mut tab = RepoTab {
            last_error: "fatal: bad revision".into(),
            last_error_from_fetch: false,
            ..RepoTab::default()
        };
        tab.fetch_settled("", true);
        assert_eq!(tab.fetch_failures, 0);
        assert_eq!(tab.last_error, "fatal: bad revision");
    }

    #[test]
    fn a_fetch_nobody_pressed_fails_without_a_word_to_the_panel() {
        let mut tab = RepoTab::default();
        // Offline, and the tab has only just opened — or the timer fired.
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
        // The first failure's line has been dismissed, and a background
        // read has written its own news since.
        let mut tab = RepoTab {
            fetch_failures: 1,
            fetch_log_raised: true,
            last_error: "fatal: bad revision".into(),
            last_error_from_fetch: false,
            ..RepoTab::default()
        };
        tab.fetch_settled("fatal: unable to access", true);
        assert_eq!(tab.fetch_failures, 2);
        // Same news as the first: it neither touches nor claims the line.
        assert_eq!(tab.last_error, "fatal: bad revision");
        assert!(!tab.last_error_from_fetch);
        // And the recovery that follows respects the standing owner.
        tab.fetch_settled("", true);
        assert_eq!(tab.last_error, "fatal: bad revision");
    }
}
