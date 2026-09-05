use std::sync::Arc;

use qtbridge::{QObjectHolder, qobject};

use crate::hub::{Feed, HeadMsg, OpProgressMsg, StateMsg, StatusMsg};

use super::qml_register;

// ---------------------------------------------------------------------------
// WorkTreeModel: where the tree stands, as one record. HEAD and everything
// derived from it (branch / detached / unborn / published / held elsewhere),
// the standing operation, and the counts of the last status. **The one
// place QML reads HEAD from**: every other model that draws something at
// HEAD is told the same report by the session (`hub::sink`), and none of
// them is a source (rules-refs/app-ui.md). The working-tree file list
// itself is a NavSectionModel rendered by the right pane's WIP view.
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct WorkTreeModel {
    /// The first status snapshot has landed. Counts of zero mean clean only
    /// after this edge; before it they mean no answer yet.
    loaded: bool,
    /// A read has reported where HEAD is (`head_seq` is a report's).
    /// Before this edge `headOid` empty means "not read yet", after it
    /// "no commits yet" (`unborn`).
    head_known: bool,
    branch: String,
    /// Commit HEAD is on, branch or not (empty before the first commit).
    head_oid: String,
    detached: bool,
    /// A branch with no commits yet: HEAD is known and names none.
    unborn: bool,
    /// The tip of the branch HEAD is on — `head_oid` on a branch, empty
    /// detached or unborn. What the default selection and the verbs that
    /// walk from HEAD open on: detached, the newest row is theirs.
    branch_oid: String,
    /// The number of the report of HEAD in hand — a move, or the first
    /// read to land after a write (`session::standing`). What a landing
    /// arms on: a write's answer names the first number a report after
    /// it can carry, and a report at or above it looked after the write.
    head_seq: i32,
    /// Whether a remote already has the commit HEAD is on — the `already
    /// pushed` an amend wears. Read off the walk's own marks by the
    /// session (`SessionEvent::HeadPublished`); false while the answer
    /// in hand is about a commit HEAD is no longer on.
    head_published: bool,
    /// The commit `head_published` was answered for, so a HEAD that moved
    /// stops wearing the last commit's answer until the walk answers again.
    head_published_oid: String,
    /// Whether something other than the current branch still reaches its
    /// tip — whether a rewrite here leaves the old commits drawn or leaves
    /// them to the reflog. False until the session says otherwise, which
    /// is the answer that asks more of the person doing it.
    head_reached_elsewhere: bool,
    /// The number of the report of HEAD the last status stands beside
    /// (`StatusMsg::head_seq`) — the status's own word for which reading
    /// of HEAD its counts belong to. **Not where HEAD is**: that is
    /// `head_seq`'s report, which the refs read moves ahead of the status
    /// that follows it after a write. Read where a claim is about the
    /// counts: the reset landing's `files=` is the tree the reset left,
    /// and only a status numbered at or above the write's answer has
    /// counted it.
    status_seq: i32,
    /// The branch the last status read HEAD on, for `counts_settled`.
    status_branch: String,
    /// Whether `upstream` / `ahead` / `behind` are about the branch the
    /// record names — the status they came with read HEAD on it. Between
    /// a move of HEAD and the status behind it, the three are blank
    /// rather than another branch's, and the push standing is closed.
    counts_settled: bool,
    /// What the last status said of its branch's standing, shown through
    /// the three below only while it is the branch HEAD is on (`settle`).
    status_upstream: String,
    status_upstream_tracked: bool,
    status_ahead: i32,
    status_behind: i32,
    upstream: String,
    /// Whether `ahead` / `behind` mean anything: a branch whose upstream has
    /// no remote-tracking ref yet compares against nothing, and zeroes there
    /// would read as "already sent".
    upstream_tracked: bool,
    ahead: i32,
    behind: i32,
    /// Where this branch's own mark sends a push
    /// (`branch.<branch>.pushRemote`), empty where it marks none. It beats
    /// the repository's `RepoTab.pushDefault`, so the toolbar's
    /// destination and standing both have to read it — it rides here
    /// rather than on the tab because it belongs to the branch above,
    /// and arrives with it.
    push_remote: String,
    op_text: String,
    /// The same operation in git's own spelling (`cherry-pick`), for the
    /// one place a pill has to say the whole command rather than a word
    /// on a badge (§git 用語のコード表記). Core answers it, so the pill
    /// and the exit card cannot come to name two different operations.
    op_command: String,
    /// The second name, when bisect is running alongside something else.
    /// Two fields rather than one joined string: what goes between them
    /// is a mark the showing side draws (規約 §余白), and `op_text` stays
    /// empty exactly when nothing is running — which is what every
    /// `opText === ""` test in the UI is asking.
    op_also: String,
    has_conflicts: bool,
    staged_count: i32,
    unstaged_count: i32,
    untracked_count: i32,
    conflict_count: i32,
    /// How many files a `reset --hard` would take with it
    /// (`status::Counts::hard_reset_takes`). Not the sum of the three
    /// above: a file changed on both sides is one loss, and the untracked
    /// ones are left where they are.
    hard_reset_takes: i32,
    /// Rebase progress; both zero when nothing is stepping. Only a rebase
    /// keeps a count — `op_stepping` is what says whether the operation
    /// steps at all.
    op_step: i32,
    op_steps: i32,
    /// Whether the stopped operation takes `--skip` / `--quit`. A merge
    /// steps through nothing, so it has no commit to leave out and
    /// nowhere to stop stepping; a rebase, a cherry-pick and a revert all
    /// do. Read from the operation itself rather than from the progress
    /// count: only a rebase writes one, and a cherry-pick that steps
    /// would look like a merge if the count were the test.
    op_stepping: bool,
    /// Whether the stopped operation is a merge. The one the commit box
    /// finishes, so the one whose box opens filled in — and told apart
    /// from `op_text` because that is a word on screen, not a question
    /// to branch on.
    op_merging: bool,
    /// The message that merge is about to record, split the way the two
    /// boxes hold it. Empty unless a merge is standing.
    op_subject: String,
    op_body: String,
    /// What to call each side of a conflict. **The two swap over during a
    /// rebase** (the commits being replayed are "theirs"), which is why
    /// these are read from the operation rather than worked out here.
    /// Empty where git left nothing to name a side by.
    side_ours: String,
    side_theirs: String,
    /// The merge tool git would launch, for the menu row to name. Empty
    /// with none configured — the row becomes the way to set one. Display
    /// only: `conflict::mergetool` resolves the tool itself at launch, so
    /// a name that went stale between poll and click cannot start anything.
    merge_tool: String,
    /// Staged files whose change says something about its line endings.
    /// A commit carries the index, so the working-tree side is not counted
    /// here — it is a warning about the next `git add`, not this commit.
    eol_staged_count: i32,
    /// How many rows of each change kind the file list holds, for the
    /// graph's uncommitted row to name (`status::Kinds`). Rows rather than
    /// files, so the row's tally and the list below it cannot disagree —
    /// and read off the status this model already has, so the row costs no
    /// git of its own. Conflicts are already counted above.
    wip_added: i32,
    wip_modified: i32,
    wip_deleted: i32,
    wip_renamed: i32,
    wip_copied: i32,
    /// What the working tree lets a stash do — `unborn` / `conflicts` /
    /// `clean` / `ready` (`platitude_core::stash::standing`). Empty until
    /// `loaded`, like every count here.
    stash_standing: String,
    /// Whether a move has to clear the way first — an operation standing
    /// or unmerged paths (`platitude_core::offers::moves_blocked`). What
    /// `RepoPage.standsInTheWay` reads, less the consent already given.
    moves_blocked: bool,
    /// Whether putting the standing operation down costs anything, and
    /// the command the question opens with — `rebase --abort` where it
    /// does, `stash` everywhere else (`offers::leaving_undoes` /
    /// `offers::leave_code`). The pair the blocked-move question is
    /// shaped by.
    leave_undoes: bool,
    leave_code: String,
    /// Whether the exit card's `--skip` loses nothing: the stop is on a
    /// commit that came out empty (`offers::skip_is_free`).
    op_skip_free: bool,
    /// Whether the standing rebase stopped on purpose at an `edit` step —
    /// the stop whose tree is as clean as the empty one, told apart by
    /// git's own marker (`integrate::RebaseStop`). The exit card's words
    /// and its `--skip`'s cost both turn on it.
    op_editing: bool,
    /// The commit that stop left HEAD on, full hex; empty where git wrote
    /// none. The card abbreviates it itself — it is one the reader can go
    /// and find, which the todo's own id is not once anything ahead of the
    /// `edit` step rewrote history (`integrate::RebaseStop::oid`).
    op_edit_oid: String,
    /// Bumped when a status moves any of the four bucket counts — what
    /// "somebody moved the tree" is read off, so a status that moved no
    /// count (the answer to this window's own poll) does not re-read an
    /// open diff. Counts, not rows: a second line staged out of a file
    /// already on both sides moves no row, and is exactly the change the
    /// reader of this has to hear about.
    tree_revision: i32,
    /// Whether this status leaves the synthetic working-tree row standing
    /// at the head of the graph (`platitude_core::graph::wip_row_stands`).
    ///
    /// The graph is walked from the same answer, one read behind: the walk
    /// asks it of the record this status wrote, so between a status that
    /// moves this and the pass it asks for, the rows on screen are the
    /// previous answer's. What reads the two together is what has to know
    /// the picture is still arriving (`GraphModel::wip_row`).
    wip_row_stands: bool,
    /// The counts `tree_revision` last spoke for; `None` before the
    /// first status, which always counts as movement.
    seen_counts: Option<(i32, i32, i32, i32)>,
    /// The counts of the last status, kept for the answers derived from
    /// them and from HEAD together (`settle`): a HEAD report can land
    /// between two statuses, and the stash standing has to follow it.
    counts: platitude_core::status::Counts,
    /// The last status's standing operation, kept for the same reason.
    op_state: platitude_core::opstate::OpState,
    feed: Option<Arc<Feed<StateMsg>>>,
    /// The badge's own halves, arriving several times a second while the
    /// feed above arrives every ten (`Feeds::op_progress`). Both wake the
    /// one `drain` slot, which is why they are read there in the order the
    /// screen wants them: a status snapshot carries a count of its own,
    /// and the one it carries is the older of the two.
    progress_feed: Option<Arc<Feed<OpProgressMsg>>>,
    tab_id: i32,
}

#[qobject(ConvertToCamelCase, NoQmlElement)]
impl WorkTreeModel {
    qproperty!("loaded", Member = loaded, Notify = changed);
    qproperty!("headKnown", Member = head_known, Notify = changed);
    qproperty!("branch", Member = branch, Notify = changed);
    qproperty!("headOid", Member = head_oid, Notify = changed);
    qproperty!("detached", Member = detached, Notify = changed);
    qproperty!("unborn", Member = unborn, Notify = changed);
    qproperty!("branchOid", Member = branch_oid, Notify = changed);
    qproperty!("headSeq", Member = head_seq, Notify = changed);
    qproperty!("headPublished", Member = head_published, Notify = changed);
    qproperty!(
        "headReachedElsewhere",
        Member = head_reached_elsewhere,
        Notify = changed
    );
    qproperty!("statusSeq", Member = status_seq, Notify = changed);
    qproperty!("countsSettled", Member = counts_settled, Notify = changed);
    qproperty!("upstream", Member = upstream, Notify = changed);
    qproperty!(
        "upstreamTracked",
        Member = upstream_tracked,
        Notify = changed
    );
    qproperty!("ahead", Member = ahead, Notify = changed);
    qproperty!("behind", Member = behind, Notify = changed);
    qproperty!("pushRemote", Member = push_remote, Notify = changed);
    qproperty!("opText", Member = op_text, Notify = changed);
    qproperty!("opCommand", Member = op_command, Notify = changed);
    qproperty!("opAlso", Member = op_also, Notify = changed);
    qproperty!("hasConflicts", Member = has_conflicts, Notify = changed);
    qproperty!("stagedCount", Member = staged_count, Notify = changed);
    qproperty!("unstagedCount", Member = unstaged_count, Notify = changed);
    qproperty!("untrackedCount", Member = untracked_count, Notify = changed);
    qproperty!("conflictCount", Member = conflict_count, Notify = changed);
    qproperty!(
        "hardResetTakes",
        Member = hard_reset_takes,
        Notify = changed
    );
    qproperty!("opStep", Member = op_step, Notify = changed);
    qproperty!("opSteps", Member = op_steps, Notify = changed);
    qproperty!("opStepping", Member = op_stepping, Notify = changed);
    qproperty!("opMerging", Member = op_merging, Notify = changed);
    qproperty!("opSubject", Member = op_subject, Notify = changed);
    qproperty!("opBody", Member = op_body, Notify = changed);
    qproperty!("sideOurs", Member = side_ours, Notify = changed);
    qproperty!("sideTheirs", Member = side_theirs, Notify = changed);
    qproperty!("mergeTool", Member = merge_tool, Notify = changed);
    qproperty!(
        "eolStagedCount",
        Member = eol_staged_count,
        Notify = changed
    );
    qproperty!("wipAdded", Member = wip_added, Notify = changed);
    qproperty!("wipModified", Member = wip_modified, Notify = changed);
    qproperty!("wipDeleted", Member = wip_deleted, Notify = changed);
    qproperty!("wipRenamed", Member = wip_renamed, Notify = changed);
    qproperty!("wipCopied", Member = wip_copied, Notify = changed);
    qproperty!("stashStanding", Member = stash_standing, Notify = changed);
    qproperty!("movesBlocked", Member = moves_blocked, Notify = changed);
    qproperty!("leaveUndoes", Member = leave_undoes, Notify = changed);
    qproperty!("leaveCode", Member = leave_code, Notify = changed);
    qproperty!("opSkipFree", Member = op_skip_free, Notify = changed);
    qproperty!("opEditing", Member = op_editing, Notify = changed);
    qproperty!("opEditOid", Member = op_edit_oid, Notify = changed);
    qproperty!("treeRevision", Member = tree_revision, Notify = changed);
    qproperty!("wipRowStands", Member = wip_row_stands, Notify = changed);

    #[qsignal]
    fn changed(&mut self);

    #[qslot]
    fn attach(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
        let invoker = self.get_qml_method_invoker();
        self.feed = crate::hub::attach_feed(tab_id, |f| &f.status, invoker);
        let invoker = self.get_qml_method_invoker();
        self.progress_feed = crate::hub::attach_feed(tab_id, |f| &f.op_progress, invoker);
    }

    #[qslot]
    fn drain(&mut self) {
        let progressed = self.drain_progress();
        let Some(feed) = self.feed.clone() else {
            if progressed {
                self.changed();
            }
            return;
        };
        let arrived = self.absorb(feed.drain());
        if arrived || progressed {
            self.changed();
        }
    }
}

impl WorkTreeModel {
    /// Everything the feed had waiting, folded in — in the order the
    /// session said it, so a status never lands ahead of the HEAD report
    /// it was read beside. Answers whether anything arrived; the derived
    /// answers are settled once at the end, off HEAD and the counts
    /// together.
    pub(crate) fn absorb(&mut self, batch: Vec<StateMsg>) -> bool {
        let mut arrived = false;
        for msg in batch {
            arrived = true;
            match msg {
                StateMsg::Head(head) => self.take_head(head),
                StateMsg::Status(status) => self.take_status(*status),
                StateMsg::HeadPublished { oid_hex, published } => {
                    self.head_published_oid = oid_hex;
                    self.head_published = published;
                }
                StateMsg::HeadReach { reached_elsewhere } => {
                    self.head_reached_elsewhere = reached_elsewhere;
                }
            }
        }
        if arrived {
            self.settle();
        }
        arrived
    }

    fn take_head(&mut self, head: HeadMsg) {
        self.head_oid = head.oid_hex;
        self.branch = head.branch;
        self.detached = head.detached;
        self.head_seq = i32::try_from(head.seq).unwrap_or(i32::MAX);
    }

    fn take_status(&mut self, msg: StatusMsg) {
        let StatusMsg {
            status,
            head_seq,
            op_state,
            progress,
            sides,
            op_message,
            merge_tool,
            push_remote,
            eol_marks,
            stop,
        } = msg;
        self.side_ours = sides.ours;
        self.loaded = true;
        self.side_theirs = sides.theirs;
        self.merge_tool = merge_tool;
        self.eol_staged_count =
            i32::try_from(eol_marks.iter().filter(|m| m.staged).count()).unwrap_or(i32::MAX);
        // HEAD itself is not read off here: the session reports it in
        // its own message, ahead of this one where this status is what
        // moved it (`StateMsg::Head`). What is kept is which report the
        // counts stand beside, and which branch they were read with —
        // they are shown only while that is the branch HEAD is on
        // (`settle`).
        self.status_seq = i32::try_from(head_seq).unwrap_or(i32::MAX);
        self.status_branch = status.branch_head.clone().unwrap_or_default();
        self.status_upstream = status.upstream.clone().unwrap_or_default();
        self.status_upstream_tracked = status.upstream_tracked;
        self.status_ahead = status.ahead;
        self.status_behind = status.behind;
        self.push_remote = push_remote;
        self.has_conflicts = status.has_conflicts();
        self.settle_op(&op_state, &op_message);
        // One pass, not five: `-uall` lists every untracked file, so the
        // list is as long as the working tree is dirty.
        // Same pass, same source: the kinds are the letters the file rows
        // carry, so the graph row's tally is the list it sits above.
        let kinds = platitude_core::status::Kinds::of(&status);
        self.wip_added = kinds.added as i32;
        self.wip_modified = kinds.modified as i32;
        self.wip_deleted = kinds.deleted as i32;
        self.wip_renamed = kinds.renamed as i32;
        self.wip_copied = kinds.copied as i32;
        let counts = platitude_core::status::Counts::of(&status);
        self.staged_count = counts.staged as i32;
        self.unstaged_count = counts.unstaged as i32;
        self.untracked_count = counts.untracked as i32;
        self.conflict_count = counts.conflicted as i32;
        self.hard_reset_takes = counts.hard_reset_takes() as i32;
        self.counts = counts;
        self.wip_row_stands = platitude_core::graph::wip_row_stands(&status, &op_state);
        self.op_state = op_state;
        self.op_editing = stop.editing;
        self.op_edit_oid = stop.oid;
        let tally = (
            self.staged_count,
            self.unstaged_count,
            self.untracked_count,
            self.conflict_count,
        );
        if self.seen_counts != Some(tally) {
            self.seen_counts = Some(tally);
            self.tree_revision += 1;
        }
        (self.op_step, self.op_steps) = match progress {
            Some(p) => (p.current as i32, p.total as i32),
            None => (0, 0),
        };
    }

    /// The answers derived from HEAD and the last status together —
    /// settled once per drain, after both have been taken, so neither
    /// can be read against the other's previous value.
    fn settle(&mut self) {
        self.head_known = self.head_seq > 0;
        self.unborn = self.head_known && self.head_oid.is_empty();
        self.branch_oid = if self.detached {
            String::new()
        } else {
            self.head_oid.clone()
        };
        self.stash_standing =
            platitude_core::stash::standing(self.head_oid.is_empty(), &self.counts)
                .as_str()
                .to_string();
        self.moves_blocked = platitude_core::offers::moves_blocked(&self.op_state, &self.counts);
        let in_progress = platitude_core::integrate::InProgress::from_state(&self.op_state);
        self.leave_undoes = platitude_core::offers::leaving_undoes(in_progress);
        self.leave_code = platitude_core::offers::leave_code(in_progress).to_string();
        self.op_skip_free = platitude_core::offers::skip_is_free(&self.counts, self.op_editing);
        // The walk's answer is about one commit; a HEAD that has moved on
        // wears no warning until the walk has answered for where it is.
        if self.head_published_oid != self.head_oid {
            self.head_published = false;
        }
        // The counts are about the branch they were read with. A report
        // that moved HEAD to another branch lands ahead of the status
        // read behind it, and until that status the three say nothing
        // rather than the old branch's numbers under the new name.
        self.counts_settled = self.loaded && self.status_branch == self.branch;
        if self.counts_settled {
            self.upstream.clone_from(&self.status_upstream);
            self.upstream_tracked = self.status_upstream_tracked;
            self.ahead = self.status_ahead;
            self.behind = self.status_behind;
        } else {
            self.upstream.clear();
            self.upstream_tracked = false;
            self.ahead = 0;
            self.behind = 0;
        }
    }

    /// The badge, taken before the snapshot beside it. Answers whether it
    /// moved.
    ///
    /// **Only the badge moves.** Nothing else a status carries can have
    /// changed without a write answering for it, and the badge is the one
    /// thing on screen counting — so this is the whole of what the short
    /// tick pays for (`RepoSession::refresh_op_progress`).
    ///
    /// **The word comes with the count**, or the count would arrive at a
    /// badge that is not up: a rebase of a few hundred commits is over
    /// long before the ten-second tick that would have raised it, and
    /// `opText` is what the band draws on.
    ///
    /// Silent when nothing arrived, and silent about "nothing standing" as
    /// well: this tick runs only while a write that replays is out, and
    /// the moment before git writes its first marker it would otherwise
    /// answer "no operation" — which, taken, is the badge flickering off
    /// at the very start of the thing it is there to announce. What ends
    /// the badge is the status read after the write lands.
    fn drain_progress(&mut self) -> bool {
        let Some(feed) = self.progress_feed.clone() else {
            return false;
        };
        let Some(OpProgressMsg {
            op_state,
            progress: Some(progress),
        }) = feed.drain().pop()
        else {
            return false;
        };
        let (step, steps) = (
            i32::try_from(progress.current).unwrap_or(i32::MAX),
            i32::try_from(progress.total).unwrap_or(i32::MAX),
        );
        // Both words, not just the first: everything else `settle_op`
        // writes is derived from the one operation it names, but the
        // second name is bisect's — which runs alongside rather than
        // instead, and so can arrive while the first has not moved.
        let said = (self.op_text.clone(), self.op_also.clone());
        self.settle_op(&op_state, "");
        let moved = (self.op_step, self.op_steps) != (step, steps)
            || (self.op_text.as_str(), self.op_also.as_str()) != (said.0.as_str(), said.1.as_str());
        (self.op_step, self.op_steps) = (step, steps);
        moved
    }

    /// The operation banner's fields, off the op state in one place.
    ///
    /// One name, not every flag that happens to be set: a rebase stopped
    /// on a pick writes CHERRY_PICK_HEAD too, and joining the two said
    /// `REBASING · CHERRY-PICKING` for what is one rebase. Core already
    /// answers which operation is the live one — it is the same answer
    /// the continuations act on.
    fn settle_op(&mut self, op_state: &platitude_core::opstate::OpState, op_message: &str) {
        use platitude_core::integrate::InProgress;
        let mut ops: Vec<&str> = Vec::new();
        self.op_command = InProgress::from_state(op_state)
            .map(InProgress::command)
            .unwrap_or_default()
            .to_string();
        if let Some(op) = InProgress::from_state(op_state) {
            ops.push(match op {
                InProgress::Rebase => "REBASING",
                InProgress::Merge => "MERGING",
                InProgress::CherryPick => "CHERRY-PICKING",
                InProgress::Revert => "REVERTING",
            });
        }
        // Bisect is not one of those — it runs alongside rather than
        // instead, and it is the one thing here that can share the line.
        if op_state.bisecting {
            ops.push("BISECTING");
        }
        let mut named = ops.into_iter();
        self.op_text = named.next().unwrap_or_default().to_string();
        self.op_also = named.next().unwrap_or_default().to_string();
        // A merge steps through nothing, so it takes neither skip nor
        // quit; everything else here does.
        self.op_stepping = !matches!(
            InProgress::from_state(op_state),
            None | Some(InProgress::Merge)
        );
        self.op_merging = InProgress::from_state(op_state) == Some(InProgress::Merge);
        (self.op_subject, self.op_body) = platitude_core::commit::split_message(op_message);
    }
}

qml_register!(WorkTreeModel, "WorkTreeModel", singleton = false);

#[cfg(test)]
mod tests {
    use super::*;
    use platitude_core::status::{StatusItem, WorkTreeStatus};

    const ROOT: &str = "1111111111111111111111111111111111111111";
    const NEXT: &str = "2222222222222222222222222222222222222222";

    fn head_on(branch: &str, oid_hex: &str, seq: u64) -> StateMsg {
        StateMsg::Head(HeadMsg {
            oid_hex: oid_hex.to_string(),
            branch: branch.to_string(),
            detached: false,
            seq,
        })
    }

    fn head(oid_hex: &str, seq: u64) -> StateMsg {
        head_on("main", oid_hex, seq)
    }

    fn status(items: Vec<StatusItem>) -> StateMsg {
        status_of("main", None, 0, 1, items)
    }

    fn status_of(
        branch: &str,
        upstream: Option<&str>,
        ahead: i32,
        head_seq: u64,
        items: Vec<StatusItem>,
    ) -> StateMsg {
        StateMsg::Status(Box::new(StatusMsg {
            status: WorkTreeStatus {
                branch_oid: None,
                branch_head: Some(branch.to_string()),
                upstream: upstream.map(str::to_string),
                upstream_tracked: upstream.is_some(),
                ahead,
                behind: 0,
                items,
            },
            head_seq,
            op_state: platitude_core::opstate::OpState::default(),
            progress: None,
            sides: platitude_core::conflict::Sides::default(),
            op_message: String::new(),
            merge_tool: String::new(),
            push_remote: String::new(),
            eol_marks: Arc::new(Vec::new()),
            stop: platitude_core::integrate::RebaseStop::default(),
        }))
    }

    /// HEAD comes off its own report and nothing else: a status that
    /// names no commit does not make the tree unborn once a HEAD is known.
    #[test]
    fn head_is_the_reports_own_and_the_status_does_not_overwrite_it() {
        let mut model = WorkTreeModel::default();
        assert!(!model.absorb(Vec::new()), "nothing arrived, nothing said");
        assert!(!model.head_known);
        assert!(model.absorb(vec![head(ROOT, 1), status(Vec::new())]));
        assert!(model.head_known);
        assert!(model.loaded);
        assert_eq!(model.head_oid, ROOT);
        assert_eq!(
            model.branch_oid, ROOT,
            "on a branch, its tip is HEAD's commit"
        );
        assert_eq!(model.head_seq, 1);
        assert!(!model.unborn);
        assert_eq!(model.stash_standing, "clean");
    }

    #[test]
    fn a_branch_with_no_commits_is_unborn_once_head_has_been_read() {
        let mut model = WorkTreeModel::default();
        model.absorb(vec![status(Vec::new())]);
        assert!(!model.unborn, "no HEAD report yet is not an unborn branch");
        model.absorb(vec![head("", 1)]);
        assert!(model.unborn);
        assert_eq!(model.stash_standing, "unborn");
    }

    /// Detached, the branch has no tip of its own: what opens on the
    /// branch's commit opens on the newest row instead.
    #[test]
    fn detached_names_no_branch_tip() {
        let mut model = WorkTreeModel::default();
        model.absorb(vec![StateMsg::Head(HeadMsg {
            oid_hex: ROOT.to_string(),
            branch: String::new(),
            detached: true,
            seq: 1,
        })]);
        assert_eq!(model.head_oid, ROOT);
        assert_eq!(model.branch_oid, "");
    }

    /// The walk's answer is about the commit it names: a HEAD that has
    /// moved on wears no `already pushed` until the walk answers again,
    /// and the answer that then arrives for the new commit puts it back.
    #[test]
    fn the_published_answer_follows_the_commit_it_is_about() {
        let mut model = WorkTreeModel::default();
        model.absorb(vec![
            head(ROOT, 1),
            StateMsg::HeadPublished {
                oid_hex: ROOT.to_string(),
                published: true,
            },
        ]);
        assert!(model.head_published);
        model.absorb(vec![head(NEXT, 2)]);
        assert!(!model.head_published, "an answer about the last commit");
        model.absorb(vec![StateMsg::HeadPublished {
            oid_hex: NEXT.to_string(),
            published: true,
        }]);
        assert!(model.head_published);
    }

    /// The report the first read after a write sends — HEAD unmoved —
    /// still counts: `headSeq` is what a landing arms against.
    #[test]
    fn a_report_that_moved_nothing_still_counts() {
        let mut model = WorkTreeModel::default();
        model.absorb(vec![head(ROOT, 1)]);
        model.absorb(vec![head(ROOT, 2)]);
        assert_eq!(model.head_oid, ROOT);
        assert_eq!(model.head_seq, 2);
    }

    /// A status says which report of HEAD it stands beside, apart from
    /// the report in hand: after a write the refs read moves HEAD ahead
    /// of the status that follows, and a claim about the tree waits on
    /// the status's own number.
    #[test]
    fn the_counts_stand_beside_the_report_they_were_read_under() {
        let mut model = WorkTreeModel::default();
        model.absorb(vec![
            head(ROOT, 1),
            status_of("main", None, 0, 1, Vec::new()),
        ]);
        assert_eq!(model.status_seq, 1);
        model.absorb(vec![head(NEXT, 2)]);
        assert_eq!(model.head_seq, 2);
        assert_eq!(model.status_seq, 1, "the counts are still the old tree's");
        model.absorb(vec![status_of("main", None, 0, 2, Vec::new())]);
        assert_eq!(model.status_seq, 2);
    }

    /// The counts are about the branch they were read with. A move to
    /// another branch is reported ahead of the status behind it, and
    /// until that status the standing is blank rather than the old
    /// branch's under the new name.
    #[test]
    fn the_counts_are_the_branch_they_were_read_with() {
        let mut model = WorkTreeModel::default();
        model.absorb(vec![
            head_on("feature", ROOT, 1),
            status_of("feature", Some("origin/feature"), 2, 1, Vec::new()),
        ]);
        assert!(model.counts_settled);
        assert_eq!(model.upstream, "origin/feature");
        assert_eq!(model.ahead, 2);

        model.absorb(vec![head_on("main", NEXT, 2)]);
        assert!(!model.counts_settled, "feature's counts are not main's");
        assert_eq!(model.upstream, "");
        assert!(!model.upstream_tracked);
        assert_eq!(model.ahead, 0);

        model.absorb(vec![status_of(
            "main",
            Some("origin/main"),
            0,
            2,
            Vec::new(),
        )]);
        assert!(model.counts_settled);
        assert_eq!(model.upstream, "origin/main");
        assert_eq!(model.ahead, 0);
    }

    /// The counts and HEAD settle together: a status arriving between two
    /// HEAD reports reads the stash standing against the HEAD in hand.
    #[test]
    fn the_stash_standing_is_settled_off_head_and_the_counts_together() {
        let mut model = WorkTreeModel::default();
        model.absorb(vec![head("", 1)]);
        model.absorb(vec![status(vec![StatusItem::Untracked {
            path: "f.txt".to_string(),
        }])]);
        assert_eq!(model.stash_standing, "unborn", "no commit to stash on");
        model.absorb(vec![head(ROOT, 2)]);
        assert_eq!(
            model.stash_standing, "ready",
            "the same tree, once it has a commit"
        );
    }
}
