use std::sync::Arc;

use qtbridge::{QObjectHolder, qobject};

use crate::hub::{Feed, Hub, StatusMsg};

use super::qml_register;

// ---------------------------------------------------------------------------
// WorkTreeModel: always-on header state (branch / ops / conflicts / counts).
// The working-tree file list itself lives in the sidebar (NavSectionModel).
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct WorkTreeModel {
    branch: String,
    /// Commit HEAD is on, branch or not (empty before the first commit).
    head_oid: String,
    detached: bool,
    upstream: String,
    ahead: i32,
    behind: i32,
    op_text: String,
    has_conflicts: bool,
    staged_count: i32,
    unstaged_count: i32,
    untracked_count: i32,
    conflict_count: i32,
    /// Files changed on both sides at once. `git stash push --staged`
    /// cannot separate those, so the option is withheld while any exist.
    partially_staged_count: i32,
    /// Rebase progress; both zero when nothing is stepping.
    op_step: i32,
    op_steps: i32,
    feed: Option<Arc<Feed<StatusMsg>>>,
    tab_id: i32,
}

#[qobject(ConvertToCamelCase, NoQmlElement)]
impl WorkTreeModel {
    qproperty!("branch", Member = branch, Notify = changed);
    qproperty!("headOid", Member = head_oid, Notify = changed);
    qproperty!("detached", Member = detached, Notify = changed);
    qproperty!("upstream", Member = upstream, Notify = changed);
    qproperty!("ahead", Member = ahead, Notify = changed);
    qproperty!("behind", Member = behind, Notify = changed);
    qproperty!("opText", Member = op_text, Notify = changed);
    qproperty!("hasConflicts", Member = has_conflicts, Notify = changed);
    qproperty!("stagedCount", Member = staged_count, Notify = changed);
    qproperty!("unstagedCount", Member = unstaged_count, Notify = changed);
    qproperty!("untrackedCount", Member = untracked_count, Notify = changed);
    qproperty!("conflictCount", Member = conflict_count, Notify = changed);
    qproperty!(
        "partiallyStagedCount",
        Member = partially_staged_count,
        Notify = changed
    );
    qproperty!("opStep", Member = op_step, Notify = changed);
    qproperty!("opSteps", Member = op_steps, Notify = changed);

    #[qsignal]
    fn changed(&mut self);

    #[qslot]
    fn attach(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
        if let Some(Some(feeds)) = Hub::with(|hub| hub.feeds(tab_id)) {
            let feed = Arc::clone(&feeds.status);
            feed.attach(self.get_qml_method_invoker());
            self.feed = Some(feed);
        }
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
        }) = feed.drain().pop()
        else {
            return;
        };

        self.branch = status.branch_head.clone().unwrap_or_default();
        self.head_oid = status
            .branch_oid
            .as_ref()
            .map(|oid| oid.to_hex())
            .unwrap_or_default();
        self.detached = status.branch_head.is_none() && status.branch_oid.is_some();
        self.upstream = status.upstream.clone().unwrap_or_default();
        self.ahead = status.ahead;
        self.behind = status.behind;
        self.has_conflicts = status.has_conflicts();
        let mut ops: Vec<&str> = Vec::new();
        if op_state.rebasing {
            ops.push("REBASING");
        }
        if op_state.merging {
            ops.push("MERGING");
        }
        if op_state.cherry_picking {
            ops.push("CHERRY-PICKING");
        }
        if op_state.reverting {
            ops.push("REVERTING");
        }
        if op_state.bisecting {
            ops.push("BISECTING");
        }
        self.op_text = ops.join(" · ");
        self.staged_count = status.staged().count() as i32;
        self.unstaged_count = status.unstaged().count() as i32;
        self.untracked_count = status.untracked().count() as i32;
        self.conflict_count = status.conflicted().count() as i32;
        self.partially_staged_count = status.partially_staged().count() as i32;
        (self.op_step, self.op_steps) = match progress {
            Some(p) => (p.current as i32, p.total as i32),
            None => (0, 0),
        };
        self.changed();
    }
}
qml_register!(WorkTreeModel, "WorkTreeModel", singleton = false);
