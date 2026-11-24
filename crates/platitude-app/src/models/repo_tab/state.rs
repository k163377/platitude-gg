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
            settled: std::collections::VecDeque::new(),
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
            gone: crate::ops::Rows::default(),
            staging_seq: 0,
            staging_all_out: false,
            staging_moving: Vec::new(),
            staging_unseen: false,
            staging_rows_unseen: false,
            ahead_unseen: false,
            diff_writes_out: false,
            outstanding: Vec::new(),
            commit_waits: false,
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
            commit_out: crate::ops::TreeOut::default(),
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
            write_report: Reported::default(),
            move_ask_local: String::new(),
            move_ask_start: String::new(),
            move_ask_lost: 0,
            move_ask_seq: 0,
            unshown: Vec::new(),
            fetching: false,
            pushing: false,
            auto_fetch_seq: 0,
            fetch_failures: 0,
            fetch_log_raised: false,
            auto_fetch_suspended: false,
            feed: None,
            self_handle: None,
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
    ///
    /// Says what moved, each on its own notify (`settling_changed`,
    /// `commit_waits_changed`): `settling`, and the writes out the Commit
    /// button waits on (`commit_waits`) — a queued write counts from
    /// here.
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
        if let Some(id) = asked {
            // From here, not from its start: a write queued behind another
            // starts only once that one has answered, and where its kind
            // joins no burst, once it has settled (`session::write`).
            if self.unsettle(id) {
                self.settling_changed();
            }
            let waiting = self.commit_waits;
            self.write_out(id);
            if self.commit_waits != waiting {
                self.commit_waits_changed();
            }
        }
        asked
    }

    /// Runs `f`, then raises `settling_changed`, `commit_waits_changed` and
    /// `unshown_changed`, in that order, for whichever of the three `f`
    /// moved. Not around [`Self::ask_session`], which raises the first two
    /// itself — wrapped, a write it counts out on a settled tab raises
    /// `settling_changed` twice.
    pub(super) fn telling_waits(&mut self, f: impl FnOnce(&mut Self)) {
        let was_settling = self.settling;
        let waiting = self.commit_waits;
        let turning = (self.fetching, self.pushing);
        f(self);
        if self.settling != was_settling {
            self.settling_changed();
        }
        if self.commit_waits != waiting {
            self.commit_waits_changed();
        }
        if (self.fetching, self.pushing) != turning {
            self.unshown_changed();
        }
    }

    /// A write is out — accepted at this tab's door, or seen starting where it
    /// was asked elsewhere (`TabMsg::WriteState`) — until its answer
    /// (`write_answered`): the Commit button waits on it
    /// (`read_writes_out`).
    pub(super) fn write_out(&mut self, id: u64) {
        if !self.outstanding.contains(&id) {
            self.outstanding.push(id);
        }
        self.read_writes_out(false);
    }

    /// git's answer to any write: no longer out for the Commit button, and
    /// handed to the stage presses by id — refused, a press's rows come back
    /// within this drain, so its one notify tells the refusal about rows
    /// already back on screen; landed, they stay where they went.
    ///
    /// No notify of its own: this runs inside `drain`.
    pub(super) fn write_answered(&mut self, id: u64, failed: bool, reads_from: u64) {
        self.outstanding.retain(|out| *out != id);
        let moved =
            crate::hub::stage_ahead(self.tab_id, |ahead| ahead.answered(id, failed, reads_from));
        self.read_writes_out(moved);
    }

    /// Copies what the stage presses hold into the plain members bindings
    /// read, moving `staging_seq` where `moved` says the rows did, where a
    /// press no longer out lets its rows' marks go, or where the screen has
    /// caught up with the last press standing (`unseen` — an open diff
    /// waiting in place lands then, `RepoPage.followLanding`), or a part of
    /// the open diff went (`ahead_unseen` — the diff draws again); and
    /// whether the Commit button waits — on a write out, a press holding
    /// the index's writes back (`StageAhead::index_unseen`) or the stand
    /// (`commit_waits`). Answers whether anything QML reads moved.
    pub(super) fn read_writes_out(&mut self, moved: bool) -> bool {
        let (all_out, moving, unseen, index_unseen, ahead_unseen, diff_writes_out) =
            crate::hub::stage_ahead(self.tab_id, |ahead| {
                (
                    ahead.all_out(),
                    ahead.moving_paths(),
                    ahead.unseen(),
                    ahead.index_unseen(),
                    ahead.ahead_unseen(),
                    self.outstanding.iter().any(|id| !ahead.part_on_drawn(*id)),
                )
            });
        let waits = !self.outstanding.is_empty() || index_unseen || self.standing;
        let staged = moved
            || all_out != self.staging_all_out
            || moving != self.staging_moving
            || unseen != self.staging_rows_unseen
            || index_unseen != self.staging_unseen
            || ahead_unseen != self.ahead_unseen
            || diff_writes_out != self.diff_writes_out;
        let shifted = staged || waits != self.commit_waits;
        if staged {
            self.staging_seq = self.staging_seq.wrapping_add(1);
        }
        self.staging_all_out = all_out;
        self.staging_moving = moving;
        self.staging_rows_unseen = unseen;
        self.staging_unseen = index_unseen;
        self.ahead_unseen = ahead_unseen;
        self.diff_writes_out = diff_writes_out;
        self.commit_waits = waits;
        shifted
    }

    /// A write another model asked for in this same turn
    /// (`DiscardModel.restoreAt`), counted out from its acceptance as the
    /// tab's own are (`ask_session`): settling, and out for the Commit
    /// button.
    pub(super) fn counted_out(&mut self, id: u64) {
        self.unsettle(id);
        self.write_out(id);
    }

    /// Behind `countWrite`: the id as the page holds it, counted out and
    /// told; one that names no write (negative, or 0) counts nothing.
    pub(super) fn count_write_told(&mut self, id: i32) {
        let Ok(id) = u64::try_from(id) else {
            return;
        };
        if id == 0 {
            return;
        }
        self.telling_waits(|tab| {
            tab.counted_out(id);
        });
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
        self.read_writes_out(false);
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

    /// Behind `takeStashLanding` (`ops::TreeOut`): the write's fence where
    /// the stash took the tree.
    pub(super) fn stash_landing_taken(&mut self) -> Option<u64> {
        self.stash_out.taken(self.tree_seen, self.tree_emptied)
    }

    /// Behind `takeCommitLanding` (`ops::TreeOut`): the write's fence where
    /// the editor's commit took the whole tree.
    pub(super) fn commit_landing_taken(&mut self) -> Option<u64> {
        self.commit_out.taken(self.tree_seen, self.tree_emptied)
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
    /// Whoever asked: the fetch button speaks for how fetching is going.
    /// One success, a pressed one too, puts the count back to 0 — else a
    /// press that went through would keep saying the last one failed.
    /// A pull's fetch is not one: it answers as a write (`OperationKind::Pull`)
    /// — the button counts how `git fetch` is doing, and a pull also fails
    /// on a conflict or a dirty tree.
    ///
    /// `announce` is whether this one may raise the command log: only a
    /// fetch the user pressed may, once per run of failures. The timer's and
    /// the opening's count (the `!` shows) but say nothing beyond the
    /// button (user decision — ask before changing) — a machine that slept
    /// or went offline fails them as a matter of course, the reader has
    /// nothing to do about it, and the panel would be thrown up at every
    /// interval and every tab opened (the one exception to a failure raising
    /// the log, デザイン規約 §リモートから取り込む).
    pub(super) fn fetch_settled(&mut self, error: &str, announce: bool) {
        if error.is_empty() {
            self.fetch_failures = 0;
            self.fetch_log_raised = false;
            // The header line is state, and a landed fetch makes it untrue.
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

    /// Decodes the push-force pair QML sends. `expect` is read for
    /// `"lease"` alone: the object its door showed pins the lease, and with
    /// none on screen the lease is git's own, to its remote-tracking ref
    /// (`PushForce::WithTrackingLease`). A pin that names no object is
    /// turned away ([`Self::pin_turned_away`]); the other two words carry
    /// none.
    pub(super) fn push_force(
        force: &str,
        expect: &str,
    ) -> Result<platitude_core::remote::PushForce, platitude_core::oid::OidParseError> {
        use platitude_core::remote::PushForce;
        Ok(match force {
            "lease" => match crate::encode::pinned_oid(expect)? {
                Some(expect) => PushForce::WithLease { expect },
                None => PushForce::WithTrackingLease,
            },
            "force" => PushForce::Force,
            _ => PushForce::None,
        })
    }

    /// A write turned away before the session over a pin that names no
    /// object — a lease's, or the commit or entry a write is made on:
    /// nothing goes to git. Said to the watch as the door says a
    /// write it turns down ([`Self::ask_session`]) — a waiting reader must
    /// not expect an answer — and answered as that door answers, with no
    /// id. The rows that hand the pin over are the models' own
    /// (`Oid::to_hex`), so this is the belt under them.
    pub(super) fn pin_turned_away(&mut self, what: &str) -> Option<u64> {
        tracing::warn!(what, "turned away: its pin names no object");
        self.write_watch.asked(None);
        None
    }

    /// [`Self::ask_session`] for a write behind a pin as QML handed it
    /// back: asked with what the pin decodes to, and turned away where
    /// it names no object ([`Self::pin_turned_away`]) — the one door of
    /// every such write.
    pub(super) fn ask_pinned<T>(
        &mut self,
        what: &str,
        pin: Result<T, platitude_core::oid::OidParseError>,
        f: impl FnOnce(
            &Arc<platitude_core::session::RepoSession>,
            T,
        ) -> Option<platitude_core::OperationId>,
    ) -> Option<u64> {
        match pin {
            Ok(pin) => self.ask_session(|s| f(s, pin)),
            Err(_) => self.pin_turned_away(what),
        }
    }

    /// [`Self::ask_pinned`] for a write made on the object its row showed
    /// — a name taken off a remote leased to it, a commit rewritten or a
    /// stash entry renamed by it: with none to pin to — an empty pin as much
    /// as a malformed one — nothing is asked. Where the write is two
    /// commands, the first has not run either.
    pub(super) fn ask_on_object(
        &mut self,
        what: &str,
        object: &str,
        f: impl FnOnce(
            &Arc<platitude_core::session::RepoSession>,
            platitude_core::Oid,
        ) -> Option<platitude_core::OperationId>,
    ) -> Option<u64> {
        let pin = crate::encode::pinned_oid(object)
            .and_then(|pin| pin.ok_or(platitude_core::oid::OidParseError));
        self.ask_pinned(what, pin, f)
    }

    /// A remote row as a press on it is carried ([`TrackedBranch::listed`]),
    /// read against the remotes this tab lists. None for a name no slash
    /// divides — no remote to address, and nothing is asked.
    pub(super) fn tracked(&self, remote_ref: &str) -> Option<TrackedBranch> {
        TrackedBranch::listed(remote_ref, self.remote_names.iter().map(String::as_str))
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

    /// Whether a write that takes the index whole — a commit, the details
    /// pane's amend of HEAD, a stash, the stopped operation's `--continue` —
    /// is turned away here: a press that writes the index holds it back
    /// (`StageAhead::index_unseen`) — the rows on screen are the press's
    /// word until the status after it is drawn, and git would take its own
    /// index. Not the press a failed status read lets off the wait, which
    /// that rule names: the write goes over rows that are its word still.
    /// Their buttons wait then (`stagingUnseen`, `Words.stagingWait`); this
    /// is the belt under them.
    pub(super) fn staging_turns_away(&self, what: &str) -> bool {
        let stands = crate::hub::stage_ahead(self.tab_id, |ahead| ahead.index_unseen());
        if stands {
            tracing::info!(what, "turned away: a stage press is still standing");
        }
        stands
    }

    /// The commit slot's two fields, joined and optioned for core.
    ///
    /// The press records the id it was accepted under in the same breath:
    /// the answer cannot say whose press it was, and only its own answer
    /// empties the editor — and only its own tree read, taken after that
    /// answer, lands the page (`ops::TreeOut`).
    ///
    /// **Not while a stage or unstage press holds it back**
    /// ([`Self::staging_turns_away`]).
    pub(super) fn commit_from_fields(
        &mut self,
        subject: String,
        body: String,
        amend: bool,
        reset_author: bool,
    ) {
        if self.staging_turns_away("commit") {
            self.commit_out.asked(None);
            return;
        }
        let message = platitude_core::commit::join_message(&subject, &body);
        let options = platitude_core::commit::CommitOptions {
            amend,
            // Not offered across the bridge: the screen has no road to an
            // empty commit — Commit goes through only with something staged,
            // an amend aside (`WipCommitBlock`), where core adds the flag
            // itself for an empty HEAD (`commit::commit`).
            allow_empty: false,
            reset_author: amend && reset_author,
        };
        let asked = self.ask_session(|s| s.commit(message, options));
        self.commit_out.asked(asked);
    }

    /// The details pane's save of a commit's message: HEAD is amended,
    /// anything older replayed. Every message the pane saves is HEAD's
    /// (`RepoPage.messageEdit` takes typing on HEAD alone), and the amend
    /// takes what is staged too.
    ///
    /// **Not while a stage or unstage press holds it back**
    /// ([`Self::staging_turns_away`]).
    pub(super) fn reword_from_fields(&mut self, oid: String, subject: String, body: String) {
        if self.staging_turns_away("amend") {
            return;
        }
        let message = platitude_core::commit::join_message(&subject, &body);
        if message.is_empty() {
            return;
        }
        self.ask_on_object("reword", &oid, |s, oid| s.reword(oid, message));
    }

    /// Behind `watchAmend`: says to the session whether an amend is in view,
    /// taking the last answer down as the watch starts.
    pub(super) fn amend_in_view(&mut self, on: bool) {
        self.amend_watched = on;
        if on && self.amend_empties {
            self.amend_empties = false;
            self.changed();
        }
        self.with_session(|s| s.watch_amend(on));
    }

    /// Behind `holdDraft`: hands the commit editor's words to the hub.
    pub(super) fn keep_draft(&self, subject: String, body: String, amending: bool) {
        let id = self.tab_id;
        let draft = crate::hub::Draft {
            subject,
            body,
            amending,
        };
        Hub::with(|hub| hub.hold_draft(id, draft));
    }

    /// The words the hub holds for this tab: empty where it holds none, or
    /// where there is no hub.
    pub(super) fn held_draft(&self) -> crate::hub::Draft {
        Hub::with(|hub| hub.draft(self.tab_id)).unwrap_or_default()
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
    use platitude_core::remote::PushForce;

    const SHA1: &str = "a9771664a8219f845abfd8ce6f5d7af521247539";

    /// A write turned away over its lease says so to the watch: an armed
    /// run is not left waiting for an answer nothing will bring.
    #[test]
    fn a_lease_that_names_no_object_tells_the_watch_the_write_was_turned_away() {
        let mut tab = RepoTab::default();
        assert!(tab.write_watch.arm("probe").is_none());
        assert_eq!(tab.pin_turned_away("probe"), None);
        assert_eq!(tab.write_watch.stage(), "turned-down");
    }

    /// The pin is the lease's alone: a plain push and a force carry none,
    /// whatever came beside the word — and a lease with none on screen is
    /// git's own.
    #[test]
    fn the_force_word_decides_whether_the_pin_is_read() {
        let pinned = platitude_core::Oid::from_hex_str(SHA1).expect("an object name");
        assert_eq!(
            RepoTab::push_force("lease", SHA1),
            Ok(PushForce::WithLease { expect: pinned })
        );
        assert_eq!(
            RepoTab::push_force("lease", ""),
            Ok(PushForce::WithTrackingLease)
        );
        assert!(RepoTab::push_force("lease", "origin/main").is_err());
        assert_eq!(RepoTab::push_force("", "origin/main"), Ok(PushForce::None));
        assert_eq!(RepoTab::push_force("force", ""), Ok(PushForce::Force));
        assert_eq!(
            RepoTab::push_force("force", "origin/main"),
            Ok(PushForce::Force)
        );
    }

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

    /// An operation that could not reach its remote names why beside git's
    /// line, for the command log's header (`Words.unreachedWhy`).
    #[test]
    fn an_unreached_remote_names_why_beside_its_line() {
        let mut tab = RepoTab::default();
        tab.absorb(vec![TabMsg::OpError {
            message:
                "fatal: could not read Username for 'https://github.com': terminal prompts disabled"
                    .into(),
            why: "credentials".into(),
        }]);
        assert_eq!(tab.last_error_why, "credentials");
        assert!(tab.last_error.starts_with("fatal: could not read Username"));
        assert!(!tab.last_error_from_fetch);
    }
}
