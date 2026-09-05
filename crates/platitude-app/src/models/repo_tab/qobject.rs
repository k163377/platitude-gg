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
    qproperty!("replaying", Member = replaying, Notify = changed);
    qproperty!("mergeTools", Member = merge_tools, Notify = changed);
    qproperty!(
        "mergeToolsLoading",
        Member = merge_tools_loading,
        Notify = changed
    );
    qproperty!("remoteNames", Member = remote_names, Notify = changed);
    qproperty!(
        "remoteBranchAsked",
        Member = remote_branch_asked,
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
    // card reads its own and goes or turns its row (`RefBranchMenu`). The
    // seq says which write's answer it is (`RepoPage`).
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
        "branchDeleteSeq",
        Member = branch_delete_seq,
        Notify = changed
    );
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
    // The last answer, classified in drain::settle_write — the page reads
    // meanings, never op names (app-ui.md). What a reader waiting for one
    // particular answer asks instead is `writeAnswerCount` and the five
    // beside it: a drain can carry several answers and these describe
    // only the last of them.
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
    qproperty!("writeCommitted", Member = write_committed, Notify = changed);
    qproperty!("writeReworded", Member = write_reworded, Notify = changed);
    qproperty!("writeStashed", Member = write_stashed, Notify = changed);
    qproperty!("writeBranchOp", Member = write_branch_op, Notify = changed);
    qproperty!("writePushed", Member = write_pushed, Notify = changed);
    // What a write that did not happen has to say for itself — the page
    // makes a notice out of these rather than an error
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
    qproperty!("writeRunning", Member = write_running, Notify = changed);
    qproperty!("moveAskLocal", Member = move_ask_local, Notify = changed);
    qproperty!("moveAskStart", Member = move_ask_start, Notify = changed);
    qproperty!("moveAskSeq", Member = move_ask_seq, Notify = changed);
    qproperty!(
        "autoFetchRunning",
        Member = auto_fetch_running,
        Notify = changed
    );
    qproperty!(
        "autoFetchError",
        Member = auto_fetch_error,
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
    /// Read one field at a time the way the remotes are, rather than
    /// packed into a string: a decode in QML would be data handling, and
    /// that belongs on this side of the bridge (app-ui.md).
    #[qslot]
    fn write_answer_count(&self) -> i32 {
        i32::try_from(self.write_answers.len()).unwrap_or(i32::MAX)
    }

    /// What `write_seq` counted that answer at — how a reader tells the
    /// answer to its own press from one counted before it.
    #[qslot]
    fn write_answer_seq(&self, index: i32) -> i32 {
        self.write_answer_at(index).map_or(0, |a| a.seq)
    }

    /// The number the session named for the first report of HEAD after
    /// that write (`WriteAnswer::head_seq`): a landing on what the write
    /// left arms on it, and `WorkTreeModel.headSeq` at or above it is a
    /// report that looked after the write — whichever of the two this
    /// page reads first.
    #[qslot]
    fn write_answer_head_seq(&self, index: i32) -> i32 {
        self.write_answer_at(index).map_or(0, |a| a.head_seq)
    }

    /// Which write answered. Raw data, the way `lastWriteError` is: what
    /// an answer *means* is the three below.
    #[qslot]
    fn write_answer_op(&self, index: i32) -> String {
        self.write_answer_at(index)
            .map(|a| a.op.clone())
            .unwrap_or_default()
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

    /// It left a commit at the tip to go to.
    #[qslot]
    fn write_answer_at_tip(&self, index: i32) -> bool {
        self.write_answer_at(index).is_some_and(|a| a.at_tip)
    }

    /// Local branch name a remote-tracking ref would take: the ref with
    /// its remote's prefix removed.
    ///
    /// Matched against the configured remotes rather than cut at the first
    /// slash — a remote may be named `my/fork`, and the longest matching
    /// prefix is the right one.
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

    // Read back one field at a time rather than as one packed string: a
    // decode in QML would be data handling, which belongs on this side of
    // the bridge (app-ui.md).
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
    /// the session is busy are dropped there, not queued.
    #[qslot]
    fn refresh_poll(&mut self) {
        self.with_session(|s| s.refresh_poll());
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
        self.with_session(|s| s.stage_paths(vec![path.clone()]));
    }

    #[qslot]
    fn unstage_path(&mut self, path: String) {
        self.with_session(|s| s.unstage_paths(vec![path.clone()]));
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
    /// the choice's to say, not status's (`ops_stage::chosen_row`).
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
        self.with_session(|s| s.stage_all());
    }

    #[qslot]
    fn unstage_all(&mut self) {
        self.with_session(|s| s.unstage_all());
    }

    /// `git add` over every conflicted path — the conflicted bucket's
    /// whole-bucket affordance. The set is read under the write lock, so
    /// no paths are gathered here.
    #[qslot]
    fn stage_conflicted(&mut self) {
        self.with_session(|s| s.stage_conflicted());
    }

    /// Stages (or unstages) part of one file's diff, addressed by the row
    /// the user clicked. `kind` is the diff-key prefix (`unstaged` /
    /// `staged` / `untracked`); a negative `line` takes the whole hunk.
    /// Answers whether a write went out (see [`Self::stage_chosen`]) —
    /// the caller's wait is armed by this answer, not by the asking.
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
    /// stands, so that agreement travels with the move rather than
    /// running ahead of it (デザイン規約 §進行中の操作から出る).
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
        self.with_session(|s| s.checkout_moving_branch(local.clone(), start.clone(), leaving));
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
        self.with_session(|s| s.create_branch(name.clone(), start.clone(), switch_to));
    }

    /// Deletes a local branch. Without `force`, git refuses an unmerged one
    /// — and that answer is the menu's to catch, so the plain form is
    /// written down first (`branch_delete`).
    #[qslot]
    fn delete_branch(&mut self, name: String, force: bool) {
        self.branch_delete(name, force)
    }

    #[qslot]
    fn rename_branch(&mut self, from: String, to: String, force: bool) {
        self.with_session(|s| s.rename_branch(from.clone(), to.clone(), force));
    }

    /// Records the remote branch `branch` is measured against. The two
    /// halves the question was answered with are joined here rather than
    /// in QML: what git is given is the full remote-tracking refname,
    /// the one spelling a local branch of the same name cannot make
    /// ambiguous (`branch::set_upstream`).
    #[qslot]
    fn set_upstream(&mut self, branch: String, remote: String, remote_branch: String) {
        if branch.is_empty() || remote.is_empty() || remote_branch.is_empty() {
            return;
        }
        let upstream = format!("refs/remotes/{remote}/{remote_branch}");
        self.with_session(|s| s.set_upstream(branch.clone(), upstream.clone()));
    }

    /// Puts a lightweight tag on `commit` (HEAD when empty). Never
    /// forced: a name already taken is git's to refuse.
    #[qslot]
    fn create_tag(&mut self, name: String, commit: String) {
        self.with_session(|s| s.create_tag(name.clone(), commit.clone()));
    }

    /// Sends one tag to one remote. `lease_expect` pins a leased
    /// overwrite to the commit that remote was last seen holding the name
    /// on; empty sends it plain.
    #[qslot]
    fn push_tag(&mut self, remote: String, tag: String, lease_expect: String) {
        self.with_session(|s| s.push_tag(remote.clone(), tag.clone(), lease_expect.clone()));
    }

    /// `git push <remote> --delete refs/tags/<tag>`. Fully qualified,
    /// because a bare name a branch shares over there is refused and
    /// neither is deleted (measured — `remote::delete_remote_tag`).
    #[qslot]
    fn delete_remote_tag(&mut self, remote: String, tag: String) {
        self.with_session(|s| s.delete_remote_tag(remote.clone(), tag.clone()));
    }

    /// `git tag --delete` and then the remote's copy, as one queued
    /// write: the local half first, so a pair that stops part-way never
    /// leaves the name gone from the remote and still in the sidebar.
    #[qslot]
    fn delete_tag_everywhere(&mut self, tag: String, remote: String) {
        self.with_session(|s| s.delete_tag_everywhere(tag.clone(), remote.clone()));
    }

    /// Renames a tag. git has none, so core builds it out of a new name on
    /// the same object and a delete of the old one.
    #[qslot]
    fn rename_tag(&mut self, from: String, to: String) {
        self.with_session(|s| s.rename_tag(from.clone(), to.clone()));
    }

    /// Deletes a tag: only the name goes.
    #[qslot]
    fn delete_tag(&mut self, name: String) {
        self.with_session(|s| s.delete_tag(name.clone()));
    }

    /// Renames a stash entry. Built the same way, out of a re-store and a
    /// drop — so the entry moves to the top of the list.
    #[qslot]
    fn rename_stash(&mut self, selector: String, message: String) {
        self.with_session(|s| s.rename_stash(selector.clone(), message.clone()));
    }

    /// `git stash push -u` over the whole working tree, on the press —
    /// the button asks nothing first (デザイン規約 §変更を退避する).
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

    /// `git stash pop` on the given selector (stash-row action).
    #[qslot]
    fn pop_stash(&mut self, selector: String) {
        self.with_session(|s| s.stash_pop(selector.clone()));
    }

    /// `git stash apply` on the given selector (keeps the stash).
    #[qslot]
    fn apply_stash(&mut self, selector: String) {
        self.with_session(|s| s.stash_apply(selector.clone()));
    }

    /// `git stash drop` (destructive).
    #[qslot]
    fn drop_stash(&mut self, selector: String) {
        self.with_session(|s| s.stash_drop(selector.clone()));
    }

    /// `git fetch --prune`; an empty remote fetches all of them.
    #[qslot]
    fn fetch(&mut self, remote: String) {
        let remote = (!remote.is_empty()).then_some(remote);
        self.with_session(|s| s.fetch(remote.clone()));
    }

    /// Pushes the branch that is checked out to wherever it tracks, or to
    /// the default remote when it tracks nothing yet. `force` is `""` /
    /// `"lease"` / `"force"`; `lease_expect` pins the remote commit the
    /// user actually saw.
    #[qslot]
    fn push_current(&mut self, force: String, lease_expect: String) {
        let fallback = self.default_remote.clone();
        let force = Self::push_force(&force, &lease_expect);
        self.with_session(|s| s.push_current(fallback.clone(), force.clone()));
    }

    /// The first push of a branch, to the remote and name the question
    /// just took. Records the answer as the upstream, so the branch never
    /// asks again.
    #[qslot]
    fn publish_current(&mut self, remote: String, remote_branch: String, expect: String) {
        self.with_session(|s| {
            s.publish_current(remote.clone(), remote_branch.clone(), expect.clone())
        });
    }

    /// `git remote add <name> <url>`. Contacts nothing — a URL that goes
    /// nowhere is recorded just the same, and the push finds out.
    #[qslot]
    fn add_remote(&mut self, name: String, url: String) {
        self.with_session(|s| s.add_remote(name.clone(), url.clone()));
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

    /// Marks the remote a push goes to (`remote.pushDefault`); an empty
    /// name clears the mark. Clearing reaches this repository's config
    /// only — a mark set for every repository stays, and marking another
    /// remote is what moves it.
    #[qslot]
    fn set_push_default(&mut self, name: String) {
        self.with_session(|s| s.set_push_default(name.clone()));
    }

    /// `git remote set-url <name> <url>` — the way back from a typo.
    #[qslot]
    fn set_remote_url(&mut self, name: String, url: String) {
        self.with_session(|s| s.set_remote_url(name.clone(), url.clone()));
    }

    /// Asks what a push under this branch name would meet on that remote.
    /// Reaches the network, so it is asked while the question stands and
    /// not on a poll. The answer arrives as `remoteBranchAsked` /
    /// `remoteBranchState`.
    #[qslot]
    fn check_remote_branch(&mut self, remote: String, branch: String) {
        self.look_up_remote_branch(remote, branch)
    }

    /// Asks git whether `branch --delete` would refuse this branch (not
    /// merged into its upstream, or HEAD without one), so a menu's
    /// delete row can wear `-D` from the start. The answer arrives as
    /// `branchDeleteAsked` / `branchDeleteMerged`; no answer arrives
    /// where the reads fail, and the row stays on its plain form.
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

    /// Renames a branch on a remote. git has none, so core pushes the new
    /// name and deletes the old — the UI holds the answer down first,
    /// because the old name is destroyed rather than moved.
    #[qslot]
    fn rename_remote_branch(&mut self, remote: String, from: String, to: String) {
        self.with_session(|s| s.rename_remote_branch(remote.clone(), from.clone(), to.clone()));
    }

    /// `git push <remote> --delete <branch>` (destructive).
    #[qslot]
    fn delete_remote_branch(&mut self, remote: String, branch: String) {
        self.with_session(|s| s.delete_remote_branch(remote.clone(), branch.clone()));
    }

    /// `git branch --delete` (`-D` under `force`) and then
    /// `git push <remote> --delete`, as one queued write: the local half
    /// refuses first where it refuses at all, and then the remote is
    /// never touched.
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
        self.with_session(|s| s.merge(rev.clone(), options.clone()));
    }

    /// `git rebase <upstream>`; an empty `onto` uses `upstream` as the base.
    ///
    /// There is no autostash knob to pass: uncommitted work is carried
    /// across by core, the same way every other rewrite here carries it
    /// (デザイン規約 §未コミット変更がある状態で履歴を書き換える).
    #[qslot]
    fn rebase(&mut self, upstream: String, onto: String, update_refs: bool) {
        let options = platitude_core::integrate::RebaseOptions {
            onto: (!onto.is_empty()).then_some(onto),
            branch: None,
            update_refs,
            root: false,
        };
        self.with_session(|s| s.rebase(upstream.clone(), options.clone()));
    }

    #[qslot]
    fn cherry_pick(&mut self, rev: String) {
        self.with_session(|s| s.cherry_pick(vec![rev.clone()]));
    }

    /// Folds a commit into its parent (one-commit interactive rebase).
    #[qslot]
    fn squash_into_parent(&mut self, oid: String) {
        self.with_session(|s| s.squash_into_parent(oid.clone()));
    }

    /// Leaves one commit out of the history, replaying what came after it.
    #[qslot]
    fn drop_commit(&mut self, oid: String) {
        self.with_session(|s| s.drop_commit(oid.clone()));
    }

    /// Replaces one commit's message. HEAD is amended; anything older is
    /// replayed, which rewrites every commit after it.
    #[qslot]
    fn reword_commit(&mut self, oid: String, subject: String, body: String) {
        let message = platitude_core::commit::join_message(&subject, &body);
        if message.is_empty() {
            return;
        }
        self.with_session(|s| s.reword(oid.clone(), message.clone()));
    }

    #[qslot]
    fn revert(&mut self, rev: String) {
        self.with_session(|s| s.revert(vec![rev.clone()]));
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
    /// Named paths only, never "all of them": git walks a bare `mergetool`
    /// one file at a time and the whole walk holds the write queue, so an
    /// unnamed launch would block every other write for as many tool
    /// sessions as there are conflicts.
    #[qslot]
    fn open_mergetool(&mut self) {
        self.run_merge_tool()
    }

    /// Records which merge tool to launch; empty clears the choice.
    #[qslot]
    fn set_merge_tool(&mut self, tool: String) {
        self.with_session(|s| s.set_merge_tool(tool.clone()));
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
    /// the answer is about the person, not the project.
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
