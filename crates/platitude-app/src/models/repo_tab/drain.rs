//! One tab's feed, drained into the properties QML reads.

use super::*;
use platitude_core::RemoteWhy;
use platitude_core::discards::Restored;

impl RepoTab {
    pub(super) fn take_feed(&mut self) {
        let Some(feed) = self.feed.clone() else {
            return;
        };
        let was_settling = self.settling;
        self.absorb(feed.drain());
        self.changed();
        if self.settling != was_settling {
            self.settling_changed();
        }
    }

    /// Everything the feed had waiting, folded into the properties QML
    /// reads under one `changed()` — which is why the write answers are
    /// kept as a list (`write_answers`).
    #[expect(clippy::too_many_lines)]
    pub(super) fn absorb(&mut self, batch: Vec<TabMsg>) {
        // The last notify's answers are over, the owners' and the group's
        // alike: a classification read a second time sequences the screen
        // twice off one answer.
        self.write_answers.clear();
        self.paced_worktrees.clear();
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
        self.worktree_out.new_notify();
        self.worktree_move_out.new_notify();
        self.read_worktree_out();
        self.clear_write_group();
        for msg in batch {
            match msg {
                TabMsg::Opened { title, path } => {
                    self.state = "open".into();
                    self.title = title;
                    // The picker opens beside the repository, not the
                    // linked worktree the tab stands in (`Hub::home_worktree`) —
                    // the worktrees' folder holds only worktrees of this one.
                    let beside = Hub::with(|hub| hub.home_worktree(self.tab_id))
                        .flatten()
                        .unwrap_or_else(|| path.clone());
                    self.picker_folder_url = picker_folder_url(std::path::Path::new(&beside));
                    self.repo_path = path;
                    self.stood();
                    // A session of its own, which has not heard the page's
                    // word: an amend in view across a stand stays in view.
                    if self.amend_watched {
                        self.with_session(|s| s.watch_amend(true));
                    }
                }
                TabMsg::OpenFailed {
                    kind,
                    path,
                    message,
                } => {
                    // A linked worktree that would not open is stood back in
                    // the repository's own one, silently (デザイン規約
                    // §タブの所作「立てない所へは立たない」); only a refusal
                    // with nowhere left to fall reaches the screen. The
                    // doors stay held (`RepoTab::stood`): let go of here,
                    // they would open for a turn on no worktree before the
                    // next stand.
                    if Hub::with(|hub| hub.home_worktree(self.tab_id))
                        .flatten()
                        .is_some()
                    {
                        self.stand_home_asked();
                    } else {
                        self.state = "error".into();
                        self.error_kind = kind.into();
                        self.error_path = path;
                        self.error = message;
                        // Still an answer: the doors are let go of.
                        self.stood();
                    }
                }
                TabMsg::OpError { message, why } => {
                    self.last_error = message;
                    self.last_error_why = why;
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
                    self.identity_read = true;
                    self.identity_unread = false;
                    self.signs_commits = sign_commits;
                    self.signing_format = signing_format;
                    self.compare_head_author();
                }
                // An answer too: the button stops waiting and says why it cannot be pressed.
                TabMsg::AuthorUnread => {
                    self.identity_read = true;
                    self.identity_unread = true;
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
                    checkout_default,
                } => {
                    // The marked remote is where pushes go; unmarked,
                    // `origin`, else the first remote. A mark naming a
                    // missing remote is ignored — git would take that
                    // name for a URL.
                    let marked = names.iter().find(|r| **r == push_default);
                    self.default_remote = marked
                        .or_else(|| names.iter().find(|r| *r == "origin"))
                        .or_else(|| names.first())
                        .cloned()
                        .unwrap_or_default();
                    self.push_default = marked.cloned().unwrap_or_default();
                    self.push_default_local = push_default_local;
                    // Both keys or none: `git remote rename` moves only the
                    // push's key, and that remote still has the mark to
                    // finish.
                    self.marked_origin = if self.push_default == checkout_default {
                        self.push_default.clone()
                    } else {
                        String::new()
                    };
                    self.remote_count = names.len() as i32;
                    self.remote_names = names;
                    self.remote_urls = urls;
                }
                TabMsg::RemoteBranch {
                    remote,
                    branch,
                    local,
                    state,
                    tip,
                    theirs,
                } => {
                    self.remote_branch_asked = Optional::some(super::RemoteBranch {
                        remote,
                        branch,
                        local,
                    });
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
                TabMsg::AmendChecked { empties } => self.amend_empties = empties,
                TabMsg::AutoFetch { running, error } => {
                    self.auto_fetch_running = running;
                    if !running {
                        self.fetch_settled(&error, false);
                    }
                }
                TabMsg::MoveNeedsAsk { local, start, lost } => {
                    self.move_ask_local = local;
                    self.move_ask_start = start;
                    self.move_ask_lost = lost;
                    self.move_ask_seq += 1;
                }
                TabMsg::DiscardsChanged { restored } => {
                    self.discard_seq += 1;
                    if restored.is_empty() {
                        self.taken_seq += 1;
                    } else {
                        self.restore_how = restore_how(&restored).to_string();
                        self.restore_seq += 1;
                    }
                }
                TabMsg::MergeTools { names, settled } => {
                    self.merge_tools = names;
                    // The fast half arrives first; the indicator turns
                    // until the slow read lands.
                    if settled {
                        self.merge_tools_loading = false;
                    }
                }
                // Arrives between this write's start and its end, so the
                // flag is already standing when the answer below is read.
                TabMsg::WriteStopped => self.last_write_stopped = true,
                TabMsg::PacedRead { worktree: None } => {
                    self.paced_seq = self.paced_seq.wrapping_add(1)
                }
                TabMsg::PacedRead {
                    worktree: Some(path),
                } => self.paced_worktrees.push(path),
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
                        // Counted already where this tab asked for it
                        // (`ask_session`); a write asked elsewhere — a
                        // restore off the discard record — counts here.
                        self.unsettle(id);
                        self.replaying = kind.replays_history();
                        self.busy_op = kind.label().to_string();
                        // Whatever the last write left standing, this one
                        // has not stopped yet.
                        self.last_write_stopped = false;
                    } else {
                        self.settle_write(id, kind, error, report, head_seq, reads_from);
                    }
                }
                // The last of the three boundaries: everything the write
                // invalidated has been read again and published. An unasked
                // fetch settles too, under an id nothing here started.
                TabMsg::WriteSettled { id, .. } => {
                    self.write_watch.settled(id);
                    if let Some(at) = self.unsettled.iter().position(|out| *out == id) {
                        self.unsettled.remove(at);
                        if self.unsettled.is_empty()
                            && let Some(feed) = &self.feed
                        {
                            feed.push(TabMsg::ReadsDrained);
                        }
                    }
                }
                // A write asked for since keeps the tab settling.
                TabMsg::ReadsDrained => self.settling = !self.unsettled.is_empty(),
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

    /// One text field of that answer, empty past the end.
    pub(super) fn write_answer_text(
        &self,
        index: i32,
        field: impl FnOnce(&WriteAnswer) -> &str,
    ) -> String {
        self.write_answer_at(index)
            .map(field)
            .unwrap_or_default()
            .to_string()
    }

    /// What git makes of one commit's signature, for the pane — dropped
    /// if the question moved on meanwhile (`signature_wanted`).
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

    /// One write's answer, turned into meanings on this side of the bridge
    /// (app-ui.md: no business logic in QML) and handed to whoever pressed
    /// for it.
    ///
    /// A press that wrote its id down (`ops::Press`, `ops::StandIn`) is
    /// handed its answer; only the answers nobody named fold into the
    /// group. One drain notifies once, so the group can describe only one
    /// answer — written for all, it would hide the earlier ones.
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
        // Who said no and what about. It rides the answer below: a drain
        // can bring several, each with its own report.
        let reported = report.is_some();
        let (report_kind, remote, name, fact) = match report {
            Some(report) => (
                report_word(report.kind),
                report.remote,
                report.name,
                report.fact,
            ),
            None => (String::new(), String::new(), String::new(), String::new()),
        };
        // A fetch the user asked for counts as the timer's do.
        if kind == OperationKind::Fetch {
            self.fetch_settled(&error, true);
        }
        let landed = error.is_empty();
        // A name taken here may be one a restore was to go under, and a
        // branch or tag made on a lost commit holds it again: the open log
        // reads again (`RepoTab.discardSeq`).
        if landed && names_taken(kind) {
            self.discard_seq += 1;
        }
        // Puts back the rows a delete took off the screen, by id;
        // `reads_from` is what the listings that remove them for good
        // are measured against (`ops_delete::delete_answered`).
        self.delete_answered(id, !landed, reads_from);
        // Stopped part-way answers with the worktree: there is no
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
        // The middle of the write's three boundaries. Matched by
        // equality: the ids are not a sequence (`write_watch`).
        self.write_watch.answered(id);
        // Kept whole: what an owner is handed (`write_answers`).
        self.write_answers.push(WriteAnswer {
            id,
            seq: self.write_seq,
            kind,
            stopped: self.last_write_stopped,
            failed: !landed,
            error: error.clone(),
            at_tip,
            head_seq,
            reads_from,
            report_kind,
            report_remote: remote,
            report_name: name,
            report_fact: fact,
        });
        let at = self.write_answers.len() - 1;
        // Handed to whoever named this write at the press, and stops
        // there: a copy in the group would have the page act on the one
        // answer twice.
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
        if self.worktree_out.answered(id, at) || self.worktree_move_out.answered(id, at) {
            self.read_worktree_out();
            answered_for = true;
        }
        if !answered_for {
            self.fold_into_group(kind, landed, at);
        }
        self.last_write_error = error;
    }

    /// What the editor's commit is waiting for, copied to a plain member
    /// for bindings to read.
    fn read_commit_out(&mut self) {
        self.commit_answer = self
            .commit_out
            .answer()
            .and_then(|at| i32::try_from(at).ok())
            .unwrap_or(-1);
    }

    /// The same for the stash press that took the worktree away. The
    /// tree it waits for is not copied: asking spends it
    /// (`takeStashLanding`).
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

    /// The same for the worktree a menu asked for, and the one a name box
    /// renamed. Their folders are the presses' own, written down as they were
    /// asked (`worktree_add` / `worktree_move`).
    fn read_worktree_out(&mut self) {
        self.worktree_answer = self
            .worktree_out
            .answer()
            .and_then(|at| i32::try_from(at).ok())
            .unwrap_or(-1);
        self.worktree_move_answer = self
            .worktree_move_out
            .answer()
            .and_then(|at| i32::try_from(at).ok())
            .unwrap_or(-1);
    }

    /// One answer nobody was waiting for by name, folded into the group
    /// the page reads when any answer will do. Each rewrites the whole
    /// group, and the drain clears it at its top
    /// ([`Self::clear_write_group`]), so nothing stays set for a later
    /// write to trip over.
    fn fold_into_group(&mut self, kind: OperationKind, landed: bool, at: usize) {
        // A landed write moved what the two sides hold; a refused stage,
        // unstage or discard may have been refused *because* the rows on
        // screen drifted (the fingerprint's refusal), and the shown diff
        // then no longer describes the file. Refused for anything else —
        // another git's lock — the read finds the file as it was.
        self.write_stale_diff = matches!(
            kind,
            OperationKind::Stage | OperationKind::Unstage | OperationKind::Discard
        ) || (landed
            && matches!(kind, OperationKind::Commit | OperationKind::Stash));
        self.write_refused = !landed;
        self.write_moved_head =
            landed && matches!(kind, OperationKind::Checkout | OperationKind::Reset);
        self.write_reworded = landed && kind == OperationKind::Reword;
        // A delete that reaches the remote too took a branch's row all
        // the same.
        self.write_branch_op = matches!(
            kind,
            OperationKind::Branch | OperationKind::DeleteBranchEverywhere
        );
        // The same for tags.
        self.write_tag_op = matches!(
            kind,
            OperationKind::Tag | OperationKind::DeleteTagEverywhere
        );
        self.write_fetched = kind == OperationKind::Fetch;
        let Some(answer) = self.write_answers.get(at) else {
            return;
        };
        self.write_report_kind = answer.report_kind.clone();
        self.write_report_remote = answer.report_remote.clone();
        self.write_report_name = answer.report_name.clone();
        self.write_report_fact = answer.report_fact.clone();
    }

    /// The group at rest — values that ask the page to do nothing.
    fn clear_write_group(&mut self) {
        self.write_refused = false;
        self.write_stale_diff = false;
        self.write_moved_head = false;
        self.write_reworded = false;
        self.write_branch_op = false;
        self.write_tag_op = false;
        self.write_fetched = false;
        self.write_report_kind = String::new();
        self.write_report_remote = String::new();
        self.write_report_name = String::new();
        self.write_report_fact = String::new();
    }
}

/// The word the page picks a report's two lines by (`Words.writeReported`,
/// `Words.writeReportedWhy`). A far-side refusal is two words, what was
/// asked and what for (`update-rule`): the heading turns on the first, the
/// line under it on the second.
fn report_word(kind: ReportKind) -> String {
    let word = match kind {
        ReportKind::RemoteDelete(why) => return format!("delete-{}", remote_why_word(why)),
        ReportKind::RemoteUpdate(why) => return format!("update-{}", remote_why_word(why)),
        ReportKind::PrePush => "pre-push",
        ReportKind::PrePushDelete => "pre-push-delete",
        ReportKind::Outdated => "outdated",
        ReportKind::Moved => "moved",
        ReportKind::MovedDelete => "moved-delete",
        ReportKind::TagElsewhere => "tag-elsewhere",
        ReportKind::Commit => "commit",
        ReportKind::CommitHook => "commit-hook",
        ReportKind::CommitUnsigned => "commit-unsigned",
        ReportKind::CommitLocked => "commit-locked",
        ReportKind::StaleStage => "stale-stage",
        ReportKind::StaleUnstage => "stale-unstage",
        ReportKind::StaleDiscard => "stale-discard",
        ReportKind::ConflictedPart => "conflicted-part",
        ReportKind::StageLocked => "stage-locked",
        ReportKind::UnstageLocked => "unstage-locked",
        ReportKind::DiscardLocked => "discard-locked",
        ReportKind::RenameRefused => "rename",
        ReportKind::RenameLocked => "rename-locked",
        ReportKind::RenameUnfinished => "rename-unfinished",
        ReportKind::HalfRenamed => "half-rename",
        ReportKind::StashRenameRefused => "stash-rename",
        ReportKind::StashRenameLocked => "stash-rename-locked",
        ReportKind::StashGone => "stash-gone",
        ReportKind::WorktreeKept => "worktree-kept",
        ReportKind::WorktreeLocked => "worktree-locked",
        ReportKind::WorktreeRemoveRefused => "worktree-refused",
        ReportKind::WorktreeHalfRemoved => "worktree-half",
        ReportKind::WorktreeNotAdded => "worktree-not-added",
        ReportKind::WorktreePlaceTaken => "worktree-place-taken",
        ReportKind::WorktreeBranchLeft => "worktree-branch-left",
        ReportKind::WorktreeHookFailed => "worktree-hook",
        ReportKind::WorktreeFolderTaken => "worktree-folder-taken",
        ReportKind::CarriedInTheWay => "carried-in-the-way",
        ReportKind::CarriedFileInTheWay => "carried-file-in-the-way",
        ReportKind::CarriedUnread => "carried-unread",
        ReportKind::CarriedRefused => "carried-refused",
        ReportKind::CarriedLocked => "carried-locked",
        ReportKind::WorktreeNotRenamed => "worktree-not-renamed",
        ReportKind::WorktreeRenameTaken => "worktree-rename-taken",
        ReportKind::RewriteAcrossMerge => "across-merge",
        ReportKind::RewriteOffBranch => "off-branch",
        ReportKind::FoldFirstCommit => "fold-first",
        ReportKind::RewriteUnfetchedBase => "unfetched-base",
        ReportKind::DropAllCommits => "drop-all",
        ReportKind::RewriteTipMoved => "tip-moved",
        ReportKind::RewriteWhileStanding => "op-standing",
        ReportKind::RewritePreRebase => "pre-rebase",
        ReportKind::RewriteUnsigned => "rewrite-unsigned",
        ReportKind::RewriteCommitHook => "rewrite-commit-hook",
        ReportKind::RewriteLocked => "rewrite-locked",
        ReportKind::RewriteUntrackedInTheWay => "rewrite-untracked",
    };
    word.to_string()
}

fn remote_why_word(why: RemoteWhy) -> &'static str {
    match why {
        RemoteWhy::Rule => "rule",
        RemoteWhy::Rewrite => "rewrite",
        RemoteWhy::NoDeletes => "no-deletes",
        RemoteWhy::CheckedOut => "checked-out",
        RemoteWhy::Current => "current",
        RemoteWhy::TooLarge => "too-large",
        RemoteWhy::Shallow => "shallow",
        RemoteWhy::Locked => "locked",
        RemoteWhy::Other => "refused",
    }
}

/// Whether a write of this kind can take a name a discard log's restore
/// would go under (破棄記録仕様.md §4) — a worktree made takes its
/// folder and the branch it makes — or hold a lost commit again.
fn names_taken(kind: OperationKind) -> bool {
    matches!(
        kind,
        OperationKind::Branch
            | OperationKind::Tag
            | OperationKind::Checkout
            | OperationKind::Worktree
    )
}

/// How a restore's work came back, as the page words it: the least whole
/// of its parts.
fn restore_how(restored: &[Restored]) -> &'static str {
    if restored.contains(&Restored::AsStash) {
        "stash"
    } else if restored.contains(&Restored::Unstaged) {
        "unstaged"
    } else {
        "whole"
    }
}
