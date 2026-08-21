use std::sync::Arc;

use qtbridge::{QObjectHolder, qobject};

use crate::hub::{Feed, StatusMsg};

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
    op_text: String,
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
    feed: Option<Arc<Feed<StatusMsg>>>,
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
    qproperty!("opText", Member = op_text, Notify = changed);
    qproperty!("opAlso", Member = op_also, Notify = changed);
    qproperty!("hasConflicts", Member = has_conflicts, Notify = changed);
    qproperty!("stagedCount", Member = staged_count, Notify = changed);
    qproperty!("unstagedCount", Member = unstaged_count, Notify = changed);
    qproperty!("untrackedCount", Member = untracked_count, Notify = changed);
    qproperty!("conflictCount", Member = conflict_count, Notify = changed);
    qproperty!("opStep", Member = op_step, Notify = changed);
    qproperty!("opSteps", Member = op_steps, Notify = changed);
    qproperty!("opStepping", Member = op_stepping, Notify = changed);
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

    #[qsignal]
    fn changed(&mut self);

    #[qslot]
    fn attach(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
        let invoker = self.get_qml_method_invoker();
        self.feed = crate::hub::attach_feed(tab_id, |f| &f.status, invoker);
    }

    #[qslot]
    fn drain(&mut self) {
        let Some(feed) = self.feed.clone() else {
            return;
        };
        let Some(StatusMsg {
            status,
            op_state,
            progress,
            sides,
            merge_tool,
            eol_marks,
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
        self.has_conflicts = status.has_conflicts();
        // One name, not every flag that happens to be set: a rebase
        // stopped on a pick writes CHERRY_PICK_HEAD too, and joining the
        // two said `REBASING · CHERRY-PICKING` for what is one rebase.
        // Core already answers which operation is the live one — it is
        // the same answer the continuations act on.
        use platitude_core::integrate::InProgress;
        let mut ops: Vec<&str> = Vec::new();
        if let Some(op) = InProgress::from_state(&op_state) {
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
            InProgress::from_state(&op_state),
            None | Some(InProgress::Merge)
        );
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
        (self.op_step, self.op_steps) = match progress {
            Some(p) => (p.current as i32, p.total as i32),
            None => (0, 0),
        };
        self.changed();
    }
}
qml_register!(WorkTreeModel, "WorkTreeModel", singleton = false);
