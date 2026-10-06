//! The whole of what QML sees of `RepoTab`, and as short as this file
//! gets without dropping slots.
//!
//! The `#[qobject]` block cannot be divided (structure.md §分割), so what
//! is left is the face — `qproperty!`, slot and signal signatures, and
//! their doc — over bodies that shape their arguments and forward once.
//! Every body that could be delegated has been: the feed's answers to
//! `drain`, staging and discard to `ops_stage`, conflicts to
//! `ops_conflict`, remotes to `ops_remote`, config to `ops_config`,
//! deletes to `ops_delete`, standing in another copy to `restand`, commit
//! and reset to `state`, the awaited write to `write_watch`.
//!
//! The only way shorter is fewer slots: a pure rule that reads nothing
//! but its arguments belongs in `GitFacts`, not here.

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
    // The pair itself is `remoteBranchAsked()` (an `Optional` cannot be a
    // property); bindings follow the revision.
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
    // git's answer to the plain delete a card stayed up for, by name
    // (`RefBranchMenu`), and where it stands in this notify, -1 for none
    // (`ops::BranchDeleteOut`).
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
    // The rows a delete has taken off the screen, one name per list
    // (デザイン規約 §消す操作は先に画面から消す); when they go and come back
    // is `ops::StandIn`'s.
    qproperty!("goneBranch", Member = gone_branch, Notify = changed);
    qproperty!("goneRemote", Member = gone_remote, Notify = changed);
    qproperty!("goneTag", Member = gone_tag, Notify = changed);
    qproperty!("goneStash", Member = gone_stash, Notify = changed);
    qproperty!("goneWorktree", Member = gone_worktree, Notify = changed);
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
    // Every change to what the discard log lists (`TabMsg::DiscardsChanged`),
    // and the same less the restores.
    qproperty!("discardSeq", Member = discard_seq, Notify = changed);
    qproperty!("takenSeq", Member = taken_seq, Notify = changed);
    // Restores that brought something back, and how the last one's work
    // came back: "whole" | "unstaged" | "stash".
    qproperty!("restoreSeq", Member = restore_seq, Notify = changed);
    qproperty!("restoreHow", Member = restore_how, Notify = changed);
    // This tree's paced reads that have ended, and the other copies whose
    // paced read ended in this notify (`TabMsg::PacedRead`).
    qproperty!("pacedSeq", Member = paced_seq, Notify = changed);
    qproperty!("pacedCopies", Member = paced_copies, Notify = changed);
    // Where the editor's own commit answered in this notify, or -1 —
    // matched by the press's id, whatever order the answers came in
    // (`ops::Press`).
    qproperty!("commitAnswer", Member = commit_answer, Notify = changed);
    // …and where the stash press that took the working tree away
    // answered. Its other wait — the tree itself — is asked for
    // (`takeStashLanding`).
    qproperty!("stashAnswer", Member = stash_answer, Notify = changed);
    // …and where the toolbar's push answered, with the branch it was sent
    // for: refusals are remembered per branch and an answer does not name
    // it (`ops::PushOut`).
    qproperty!("pushAnswer", Member = push_answer, Notify = changed);
    qproperty!(
        "pushAnswerBranch",
        Member = push_answer_branch,
        Notify = changed
    );
    // …and the same pair for a ref row's pushes, which answer under the
    // same word (`ops::PushOut`).
    qproperty!("refPushAnswer", Member = ref_push_answer, Notify = changed);
    qproperty!("refPushTarget", Member = ref_push_target, Notify = changed);
    // …and where the working copy a menu asked for answered, with the
    // folder it was to be made in (`ops::Press`).
    qproperty!("copyAnswer", Member = copy_answer, Notify = changed);
    qproperty!(
        "copyAnswerPath",
        Member = copy_answer_path,
        Notify = changed
    );
    // Meanings, classified in `drain::settle_write`, of the last answer in
    // this notify nobody was waiting for by name; a reader waiting for one
    // answer reads the `writeAnswer*` slots.
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
    // What a write that did not happen says for itself, for the page's
    // notice (デザイン規約 §答えの要らない報せ).
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

    /// The copy this tab stands in would not open, and the repository has
    /// its own to stand in instead (`Hub::home_copy`); the strip moves the
    /// tab (`TabsModel::standTabHome`).
    ///
    /// Whoever hears it answers a turn later (`RepoPage`): it is emitted
    /// mid-drain, and moving the tab takes this page down
    /// (rules/app-ui.md §Qt Bridges・QML の不変条件 — 再入 borrow).
    #[qsignal]
    pub(super) fn stand_home_asked(&mut self);

    /// The first fetch of a run to fail — only the first, or an offline
    /// machine reopens the panel every interval.
    #[qsignal]
    pub(super) fn fetch_first_failed(&mut self);

    /// Starts the timer again on the interval it was set to, and fetches
    /// now — the hold on the toolbar button is what reaches this.
    #[qslot]
    fn resume_auto_fetch(&mut self) {
        self.restart_auto_fetch()
    }

    /// Re-reads the identity's assigned picture after an assignment
    /// changes; the identity did not, so nothing is asked of git.
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
    /// (`RepoTab::write_answers`), read one field at a time by the slots
    /// below (rules/app-ui.md「QML は表示とインタラクションだけ」).
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
    /// list, or -1 — a reader holding an id reads nothing into the order
    /// or count of the rest.
    #[qslot]
    fn write_answer_index(&self, id: i32) -> i32 {
        u64::try_from(id)
            .ok()
            .and_then(|id| self.write_answer_index_of(id))
            .and_then(|index| i32::try_from(index).ok())
            .unwrap_or(-1)
    }

    /// The number of the first HEAD report after that write
    /// (`WriteAnswer::head_seq`): `WorkingTreeModel.headSeq` at or above it
    /// looked after the write, whichever of the two arrives first.
    #[qslot]
    fn write_answer_head_seq(&self, index: i32) -> i32 {
        self.write_answer_at(index)
            .map_or(0, |a| bridge_id(a.head_seq))
    }

    /// The smallest stamp a listing read after that write can carry
    /// (`WriteAnswer::reads_from`): a section whose `listingLooked` is at
    /// or above it shows what the write left. Past the end, a stamp no
    /// listing reaches — the resting value is the failing one.
    #[qslot]
    fn write_answer_reads_from(&self, index: i32) -> i32 {
        self.write_answer_at(index)
            .map_or(i32::MAX, |a| bridge_id(a.reads_from))
    }

    /// Which write answered, as raw data; what it means is
    /// `writeAnswerStopped` / `writeAnswerFailed` / `writeAnswerAtTip`.
    #[qslot]
    fn write_answer_op(&self, index: i32) -> String {
        self.write_answer_at(index)
            .map(|a| a.kind.label().to_string())
            .unwrap_or_default()
    }

    /// Whether anybody pressed for that one, or it is a fetch the page
    /// made on its own (`asked_for`, the set the write ids move by).
    #[qslot]
    fn write_answer_asked(&self, index: i32) -> bool {
        self.write_answer_at(index)
            .is_some_and(|a| super::asked_for(a.kind))
    }

    /// The next write this tab is asked to make is the one to wait for
    /// (`write_watch`); answers the breach, empty where arming is fine.
    /// Called just before the input goes in, so the ask it produces is
    /// the one kept.
    #[qslot]
    pub(crate) fn watch_next_write(&mut self, what: String) -> String {
        self.write_watch.arm(&what).unwrap_or_default()
    }

    /// The run's input has gone in; from here the run is waiting on that
    /// write, however it waits.
    #[qslot]
    pub(crate) fn write_input_went(&mut self) {
        self.write_watch.input_went();
    }

    /// The run moves on from the write it pressed for without waiting it
    /// out — a composite must say so, or it reads as forgetting to wait
    /// (`write_watch`).
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

    /// Whether the contract is broken — final: nothing arms or completes
    /// past it.
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

    /// Whether the watched write is through both boundaries: git answered
    /// it, and everything it invalidated has been read again and
    /// published. Matched by id (`write_watch`) — another write moves
    /// every count and flag and leaves this false.
    #[qslot]
    pub(crate) fn wrote_through(&self) -> bool {
        self.write_watch.through()
    }

    /// Where the watch stands and the id it holds, for a stalled run's line
    /// (`AutoShotDriver` の watchdog): `asleep` / `armed` / `turned-down` /
    /// `held` / `answered` / `settled`, and id 0 for "no ask taken".
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

    /// Which report an outside refusal made (a protected branch, a
    /// repository rule, a hook): `delete` / `update` / `outdated` /
    /// `commit` …, read with the three below. Empty is git's plain
    /// refusal, the command log's news (デザイン規約 §答えの要らない報せ).
    ///
    /// Read off the answer, or a drain with two refusals gives the earlier
    /// press the later one's words.
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

    /// Local branch name a remote-tracking ref would take: the ref minus
    /// the longest matching configured remote (a remote may be `my/fork`).
    #[qslot]
    fn local_name_for(&self, remote_ref: String) -> String {
        self.local_name_of(remote_ref)
    }

    #[qslot]
    fn attach(&mut self, tab_id: i32) {
        self.attach_feed(tab_id)
    }

    /// Called when this page becomes the visible one: a restored tab opens
    /// its repository here; an open one does nothing.
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
    /// is showing (`Hub::restand_tab`), and the page is staying. What goes
    /// and what stays is `forget_the_copy`'s.
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

    // Read back one field at a time, like the write answers.
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

    /// Refs, status, the listings and a pass over the other copies, with
    /// one walk (`RepoSession::refresh_quick`) — for the harness; a page on
    /// screen is read at its pace (`setPaced`).
    #[qslot]
    fn refresh_quick(&mut self) {
        self.with_session(|s| s.refresh_quick());
    }

    /// Whether the page is on screen to be read for: this tree and the
    /// other copies are then read on the session's pace
    /// (`RepoSession::set_paced`), and each read that ends says so
    /// (`pacedRead`).
    #[qslot]
    fn set_paced(&mut self, paced: bool) {
        self.with_session(|s| s.set_paced(paced));
    }

    /// The window came back: this tree now, with its stashes
    /// (`RepoSession::poll_now`). The other copies keep their pace.
    #[qslot]
    fn poll_now(&mut self) {
        self.with_session(|s| s.poll_now());
    }

    /// What one other copy is holding, file by file — the pane's read,
    /// asked when a copy's row is selected and when the window comes back.
    /// It stands the pane on that copy: each read of the copy hands the
    /// pane its list from there on.
    #[qslot]
    fn read_carried_status(&mut self, path: String, name: String) {
        self.with_session(|s| s.read_carried_status(path, name));
    }

    /// The pane is about this window's own tree again: no read of the
    /// copies hands it another copy's list
    /// (`RepoSession::leave_carried_status`).
    #[qslot]
    fn leave_carried_status(&mut self) {
        self.with_session(|s| s.leave_carried_status());
    }

    /// The fast tick while a replaying write is out: how far it has got,
    /// from two file reads and no process
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
    // The session serializes these and reports progress through
    // `busyCount`. Paths arrive one per call (see `pending_paths`).

    #[qslot]
    fn stage_path(&mut self, path: String) {
        self.ask_session(|s| s.stage_paths(vec![path.clone()]));
    }

    #[qslot]
    fn unstage_path(&mut self, path: String) {
        self.ask_session(|s| s.unstage_paths(vec![path.clone()]));
    }

    /// The same two over the gathered set: one git command however many
    /// rows were chosen (デザイン規約 §その他の操作).
    #[qslot]
    fn stage_paths(&mut self) {
        self.drain_paths(|s, paths| s.stage_paths(paths));
    }

    #[qslot]
    fn unstage_paths(&mut self) {
        self.drain_paths(|s, paths| s.unstage_paths(paths));
    }

    /// Opens a set of paths for the next write, filled one per call by
    /// `addPath` (see [`RepoTab::pending_paths`]).
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
    /// `discardOnly`. Rows arrive as `<bucket>:<path>`, the key the pane's
    /// choice already speaks (`ops_stage::chosen_row`).
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

    /// Stages part of one file's diff — or unstages it, for a `staged`
    /// `kind` — addressed by the clicked row. `kind` is the diff-key prefix
    /// (`unstaged` / `staged` / `untracked`); a negative `line` takes the
    /// whole hunk. Answers whether a write went out
    /// (see [`Self::stage_chosen`]); the caller's wait is armed by it.
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

    /// `leaving`: the reader has already agreed to undo the operation in
    /// the move's way, which git would otherwise refuse
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

    /// Moves the current branch to `rev`; `mode` is git's `"soft"` /
    /// `"mixed"` / `"hard"`.
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

    /// Records the remote branch `branch` is measured against, as
    /// answered — an unfetched name included; which of git's two writes
    /// records it is core's to decide (`branch::set_upstream`).
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
    /// destroyed. `expect` is the commit the old name was shown on, which
    /// its delete is leased to.
    #[qslot]
    fn replace_remote_tag(&mut self, remote: String, from: String, to: String, expect: String) {
        let asked = self.ask_session(|s| {
            s.replace_remote_tag(remote.clone(), from.clone(), to.clone(), expect.clone())
        });
        // Keyed like every outgoing tag write (`ops_delete::remote_tag_delete`),
        // on the name that goes, so the page reports this press's answer.
        self.ref_push_asked(
            &format!("{remote}/{from}"),
            asked.map(platitude_core::OperationId::as_u64),
        );
    }

    /// The remote's copy of a tag. `rowGoes` is whether the sidebar's row
    /// leaves with it — true where only the remote had the name
    /// (`ops_delete::remote_tag_delete`). `expect` is the commit the row
    /// showed the tag on, which the delete is leased to.
    #[qslot]
    fn delete_remote_tag(&mut self, remote: String, tag: String, row_goes: bool, expect: String) {
        self.remote_tag_delete(remote, tag, row_goes, expect);
    }

    /// Both copies of a tag, as one queued write, leased as above.
    #[qslot]
    fn delete_tag_everywhere(&mut self, tag: String, remote: String, expect: String) {
        self.tag_delete_everywhere(tag, remote, expect);
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
    /// `message` is the commit box's text; empty lets git name the entry.
    #[qslot]
    fn push_stash(&mut self, message: String) {
        self.stash_push(message)
    }

    /// `git stash push -- <paths>` over the gathered files, untracked ones
    /// included: a path the user pointed at goes whether or not git tracks
    /// it yet.
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
    /// comes back: the entry stays, so nothing waits on this answer.
    #[qslot]
    fn apply_stash(&mut self, selector: String) {
        self.ask_session(|s| s.stash_apply(selector.clone()));
    }

    /// `git stash drop` (destructive): the entry's row goes at the press.
    #[qslot]
    fn drop_stash(&mut self, selector: String) {
        self.stash_drop(selector);
    }

    /// `git worktree add`: a new working copy at `path`. `mode` says what
    /// it stands on — `new` (a branch `branch` made at `start`), `branch`
    /// (the local branch `branch`) or `track` (a local `branch` made off
    /// the remote branch `start`, following it); `name` is its folder, for
    /// a refusal's heading. The answer is the page's to wait for
    /// (`copyAnswer`).
    #[qslot]
    fn add_worktree(
        &mut self,
        path: String,
        mode: String,
        branch: String,
        start: String,
        name: String,
    ) {
        self.add_copy(path, &mode, branch, start, name);
    }

    /// `git worktree remove` (destructive): the copy's row goes at the
    /// press. `name` is the row's own word for it, for a refusal's heading.
    #[qslot]
    fn remove_worktree(&mut self, path: String, name: String) {
        self.worktree_remove(path, name);
    }

    /// One of the sidebar's lists has drawn a reading; what that proves
    /// about a pending delete is the tab's to decide
    /// (`ops_delete::note_listing_drawn`).
    #[qslot]
    fn listing_drawn(&mut self) {
        self.note_listing_drawn();
    }

    /// A status has been applied beside HEAD report `seen`, leaving the
    /// working tree empty or not (`RepoTab::tree_was_read`).
    ///
    /// `seen` is the counts' own number (`WorkingTreeModel.statusSeq`), not
    /// `headSeq`: a HEAD report also arrives on its own, carrying no counts.
    #[qslot]
    fn note_tree_read(&mut self, seen: i32, emptied: bool) {
        let Ok(seen) = u64::try_from(seen) else {
            return;
        };
        self.tree_was_read(seen, emptied);
    }

    /// The page is acting on the working tree it has: answers whether
    /// this window's own stash is what emptied it, and settles that press
    /// (`ops::StashOut`). Asked from both halves of the pair (a status
    /// landing, a write answering) — the first gets nothing — and asking
    /// spends it, so there is nothing for a binding to follow.
    #[qslot]
    fn take_stash_landing(&mut self) -> bool {
        self.stash_landing_taken()
    }

    /// The open file was read again because a write answered, so the
    /// status that write publishes behind it is not news
    /// (`ops::DiffReread`). Fenced at the last answer's first HEAD report
    /// after it; a notify that carried no answer arms nothing.
    #[qslot]
    fn note_diff_read(&mut self) {
        self.read_diff_for_answers();
    }

    /// The page is reading the status it has just been given: whether the
    /// re-read standing here already answers for it, spending it where it
    /// does. Measured against `noteTreeRead`'s tree, so the two agree on
    /// which status is in hand.
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

    /// `git pull` naming nothing: git resolves the branch and its upstream,
    /// and how the far side comes in is git's own setting
    /// (デザイン規約 §取り込んで合流させる, `remote::pull`).
    #[qslot]
    fn pull(&mut self) {
        self.ask_session(platitude_core::session::RepoSession::pull);
    }

    /// Pushes the checked-out branch to wherever it tracks, or to the
    /// default remote when it tracks nothing yet. `force` is `""` /
    /// `"lease"` / `"force"`; `lease_expect` pins the remote commit the
    /// user saw. `branch` is what the button shows, recorded with the
    /// press's id so the answer comes back to it (`ops::PushOut`).
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
    /// repository does not have.
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

    /// `git remote set-url <name> <url>`.
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
    /// `branchDeleteAsked` / `branchDeleteMerged`, even where the reads
    /// fail — as `"unknown"`, drawn plain like a merged branch.
    ///
    /// The slow way: the menu asks the graph first
    /// (`GraphModel.branchDeleteMerged`) and comes here only for a tip or
    /// reference point older than the window.
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

    /// Replaces a branch on a remote the way `replaceRemoteTag` replaces a
    /// tag, the old name's delete leased to `expect`.
    #[qslot]
    fn replace_remote_branch(&mut self, remote: String, from: String, to: String, expect: String) {
        let row = format!("{remote}/{from}");
        let asked = self.ask_session(|s| {
            s.replace_remote_branch(remote.clone(), from.clone(), to.clone(), expect.clone())
        });
        self.ref_push_asked(&row, asked.map(platitude_core::OperationId::as_u64));
    }

    /// `git push <remote> --delete <branch>` (destructive), leased to
    /// `expect`, the commit the row showed.
    #[qslot]
    fn delete_remote_branch(&mut self, remote: String, branch: String, expect: String) {
        self.remote_branch_delete(remote, branch, expect);
    }

    /// `git branch --delete` (`-D` under `force`) and then
    /// `git push <remote> --delete` leased to `expect`, as one queued
    /// write: the local half refuses first where it refuses at all,
    /// leaving the remote as it was.
    #[qslot]
    fn delete_branch_everywhere(
        &mut self,
        branch: String,
        remote: String,
        remote_branch: String,
        force: bool,
        expect: String,
    ) {
        self.branch_delete_everywhere(branch, remote, remote_branch, force, expect)
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

    /// Opens the chosen conflicted paths in the configured merge tool.
    /// Named paths only: a bare `mergetool` walks every conflict while
    /// holding the write queue.
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
