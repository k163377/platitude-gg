use std::sync::Arc;

use qtbridge::{QObjectHolder, qobject};

use crate::hub::{Feed, OpProgressMsg, StatusMsg};

use super::qml_register;

// ---------------------------------------------------------------------------
// WorkTreeModel: always-on header state (branch / ops / conflicts / counts).
// The working-tree file list itself is a NavSectionModel rendered by the
// right pane's WIP view.
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct WorkTreeModel {
    /// The first status snapshot has landed. Counts of zero mean clean only
    /// after this edge; before it they mean no answer yet.
    loaded: bool,
    branch: String,
    /// Commit HEAD is on, branch or not (empty before the first commit).
    head_oid: String,
    detached: bool,
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
    /// The commit that stop is about, abbreviated as git wrote it; empty
    /// where it wrote none.
    op_edit_oid: String,
    /// Bumped when a status moves any of the four bucket counts — what
    /// "somebody moved the tree" is read off, so a status that moved no
    /// count (the answer to this window's own poll) does not re-read an
    /// open diff. Counts, not rows: a second line staged out of a file
    /// already on both sides moves no row, and is exactly the change the
    /// reader of this has to hear about.
    tree_revision: i32,
    /// The counts `tree_revision` last spoke for; `None` before the
    /// first status, which always counts as movement.
    seen_counts: Option<(i32, i32, i32, i32)>,
    /// Bumped on every drained status, moved or not — the freshness token
    /// for readers that must not act on a status already in flight when
    /// they armed (the page's detached landing). `tree_revision` cannot
    /// serve there: a clean-tree write moves no count.
    status_seq: i32,
    feed: Option<Arc<Feed<StatusMsg>>>,
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
    qproperty!("branch", Member = branch, Notify = changed);
    qproperty!("headOid", Member = head_oid, Notify = changed);
    qproperty!("detached", Member = detached, Notify = changed);
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
    qproperty!("statusSeq", Member = status_seq, Notify = changed);

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
        self.drain_progress();
        let Some(feed) = self.feed.clone() else {
            return;
        };
        let Some(StatusMsg {
            status,
            op_state,
            progress,
            sides,
            op_message,
            merge_tool,
            push_remote,
            eol_marks,
            stop,
        }) = feed.drain().pop()
        else {
            return;
        };
        self.side_ours = sides.ours;
        self.loaded = true;
        self.side_theirs = sides.theirs;
        self.merge_tool = merge_tool;
        self.eol_staged_count =
            i32::try_from(eol_marks.iter().filter(|m| m.staged).count()).unwrap_or(i32::MAX);

        self.branch = status.branch_head.clone().unwrap_or_default();
        self.head_oid = status
            .branch_oid
            .as_ref()
            .map(|oid| oid.to_hex())
            .unwrap_or_default();
        self.detached = status.branch_head.is_none() && status.branch_oid.is_some();
        self.upstream = status.upstream.clone().unwrap_or_default();
        self.upstream_tracked = status.upstream_tracked;
        self.ahead = status.ahead;
        self.behind = status.behind;
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
        self.stash_standing = platitude_core::stash::standing(self.head_oid.is_empty(), &counts)
            .as_str()
            .to_string();
        self.moves_blocked = platitude_core::offers::moves_blocked(&op_state, &counts);
        let in_progress = platitude_core::integrate::InProgress::from_state(&op_state);
        self.leave_undoes = platitude_core::offers::leaving_undoes(in_progress);
        self.leave_code = platitude_core::offers::leave_code(in_progress).to_string();
        self.op_editing = stop.editing;
        self.op_edit_oid = stop.oid;
        self.op_skip_free = platitude_core::offers::skip_is_free(&counts, stop.editing);
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
        self.status_seq += 1;
        (self.op_step, self.op_steps) = match progress {
            Some(p) => (p.current as i32, p.total as i32),
            None => (0, 0),
        };
        self.changed();
    }
}

impl WorkTreeModel {
    /// The badge, taken before the snapshot beside it.
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
    fn drain_progress(&mut self) {
        let Some(feed) = self.progress_feed.clone() else {
            return;
        };
        let Some(OpProgressMsg {
            op_state,
            progress: Some(progress),
        }) = feed.drain().pop()
        else {
            return;
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
        if moved {
            self.changed();
        }
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
