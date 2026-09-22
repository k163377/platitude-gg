//! One tab's feed, drained into the properties QML reads.

use super::*;

impl RepoTab {
    pub(super) fn take_feed(&mut self) {
        let Some(feed) = self.feed.clone() else {
            return;
        };
        self.absorb(feed.drain());
        self.changed();
    }

    /// Everything the feed had waiting, folded into the properties QML
    /// reads — **one `changed()` for the lot of them**, which is why the
    /// write answers are kept as a list beside the group they rewrite
    /// (`write_answers`).
    #[expect(clippy::too_many_lines)]
    pub(super) fn absorb(&mut self, batch: Vec<TabMsg>) {
        // Whatever the last notify carried is over: this one answers for
        // itself, and an empty list is a drain that brought no write
        // answer at all. The owners waiting for one answer of their own
        // are put down with it, and so is the group left over for the
        // readers that wait for none — **a classification read a second
        // time is a screen sequenced twice off one answer**, which is
        // what "the editor is emptied once" rests on.
        self.write_answers.clear();
        self.commit_out.new_notify();
        self.read_commit_out();
        self.branch_delete_out.new_notify();
        self.read_branch_delete_out();
        self.stash_out.new_notify();
        self.read_stash_out();
        self.push_out.new_notify();
        self.read_push_out();
        self.ref_push_out.new_notify();
        self.read_ref_push_out();
        self.clear_write_group();
        for msg in batch {
            match msg {
                TabMsg::Opened { title, path } => {
                    self.state = "open".into();
                    self.title = title;
                    self.picker_folder_url = picker_folder_url(std::path::Path::new(&path));
                    self.repo_path = path;
                    self.stood();
                }
                TabMsg::OpenFailed {
                    kind,
                    path,
                    message,
                } => {
                    self.state = "error".into();
                    self.error_kind = kind.into();
                    self.error_path = path;
                    self.error = message;
                    // The copy would not open, which is still an answer:
                    // the doors are let go of and the page says what
                    // became of the folder.
                    self.stood();
                }
                TabMsg::OpError { message } => {
                    self.last_error = message;
                    self.last_error_from_fetch = false;
                }
                TabMsg::Author {
                    name,
                    email,
                    complete,
                    sign_commits,
                    signing_format,
                } => {
                    self.author_name = name;
                    self.author_email = email;
                    self.read_author_avatar();
                    self.identity_ready = complete;
                    self.signs_commits = sign_commits;
                    self.signing_format = signing_format;
                    self.compare_head_author();
                }
                TabMsg::Signature {
                    oid,
                    kind,
                    code,
                    signer,
                } => self.settle_signature(oid, kind, code, signer),
                TabMsg::Remotes {
                    names,
                    urls,
                    push_default,
                    push_default_local,
                } => {
                    // The marked remote is where pushes go. Only where
                    // nothing is marked does the old guess stand: `origin`
                    // when there is one, otherwise whichever remote comes
                    // first. A mark naming a remote this repository does
                    // not have is left out of it — git would take that
                    // name for a URL, and this application has nothing to
                    // point at.
                    let marked = names.iter().find(|r| **r == push_default);
                    self.default_remote = marked
                        .or_else(|| names.iter().find(|r| *r == "origin"))
                        .or_else(|| names.first())
                        .cloned()
                        .unwrap_or_default();
                    self.push_default = marked.cloned().unwrap_or_default();
                    self.push_default_local = push_default_local;
                    self.remote_count = names.len() as i32;
                    self.remote_names = names.clone();
                    self.remotes = names;
                    self.remote_urls = urls;
                }
                TabMsg::RemoteBranch {
                    remote,
                    branch,
                    state,
                    tip,
                    theirs,
                } => {
                    self.remote_branch_asked =
                        Optional::some(super::RemoteBranch { remote, branch });
                    self.remote_branch_revision = self.remote_branch_revision.wrapping_add(1);
                    self.remote_branch_state = state;
                    self.remote_branch_tip = tip;
                    self.remote_branch_theirs = theirs;
                }
                TabMsg::BranchDelete { branch, merged } => {
                    self.branch_delete_asked = branch;
                    self.branch_delete_merged = merged.into();
                }
                TabMsg::HeadCommit {
                    message,
                    author_name,
                    author_email,
                } => {
                    let (subject, body) = platitude_core::commit::split_message(&message);
                    self.head_subject = subject;
                    self.head_body = body;
                    self.head_author_name = author_name;
                    self.head_author_email = author_email;
                    self.compare_head_author();
                    self.head_commit_seq += 1;
                }
                TabMsg::AutoFetch {
                    running,
                    error,
                    announce,
                } => {
                    self.auto_fetch_running = running;
                    if !running {
                        self.fetch_settled(&error, announce);
                    }
                }
                TabMsg::MoveNeedsAsk { local, start } => {
                    self.move_ask_local = local;
                    self.move_ask_start = start;
                    self.move_ask_seq += 1;
                }
                TabMsg::MergeTools { names, settled } => {
                    self.merge_tools = names;
                    // The fast half arrives first; the indicator keeps
                    // turning until the slow read has had its say.
                    if settled {
                        self.merge_tools_loading = false;
                    }
                }
                // Arrives between this write's start and its end, so the
                // flag is already standing when the answer below is read.
                TabMsg::WriteStopped => self.last_write_stopped = true,
                TabMsg::WriteState {
                    id,
                    kind,
                    running,
                    error,
                    report,
                    head_seq,
                    reads_from,
                } => {
                    if running {
                        self.busy_count += 1;
                        // Only the write's own answer knows what to make
                        // of a command that fails inside it
                        // (`write_running`).
                        self.write_running = true;
                        self.replaying = kind.replays_history();
                        // The word the band reads (`busyOp`), made here
                        // from the kind: the kind itself is what the
                        // bridge carries.
                        self.busy_op = kind.label().to_string();
                        // Whatever the last write left standing, this one
                        // has not stopped yet.
                        self.last_write_stopped = false;
                    } else {
                        self.settle_write(id, kind, error, report, head_seq, reads_from);
                    }
                }
                // The last of the three: everything this write invalidated
                // has been read again and published, which is what a run
                // photographing the page a write leaves has to wait for.
                TabMsg::WriteSettled { id, .. } => self.write_watch.settled(id),
            }
        }
    }

    /// Where the answer to the write with this id stands in this
    /// notify's list, or `None` where it did not answer in this notify.
    pub(super) fn write_answer_index_of(&self, id: u64) -> Option<usize> {
        self.write_answers.iter().position(|a| a.id == id)
    }

    /// One of the answers this notify carried, or nothing where the
    /// index is past their end — a reader that asks after the list has
    /// been emptied gets the resting values, which are the failing ones.
    pub(super) fn write_answer_at(&self, index: i32) -> Option<&WriteAnswer> {
        usize::try_from(index)
            .ok()
            .and_then(|i| self.write_answers.get(i))
    }

    /// What git makes of one commit's signature, for the pane to read —
    /// **unless the question moved on while it was being answered**
    /// (`signature_wanted`).
    pub(super) fn settle_signature(
        &mut self,
        oid: String,
        kind: String,
        code: String,
        signer: String,
    ) {
        if oid != self.signature_wanted {
            tracing::debug!(
                answered = %oid,
                asked = %self.signature_wanted,
                "signature answer about a selection left behind"
            );
            return;
        }
        self.signature_oid = oid;
        self.signature_kind = kind;
        self.signature_code = code;
        self.signature_signer = signer;
    }

    /// One write's answer, **handed to whoever pressed for it**.
    ///
    /// The op names are turned into meanings here, on this side of the
    /// bridge, so the page sequences the screen — reload the diff, arm a
    /// landing, put taken rows back — off meanings alone
    /// (app-ui.md: no business logic in QML).
    ///
    /// **Where those meanings go is decided by the id.** A press that
    /// wrote its id down at the time is waiting for this one answer and
    /// no other, so the answer goes to it and stops there
    /// (`ops::Press`, `ops::StandIn`). What is left — the answers
    /// nobody named — is folded into the group the page reads when any
    /// answer will do. A drain empties the whole queue and notifies once,
    /// so that group can only ever describe one of the answers it
    /// carried: written for all of them, it says whichever finished last
    /// and the earlier ones are read as never having answered.
    pub(super) fn settle_write(
        &mut self,
        id: u64,
        kind: OperationKind,
        error: String,
        report: Option<platitude_core::WriteReport>,
        head_seq: u64,
        reads_from: u64,
    ) {
        self.busy_count = (self.busy_count - 1).max(0);
        if self.busy_count == 0 {
            self.busy_op = String::new();
            self.replaying = false;
        }
        self.write_running = self.busy_count > 0;
        // Who said no and what about — the halves the notice is
        // made of. **The kind is named here**, on this side of the
        // bridge, so the page picks its sentence off meanings alone
        // (app-ui.md: no business logic in QML). It rides the answer
        // below: a report is the answer's own, and a drain can bring
        // several of them, so each one carries the report it was
        // refused with.
        let reported = report.is_some();
        let (report_kind, remote, name, reason) = match report {
            Some(report) => (
                match report.kind {
                    ReportKind::RemoteDelete => "delete",
                    ReportKind::RemoteUpdate => "update",
                    ReportKind::Outdated => "outdated",
                    ReportKind::Commit => "commit",
                    ReportKind::StaleStage => "stale-stage",
                    ReportKind::StaleUnstage => "stale-unstage",
                    ReportKind::StaleDiscard => "stale-discard",
                    ReportKind::ConflictedPart => "conflicted-part",
                    ReportKind::RenameRefused => "rename",
                    ReportKind::HalfRenamed => "half-rename",
                    ReportKind::RewriteAcrossMerge => "across-merge",
                    ReportKind::RewriteOffBranch => "off-branch",
                    ReportKind::FoldFirstCommit => "fold-first",
                    ReportKind::RewriteUnfetchedBase => "unfetched-base",
                    ReportKind::DropAllCommits => "drop-all",
                    ReportKind::RewriteTipMoved => "tip-moved",
                    ReportKind::RewriteWhileStanding => "op-standing",
                }
                .to_string(),
                report.remote,
                report.name,
                report.reason,
            ),
            None => (String::new(), String::new(), String::new(), String::new()),
        };
        // A fetch the user asked for counts the same way the
        // timer's do: what the button says is about fetching,
        // whoever asked.
        if kind == OperationKind::Fetch {
            self.fetch_settled(&error, true);
        }
        let landed = error.is_empty();
        // The rows a delete took off the screen, answered by name:
        // what answered in between is somebody else's, and this is
        // the only thing that puts them back. `reads_from` goes
        // with it — the listings that take the rows away for good
        // are measured against it (`ops_delete::delete_answered`).
        self.delete_answered(id, !landed, reads_from);
        // Stopped part-way answers with the working tree: there is no
        // commit at the tip to go to (`last_write_stopped`, raised by
        // the message before this one).
        let at_tip = landed
            && !self.last_write_stopped
            && matches!(
                kind,
                OperationKind::Revert
                    | OperationKind::CherryPick
                    | OperationKind::Merge
                    | OperationKind::Pull
            );
        self.write_seq += 1;
        // The middle of the write's three boundaries, for whoever is
        // waiting on this one by the id its own ask was given. Matched
        // by equality: the ids are not a sequence (`write_watch`).
        self.write_watch.answered(id);
        // The answer as it came, kept whole: this is what an owner is
        // handed and what every reader waiting for one write by name
        // reads its meanings off (`write_answers`).
        self.write_answers.push(WriteAnswer {
            id,
            seq: self.write_seq,
            kind,
            stopped: self.last_write_stopped,
            failed: !landed,
            error: error.clone(),
            at_tip,
            head_seq,
            report_kind,
            report_remote: remote,
            report_name: name,
            report_reason: reason,
        });
        let at = self.write_answers.len() - 1;
        // Handed to whoever named this write at the press.
        //
        // **How much of an answer an owner takes is the owner's own.**
        // Everything the page does with the answer to a press that named
        // its write is that press's own — the file it left stale, the
        // words it was refused with, the boxes or the pane it puts down —
        // so the answer stops there, and a copy in the group below would
        // have the page act on the one answer twice. What is left over is
        // the answers nobody named: the fetch on its timer, the write a
        // page that has since gone away sent.
        let mut answered_for = false;
        if self.commit_out.answered(id, at) {
            self.read_commit_out();
            answered_for = true;
        }
        if self.branch_delete_out.answered(id, at, !landed, reported) {
            self.read_branch_delete_out();
            answered_for = true;
        }
        if self.stash_out.answered(id, at, !landed, head_seq) {
            self.read_stash_out();
            answered_for = true;
        }
        if self.push_out.answered(id, at) {
            self.read_push_out();
            answered_for = true;
        }
        if self.ref_push_out.answered(id, at) {
            self.read_ref_push_out();
            answered_for = true;
        }
        if !answered_for {
            self.fold_into_group(kind, landed, at);
        }
        self.last_write_error = error;
    }

    /// The picture QML is handed of what the editor's commit is waiting
    /// for — a copy, so a binding reads a plain member
    /// (`ops_delete::stand_in` keeps the delete's four the same
    /// way).
    fn read_commit_out(&mut self) {
        self.commit_answer = self
            .commit_out
            .answer()
            .and_then(|at| i32::try_from(at).ok())
            .unwrap_or(-1);
    }

    /// The same for the stash press that took the working tree away.
    /// **The tree it is waiting for is asked for** — that answer
    /// exists only at the moment the page acts on a tree, and asking
    /// spends it (`takeStashLanding`).
    fn read_stash_out(&mut self) {
        self.stash_answer = self
            .stash_out
            .answer()
            .and_then(|at| i32::try_from(at).ok())
            .unwrap_or(-1);
    }

    /// The same for the toolbar's push: where its answer stands, and the
    /// branch that press was sent for.
    fn read_push_out(&mut self) {
        self.push_answer = self
            .push_out
            .answer()
            .and_then(|at| i32::try_from(at).ok())
            .unwrap_or(-1);
        self.push_answer_branch = self.push_out.branch().to_string();
    }

    /// The same for the pushes a ref row sends: where their answer
    /// stands, and the row that press was about.
    fn read_ref_push_out(&mut self) {
        self.ref_push_answer = self
            .ref_push_out
            .answer()
            .and_then(|at| i32::try_from(at).ok())
            .unwrap_or(-1);
        self.ref_push_target = self.ref_push_out.branch().to_string();
    }

    /// One answer nobody was waiting for by name, folded into the group
    /// the page reads when any answer will do.
    ///
    /// Every such answer rewrites the whole of it, so nothing stays armed
    /// for a later write to trip over; the drain puts it down again at
    /// its top ([`Self::clear_write_group`]), so a notify carrying
    /// none of these says so and the last one goes down with the
    /// drain that carried it.
    fn fold_into_group(&mut self, kind: OperationKind, landed: bool, at: usize) {
        // A landed write moved what the two sides hold; a refused stage,
        // unstage or discard was refused *because* the rows on screen
        // drifted (the fingerprint refuses on nothing else). Both mean
        // the shown diff no longer describes the file.
        self.write_stale_diff = matches!(
            kind,
            OperationKind::Stage | OperationKind::Unstage | OperationKind::Discard
        ) || (landed
            && matches!(kind, OperationKind::Commit | OperationKind::Stash));
        self.write_refused = !landed;
        self.write_moved_head =
            landed && matches!(kind, OperationKind::Checkout | OperationKind::Reset);
        self.write_reworded = landed && kind == OperationKind::Reword;
        // The delete that reaches over to the remote answers as a branch
        // op the way its label reads: the row it took was a branch's
        // either way.
        self.write_branch_op = matches!(
            kind,
            OperationKind::Branch | OperationKind::DeleteBranchEverywhere
        );
        self.write_fetched = kind == OperationKind::Fetch;
        // The report travels on the answer; the group shows the one that
        // came with the answer it is describing.
        let Some(answer) = self.write_answers.get(at) else {
            return;
        };
        self.write_report_kind = answer.report_kind.clone();
        self.write_report_remote = answer.report_remote.clone();
        self.write_report_name = answer.report_name.clone();
        self.write_report_reason = answer.report_reason.clone();
    }

    /// Every property of that group put down — the resting values, which
    /// are the ones that ask the page to do nothing at all.
    fn clear_write_group(&mut self) {
        self.write_refused = false;
        self.write_stale_diff = false;
        self.write_moved_head = false;
        self.write_reworded = false;
        self.write_branch_op = false;
        self.write_fetched = false;
        self.write_report_kind = String::new();
        self.write_report_remote = String::new();
        self.write_report_name = String::new();
        self.write_report_reason = String::new();
    }
}
