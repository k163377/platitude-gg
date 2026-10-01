use std::sync::Arc;

use qtbridge::{QmlObject, qobject};

use crate::hub::{Feed, HeadMsg, OpProgressMsg, StateMsg, StatusMsg};

use super::qml_register;

// ---------------------------------------------------------------------------
// WorkTreeModel: where the tree stands, as one record — HEAD and what
// derives from it, the standing operation, and the last status's counts.
// The one place QML reads HEAD from: other models are told the same report
// (`hub::sink`) but are not a source
// (rules-refs/app-ui.md「HEAD の oid の正は」). The file list itself is a
// NavSectionModel.
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct WorkTreeModel {
    /// The first status snapshot has landed. Counts of zero mean clean only
    /// after this edge; before it they mean no answer yet.
    loaded: bool,
    /// A HEAD report has landed. Before this edge an empty `headOid` means
    /// "not read yet", after it "no commits yet" (`unborn`).
    head_known: bool,
    branch: String,
    /// Commit HEAD is on, branch or not (empty before the first commit).
    head_oid: String,
    detached: bool,
    /// A branch with no commits yet: HEAD is known and names none.
    unborn: bool,
    /// The tip of the branch HEAD is on — `head_oid` on a branch, empty
    /// detached or unborn. What the default selection and HEAD-walking
    /// verbs open on (detached: the newest row).
    branch_oid: String,
    /// The number of the HEAD report in hand (a move, or the first read
    /// after a write — `session::standing`). A write's answer names the
    /// first number a later report can carry, so a report at or above it
    /// looked after the write.
    head_seq: i32,
    /// Whether a remote already has HEAD's commit — the `already pushed`
    /// an amend wears (`SessionEvent::HeadPublished`). False while the
    /// answer in hand is about a commit HEAD has left.
    head_published: bool,
    /// The commit `head_published` answers for.
    head_published_oid: String,
    /// Whether something other than the current branch reaches its tip —
    /// whether a rewrite leaves the old commits drawn or only in the
    /// reflog. False until told: the answer that asks more of the person.
    head_reached_elsewhere: bool,
    /// The HEAD report number the last status's counts belong to
    /// (`StatusMsg::head_seq`). Not where HEAD is: after a write the refs
    /// read moves `head_seq` ahead of the status that follows. Read by
    /// claims about the counts — only a status numbered at or above a
    /// write's answer has counted that write's tree.
    status_seq: i32,
    /// The branch the last status read HEAD on, for `counts_settled`.
    status_branch: String,
    /// Whether `upstream` / `ahead` / `behind` (and the push's own
    /// `push_*` three) are about the branch the record names. Between a
    /// move of HEAD and the status behind it they are blank and the push
    /// standing is closed.
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
    /// Whether the `pull` row is greyed: the two sides have diverged, which
    /// is brought together by choosing rebase or merge
    /// (デザイン規約 §取り込んで合流させる). Derived from the counts, so
    /// blank whenever they are.
    pull_blocked: bool,
    /// This branch's own push mark (`branch.<branch>.pushRemote`), empty
    /// where none. It beats `RepoTab.pushDefault`, so the toolbar's
    /// destination and standing both read it; it rides with the status so
    /// it never pairs with another branch.
    push_remote: String,
    /// What the last status said of the branch against where a mark sends
    /// its push (`StatusMsg::push_track`), shown through the three below
    /// only while it is the branch HEAD is on (`settle`).
    status_push_track: platitude_core::remote::PushTrack,
    /// The remote branch the push's own counts are against, where a mark
    /// sends the push to another remote than the upstream's; empty
    /// everywhere else, and where nothing here tracks the branch over
    /// there. The toolbar reads these in place of `ahead` / `behind` there
    /// (`platitude_core::remote::push_standing`).
    push_tracking: String,
    push_ahead: i32,
    push_behind: i32,
    op_text: String,
    /// The same operation in git's spelling (`cherry-pick`), for the pill
    /// that says the whole command (デザイン規約 §git 用語のコード表記).
    /// From core, so the pill and the exit card cannot name different
    /// operations.
    op_command: String,
    /// The second name, when bisect runs alongside something else. A
    /// separate field so the separator is the view's to draw (規約 §余白)
    /// and `op_text` stays empty exactly when nothing runs — what every
    /// `opText === ""` test asks.
    op_also: String,
    has_conflicts: bool,
    staged_count: i32,
    unstaged_count: i32,
    untracked_count: i32,
    conflict_count: i32,
    /// How many files a `reset --hard` would take
    /// (`status::Counts::hard_reset_takes`) — not the sum of the three
    /// above.
    hard_reset_takes: i32,
    /// Rebase progress; both zero when nothing is stepping. Only a rebase
    /// keeps a count — `op_stepping` is what says whether the operation
    /// steps at all.
    op_step: i32,
    op_steps: i32,
    /// Whether the stopped operation takes `--skip` / `--quit` (all but a
    /// merge, which steps through nothing). Read from the operation, not
    /// the count: only a rebase writes one, and a stepping cherry-pick
    /// would look like a merge.
    op_stepping: bool,
    /// Whether the stopped operation is a merge — the one the commit box
    /// finishes, so its box opens filled in. Not read off `op_text`,
    /// which is a word on screen.
    op_merging: bool,
    /// The message that merge is about to record, split the way the two
    /// boxes hold it. Empty unless a merge is standing.
    op_subject: String,
    op_body: String,
    /// What to call each side of a conflict — read from the operation,
    /// because the two swap over during a rebase (the replayed commits
    /// are "theirs"). Empty where git left nothing to name a side by.
    side_ours: String,
    side_theirs: String,
    /// The merge tool git would launch, for the menu row to name. Empty
    /// with none configured — the row becomes the way to set one. Display
    /// only: `conflict::mergetool` resolves the tool itself at launch.
    merge_tool: String,
    /// Staged files whose change says something about its line endings.
    /// The working-tree side is not counted: a commit carries the index.
    eol_staged_count: i32,
    /// Pending files that need Git LFS where git cannot run it — the band's
    /// `NO LFS` (デザイン規約 §ウィンドウの縁). 0 wherever git runs it.
    lfs_needed: i32,
    /// File-list rows of each change kind, for the graph's uncommitted row
    /// (`status::Kinds`), off this status at no git of its own. Conflicts
    /// are `conflict_count`.
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
    /// the command the blocked-move question opens with — `rebase --abort`
    /// where it does, `stash` elsewhere (`offers::leaving_undoes` /
    /// `offers::leave_code`).
    leave_undoes: bool,
    leave_code: String,
    /// Whether the exit card's `--skip` loses nothing: the stop is on a
    /// commit that came out empty (`offers::skip_is_free`).
    op_skip_free: bool,
    /// Whether the standing rebase stopped on purpose at an `edit` step —
    /// a stop as clean as the empty one, told apart by git's own marker
    /// (`integrate::RebaseStop`). The exit card's words and its `--skip`'s
    /// cost both turn on it.
    op_editing: bool,
    /// The commit that stop left HEAD on, full hex (empty where git wrote
    /// none). Not the todo's id, which names no findable commit once a
    /// step before `edit` rewrote history (`integrate::RebaseStop::oid`).
    op_edit_oid: String,
    /// Bumped when a status moves any of the four bucket counts — what
    /// "somebody moved the tree" is read off, so this window's own poll
    /// answer (no count moved) does not re-read an open diff. The counts
    /// are per file: a second hunk staged out of a file already on both
    /// sides moves none of them. This window's own press is re-read at its
    /// answer (`ops::DiffReread`); one made elsewhere, by the page's tick on
    /// the open file (`RepoPage.pollDiff`).
    tree_revision: i32,
    /// Whether this status leaves the synthetic working-tree row at the
    /// head of the graph (`platitude_core::graph::wip_row_stands`). The
    /// graph answers the same one read behind, so a reader comparing the
    /// two (`GraphModel::wip_row`) can tell the picture is still arriving.
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
    /// The badge's word and count, on a faster tick than the feed above
    /// (`Feeds::op_progress`). Both carry the badge and neither arrives in
    /// the order it looked — a status reads the count after its whole
    /// `git status`, and a tick taken meanwhile lands first — so whichever
    /// looked last holds it (`badge_looked`).
    progress_feed: Option<Arc<Feed<OpProgressMsg>>>,
    /// The `looked` stamp of the read the badge's word and count came from.
    badge_looked: u64,
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
    qproperty!("pullBlocked", Member = pull_blocked, Notify = changed);
    qproperty!("pushRemote", Member = push_remote, Notify = changed);
    qproperty!("pushTracking", Member = push_tracking, Notify = changed);
    qproperty!("pushAhead", Member = push_ahead, Notify = changed);
    qproperty!("pushBehind", Member = push_behind, Notify = changed);
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
    qproperty!("lfsNeeded", Member = lfs_needed, Notify = changed);
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

    /// The tab now stands in another working copy (`Hub::restand_tab`):
    /// every answer here was the old copy's, so all of it goes back to
    /// before the first status — `loaded` included, so the counts do not
    /// read as clean before the new status lands. The feeds and the tab
    /// are kept: the next session pushes to the same ones.
    #[qslot]
    fn restand(&mut self) {
        self.forget_the_copy();
        self.changed();
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
    /// Back to before the first read, for [`WorkTreeModel::restand`]:
    /// every field but the tab and the two feeds — a field added to this
    /// model belongs here too.
    ///
    /// Written out, not `*self = Self::default()`: the bridge deletes the
    /// QObject when the value it is keyed to is dropped, and the next slot
    /// call aborts with "No proxy" (qtbridge `QObjectHolder`).
    fn forget_the_copy(&mut self) {
        self.loaded = false;
        self.head_known = false;
        self.branch = String::new();
        self.head_oid = String::new();
        self.detached = false;
        self.unborn = false;
        self.branch_oid = String::new();
        self.head_seq = 0;
        self.head_published = false;
        self.head_published_oid = String::new();
        self.head_reached_elsewhere = false;
        self.status_seq = 0;
        self.status_branch = String::new();
        self.counts_settled = false;
        self.status_upstream = String::new();
        self.status_upstream_tracked = false;
        self.status_ahead = 0;
        self.status_behind = 0;
        self.upstream = String::new();
        self.upstream_tracked = false;
        self.ahead = 0;
        self.behind = 0;
        self.pull_blocked = false;
        self.push_remote = String::new();
        self.status_push_track = platitude_core::remote::PushTrack::default();
        self.push_tracking = String::new();
        self.push_ahead = 0;
        self.push_behind = 0;
        self.op_text = String::new();
        self.op_command = String::new();
        self.op_also = String::new();
        self.has_conflicts = false;
        self.staged_count = 0;
        self.unstaged_count = 0;
        self.untracked_count = 0;
        self.conflict_count = 0;
        self.hard_reset_takes = 0;
        self.op_step = 0;
        self.op_steps = 0;
        self.op_stepping = false;
        self.op_merging = false;
        self.op_subject = String::new();
        self.op_body = String::new();
        self.side_ours = String::new();
        self.side_theirs = String::new();
        self.merge_tool = String::new();
        self.eol_staged_count = 0;
        self.lfs_needed = 0;
        self.wip_added = 0;
        self.wip_modified = 0;
        self.wip_deleted = 0;
        self.wip_renamed = 0;
        self.wip_copied = 0;
        self.stash_standing = String::new();
        self.moves_blocked = false;
        self.leave_undoes = false;
        self.leave_code = String::new();
        self.op_skip_free = false;
        self.op_editing = false;
        self.op_edit_oid = String::new();
        self.tree_revision = 0;
        self.wip_row_stands = false;
        self.seen_counts = None;
        self.counts = platitude_core::status::Counts::default();
        self.op_state = platitude_core::opstate::OpState::default();
        // Each session counts its stamps from the start.
        self.badge_looked = 0;
    }

    /// Folds in the feed's batch in the session's order, so a status never
    /// lands ahead of the HEAD report it was read beside. Answers whether
    /// anything arrived; derived answers are settled once at the end
    /// (`settle`).
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
            looked,
            op_state,
            progress,
            sides,
            op_message,
            merge_tool,
            push_remote,
            push_track,
            eol_marks,
            lfs_needed,
            stop,
        } = msg;
        self.side_ours = sides.ours;
        self.loaded = true;
        self.side_theirs = sides.theirs;
        self.merge_tool = merge_tool;
        self.eol_staged_count =
            i32::try_from(eol_marks.iter().filter(|m| m.staged).count()).unwrap_or(i32::MAX);
        self.lfs_needed = i32::try_from(lfs_needed).unwrap_or(i32::MAX);
        // HEAD is not read off here — it has its own message, ahead of
        // this one (`StateMsg::Head`). Kept: which report and branch the
        // counts were read with (`settle`).
        self.status_seq = i32::try_from(head_seq).unwrap_or(i32::MAX);
        self.status_branch = status.branch_head.clone().unwrap_or_default();
        self.status_upstream = status.upstream.clone().unwrap_or_default();
        self.status_upstream_tracked = status.upstream_tracked;
        self.status_ahead = status.ahead;
        self.status_behind = status.behind;
        self.push_remote = push_remote;
        self.status_push_track = push_track;
        self.has_conflicts = status.has_conflicts();
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
        // The badge goes to whichever read looked last (`badge_looked`). Only
        // the badge: `op_state` and the stop above stay this status's, so
        // after a tick that looked later they can stand behind it until the
        // next status. The tick runs only while a replaying write is out.
        if looked >= self.badge_looked {
            self.badge_looked = looked;
            self.settle_op(&op_state, &op_message);
            (self.op_step, self.op_steps) = match progress {
                Some(p) => (p.current as i32, p.total as i32),
                None => (0, 0),
            };
        }
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
        if self.head_published_oid != self.head_oid {
            self.head_published = false;
        }
        self.counts_settled = self.loaded && self.status_branch == self.branch;
        if self.counts_settled {
            self.upstream.clone_from(&self.status_upstream);
            self.upstream_tracked = self.status_upstream_tracked;
            self.ahead = self.status_ahead;
            self.behind = self.status_behind;
            self.pull_blocked = self.status_ahead > 0 && self.status_behind > 0;
            self.push_tracking
                .clone_from(&self.status_push_track.tracking);
            self.push_ahead = self.status_push_track.ahead;
            self.push_behind = self.status_push_track.behind;
        } else {
            self.upstream.clear();
            self.upstream_tracked = false;
            self.ahead = 0;
            self.behind = 0;
            self.pull_blocked = false;
            self.push_tracking.clear();
            self.push_ahead = 0;
            self.push_behind = 0;
        }
    }

    /// The badge, dropped where a read that looked later is already on
    /// screen. Answers whether it moved.
    ///
    /// Only the badge's word and count move here
    /// (`RepoSession::refresh_op_progress`), and "nothing standing" is
    /// ignored — the status after the write ends the badge
    /// (rules-refs/app-ui.md「バッジの `n/m` は進捗専用の tick が運ぶ」).
    fn drain_progress(&mut self) -> bool {
        let Some(feed) = self.progress_feed.clone() else {
            return false;
        };
        let Some(OpProgressMsg {
            op_state,
            progress: Some(progress),
            looked,
        }) = feed.drain().pop()
        else {
            return false;
        };
        if looked < self.badge_looked {
            return false;
        }
        self.badge_looked = looked;
        let (step, steps) = (
            i32::try_from(progress.current).unwrap_or(i32::MAX),
            i32::try_from(progress.total).unwrap_or(i32::MAX),
        );
        // Both words: bisect's second name can move while the first has
        // not.
        let said = (self.op_text.clone(), self.op_also.clone());
        self.settle_op(&op_state, "");
        let moved = (self.op_step, self.op_steps) != (step, steps)
            || (self.op_text.as_str(), self.op_also.as_str()) != (said.0.as_str(), said.1.as_str());
        (self.op_step, self.op_steps) = (step, steps);
        moved
    }

    /// The operation banner's fields, off the op state in one place.
    ///
    /// One name, from `InProgress::from_state` (what the continuations act
    /// on): a rebase stopped on a pick writes CHERRY_PICK_HEAD too, and
    /// naming both flags would call one rebase two operations.
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
        // Bisect alone runs alongside, and so can share the line.
        if op_state.bisecting {
            ops.push("BISECTING");
        }
        let mut named = ops.into_iter();
        self.op_text = named.next().unwrap_or_default().to_string();
        self.op_also = named.next().unwrap_or_default().to_string();
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
    use platitude_core::opstate::OpState;
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
            looked: 0,
            op_state: platitude_core::opstate::OpState::default(),
            progress: None,
            sides: platitude_core::conflict::Sides::default(),
            op_message: String::new(),
            merge_tool: String::new(),
            push_remote: String::new(),
            push_track: platitude_core::remote::PushTrack::default(),
            eol_marks: Arc::new(Vec::new()),
            lfs_needed: 0,
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

    /// The first read after a write reports an unmoved HEAD, and it still
    /// counts: `headSeq` is what a landing arms against.
    #[test]
    fn a_report_that_moved_nothing_still_counts() {
        let mut model = WorkTreeModel::default();
        model.absorb(vec![head(ROOT, 1)]);
        model.absorb(vec![head(ROOT, 2)]);
        assert_eq!(model.head_oid, ROOT);
        assert_eq!(model.head_seq, 2);
    }

    /// `statusSeq` stays with the status, not the HEAD report in hand
    /// (`status_seq`).
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

    /// The push destination's own counts go blank with the upstream's.
    #[test]
    fn the_push_destinations_counts_are_the_branch_they_were_read_with() {
        let mut model = WorkTreeModel::default();
        let mut read = status_of("feature", Some("origin/feature"), 0, 1, Vec::new());
        if let StateMsg::Status(status) = &mut read {
            status.push_track = platitude_core::remote::PushTrack {
                tracking: "fork/feature".to_string(),
                ahead: 1,
                behind: 2,
            };
        }
        model.absorb(vec![head_on("feature", ROOT, 1), read]);
        let pushes = |model: &WorkTreeModel| {
            (
                model.push_tracking.clone(),
                model.push_ahead,
                model.push_behind,
            )
        };
        assert_eq!(pushes(&model), ("fork/feature".to_string(), 1, 2));

        model.absorb(vec![head_on("main", NEXT, 2)]);
        assert_eq!(
            pushes(&model),
            (String::new(), 0, 0),
            "feature's destination is not main's"
        );
    }

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

    /// The badge's word is the only thing on screen naming the stopped
    /// operation (`BandStateGroup` / `BandStateCard`). A `match` over
    /// `InProgress`, so a fifth operation stops the build here rather than
    /// reaching the badge with no word.
    #[test]
    fn every_operation_that_can_stand_here_reaches_the_badge_under_its_own_word() {
        use platitude_core::integrate::InProgress;

        for op in [
            InProgress::Rebase,
            InProgress::Merge,
            InProgress::CherryPick,
            InProgress::Revert,
        ] {
            let (state, word) = match op {
                InProgress::Rebase => (
                    OpState {
                        rebasing: true,
                        ..OpState::default()
                    },
                    "REBASING",
                ),
                InProgress::Merge => (
                    OpState {
                        merging: true,
                        ..OpState::default()
                    },
                    "MERGING",
                ),
                InProgress::CherryPick => (
                    OpState {
                        cherry_picking: true,
                        ..OpState::default()
                    },
                    "CHERRY-PICKING",
                ),
                InProgress::Revert => (
                    OpState {
                        reverting: true,
                        ..OpState::default()
                    },
                    "REVERTING",
                ),
            };
            let mut model = WorkTreeModel::default();
            model.settle_op(&state, "");
            assert_eq!(model.op_text, word);
            assert!(model.op_also.is_empty(), "{word}: one operation, one word");
        }
    }

    /// The badge is measured from which slot each word lands in
    /// (`BandStateMetrics.opW`).
    #[test]
    fn a_bisect_takes_the_second_word_and_the_first_when_it_is_alone() {
        let mut model = WorkTreeModel::default();
        model.settle_op(
            &OpState {
                bisecting: true,
                ..OpState::default()
            },
            "",
        );
        assert_eq!(
            (model.op_text.as_str(), model.op_also.as_str()),
            ("BISECTING", "")
        );

        model.settle_op(
            &OpState {
                rebasing: true,
                bisecting: true,
                ..OpState::default()
            },
            "",
        );
        assert_eq!(
            (model.op_text.as_str(), model.op_also.as_str()),
            ("REBASING", "BISECTING")
        );
    }

    #[test]
    fn a_rebase_that_stopped_on_a_pick_is_one_operation_not_two() {
        let mut model = WorkTreeModel::default();
        model.settle_op(
            &OpState {
                rebasing: true,
                cherry_picking: true,
                ..OpState::default()
            },
            "",
        );
        assert_eq!(
            (model.op_text.as_str(), model.op_also.as_str()),
            ("REBASING", "")
        );

        // Nothing standing: no word, which keeps the badge off the band.
        model.settle_op(&OpState::default(), "");
        assert_eq!((model.op_text.as_str(), model.op_also.as_str()), ("", ""));
    }

    const REBASING: OpState = OpState {
        rebasing: true,
        merging: false,
        cherry_picking: false,
        reverting: false,
        bisecting: false,
    };

    /// A model whose badge a tick stamped `looked` put at `step` of `steps`.
    fn ticked(model: &mut WorkTreeModel, looked: u64, step: u32, steps: u32) {
        let feed = Arc::new(Feed::default());
        feed.push_replace(OpProgressMsg {
            op_state: REBASING,
            progress: Some(platitude_core::conflict::Progress {
                current: step,
                total: steps,
            }),
            looked,
        });
        model.progress_feed = Some(feed);
        model.drain_progress();
    }

    fn status_looked(looked: u64, op_state: OpState, count: Option<(u32, u32)>) -> StateMsg {
        let StateMsg::Status(mut msg) = status(Vec::new()) else {
            unreachable!("status() builds a status")
        };
        msg.looked = looked;
        msg.op_state = op_state;
        msg.progress =
            count.map(|(current, total)| platitude_core::conflict::Progress { current, total });
        StateMsg::Status(msg)
    }

    /// A status reads the count after its whole `git status`; a tick that
    /// looked later lands first, and the status must not take it back.
    #[test]
    fn a_status_that_looked_before_the_tick_does_not_take_the_count_back() {
        let mut model = WorkTreeModel::default();
        ticked(&mut model, 5, 7, 10);
        model.absorb(vec![status_looked(3, REBASING, Some((6, 10)))]);
        assert_eq!((model.op_step, model.op_steps), (7, 10));
        assert_eq!(model.op_text, "REBASING");
    }

    #[test]
    fn a_status_read_before_the_first_marker_does_not_take_the_badge_down() {
        let mut model = WorkTreeModel::default();
        ticked(&mut model, 5, 1, 100);
        model.absorb(vec![status_looked(3, OpState::default(), None)]);
        assert_eq!(model.op_text, "REBASING");
        assert_eq!((model.op_step, model.op_steps), (1, 100));
    }

    #[test]
    fn the_status_after_the_write_ends_the_badge() {
        let mut model = WorkTreeModel::default();
        ticked(&mut model, 5, 9, 10);
        model.absorb(vec![status_looked(8, OpState::default(), None)]);
        assert_eq!(model.op_text, "");
        assert_eq!((model.op_step, model.op_steps), (0, 0));
    }

    #[test]
    fn a_new_copy_counts_its_stamps_from_the_start() {
        let mut model = WorkTreeModel::default();
        ticked(&mut model, 50, 3, 10);
        model.forget_the_copy();
        ticked(&mut model, 1, 2, 4);
        assert_eq!((model.op_step, model.op_steps), (2, 4));
    }
}
