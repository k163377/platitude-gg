use super::*;

#[qobject(ConvertToCamelCase, NoQmlElement)]
impl RepoTab {
    qproperty!("state", Member = state, Notify = changed);
    qproperty!("title", Member = title, Notify = changed);
    qproperty!("repoPath", Member = repo_path, Notify = changed);
    qproperty!(
        "pickerFolderUrl",
        Member = picker_folder_url,
        Notify = changed
    );
    qproperty!("error", Member = error, Notify = changed);
    qproperty!("errorKind", Member = error_kind, Notify = changed);
    qproperty!("errorPath", Member = error_path, Notify = changed);
    qproperty!("lastError", Member = last_error, Notify = changed);
    qproperty!("tagsShown", Member = tags_shown, Notify = changed);
    qproperty!("busyCount", Member = busy_count, Notify = changed);
    qproperty!("busyOp", Member = busy_op, Notify = changed);
    qproperty!("standing", Member = standing, Notify = changed);
    qproperty!("replaying", Member = replaying, Notify = changed);
    qproperty!("mergeTools", Member = merge_tools, Notify = changed);
    qproperty!(
        "mergeToolsLoading",
        Member = merge_tools_loading,
        Notify = changed
    );
    qproperty!("remoteNames", Member = remote_names, Notify = changed);
    // The pair itself is `remoteBranchAsked()`: a property cannot hold
    // nothing (`encode::Optional`), so the revision is what a binding
    // follows.
    qproperty!(
        "remoteBranchRevision",
        Member = remote_branch_revision,
        Notify = changed
    );
    qproperty!(
        "remoteBranchState",
        Member = remote_branch_state,
        Notify = changed
    );
    qproperty!(
        "remoteBranchTip",
        Member = remote_branch_tip,
        Notify = changed
    );
    qproperty!(
        "remoteBranchTheirs",
        Member = remote_branch_theirs,
        Notify = changed
    );
    qproperty!(
        "branchDeleteAsked",
        Member = branch_delete_asked,
        Notify = changed
    );
    qproperty!(
        "branchDeleteMerged",
        Member = branch_delete_merged,
        Notify = changed
    );
    // git's answer to the plain delete a card stayed up for, by name: the
    // card reads its own and goes or turns its row (`RefBranchMenu`).
    // Where that answer stands in this notify — -1 where this notify
    // carried none of it — is how the page tells the answer in hand from
    // the one standing above (`ops::BranchDeleteOut`).
    qproperty!(
        "branchDeleteLanded",
        Member = branch_delete_landed,
        Notify = changed
    );
    qproperty!(
        "branchDeleteRefused",
        Member = branch_delete_refused,
        Notify = changed
    );
    qproperty!(
        "branchDeleteAnswer",
        Member = branch_delete_answer,
        Notify = changed
    );
    // The rows a delete has taken off the screen, one name per list, for
    // the lists and the chips to draw without (デザイン規約
    // §消す操作は先に画面から消す). When they go and when they come back is
    // `ops::StandIn`'s — QML is handed the picture.
    qproperty!("goneBranch", Member = gone_branch, Notify = changed);
    qproperty!("goneRemote", Member = gone_remote, Notify = changed);
    qproperty!("goneTag", Member = gone_tag, Notify = changed);
    qproperty!("goneStash", Member = gone_stash, Notify = changed);
    qproperty!("authorName", Member = author_name, Notify = changed);
    qproperty!("authorEmail", Member = author_email, Notify = changed);
    qproperty!("authorAvatar", Member = author_avatar, Notify = changed);
    qproperty!(
        "authorAvatarUrl",
        Member = author_avatar_url,
        Notify = changed
    );
    qproperty!("identityReady", Member = identity_ready, Notify = changed);
    qproperty!("signsCommits", Member = signs_commits, Notify = changed);
    qproperty!("signingFormat", Member = signing_format, Notify = changed);
    qproperty!("signatureOid", Member = signature_oid, Notify = changed);
    qproperty!("signatureKind", Member = signature_kind, Notify = changed);
    qproperty!("signatureCode", Member = signature_code, Notify = changed);
    qproperty!(
        "signatureSigner",
        Member = signature_signer,
        Notify = changed
    );
    qproperty!("remoteCount", Member = remote_count, Notify = changed);
    qproperty!("defaultRemote", Member = default_remote, Notify = changed);
    qproperty!("pushDefault", Member = push_default, Notify = changed);
    qproperty!(
        "pushDefaultLocal",
        Member = push_default_local,
        Notify = changed
    );
    qproperty!("markedOrigin", Member = marked_origin, Notify = changed);
    qproperty!("headSubject", Member = head_subject, Notify = changed);
    qproperty!("headBody", Member = head_body, Notify = changed);
    qproperty!(
        "headAuthorName",
        Member = head_author_name,
        Notify = changed
    );
    qproperty!(
        "headAuthorEmail",
        Member = head_author_email,
        Notify = changed
    );
    qproperty!(
        "headAuthorDiffers",
        Member = head_author_differs,
        Notify = changed
    );
    qproperty!("headCommitSeq", Member = head_commit_seq, Notify = changed);
    qproperty!(
        "lastWriteError",
        Member = last_write_error,
        Notify = changed
    );
    qproperty!("writeSeq", Member = write_seq, Notify = changed);
    // Where the editor's own commit answered in this notify, or -1 — the
    // press wrote its id down and the drain hands the answer back here,
    // so the page reads it where it stands, whatever order the
    // answers came in (`ops::Press`).
    qproperty!("commitAnswer", Member = commit_answer, Notify = changed);
    // …and where the stash press that took the working tree away
    // answered. Its other wait — the tree itself — is asked for
    // (`takeStashLanding`).
    qproperty!("stashAnswer", Member = stash_answer, Notify = changed);
    // …and where the toolbar's push answered, with the branch that press
    // was sent for: a refusal is remembered per branch, and the word
    // `push` on an answer does not say whose it was (`ops::PushOut`).
    qproperty!("pushAnswer", Member = push_answer, Notify = changed);
    qproperty!(
        "pushAnswerBranch",
        Member = push_answer_branch,
        Notify = changed
    );
    // …and the same pair for the pushes a ref row sends, which answer
    // under the same word and used to lose their refusal to whatever
    // else came back in the drain (`ops::PushOut`).
    qproperty!("refPushAnswer", Member = ref_push_answer, Notify = changed);
    qproperty!("refPushTarget", Member = ref_push_target, Notify = changed);
    // What is left over, classified in drain::settle_write — the page
    // reads meanings (app-ui.md). These describe the last answer of
    // this notify **nobody was waiting for by name**: a drain can
    // carry several answers, and one that went to an owner is read
    // where that owner holds it. What a reader waiting for one
    // particular answer asks is `writeAnswerCount` and the nine
    // beside it.
    qproperty!("writeRefused", Member = write_refused, Notify = changed);
    qproperty!(
        "writeStaleDiff",
        Member = write_stale_diff,
        Notify = changed
    );
    qproperty!(
        "writeMovedHead",
        Member = write_moved_head,
        Notify = changed
    );
    qproperty!("writeReworded", Member = write_reworded, Notify = changed);
    qproperty!("writeBranchOp", Member = write_branch_op, Notify = changed);
    qproperty!("writeTagOp", Member = write_tag_op, Notify = changed);
    qproperty!("writeFetched", Member = write_fetched, Notify = changed);
    // What a write that did not happen has to say for itself — the page
    // makes a notice out of these
    // (デザイン規約 §答えの要らない報せ).
    qproperty!(
        "writeReportKind",
        Member = write_report_kind,
        Notify = changed
    );
    qproperty!(
        "writeReportRemote",
        Member = write_report_remote,
        Notify = changed
    );
    qproperty!(
        "writeReportName",
        Member = write_report_name,
        Notify = changed
    );
    qproperty!(
        "writeReportReason",
        Member = write_report_reason,
        Notify = changed
    );
    qproperty!("moveAskLocal", Member = move_ask_local, Notify = changed);
    qproperty!("moveAskStart", Member = move_ask_start, Notify = changed);
    qproperty!("moveAskSeq", Member = move_ask_seq, Notify = changed);
    qproperty!(
        "autoFetchRunning",
        Member = auto_fetch_running,
        Notify = changed
    );
    qproperty!("fetchFailures", Member = fetch_failures, Notify = changed);
    qproperty!(
        "autoFetchSuspended",
        Member = auto_fetch_suspended,
        Notify = changed
    );
    qproperty!("discardCount", Member = discard_count, Notify = changed);
    qproperty!("discardOnly", Member = discard_only, Notify = changed);

    #[qsignal]
    pub(super) fn changed(&mut self);

    /// The working copy this tab is standing in would not open, and the
    /// repository has its own one to stand in instead
    /// (`Hub::home_copy`). The strip is what moves the tab
    /// (`TabsModel::standTabHome`); this only says that it is to.
    ///
    /// **Whoever hears it answers a turn later** (`RepoPage`): standing
    /// the tab elsewhere takes this page down, and this is emitted from
    /// the middle of the drain that is filling it
    /// (規約 §Qt Bridges の要点 — 再入 borrow).
    #[qsignal]
    pub(super) fn stand_home_asked(&mut self);

    /// The first fetch of a run to fail. Only the first: a machine that
    /// is simply offline fails every interval, and the panel that opens
    /// on this would then be opening over and over on the same news.
    #[qsignal]
    pub(super) fn fetch_first_failed(&mut self);

    /// Starts the timer again on the interval it was set to, and fetches
    /// now — the hold on the toolbar button is what reaches this.
    #[qslot]
    fn resume_auto_fetch(&mut self) {
        self.restart_auto_fetch()
    }

    /// Re-reads the identity's assigned picture. Called when an
    /// assignment changes: the identity did not, so there is nothing to
    /// ask git for (the same shape `Details::refresh_avatar` has).
    #[qslot]
    fn refresh_avatar(&mut self) {
        if self.read_author_avatar() {
            self.changed();
        }
    }

    /// Name of one remote (a list property would need a model of its own
    /// for three strings).
    #[qslot]
    fn remote_at(&self, index: i32) -> String {
        self.remote_name_at(index)
    }

    /// How many write answers this notify carried
    /// (`RepoTab::write_answers`). Zero on a notify raised by anything
    /// else, which is most of them.
    ///
    /// Read one field at a time the way the remotes are: a decode in
    /// QML would be data handling, and that belongs on this side of
    /// the bridge (app-ui.md).
    #[qslot]
    fn write_answer_count(&self) -> i32 {
        i32::try_from(self.write_answers.len()).unwrap_or(i32::MAX)
    }

    /// What `write_seq` counted that answer at — how a reader that holds
    /// no id tells the answer to its own press from one counted before it.
    #[qslot]
    fn write_answer_seq(&self, index: i32) -> i32 {
        self.write_answer_at(index).map_or(0, |a| a.seq)
    }

    /// Where the answer to the press given `id` stands in this notify's
    /// list, or -1 where it did not answer in this notify — a reader
    /// holding an id asks this and reads nothing into the order or the
    /// count of what else answered.
    #[qslot]
    fn write_answer_index(&self, id: i32) -> i32 {
        u64::try_from(id)
            .ok()
            .and_then(|id| self.write_answer_index_of(id))
            .and_then(|index| i32::try_from(index).ok())
            .unwrap_or(-1)
    }

    /// The number the session named for the first report of HEAD after
    /// that write (`WriteAnswer::head_seq`): a landing on what the write
    /// left arms on it, and `WorkTreeModel.headSeq` at or above it is a
    /// report that looked after the write — whichever of the two this
    /// page reads first.
    #[qslot]
    fn write_answer_head_seq(&self, index: i32) -> i32 {
        self.write_answer_at(index)
            .map_or(0, |a| bridge_id(a.head_seq))
    }

    /// Which write answered. Raw data, the way `lastWriteError` is: what
    /// an answer *means* is the three below.
    #[qslot]
    fn write_answer_op(&self, index: i32) -> String {
        self.write_answer_at(index)
            .map(|a| a.kind.label().to_string())
            .unwrap_or_default()
    }

    /// Whether anybody pressed for that one, or it is a fetch the page
    /// made on its own (`asked_for` — the same set the write ids move
    /// by). A run reading answers one by one asks this; the word
    /// `writeAnswerOp` gives it speaks for the op.
    #[qslot]
    fn write_answer_asked(&self, index: i32) -> bool {
        self.write_answer_at(index)
            .is_some_and(|a| super::asked_for(a.kind))
    }

    /// **The next write this tab is asked to make is the one to wait for**
    /// (`write_watch`).
    ///
    /// A slot because arming is a thing done, at a moment: whoever calls
    /// this is about to put an input in, and the ask that input produces
    /// is the one kept — which is the only way the id is anybody's in
    /// particular. The watch keeps the number.
    #[qslot]
    pub(crate) fn watch_next_write(&mut self, what: String) -> String {
        self.write_watch.arm(&what).unwrap_or_default()
    }

    /// The input this run put in has gone. **From here the run is waiting
    /// on that write**, however it waits — the shared barrier or a sampler
    /// of its own, which is the run's own business.
    #[qslot]
    pub(crate) fn write_input_went(&mut self) {
        self.write_watch.input_went();
    }

    /// **This run is moving on from the write it pressed for without
    /// waiting it out**, which a composite operation has to say out loud
    /// (`write_watch`) — otherwise it reads as forgetting to wait.
    #[qslot]
    pub(crate) fn let_write_go(&mut self) {
        self.write_watch.let_go();
    }

    /// A breach the run saw and this side could not. Ends the run the same
    /// way one raised here does.
    #[qslot]
    pub(crate) fn break_write_contract(&mut self, breach: String) {
        self.write_watch.broke(&breach);
    }

    /// Whether the contract is broken — **final**: nothing arms past it
    /// and nothing completes.
    #[qslot]
    pub(crate) fn write_contract_broken(&self) -> bool {
        self.write_watch.broken()
    }

    /// Where the run stands with its own write, and what it called the
    /// press: `dispatched` / `input` / `pressed` / `broken`.
    #[qslot]
    pub(crate) fn write_run_stage(&self) -> String {
        self.write_watch.run_stage().to_string()
    }

    #[qslot]
    pub(crate) fn write_wanted(&self) -> String {
        self.write_watch.wanted().to_string()
    }

    /// Whether the write being watched has been through both of the
    /// boundaries behind it: git answered it, and everything it
    /// invalidated has been read again and published.
    ///
    /// **Matched by id.** Another write somebody asked for moves every
    /// count and every flag and still leaves this false, whichever way
    /// round the two were numbered (`write_watch`).
    #[qslot]
    pub(crate) fn wrote_through(&self) -> bool {
        self.write_watch.through()
    }

    /// Where the watch stands, and the id it is holding — the two halves
    /// of what a run that stopped answering has to say for itself
    /// (`AutoShotDriver` の watchdog). `asleep` / `armed` / `turned-down` /
    /// `held` / `answered` / `settled`, and 0 for "no ask has been taken".
    #[qslot]
    pub(crate) fn write_watch_stage(&self) -> String {
        self.write_watch.stage().to_string()
    }

    #[qslot]
    pub(crate) fn watched_write_id(&self) -> i32 {
        bridge_id(self.write_watch.id())
    }

    /// git stopped part-way through that one and left the operation
    /// standing.
    #[qslot]
    fn write_answer_stopped(&self, index: i32) -> bool {
        self.write_answer_at(index).is_some_and(|a| a.stopped)
    }

    /// It did not happen: git would not do it, or could not reach the far
    /// side to.
    #[qslot]
    fn write_answer_failed(&self, index: i32) -> bool {
        self.write_answer_at(index).is_some_and(|a| a.failed)
    }

    /// git's own words for that refusal, empty where it landed. Read off
    /// the answer itself — `lastWriteError` is whichever answer came
    /// last in the drain.
    #[qslot]
    fn write_answer_error(&self, index: i32) -> String {
        self.write_answer_at(index)
            .map(|a| a.error.clone())
            .unwrap_or_default()
    }

    /// It left a commit at the tip to go to.
    #[qslot]
    fn write_answer_at_tip(&self, index: i32) -> bool {
        self.write_answer_at(index).is_some_and(|a| a.at_tip)
    }

    /// That write did not happen and something outside this application
    /// said so — a protected branch, a repository rule, a hook over there
    /// or here. Which report it is (`delete` / `update` / `outdated` /
    /// `commit` …) is what the page's sentence turns on, and the four
    /// below are read together: empty here is "nothing to report", which
    /// is git's plain refusal and the command log's news
    /// (デザイン規約 §答えの要らない報せ).
    ///
    /// **Read off the answer**, because a report is the answer's own: a
    /// drain carrying two refusals would otherwise leave only the later
    /// one's words, and the earlier press would report whatever the
    /// other write was turned down for.
    #[qslot]
    fn write_answer_report_kind(&self, index: i32) -> String {
        self.write_answer_at(index)
            .map(|a| a.report_kind.clone())
            .unwrap_or_default()
    }

    /// The remote it was about, where one was involved.
    #[qslot]
    fn write_answer_report_remote(&self, index: i32) -> String {
        self.write_answer_at(index)
            .map(|a| a.report_remote.clone())
            .unwrap_or_default()
    }

    /// The ref it was about, spelled the way the screen spells it.
    #[qslot]
    fn write_answer_report_name(&self, index: i32) -> String {
        self.write_answer_at(index)
            .map(|a| a.report_name.clone())
            .unwrap_or_default()
    }

    /// Whoever said no, in their own words — carried across as they
    /// came, the way `lastWriteError` is.
    #[qslot]
    fn write_answer_report_reason(&self, index: i32) -> String {
        self.write_answer_at(index)
            .map(|a| a.report_reason.clone())
            .unwrap_or_default()
    }

    /// Local branch name a remote-tracking ref would take: the ref with
    /// its remote's prefix removed.
    ///
    /// Matched against the configured remotes — a remote may be
    /// named `my/fork`, and the longest matching prefix is the
    /// right one.
    #[qslot]
    fn local_name_for(&self, remote_ref: String) -> String {
        self.local_name_of(remote_ref)
    }

    #[qslot]
    fn attach(&mut self, tab_id: i32) {
        self.attach_feed(tab_id)
    }

    /// Called when this page becomes the visible one. A tab restored from
    /// the last session has no repository open behind it until then — the
    /// feeds are already attached, so the page simply stops saying
    /// "loading" once this fills them. Doing nothing on a tab that is
    /// already open is the normal case.
    #[qslot]
    fn activate(&mut self) {
        let id = self.tab_id;
        Hub::with(|hub| hub.ensure_open(id));
    }

    /// Called on the way off the front: the repository behind this tab is
    /// let go of, and read again the next time it is looked at
    /// (`Hub::release_tab`).
    #[qslot]
    fn release(&mut self) {
        let id = self.tab_id;
        Hub::with(|hub| hub.release_tab(id));
    }

    /// The tab is standing in another working copy of the repository it
    /// is showing (`Hub::restand_tab`), and the page is staying.
    ///
    /// **What is kept is what the repository itself answered**: who the
    /// tab is and what it is called, the identity and the signing
    /// settings, the remotes, the merge tools. Every one of those is the
    /// same on both sides of a linked copy, and dropping them would put
    /// the "no identity" warning and an empty remote list on screen for
    /// as long as it takes to read them again.
    ///
    /// **Everything else goes**, because it is either the copy's — where
    /// HEAD is, what it is in the middle of — or an answer that is never
    /// coming: a write the closed session accepted answers to a retired
    /// sink, and a count of writes in flight left standing would hold
    /// this page's doors shut for as long as it stood
    /// (`Hub::let_go_of_session`).
    #[qslot]
    fn restand(&mut self) {
        self.forget_the_copy();
        self.changed();
    }

    /// Hands the commit editor's unsent words to the hub, which is what
    /// holds them while this tab has no page (`hub::Draft`).
    #[qslot]
    fn hold_draft(&mut self, subject: String, body: String, amending: bool) {
        let id = self.tab_id;
        let draft = crate::hub::Draft {
            subject,
            body,
            amending,
        };
        Hub::with(|hub| hub.hold_draft(id, draft));
    }

    // Read back one field at a time: a decode in QML would be data
    // handling, which belongs on this side of the bridge
    // (app-ui.md).
    #[qslot]
    fn draft_subject(&self) -> String {
        Hub::with(|hub| hub.draft(self.tab_id).subject).unwrap_or_default()
    }

    #[qslot]
    fn draft_body(&self) -> String {
        Hub::with(|hub| hub.draft(self.tab_id).body).unwrap_or_default()
    }

    #[qslot]
    fn draft_amending(&self) -> bool {
        Hub::with(|hub| hub.draft(self.tab_id).amending).unwrap_or_default()
    }

    #[qslot]
    fn drain(&mut self) {
        self.take_feed()
    }

    /// Cheap refresh: refs + status + stashes (window focus, post-op).
    #[qslot]
    fn refresh_quick(&mut self) {
        self.with_session(|s| s.refresh_quick());
    }

    /// The tick the page runs while it is on screen: refs + status, and a
    /// graph rebuild only when one of them moved. Ticks that arrive while
    /// the session is busy are dropped there.
    #[qslot]
    fn refresh_poll(&mut self) {
        self.with_session(|s| s.refresh_poll());
    }

    /// The worktree listing, on the page's tick beside the reads above.
    /// One process and nothing else (measured, 23ms), which is what lets
    /// it ride a tick the `status` of another copy may not: what the
    /// listing feeds is the WORKTREES rows and the mark saying a branch
    /// is somebody else's, and a window kept open beside another copy
    /// showed both of those frozen until it was clicked.
    #[qslot]
    fn refresh_worktrees(&mut self) {
        self.with_session(|s| {
            s.refresh_worktrees();
        });
    }

    /// What the other copies are carrying, on a slower tick of its own —
    /// a whole `status` per copy, which is why the tick is its own
    /// (`RepoSession::refresh_carried`).
    #[qslot]
    fn refresh_carried(&mut self) {
        self.with_session(|s| s.refresh_carried());
    }

    /// What one other copy is holding, file by file — the read behind the
    /// pane, asked when a copy's row is selected and again on the copies'
    /// tick while it stands open.
    ///
    /// Apart from the tick above, which keeps every copy's tallies
    /// current: this is one copy's file list, and it has to be up the
    /// moment the pane opens.
    #[qslot]
    fn read_carried_status(&mut self, path: String, name: String) {
        self.with_session(|s| s.read_carried_status(path, name));
    }

    /// The other tick, run several times a second while a write that
    /// replays is out: how far it has got and nothing else. Two file reads
    /// and no process, which is what lets it be that often — the tick
    /// above carries a whole `git status` and is ten seconds apart
    /// (`RepoSession::refresh_op_progress`).
    #[qslot]
    fn refresh_op_progress(&mut self) {
        self.with_session(|s| s.refresh_op_progress());
    }

    /// Full refresh: restarts the log stream as well (manual refresh).
    #[qslot]
    fn refresh_all(&mut self) {
        self.with_session(|s| {
            s.restart_log();
            s.refresh_quick();
        });
    }

    #[qslot]
    fn clear_last_error(&mut self) {
        self.last_error = String::new();
        self.last_error_from_fetch = false;
        self.changed();
    }

    // --- write operations -----------------------------------------------
    //
    // Every one of these is fire-and-forget: the session serializes them,
    // reports progress through `busyCount` and routes git's own error text
    // into `lastError`. Paths arrive one per call (see `pending_paths`).

    #[qslot]
    fn stage_path(&mut self, path: String) {
        self.ask_session(|s| s.stage_paths(vec![path.clone()]));
    }

    #[qslot]
    fn unstage_path(&mut self, path: String) {
        self.ask_session(|s| s.unstage_paths(vec![path.clone()]));
    }

    /// The same two over the gathered set, for when several rows are
    /// highlighted and the affordance on one of them is pressed: one git
    /// command however many rows were chosen (デザイン規約 §その他の操作).
    #[qslot]
    fn stage_paths(&mut self) {
        self.drain_paths(|s, paths| s.stage_paths(paths));
    }

    #[qslot]
    fn unstage_paths(&mut self) {
        self.drain_paths(|s, paths| s.unstage_paths(paths));
    }

    /// Opens a set of paths for the next write, and adds to it. One call
    /// per path (see [`RepoTab::pending_paths`]); the write that follows
    /// takes them all in one git command, however many rows were chosen.
    #[qslot]
    fn begin_paths(&mut self) {
        self.pending_paths.clear();
    }

    #[qslot]
    fn add_path(&mut self, path: String) {
        self.pending_paths.push(path);
    }

    /// Works out what a discard of the gathered rows would take, before
    /// anything is written; the answer lands on `discardCount` /
    /// `discardOnly`. Rows arrive as `<bucket>:<path>` — the key the
    /// pane's choice already speaks — because which row was chosen is
    /// the choice's to say (`ops_stage::chosen_row`).
    #[qslot]
    fn plan_discard(&mut self) {
        self.plan_discard_rows();
        self.changed();
    }

    /// Discards the gathered rows, keyed the same way (destructive).
    /// Unstaged edits, untracked files and staged changes each go by
    /// their own command, and a staged rename takes the name it came
    /// from with it (`RepoSession::discard_chosen`).
    #[qslot]
    fn discard_rows(&mut self) {
        self.discard_chosen_rows()
    }

    #[qslot]
    fn stage_all(&mut self) {
        self.ask_session(|s| s.stage_all());
    }

    #[qslot]
    fn unstage_all(&mut self) {
        self.ask_session(|s| s.unstage_all());
    }

    /// `git add` over every conflicted path — the conflicted bucket's
    /// whole-bucket affordance. The set is read under the write lock, so
    /// no paths are gathered here.
    #[qslot]
    fn stage_conflicted(&mut self) {
        self.ask_session(|s| s.stage_conflicted());
    }

    /// Stages (or unstages) part of one file's diff, addressed by the row
    /// the user clicked. `kind` is the diff-key prefix (`unstaged` /
    /// `staged` / `untracked`); a negative `line` takes the whole hunk.
    /// Answers whether a write went out (see [`Self::stage_chosen`]) —
    /// the caller's wait is armed by this answer.
    ///
    /// A staged diff is unstaged by the same call — the direction follows
    /// from which side the file is being looked at.
    #[qslot]
    fn stage_selection(
        &mut self,
        kind: String,
        path: String,
        orig_path: String,
        hunk: i32,
        line: i32,
        fingerprint: String,
    ) -> bool {
        self.stage_chosen(kind, path, orig_path, hunk, line, fingerprint)
    }

    /// Throws away part of one file's unstaged diff, addressed the same way
    /// (destructive), answering the same way. Only the unstaged side has a
    /// piece to throw away: what is staged is unstaged first, by the
    /// affordance beside this one.
    #[qslot]
    fn discard_selection(
        &mut self,
        kind: String,
        path: String,
        orig_path: String,
        hunk: i32,
        line: i32,
        fingerprint: String,
    ) -> bool {
        self.discard_chosen(kind, path, orig_path, hunk, line, fingerprint)
    }

    /// Commits the index from the editor's two fields. Both empty is only
    /// valid with `amend`, where it keeps the existing message.
    ///
    /// `reset_author` only means anything on an amend: it puts the current
    /// identity on a commit written under another one.
    #[qslot]
    fn commit(&mut self, subject: String, body: String, amend: bool, reset_author: bool) {
        self.commit_from_fields(subject, body, amend, reset_author);
    }

    /// Reads HEAD's message and author (an amend starts from them).
    #[qslot]
    fn request_head_commit(&mut self) {
        self.with_session(|s| s.load_head_commit());
    }

    // Every move is one job, whichever way it gets there: a failed half
    // cannot let the switch happen regardless.

    /// `leaving` says the reader has already agreed to undo the operation
    /// standing in the move's way — git refuses every move while one
    /// stands, so that agreement travels with the move
    /// (デザイン規約 §進行中の操作から出る).
    #[qslot]
    fn checkout_branch(&mut self, name: String, leaving: bool) {
        let target = platitude_core::branch::CheckoutTarget::Branch { name };
        self.move_head(target, leaving);
    }

    /// Creates a local branch tracking a remote-tracking ref and switches.
    #[qslot]
    fn checkout_remote(&mut self, remote_ref: String, local: String, leaving: bool) {
        let target = platitude_core::branch::CheckoutTarget::Track { remote_ref, local };
        self.move_head(target, leaving);
    }

    /// Moves an existing local branch to `start` and lands on it, but only
    /// where nothing is lost by it: a branch that has merely fallen behind
    /// fast-forwards straight away, and one holding commits of its own
    /// comes back as `moveAskSeq` for the UI to ask about.
    #[qslot]
    fn checkout_moving_branch(&mut self, local: String, start: String, leaving: bool) {
        self.ask_session(|s| s.checkout_moving_branch(local.clone(), start.clone(), leaving));
    }

    /// Moves an existing local branch to `start` and lands on it. What the
    /// branch alone had is left unreferenced, so the UI asks first.
    #[qslot]
    fn checkout_force_create(&mut self, local: String, start: String, leaving: bool) {
        let target = platitude_core::branch::CheckoutTarget::ForceCreate { local, start };
        self.move_head(target, leaving);
    }

    /// Moves the current branch to `rev`. `mode` says what becomes of the
    /// index and the working tree: `"soft"` leaves both alone (the skipped
    /// commits end up staged), `"mixed"` clears the index, `"hard"` throws
    /// away everything uncommitted.
    #[qslot]
    fn reset_to(&mut self, rev: String, mode: String) {
        self.reset_head(rev, mode);
    }

    /// Creates a branch at `start_point` (HEAD when empty).
    #[qslot]
    fn create_branch(&mut self, name: String, start_point: String, switch_to: bool) {
        let start = (!start_point.is_empty()).then_some(start_point);
        self.ask_session(|s| s.create_branch(name.clone(), start.clone(), switch_to));
    }

    /// Deletes a local branch, and takes its row off the screen behind the
    /// press (`ops_delete`). Without `force`, git refuses an unmerged one
    /// — and that answer is the menu's to catch, so the plain form is
    /// written down first (`branch_delete`).
    #[qslot]
    fn delete_branch(&mut self, name: String, force: bool) {
        self.branch_delete(name, force)
    }

    #[qslot]
    fn rename_branch(&mut self, from: String, to: String, force: bool) {
        self.ask_session(|s| s.rename_branch(from.clone(), to.clone(), force));
    }

    /// Records the remote branch `branch` is measured against. The two
    /// halves the question was answered with go down as they were
    /// answered: **a name this repository has not fetched is one of the
    /// answers**, and which of git's two writes records it is core's to
    /// decide (`branch::set_upstream`).
    #[qslot]
    fn set_upstream(&mut self, branch: String, remote: String, remote_branch: String) {
        if branch.is_empty() || remote.is_empty() || remote_branch.is_empty() {
            return;
        }
        self.ask_session(|s| s.set_upstream(branch.clone(), remote.clone(), remote_branch.clone()));
    }

    /// Puts a lightweight tag on `commit` (HEAD when empty). Always
    /// plain: a name already taken is git's to refuse.
    #[qslot]
    fn create_tag(&mut self, name: String, commit: String) {
        self.ask_session(|s| s.create_tag(name.clone(), commit.clone()));
    }

    /// Sends one tag to one remote. `lease_expect` pins a leased
    /// overwrite to the commit that remote was last seen holding the name
    /// on; empty sends it plain.
    #[qslot]
    fn push_tag(&mut self, remote: String, tag: String, lease_expect: String) {
        self.ask_session(|s| s.push_tag(remote.clone(), tag.clone(), lease_expect.clone()));
    }

    /// Replaces a tag on a remote with one under a new name. git has no
    /// command for it, so core pushes the new name and deletes the old —
    /// the UI holds the answer down first, because the old name is
    /// destroyed.
    #[qslot]
    fn replace_remote_tag(&mut self, remote: String, from: String, to: String) {
        let asked =
            self.ask_session(|s| s.replace_remote_tag(remote.clone(), from.clone(), to.clone()));
        // Keyed the way every other tag write that leaves this machine is
        // (`ops_delete::remote_tag_delete`), and on the name that goes:
        // git's answer to **this** press is what the page reports.
        self.ref_push_asked(
            &format!("{remote}/{from}"),
            asked.map(platitude_core::OperationId::as_u64),
        );
    }

    /// The remote's copy of a tag. `rowGoes` is whether the sidebar's row
    /// leaves with it — true where only the remote had the name
    /// (`ops_delete::remote_tag_delete`).
    #[qslot]
    fn delete_remote_tag(&mut self, remote: String, tag: String, row_goes: bool) {
        self.remote_tag_delete(remote, tag, row_goes);
    }

    /// Both copies of a tag, as one queued write.
    #[qslot]
    fn delete_tag_everywhere(&mut self, tag: String, remote: String) {
        self.tag_delete_everywhere(tag, remote);
    }

    /// Renames a tag. git has none, so core builds it out of a new name on
    /// the same object and a delete of the old one.
    #[qslot]
    fn rename_tag(&mut self, from: String, to: String) {
        self.ask_session(|s| s.rename_tag(from.clone(), to.clone()));
    }

    /// Deletes a tag: only the name goes.
    #[qslot]
    fn delete_tag(&mut self, name: String) {
        self.tag_delete(name);
    }

    /// Renames a stash entry. Built the same way, out of a re-store and a
    /// drop — so the entry moves to the top of the list.
    #[qslot]
    fn rename_stash(&mut self, selector: String, message: String) {
        self.ask_session(|s| s.rename_stash(selector.clone(), message.clone()));
    }

    /// `git stash push -u` over the whole working tree, straight off the
    /// press (デザイン規約 §変更を退避する).
    ///
    /// `message` is what the commit box already had in it, so the entry
    /// arrives named without anything being asked; empty names it the way
    /// git does.
    #[qslot]
    fn push_stash(&mut self, message: String) {
        self.stash_push(message)
    }

    /// `git stash push -- <paths>`: puts the gathered files' changes away
    /// and leaves the rest of the working tree as it is.
    ///
    /// Untracked files are included, since a path the user pointed at is
    /// meant to go whether or not git is tracking it yet.
    #[qslot]
    fn stash_paths(&mut self, message: String) {
        self.stash_chosen_paths(message)
    }

    /// `git stash pop` on the given selector (stash-row action). Answers
    /// with the id the queue accepted it under, so the page can find the
    /// pop's own answer among the stash answers that look alike
    /// (`writeAnswerIndex`); zero where no session took it.
    #[qslot]
    fn pop_stash(&mut self, selector: String) -> i32 {
        let asked = self.ask_session(|s| s.stash_pop(selector.clone()));
        asked.map_or(0, |id| bridge_id(id.as_u64()))
    }

    /// `git stash apply` on the given selector (keeps the stash). No id
    /// comes back: an apply leaves the entry where it is, so nothing on
    /// the page is waiting to hear which answer was this one's.
    #[qslot]
    fn apply_stash(&mut self, selector: String) {
        self.ask_session(|s| s.stash_apply(selector.clone()));
    }

    /// `git stash drop` (destructive): the entry's row goes at the press.
    #[qslot]
    fn drop_stash(&mut self, selector: String) {
        self.stash_drop(selector);
    }

    /// One of the sidebar's lists has drawn a reading. What that proves
    /// about a delete out at the press is the tab's to decide
    /// (`ops_delete::note_listing_drawn`) — this is the page saying
    /// only that a list has caught up: every row answers off the list
    /// that draws it.
    #[qslot]
    fn listing_drawn(&mut self) {
        self.note_listing_drawn();
    }

    /// A status has been applied, whose counts stood beside the report of
    /// HEAD numbered `seen` and left the working tree empty or not.
    ///
    /// **Written down before anything asks what it means**, because
    /// either half of a pair can arrive first: the status a write
    /// published can be applied before the page has read that write's
    /// answer (the two are drained apart), and a status thrown away there
    /// is a tree emptying nothing will ever account for
    /// (`ops::StashOut`, `ops::DiffReread`).
    ///
    /// **The counts' own number** (`WorkTreeModel.statusSeq`) — a report
    /// of HEAD arrives on its own as well, carrying no counts at all,
    /// and read as one of these it would speak for a tree it
    /// never saw.
    #[qslot]
    fn note_tree_read(&mut self, seen: i32, emptied: bool) {
        let Ok(seen) = u64::try_from(seen) else {
            return;
        };
        self.tree_was_read(seen, emptied);
    }

    /// The page is acting on the working tree it has: answers whether
    /// this window's own stash is what emptied it, and settles that press
    /// (`ops::StashOut`).
    ///
    /// **Asked from both sides of the pair** — where a status lands and
    /// where a write answers — because either can be the half that
    /// completes it. It answers only once both are in, so the side that
    /// arrives first asks and gets nothing.
    ///
    /// **Asked, and asking spends it.** The question has an answer
    /// only at the moment the page is acting on a tree, and asking ends
    /// the wait, so there is nothing here for a binding to follow.
    #[qslot]
    fn take_stash_landing(&mut self) -> bool {
        self.stash_landing_taken()
    }

    /// The open file was read again because a write answered, so the
    /// status that write publishes behind it is not news
    /// (`ops::DiffReread`).
    ///
    /// Measured against the number that write's answer named for the
    /// first report of HEAD after it — the answers this notify carried
    /// are read for once, so the fence is the last of them. A notify
    /// that carried none arms nothing: there is no write for a status to
    /// be the answer to.
    #[qslot]
    fn note_diff_read(&mut self) {
        self.read_diff_for_answers();
    }

    /// The page is reading the status it has just been given: says
    /// whether the re-read standing here already answers for it, and
    /// spends the re-read where it does.
    ///
    /// Measured against the tree as this tab last had it written down
    /// (`noteTreeRead`), so the two agree about which status is
    /// in hand. **Asked**, like the stash's landing.
    #[qslot]
    fn take_diff_read(&mut self) -> bool {
        self.diff_reread_taken()
    }

    /// `git fetch --prune`; an empty remote fetches all of them.
    #[qslot]
    fn fetch(&mut self, remote: String) {
        let remote = (!remote.is_empty()).then_some(remote);
        self.ask_session(|s| s.fetch(remote.clone()));
    }

    /// `git pull`, asked from either of the two rows that offer it.
    ///
    /// **Neither row names anything**: the branch the working tree is on
    /// and the upstream it is measured against are the two ends of one
    /// comparison, and git resolves that comparison itself
    /// (デザイン規約 §取り込んで合流させる). Which way the far side is
    /// brought in is git's own setting (`remote::pull`).
    #[qslot]
    fn pull(&mut self) {
        self.ask_session(platitude_core::session::RepoSession::pull);
    }

    /// Pushes the branch that is checked out to wherever it tracks, or to
    /// the default remote when it tracks nothing yet. `force` is `""` /
    /// `"lease"` / `"force"`; `lease_expect` pins the remote commit the
    /// user actually saw. `branch` is what the button is sending, as the
    /// page shows it — written down with the id the press is accepted
    /// under, so the answer comes back to this branch
    /// (`ops::PushOut`).
    #[qslot]
    fn push_current(&mut self, branch: String, force: String, lease_expect: String) {
        let fallback = self.default_remote.clone();
        let force = Self::push_force(&force, &lease_expect);
        let asked = self.ask_session(|s| s.push_current(fallback.clone(), force.clone()));
        self.push_out
            .asked(asked.map(platitude_core::OperationId::as_u64), branch);
    }

    /// The first push of a branch, to the remote and name the question
    /// just took. Records the answer as the upstream, so the branch never
    /// asks again. `branch` is the one being published, written down the
    /// way `pushCurrent` writes its own.
    #[qslot]
    fn publish_current(
        &mut self,
        branch: String,
        remote: String,
        remote_branch: String,
        expect: String,
    ) {
        let asked = self.ask_session(|s| {
            s.publish_current(remote.clone(), remote_branch.clone(), expect.clone())
        });
        self.push_out
            .asked(asked.map(platitude_core::OperationId::as_u64), branch);
    }

    /// `git remote add <name> <url>`. Contacts nothing — a URL that goes
    /// nowhere is recorded just the same, and the push finds out.
    #[qslot]
    fn add_remote(&mut self, name: String, url: String) {
        self.ask_session(|s| s.add_remote(name.clone(), url.clone()));
    }

    /// The fetch URL a remote is written down with, empty for a name this
    /// repository does not have. What the form that corrects one opens
    /// with already filled in.
    #[qslot]
    fn remote_url(&self, name: String) -> String {
        self.remotes
            .iter()
            .position(|r| *r == name)
            .and_then(|at| self.remote_urls.get(at))
            .cloned()
            .unwrap_or_default()
    }

    /// Marks a remote as origin — `remote.pushDefault` and
    /// `checkout.defaultRemote` both; an empty name clears the two.
    /// Clearing reaches this repository's config only — a mark set for
    /// every repository stays, and marking another remote is what moves it.
    #[qslot]
    fn mark_origin(&mut self, name: String) {
        self.ask_session(|s| s.mark_origin(name.clone()));
    }

    /// `git remote set-url <name> <url>` — the way back from a typo.
    #[qslot]
    fn set_remote_url(&mut self, name: String, url: String) {
        self.ask_session(|s| s.set_remote_url(name.clone(), url.clone()));
    }

    /// Asks what a push under this branch name would meet on that remote.
    /// Reaches the network, so it is asked while the question stands.
    /// The answer arrives as `remoteBranchAsked()` on the next
    /// `remoteBranchRevision`, with `remoteBranchState`.
    #[qslot]
    fn check_remote_branch(&mut self, remote: String, branch: String) {
        self.look_up_remote_branch(remote, branch)
    }

    /// The pair the last answer is about — what `checkRemoteBranch` was
    /// asked for — and nothing from the moment a question goes out until
    /// its answer lands. Read on `remoteBranchRevision`.
    #[qslot]
    fn remote_branch_asked(&self) -> Optional<RemoteBranch> {
        self.remote_branch_asked.clone()
    }

    /// Asks git whether `branch --delete` would refuse this branch (not
    /// merged into its upstream, or HEAD without one), so a menu's
    /// delete row can wear `-D` from the start. The answer arrives as
    /// `branchDeleteAsked` / `branchDeleteMerged`, and it arrives even
    /// where the reads fail — as `"unknown"`, which the row draws in its
    /// plain form the way it draws a merged branch.
    ///
    /// **The slow way, for the names the rows cannot answer**: the menu
    /// asks the graph first (`GraphModel.reaches`), which answers in the
    /// same frame for every branch the window draws, and comes here only
    /// for a tip or a reference point older than the window.
    #[qslot]
    fn check_branch_delete(&mut self, branch: String) {
        self.look_up_branch_delete(branch)
    }

    /// `git push`. `force` is `""` / `"lease"` / `"force"`; `lease_expect`
    /// pins the remote commit the user saw (empty = bare lease).
    #[qslot]
    fn push_branch(
        &mut self,
        remote: String,
        local: String,
        remote_branch: String,
        set_upstream: bool,
        force: String,
        lease_expect: String,
    ) {
        self.branch_push(
            remote,
            local,
            remote_branch,
            set_upstream,
            force,
            lease_expect,
        )
    }

    /// Points a branch at a remote branch and sends it there, in that
    /// order — the answer to a rename here that takes nothing away over
    /// there.
    #[qslot]
    fn point_upstream_and_push(&mut self, branch: String, remote: String, remote_branch: String) {
        if branch.is_empty() || remote.is_empty() || remote_branch.is_empty() {
            return;
        }
        let row = format!("{remote}/{remote_branch}");
        let asked = self.ask_session(|s| {
            s.point_upstream_and_push(branch.clone(), remote.clone(), remote_branch.clone())
        });
        self.ref_push_asked(&row, asked.map(platitude_core::OperationId::as_u64));
    }

    /// Replaces a branch on a remote with one under a new name. git has no
    /// command for it, so core pushes the new name and deletes the old —
    /// the UI holds the answer down first, because the old name is
    /// destroyed.
    #[qslot]
    fn replace_remote_branch(&mut self, remote: String, from: String, to: String) {
        let row = format!("{remote}/{from}");
        let asked =
            self.ask_session(|s| s.replace_remote_branch(remote.clone(), from.clone(), to.clone()));
        self.ref_push_asked(&row, asked.map(platitude_core::OperationId::as_u64));
    }

    /// `git push <remote> --delete <branch>` (destructive).
    #[qslot]
    fn delete_remote_branch(&mut self, remote: String, branch: String) {
        self.remote_branch_delete(remote, branch);
    }

    /// `git branch --delete` (`-D` under `force`) and then
    /// `git push <remote> --delete`, as one queued write: the local half
    /// refuses first where it refuses at all, leaving the remote
    /// as it was.
    #[qslot]
    fn delete_branch_everywhere(
        &mut self,
        branch: String,
        remote: String,
        remote_branch: String,
        force: bool,
    ) {
        self.branch_delete_everywhere(branch, remote, remote_branch, force)
    }

    /// `git merge <rev>`.
    #[qslot]
    fn merge(&mut self, rev: String, no_ff: bool, ff_only: bool, message: String) {
        let options = platitude_core::integrate::MergeOptions {
            no_ff,
            ff_only,
            squash: false,
            message: (!message.trim().is_empty()).then_some(message),
        };
        self.ask_session(|s| s.merge(rev.clone(), options.clone()));
    }

    /// `git rebase <upstream>`; an empty `onto` uses `upstream` as the base.
    ///
    /// Uncommitted work is carried across by core, the same way every
    /// other rewrite here carries it
    /// (デザイン規約 §未コミット変更がある状態で履歴を書き換える).
    #[qslot]
    fn rebase(&mut self, upstream: String, onto: String, update_refs: bool) {
        let options = platitude_core::integrate::RebaseOptions {
            onto: (!onto.is_empty()).then_some(onto),
            branch: None,
            update_refs,
            root: false,
        };
        self.ask_session(|s| s.rebase(upstream.clone(), options.clone()));
    }

    #[qslot]
    fn cherry_pick(&mut self, rev: String) {
        self.ask_session(|s| s.cherry_pick(vec![rev.clone()]));
    }

    /// Folds a commit into its parent (one-commit interactive rebase).
    #[qslot]
    fn squash_into_parent(&mut self, oid: String) {
        self.ask_session(|s| s.squash_into_parent(oid.clone()));
    }

    /// Leaves one commit out of the history, replaying what came after it.
    #[qslot]
    fn drop_commit(&mut self, oid: String) {
        self.ask_session(|s| s.drop_commit(oid.clone()));
    }

    /// Replaces one commit's message. HEAD is amended; anything older is
    /// replayed, which rewrites every commit after it.
    #[qslot]
    fn reword_commit(&mut self, oid: String, subject: String, body: String) {
        let message = platitude_core::commit::join_message(&subject, &body);
        if message.is_empty() {
            return;
        }
        self.ask_session(|s| s.reword(oid.clone(), message.clone()));
    }

    #[qslot]
    fn revert(&mut self, rev: String) {
        self.ask_session(|s| s.revert(vec![rev.clone()]));
    }

    /// Continues / aborts / skips whatever is in progress. `how` is
    /// `"continue"` / `"abort"` / `"skip"` / `"quit"`.
    #[qslot]
    fn resolve_operation(&mut self, how: String) {
        self.settle_operation(how)
    }

    /// Resolves the gathered conflicted paths by taking one side
    /// (`"ours"`/`"theirs"`), in one git command however many were chosen.
    ///
    /// Which branch each side is called is `sideOurs` / `sideTheirs` on
    /// the working-tree model — during a rebase the two swap over, so the
    /// wording cannot be worked out from the flag alone.
    #[qslot]
    fn take_side_paths(&mut self, side: String) {
        self.take_side(side)
    }

    /// Opens the chosen conflicted paths in the configured merge tool, the
    /// same path-set route the rest of the file menu takes.
    ///
    /// Named paths only: git walks a bare `mergetool` one file at a
    /// time and the whole walk holds the write queue, so an unnamed
    /// launch would block every other write for as many tool
    /// sessions as there are conflicts.
    #[qslot]
    fn open_mergetool(&mut self) {
        self.run_merge_tool()
    }

    /// Records which merge tool to launch; empty clears the choice.
    #[qslot]
    fn set_merge_tool(&mut self, tool: String) {
        self.ask_session(|s| s.set_merge_tool(tool.clone()));
    }

    /// Asks for the configured merge tool; the answer arrives on the
    /// working-tree model's `mergeTool`. The status refresh only names it
    /// where something is conflicted, so a settings field has to ask.
    #[qslot]
    fn ask_merge_tool(&mut self) {
        self.with_session(|s| s.ask_merge_tool());
    }

    /// Asks which tools could be offered; the answer lands on `mergeTools`.
    /// Off the write queue, and slow enough that `mergeToolsLoading` is
    /// worth showing while it runs.
    #[qslot]
    fn ask_merge_tools(&mut self) {
        self.list_merge_tools()
    }

    /// Asks what git makes of `oid_hex`'s signature; the answer arrives as
    /// `signatureOid` / `signatureKind` / `signatureCode` / `signatureSigner`.
    #[qslot]
    fn check_signature(&mut self, oid_hex: String) {
        self.look_up_signature(oid_hex)
    }

    /// Records `user.name` / `user.email`. `global` writes the user's own
    /// configuration, which is the right default for a first-run prompt:
    /// the answer is about the person.
    #[qslot]
    fn set_identity(&mut self, name: String, email: String, global: bool) {
        self.write_identity(name, email, global)
    }

    /// Shows/hides tags in the graph walk (restarts the stream).
    #[qslot]
    fn set_tags_shown(&mut self, shown: bool) {
        self.write_tags_shown(shown)
    }
}
qml_register!(RepoTab, "RepoTab", singleton = false);
