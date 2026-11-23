use std::collections::HashSet;
use std::sync::Arc;

use qtbridge::qobject;

use crate::hub::{Feed, HeadMsg, OpProgressMsg, StateMsg, StatusMsg};

use super::qml_register;

// ---------------------------------------------------------------------------
// WorktreeModel: where the tree stands, as one record — HEAD and what
// derives from it, the standing operation, and the last status's counts.
// The one place QML reads HEAD from: other models are told the same report
// (`hub::sink`) but are not a source. `headOid` comes off that report
// (`StateMsg::Head`) and answers detached too; `branchOid` is the tip of the
// branch HEAD is on, empty while detached. The file list itself is a
// NavSectionModel.
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct WorktreeModel {
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
    /// The branch this worktree holds: the one HEAD is on, or while HEAD
    /// is on none the one its rebase or bisect keeps to it — the branch
    /// git itself still counts as this worktree's (`HeadState::held`).
    /// What the menus' rows name as the current branch (`merge` into it,
    /// `rebase` it, `reset` it, `pull` into it), read off the tree as it
    /// is: a branch renamed outside is named anew under a menu still up
    /// (デザイン規約 §hover「中身が残っていれば字だけ替え」), and a replay's
    /// detached HEAD still names the branch it rewrites. Empty only where
    /// HEAD is on none and nothing holds one. Off HEAD alone (`take_head`).
    held_branch: String,
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
    /// The one input that makes the drop row a count of presses or a
    /// click; a derived value of the record (`StateMsg::HeadReach`) the
    /// session pushes ahead. Asked as the menu opens, the answer would move
    /// the row's `holdPresses` and `AppMenu.holdIndent` would indent every
    /// row again. Headless: `drop-commit` / `drop-commit-go`.
    head_reached_elsewhere: bool,
    /// An answer to `head_reached_elsewhere` has landed. Before this edge
    /// its `false` is "not asked yet", not the answer.
    head_reach_known: bool,
    /// The HEAD report number the last status's counts belong to
    /// (`StatusMsg::head_seq`). Not where HEAD is: after a write the refs
    /// read moves `head_seq` ahead of the status that follows. Read by
    /// claims about the counts — only a status numbered at or above a
    /// write's answer has counted that write's tree.
    status_seq: i32,
    /// The branch the last status read HEAD on, for `counts_settled`.
    status_branch: String,
    /// Whether `upstream` / `ahead` / `behind` (and the push reading,
    /// `head_push`, both marks among it) are about the branch the record
    /// names. Between a move of HEAD and the status behind it they are
    /// blank and the push stands closed, naming no destination.
    counts_settled: bool,
    /// What the last status said of its branch's standing, shown through
    /// the ones below only while it is the branch HEAD is on (`settle`).
    status_upstream: String,
    status_upstream_counts: Option<platitude_core::AheadBehind>,
    upstream: String,
    /// Zero where the upstream has no ref to count against: a branch whose
    /// upstream has no remote-tracking ref yet compares against nothing
    /// (`head_push` says which).
    ahead: i32,
    behind: i32,
    /// Whether the `pull` row is greyed: the two sides have diverged, which
    /// is brought together by choosing rebase or merge
    /// (デザイン規約 §取り込んで合流させる). Derived from the counts, so
    /// blank whenever they are.
    pull_blocked: bool,
    /// What the last status read of the push of its branch
    /// (`StatusMsg::push`), shown through `head_push` only while it is the
    /// branch HEAD is on.
    status_push: platitude_core::remote::StatusPush,
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
    ///
    /// No preset holds the two at once: a check builds
    /// `--preset rebase-conflict` with `--at`, runs `git bisect start` in
    /// it and hands it over with `--repo`.
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
    /// Whether each side is the branch HEAD is on, the name a row draws in
    /// its own colour (`FileRowMenu`; `TreeStanding::is_current`). Off HEAD
    /// and the status together (`settle`).
    side_ours_current: bool,
    side_theirs_current: bool,
    /// The merge tool git would launch, for the menu row to name. Empty
    /// with none configured — the row becomes the way to set one. Display
    /// only: `conflict::mergetool` resolves the tool itself at launch.
    merge_tool: String,
    /// Staged files whose change says something about its line endings.
    /// The working-tree side is not counted: a commit carries the index.
    eol_staged_count: i32,
    /// Pending files that need Git LFS where git cannot run it — the band's
    /// `NO LFS` (`BandStateGroup.qml`). 0 wherever git runs it.
    lfs_needed: i32,
    /// Of those, the paths a discard could not copy (the filter is
    /// required): a discard of one is not brought back, so it is held in
    /// `danger` rather than `warning` (`GitFacts.discardUnrecorded`,
    /// デザイン規約 §回数押し).
    not_copied: Vec<String>,
    /// Whether what a `reset --hard` takes holds such a path — read off
    /// the status, so the untracked ones do not count (nor in
    /// `hard_reset_takes`): most survive the reset, and the ones in the way
    /// of where it goes are not known until it is aimed. Such an untracked
    /// path stays `warning` though the copy misses it (`git add` refuses it)
    /// and the reset deletes it for good: the record's `Not-copied:` line
    /// names it. Read before the row is drawn, so the colour is settled
    /// when the menu opens.
    ///
    /// Do not colour per destination (user decision — an edge case; ask
    /// before changing): it would take one more git question as the menu
    /// opens, wherever an uncopiable untracked file stands — whether the
    /// destination holds that path, the read core's
    /// `session::discard_record::in_the_way` makes.
    hard_reset_not_copied: bool,
    /// File-list rows of each change kind, for the graph's uncommitted row
    /// (`status::Kinds`), off this status at no git of its own. Conflicts
    /// are `conflict_count`.
    wip_added: i32,
    wip_modified: i32,
    wip_deleted: i32,
    wip_renamed: i32,
    wip_copied: i32,
    /// What the worktree lets a stash do — `unborn` / `conflicts` /
    /// `clean` / `ready` (`platitude_core::stash::standing`). Empty until
    /// `loaded`, like every count here.
    stash_standing: String,
    /// Whether a move has to clear the way first — an operation standing
    /// or unmerged paths (`platitude_core::offers::moves_blocked`). What
    /// `RepoPage.standsInTheWay` reads, less the consent already given, and
    /// the switch row's `!` through the standing (`TreeStanding::moves_blocked`).
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
    /// the open file (`RepoPage.pollDiff`). Counted off the status git
    /// reported, never off the stage presses drawn over it: the tree has not
    /// moved until git has moved it.
    tree_revision: i32,
    /// Whether this status leaves the synthetic worktree row at the
    /// head of the graph (`platitude_core::graph::wip_row_stands`). The
    /// graph answers the same one read behind, so a reader comparing the
    /// two (`GraphModel::wip_row`) can tell the picture is still arriving.
    wip_row_stands: bool,
    /// The counts `tree_revision` last spoke for; `None` before the
    /// first status, which always counts as movement.
    seen_counts: Option<(i32, i32, i32, i32)>,
    /// Bumped when what the menus read of the tree (`reading`) moves —
    /// what a menu standing follows (`RepoPage.menuReadRevision`), so a
    /// poll that moved nothing stands nothing again. Never reset: a
    /// binding reads a change of value, and a count that restarted could
    /// land on the value it held.
    reading_revision: i32,
    /// The reading `reading_revision` last spoke for.
    seen_reading: Option<crate::encode::TreeReading>,
    /// The counts of the last status, kept for the answers derived from
    /// them and from HEAD together (`settle`): a HEAD report can land
    /// between two statuses, and the stash standing has to follow it. Drawn
    /// with the stage presses still out over it, as the lists draw their
    /// rows (`tally`).
    counts: platitude_core::status::Counts,
    /// The last status as git reported it, its line-ending marks and when it
    /// looked: what the counts are drawn from, with the stage presses out over
    /// it (`ops::StageAhead`), and drawn from again as those move
    /// (`restage`). Shared with the session and the bucket lists
    /// (`StatusMsg::status`).
    status: Arc<platitude_core::status::WorktreeStatus>,
    status_marks: Arc<Vec<platitude_core::session::EolMark>>,
    status_looked: u64,
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
impl WorktreeModel {
    qproperty!("loaded", Member = loaded, Notify = changed);
    qproperty!("headKnown", Member = head_known, Notify = changed);
    qproperty!("branch", Member = branch, Notify = changed);
    qproperty!("headOid", Member = head_oid, Notify = changed);
    qproperty!("detached", Member = detached, Notify = changed);
    qproperty!("heldBranch", Member = held_branch, Notify = changed);
    qproperty!("unborn", Member = unborn, Notify = changed);
    qproperty!("branchOid", Member = branch_oid, Notify = changed);
    qproperty!("headSeq", Member = head_seq, Notify = changed);
    qproperty!("headPublished", Member = head_published, Notify = changed);
    qproperty!(
        "headReachedElsewhere",
        Member = head_reached_elsewhere,
        Notify = changed
    );
    qproperty!(
        "headReachKnown",
        Member = head_reach_known,
        Notify = changed
    );
    qproperty!("statusSeq", Member = status_seq, Notify = changed);
    qproperty!("upstream", Member = upstream, Notify = changed);
    qproperty!("ahead", Member = ahead, Notify = changed);
    qproperty!("behind", Member = behind, Notify = changed);
    qproperty!("pullBlocked", Member = pull_blocked, Notify = changed);
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
    qproperty!(
        "sideOursCurrent",
        Member = side_ours_current,
        Notify = changed
    );
    qproperty!(
        "sideTheirsCurrent",
        Member = side_theirs_current,
        Notify = changed
    );
    qproperty!("mergeTool", Member = merge_tool, Notify = changed);
    qproperty!(
        "eolStagedCount",
        Member = eol_staged_count,
        Notify = changed
    );
    qproperty!("lfsNeeded", Member = lfs_needed, Notify = changed);
    qproperty!("notCopied", Member = not_copied, Notify = changed);
    qproperty!(
        "hardResetNotCopied",
        Member = hard_reset_not_copied,
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
    // The fields above the menus read, as one value (`encode::TreeReading`)
    // — built as it is read, from the fields, and behind a getter: a value
    // QML wrote would not reach Rust (`encode::tests::wire_qml`). Kept in a `var`
    // property, a read is what the property held then
    // (`RepoPage.treeAtAsk`). Read as a menu opens or stands again, never by
    // a binding: it carries `notCopied`, as long as the pending files a
    // discard cannot copy — a menu standing follows `readingRevision`.
    qproperty!("reading", Read = reading, Notify = changed);
    qproperty!(
        "readingRevision",
        Member = reading_revision,
        Notify = changed
    );

    fn reading(&self) -> crate::encode::Reading {
        crate::encode::Reading::new(self.tree_reading())
    }

    // The part of it core's rules read, and no list: what a binding that
    // follows the tree reads (`RepoPage.messageEdit`), and a press on a ref.
    qproperty!("standing", Read = standing, Notify = changed);

    fn standing(&self) -> crate::encode::Standing {
        crate::encode::Standing::new(self.tree_standing())
    }

    // The push of the branch HEAD is on (`remote::HeadPush`), as one value
    // the toolbar's push slots take (`PublishFlow`); built as it is read.
    qproperty!("headPush", Read = head_push, Notify = changed);

    fn head_push(&self) -> crate::encode::Push {
        crate::encode::Push::new(self.head_push_now())
    }

    // Said for every status drained, whether or not a row moved — the one
    // to wait on for "the status was read" (`NavSectionModel.changed` is
    // said only when its rows change) — and where the counts are drawn
    // again with the stage presses in them (`restage`), which reads
    // nothing: a read is told apart by `tree_revision` moving.
    #[qsignal]
    fn changed(&mut self);

    #[qslot]
    fn attach(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
        let consumer = crate::hub::Wake::drain(self, Self::drain);
        self.feed = crate::hub::attach_feed(tab_id, |f| &f.status, consumer);
        let consumer = crate::hub::Wake::drain(self, Self::drain);
        self.progress_feed = crate::hub::attach_feed(tab_id, |f| &f.op_progress, consumer);
    }

    /// The tab now stands in another worktree (`Hub::restand_tab`):
    /// every answer here was the old worktree's, so all of it goes back to
    /// before the first status — `loaded` included, so the counts do not
    /// read as clean before the new status lands. The feeds and the tab
    /// are kept: the next session pushes to the same ones.
    #[qslot]
    fn restand(&mut self) {
        self.forget_the_worktree();
        self.changed();
    }

    /// The stage presses out over this tree moved (`RepoTab.stagingSeq`):
    /// the counts — the Commit button's file count among them — follow the
    /// rows the lists now draw, on the same turn.
    #[qslot]
    fn restage(&mut self) {
        if self.loaded && self.tally() {
            self.settle();
            self.changed();
        }
    }
}

impl WorktreeModel {
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

    /// Where the tree stands, as core's rules read it.
    fn tree_standing(&self) -> platitude_core::offers::TreeStanding {
        platitude_core::offers::TreeStanding {
            branch: self.branch.clone(),
            detached: self.detached,
            head_oid: self.head_oid.clone(),
            op_text: self.op_text.clone(),
            op_editing: self.op_editing,
            moves_blocked: self.moves_blocked,
            upstream: self.upstream.clone(),
            diverged: self.pull_blocked,
        }
    }

    /// The push of the branch HEAD is on: the last status's reading of it,
    /// none — the branch's own mark among it — until that status is about
    /// this branch (`counts_settled`).
    fn head_push_now(&self) -> platitude_core::remote::HeadPush {
        use platitude_core::remote::{HeadBranch, HeadPush};
        if self.detached || self.branch.is_empty() {
            return HeadPush::Detached;
        }
        if !self.counts_settled {
            return HeadPush::Moving(self.branch.clone());
        }
        HeadPush::Read(HeadBranch {
            name: self.branch.clone(),
            unborn: self.unborn,
            upstream: Some(self.upstream.clone()).filter(|upstream| !upstream.is_empty()),
            upstream_counts: self.status_upstream_counts,
            status: self.status_push.clone(),
        })
    }

    /// That, and what the menus' rows draw beside it.
    fn tree_reading(&self) -> crate::encode::TreeReading {
        crate::encode::TreeReading {
            standing: self.tree_standing(),
            conflict_count: self.conflict_count,
            unborn: self.unborn,
            hard_reset_takes: self.hard_reset_takes,
            hard_reset_not_copied: self.hard_reset_not_copied,
            head_reached_elsewhere: self.head_reached_elsewhere,
            stash_standing: self.stash_standing.clone(),
            not_copied: self.not_copied.clone(),
        }
    }

    /// Back to before the first read, for [`WorktreeModel::restand`]:
    /// every field but the tab and the two feeds — a field added to this
    /// model belongs here too.
    ///
    /// Written out, not `*self = Self::default()`: the bridge deletes the
    /// QObject when the value it is keyed to is dropped, and the next slot
    /// call aborts with "No proxy" (qtbridge `QObjectHolder`).
    fn forget_the_worktree(&mut self) {
        self.loaded = false;
        self.head_known = false;
        self.branch = String::new();
        self.head_oid = String::new();
        self.detached = false;
        self.held_branch = String::new();
        self.unborn = false;
        self.branch_oid = String::new();
        self.head_seq = 0;
        self.head_published = false;
        self.head_published_oid = String::new();
        self.head_reached_elsewhere = false;
        self.head_reach_known = false;
        self.status_seq = 0;
        self.status_branch = String::new();
        self.counts_settled = false;
        self.status_upstream = String::new();
        self.status_upstream_counts = None;
        self.upstream = String::new();
        self.ahead = 0;
        self.behind = 0;
        self.pull_blocked = false;
        self.status_push = platitude_core::remote::StatusPush::default();
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
        self.side_ours_current = false;
        self.side_theirs_current = false;
        self.merge_tool = String::new();
        self.eol_staged_count = 0;
        self.lfs_needed = 0;
        self.not_copied = Vec::new();
        self.hard_reset_not_copied = false;
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
        self.seen_reading = None;
        self.counts = platitude_core::status::Counts::default();
        self.status = Arc::default();
        self.status_marks = Arc::default();
        self.status_looked = 0;
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
                    self.head_reach_known = true;
                }
            }
        }
        if arrived {
            self.settle();
            let reading = self.tree_reading();
            if self.seen_reading.as_ref() != Some(&reading) {
                self.seen_reading = Some(reading);
                self.reading_revision = self.reading_revision.wrapping_add(1);
            }
        }
        arrived
    }

    fn take_head(&mut self, head: HeadMsg) {
        self.head_oid = head.oid_hex;
        self.held_branch = if head.branch.is_empty() {
            head.held
        } else {
            head.branch.clone()
        };
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
            push,
            eol_marks,
            lfs_needed,
            not_copied,
            stop,
        } = msg;
        self.side_ours = sides.ours;
        self.loaded = true;
        self.side_theirs = sides.theirs;
        self.merge_tool = merge_tool;
        self.lfs_needed = i32::try_from(lfs_needed).unwrap_or(i32::MAX);
        self.not_copied = not_copied.as_ref().clone();
        // HEAD is not read off here — it has its own message, ahead of
        // this one (`StateMsg::Head`). Kept: which report and branch the
        // counts were read with (`settle`).
        self.status_seq = i32::try_from(head_seq).unwrap_or(i32::MAX);
        self.status_branch = status.branch_head.clone().unwrap_or_default();
        self.status_upstream = status.upstream.clone().unwrap_or_default();
        self.status_upstream_counts = status.upstream_counts;
        self.status_push = push;
        self.wip_row_stands = platitude_core::graph::wip_row_stands(&status, &op_state);
        self.op_state = op_state;
        self.op_editing = stop.editing;
        self.op_edit_oid = stop.oid;
        // Off git's own status: a press drawn over it has moved nothing yet
        // (`tree_revision`).
        let reported = platitude_core::status::Counts::of(&status);
        let tally = (
            reported.staged as i32,
            reported.unstaged as i32,
            reported.untracked as i32,
            reported.conflicted as i32,
        );
        if self.seen_counts != Some(tally) {
            self.seen_counts = Some(tally);
            self.tree_revision += 1;
        }
        self.status = status;
        self.status_marks = eol_marks;
        self.status_looked = looked;
        if crate::harness::memprobe::enabled() {
            // Held beside each bucket list's (`nav-*-all`): the counts draw
            // from it as the presses move (`restage`).
            crate::harness::memprobe::note_bytes(
                "worktree-status",
                self.tab_id,
                crate::hub::own_status_bytes(&self.status),
                self.status.items.len(),
            );
        }
        crate::hub::status_drawn(self.tab_id, crate::ops::Reader::Counts, looked);
        self.tally();
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

    /// Counts the rows the lists draw — the status git reported, with the
    /// stage presses still out over it (`hub::status_picture`) — into the
    /// counts, the kinds and the staged line-ending marks. Answers whether any
    /// of them moved; what derives from them with HEAD is `settle`'s.
    fn tally(&mut self) -> bool {
        use platitude_core::status::{Counts, Kinds, StatusItem};
        let picture = crate::hub::status_picture(
            self.tab_id,
            self.status_looked,
            &self.status,
            &self.status_marks,
        );
        let (status, marks) = match &picture {
            Some(picture) => (&picture.status, picture.marks.as_slice()),
            None => (self.status.as_ref(), self.status_marks.as_slice()),
        };
        let eol_staged =
            i32::try_from(marks.iter().filter(|m| m.staged).count()).unwrap_or(i32::MAX);
        // A set, not a scan: both lists can run to every pending file.
        let refused: HashSet<&str> = self.not_copied.iter().map(String::as_str).collect();
        let not_copied = !refused.is_empty()
            && status.items.iter().any(|item| {
                !matches!(item, StatusItem::Untracked { .. }) && refused.contains(item.path())
            });
        let conflicts = status.has_conflicts();
        let kinds = Kinds::of(status);
        let counts = Counts::of(status);
        let before = self.tallied();
        self.eol_staged_count = eol_staged;
        self.hard_reset_not_copied = not_copied;
        self.has_conflicts = conflicts;
        self.wip_added = kinds.added as i32;
        self.wip_modified = kinds.modified as i32;
        self.wip_deleted = kinds.deleted as i32;
        self.wip_renamed = kinds.renamed as i32;
        self.wip_copied = kinds.copied as i32;
        self.staged_count = counts.staged as i32;
        self.unstaged_count = counts.unstaged as i32;
        self.untracked_count = counts.untracked as i32;
        self.conflict_count = counts.conflicted as i32;
        self.hard_reset_takes = counts.hard_reset_takes() as i32;
        self.counts = counts;
        before != self.tallied()
    }

    /// What `tally` writes, as one value to tell a move by.
    fn tallied(&self) -> (platitude_core::status::Counts, [i32; 6], bool, bool) {
        (
            self.counts,
            [
                self.wip_added,
                self.wip_modified,
                self.wip_deleted,
                self.wip_renamed,
                self.wip_copied,
                self.eol_staged_count,
            ],
            self.hard_reset_not_copied,
            self.has_conflicts,
        )
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
            // The properties are Qt's own: zeroes for "nothing to count
            // against", and `int`s — a count past one is drawn as its
            // largest.
            let shown = |count: u32| i32::try_from(count).unwrap_or(i32::MAX);
            let upstream = self.status_upstream_counts;
            self.ahead = shown(upstream.map_or(0, |counts| counts.ahead));
            self.behind = shown(upstream.map_or(0, |counts| counts.behind));
            self.pull_blocked = upstream.is_some_and(platitude_core::AheadBehind::is_diverged);
        } else {
            self.upstream.clear();
            self.ahead = 0;
            self.behind = 0;
            self.pull_blocked = false;
        }
        let standing = self.tree_standing();
        self.side_ours_current = standing.is_current(&self.side_ours);
        self.side_theirs_current = standing.is_current(&self.side_theirs);
    }

    /// The badge, dropped where a read that looked later is already on
    /// screen. Answers whether it moved.
    ///
    /// Only the badge's word and count move here
    /// (`RepoSession::refresh_op_progress`), through the same `settle_op`
    /// as a status, so the words are decided in one place. "Nothing
    /// standing" is ignored — the status after the write ends the badge:
    /// the tick runs only while a replay is out, and one taken before git
    /// writes its first marker would blink the badge as it comes up.
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

qml_register!(WorktreeModel, "WorktreeModel", singleton = false);

#[cfg(test)]
mod tests;
