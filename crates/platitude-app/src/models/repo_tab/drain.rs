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
        // answer at all.
        self.write_answers.clear();
        for msg in batch {
            match msg {
                TabMsg::Opened { title, path } => {
                    self.state = "open".into();
                    self.title = title;
                    self.picker_folder_url = picker_folder_url(std::path::Path::new(&path));
                    self.repo_path = path;
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
                    self.remote_names = names.join("\u{1f}");
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
                    self.remote_branch_asked = format!("{remote}\u{1f}{branch}");
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
                    self.merge_tools = names.join("\u{1f}");
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
                    op,
                    running,
                    replays,
                    error,
                    report,
                    head_seq,
                    reads_from,
                } => {
                    if running {
                        self.busy_count += 1;
                        // A command that fails inside a write is not the
                        // write's answer, and only that answer knows what
                        // to make of it (`write_running`).
                        self.write_running = true;
                        self.replaying = replays;
                        self.busy_op = op;
                        // Whatever the last write left standing, this one
                        // has not stopped yet.
                        self.last_write_stopped = false;
                    } else {
                        self.settle_write(id, op, error, report, head_seq, reads_from);
                    }
                }
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

    /// One write's answer, folded into the properties the page reads.
    ///
    /// The op names are turned into meanings **here**, on this side of
    /// the bridge, so the page sequences the screen — reload the diff,
    /// arm a landing, put taken rows back — without ever branching on
    /// git vocabulary (app-ui.md: no business logic in QML). Every
    /// answer rewrites the whole group, so nothing stays armed for a
    /// later write to trip over; `write_seq` says which answer the
    /// group describes, and `id` names the press it answers.
    pub(super) fn settle_write(
        &mut self,
        id: u64,
        op: String,
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
        // Who said no and what about — the halves the notice is made of.
        // **The kind is named here**, on this side of the bridge, so the
        // page picks its sentence without ever branching on core's types
        // or on git vocabulary (app-ui.md: no business logic in QML).
        // Rewritten by every answer, like the rest of the group: a report
        // nobody took down would otherwise come back up under the next
        // write.
        let reported = report.is_some();
        let (kind, remote, name, reason) = match report {
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
                }
                .to_string(),
                report.remote,
                report.name,
                report.reason,
            ),
            None => (String::new(), String::new(), String::new(), String::new()),
        };
        self.write_report_kind = kind;
        self.write_report_remote = remote;
        self.write_report_name = name;
        self.write_report_reason = reason;
        // A fetch the user asked for counts the same way the timer's do:
        // what the button says is about fetching, not about who started
        // it.
        if op == "fetch" {
            self.fetch_settled(&error, true);
        }
        let landed = error.is_empty();
        self.write_refused = !landed;
        // The rows a delete took off the screen, answered by name rather
        // than by turn: what answered in between is somebody else's, and
        // this is the only thing that puts them back. `reads_from` goes
        // with it — the listings that take the rows away for good are
        // measured against it, not counted (`ops_delete::delete_answered`).
        self.delete_answered(id, !landed, reads_from);
        // A landed write moved what the two sides hold; a refused stage,
        // unstage or discard was refused *because* the rows on screen
        // drifted (the fingerprint refuses on nothing else). Both mean
        // the shown diff no longer describes the file.
        self.write_stale_diff = matches!(op.as_str(), "stage" | "unstage" | "discard")
            || (landed && matches!(op.as_str(), "commit" | "stash"));
        // Stopped part-way is not a landing: there is no commit at the
        // tip to go to, and the answer to the press is the working tree
        // (`last_write_stopped`, raised by the message before this one).
        self.write_at_tip = landed
            && !self.last_write_stopped
            && matches!(op.as_str(), "revert" | "cherry-pick" | "merge");
        self.write_moved_head = landed && matches!(op.as_str(), "checkout" | "reset");
        self.write_committed = landed && op == "commit";
        self.write_reworded = landed && op == "reword";
        self.write_stashed = landed && op == "stash";
        self.write_branch_op = op == "branch";
        self.write_pushed = op == "push";
        self.write_fetched = op == "fetch";
        self.write_seq += 1;
        // The plain branch delete's own answer, for the card that stayed
        // up to catch it, by the name it asked with. **Not part of the
        // group above**: it stands until the next plain delete is asked
        // (`branch_delete`), because a fetch answering in the same drain
        // would rewrite a group property before the card had seen it and
        // leave the card standing — the very thing it reads this for. The
        // seq says which answer it was, for a reader that needs the answer
        // in hand to be this one (`RepoPage`). A refusal that comes with a
        // report is the report's to say (the page's notice bar) and turns
        // no row; anything else git would not do is the `-D` question the
        // card asks by turning its row.
        if op == "branch" && !self.branch_delete_out.is_empty() {
            let asked = std::mem::take(&mut self.branch_delete_out);
            let (took, turned_down) = match (landed, reported) {
                (true, _) => (asked, String::new()),
                (false, false) => (String::new(), asked),
                (false, true) => (String::new(), String::new()),
            };
            self.branch_delete_landed = took;
            self.branch_delete_refused = turned_down;
            self.branch_delete_seq = self.write_seq;
        }
        // …and kept beside the group as well, because the group holds
        // only one answer and a drain can bring several. A reader waiting
        // for its own write looks for it here (`write_answers`); the
        // group is what the page reads when any answer will do.
        self.write_answers.push(WriteAnswer {
            id,
            seq: self.write_seq,
            op,
            stopped: self.last_write_stopped,
            failed: !landed,
            at_tip: self.write_at_tip,
            head_seq: i32::try_from(head_seq).unwrap_or(i32::MAX),
        });
        self.last_write_error = error;
    }
}
