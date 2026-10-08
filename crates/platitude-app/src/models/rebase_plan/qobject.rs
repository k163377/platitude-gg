//! Everything QML sees of the plan: the header fields, the feed it
//! drains, the verbs and reorders the rows take, and the run.

use super::*;

#[qobject(Base = QListModel, ConvertToCamelCase, NoQmlElement)]
impl RebasePlanModel {
    qproperty!("active", Member = active, Notify = changed);
    qproperty!("loading", Member = loading, Notify = changed);
    qproperty!("fromOid", Member = from_oid, Notify = changed);
    qproperty!("ontoOid", Member = onto_oid, Notify = changed);
    qproperty!("root", Member = root, Notify = changed);
    qproperty!("ontoSubject", Member = onto_subject, Notify = changed);
    qproperty!("ontoAuthor", Member = onto_author, Notify = changed);
    qproperty!("ontoEmail", Member = onto_email, Notify = changed);
    qproperty!("ontoAvatar", Member = onto_avatar, Notify = changed);
    qproperty!("ontoAvatarUrl", Member = onto_avatar_url, Notify = changed);
    qproperty!("ontoRef", Member = onto_ref, Notify = changed);
    qproperty!("expectHead", Member = expect_head, Notify = changed);
    qproperty!("pushedCount", Member = pushed_count, Notify = changed);
    qproperty!("dirty", Member = dirty, Notify = changed);
    qproperty!("dropCount", Member = drops, Notify = changed);
    qproperty!("stepCount", Member = step_count, Notify = changed);
    qproperty!("selectedRow", Member = selected_row, Notify = changed);
    qproperty!("selectedAction", Member = selected_action, Notify = changed);
    qproperty!("selectedOid", Member = selected_oid, Notify = changed);
    qproperty!(
        "selectedMsgSubject",
        Member = selected_msg_subject,
        Notify = changed
    );
    qproperty!(
        "selectedMsgBody",
        Member = selected_msg_body,
        Notify = changed
    );

    #[qsignal]
    pub(super) fn changed(&mut self);

    /// The plan could not be opened, before anything was touched:
    /// `across-merge` (the range holds one), `off-branch` (the click was
    /// on another branch's row) or `unfetched-base` (the range bottoms
    /// out on a commit this clone never fetched). The spellings are the
    /// row menu's (`ReportKind` → `drain`), so one `Words.rewriteRefusedWhy`
    /// writes the sentence for both doors
    /// (rules-refs/app-ui.md「Rust に文言を置かない」).
    #[qsignal]
    pub(super) fn refused_plan(&mut self, kind: String);

    /// The branch tip moved under the open plan, so it was put away;
    /// nothing ran (§答えの要らない報せ).
    #[qsignal]
    fn stale_plan(&mut self);

    /// An operation (anything the badge names) started under the open
    /// plan, so it was put away; nothing ran. Not `stalePlan`: the tip has
    /// not moved, and saying it did sends a reader looking for a rewrite.
    #[qsignal]
    fn standing_op(&mut self);

    /// The plan was handed over and put away. Said at the press, since
    /// `replaying` only rises when the queue starts the write
    /// (`RepoPage.planRunOut` holds the gap).
    #[qsignal]
    fn plan_ran(&mut self);

    #[qslot]
    fn attach(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
        let invoker = self.get_qml_method_invoker();
        self.feed = crate::hub::attach_feed(tab_id, |f| &f.plan, invoker);
    }

    #[qslot]
    fn drain(&mut self) {
        let Some(feed) = self.feed.clone() else {
            return;
        };
        for msg in feed.drain() {
            self.take(msg);
        }
    }

    /// Asks for the rows an interactive rebase from `oid_hex` would
    /// offer; the answer arrives through the feed and raises `active`,
    /// or `refusedPlan` where the range cannot be replayed.
    ///
    /// The same commit asked again while its answer is out is left to that
    /// answer ([`Self::already_asking`]) — the read walks the whole range.
    /// Another commit replaces the ask (core cancels the read behind it).
    #[qslot]
    fn open(&mut self, oid_hex: String) {
        let from = oid_hex.trim().to_string();
        if from.is_empty() || self.already_asking(&from) {
            return;
        }
        self.loading = true;
        self.asked_from = from.clone();
        self.changed();
        crate::hub::with_session(self.tab_id, |s| s.ask_rebase_plan(from.clone()));
    }

    /// Puts the plan away. Nothing ran, and nothing asks — the plan is a
    /// draft (デザイン規約 §可否・警告の出し場所).
    #[qslot]
    fn cancel_plan(&mut self) {
        self.close();
    }

    /// Asks again how many of the plan's rows a remote already has, for
    /// refs that moved under the open plan; the answer lands on
    /// `pushedCount`, matched by range.
    ///
    /// Call it on `NavSectionModel.refsMoved`, not the tick — the read is
    /// a `rev-list`, and on the tick it would poll at the status rate.
    #[qslot]
    fn refresh_pushed(&mut self) {
        if !self.active || self.publish_range.is_empty() {
            return;
        }
        let range = self.publish_range.clone();
        crate::hub::with_session(self.tab_id, |s| s.check_plan_published(range.clone()));
    }

    /// Whether a fold may stand on `row` right now — what the verb menu
    /// freezes as it opens.
    #[qslot]
    fn can_fold(&mut self, row: i32) -> bool {
        usize::try_from(row).is_ok_and(|index| self.fold_lands(index))
    }

    /// Sets row `row`'s verb; a fold with nowhere to land is refused (for
    /// callers that skip `canFold`). Leaving `reword` drops the typed
    /// draft: kept invisibly, it would resurface on the next reword.
    #[qslot]
    fn set_action(&mut self, row: i32, action: String) {
        let Ok(index) = usize::try_from(row) else {
            return;
        };
        if !ACTIONS.contains(&action.as_str()) {
            return;
        }
        if folds(&action) && !self.fold_lands(index) {
            return;
        }
        let Some(step) = self.steps.get_mut(index) else {
            return;
        };
        if step.action == action {
            return;
        }
        step.action = action;
        if step.action != "reword" {
            step.msg_subject = String::new();
            step.msg_body = String::new();
        }
        step.shown = Self::shown_of(step);
        // A drop can strand a fold above it.
        let mut touched = self.demote_orphan_folds();
        touched.push(index);
        touched.sort_unstable();
        self.notify_runs(touched.into_iter().map(|i| (i, i)));
        self.settle();
        self.changed();
    }

    /// Stores row `row`'s reword as the right pane's two boxes type it;
    /// the row's shown subject follows ([`RebasePlanModel::shown_of`]).
    #[qslot]
    fn set_message(&mut self, row: i32, subject: String, body: String) {
        let Ok(index) = usize::try_from(row) else {
            return;
        };
        let Some(step) = self.steps.get_mut(index) else {
            return;
        };
        if step.msg_subject == subject && step.msg_body == body {
            return;
        }
        step.msg_subject = subject;
        step.msg_body = body;
        step.shown = Self::shown_of(step);
        self.notify_runs([(index, index)]);
        self.settle();
        self.changed();
    }

    /// The row a commit sits on right now, or -1 where the plan does not
    /// hold it — how `RepoPage.jumpToRef` tells a parent hash inside the
    /// plan from one past the base.
    ///
    /// A linear walk: no index survives a reorder ([`Self::move_step`]).
    #[qslot]
    fn row_of(&mut self, oid_hex: String) -> i32 {
        self.steps
            .iter()
            .position(|step| step.oid_hex == oid_hex)
            .and_then(|row| i32::try_from(row).ok())
            .unwrap_or(-1)
    }

    /// Names the row the page's selection sits on (-1 = none), so the
    /// selection's verb is readable without a role query.
    #[qslot]
    fn select_row(&mut self, row: i32) {
        if self.selected_row == row {
            return;
        }
        self.selected_row = row;
        self.settle();
        self.changed();
    }

    /// Moves row `from` to sit at `to` (display indices), the selection
    /// following its row. A fold the move stranded goes back to `pick`
    /// now, or at the release when between [`Self::begin_move`] and
    /// [`Self::end_move`].
    #[qslot]
    fn move_step(&mut self, from: i32, to: i32) {
        if !self.reorder(from, to) {
            return;
        }
        self.settle();
        self.changed();
    }

    /// A reorder by hand has begun. From here to [`Self::end_move`] the
    /// rows move without the fold rule walking them.
    #[qslot]
    fn begin_move(&mut self) {
        self.dragging = true;
    }

    /// The hand let go (or the grab was taken from it): the fold rule is
    /// held once, over where the row landed.
    ///
    /// Not per move: the drag reports every row it crosses, so a `squash`
    /// carried past the oldest row and back would come out `pick` under the
    /// hand. The rule is about where a fold lands (デザイン規約 §フル
    /// interactive rebase「並べ替えで最古に落ちた fold は `pick` へ戻る」).
    #[qslot]
    fn end_move(&mut self) {
        if !self.dragging {
            return;
        }
        self.dragging = false;
        if !self.hold_fold_rule() {
            return;
        }
        self.settle();
        self.changed();
    }

    /// Runs the plan as one queued write pinned to the tip it was composed
    /// against (`RepoSession::rebase_interactive`), and puts the draft
    /// away at once — the badge, the WIP landing and the exit card take
    /// over (デザイン規約 §進行中の操作から出る).
    #[qslot]
    fn run_plan(&mut self) {
        // A drag whose release never came (its delegate taken away) leaves
        // the fold rule unheld, and git stops broken on an orphan fold.
        self.end_move();
        if !self.active || !self.dirty {
            return;
        }
        let steps = self.todo_steps();
        let upstream = self.onto_oid.clone();
        let expect = self.expect_head.clone();
        let options = platitude_core::integrate::RebaseOptions {
            onto: None,
            branch: None,
            // Branches inside the range move along with it, always
            // (デザイン規約 §履歴を合流させる — 既定のフラグ).
            update_refs: true,
            root: self.root,
        };
        crate::hub::with_session(self.tab_id, move |s| {
            s.rebase_interactive(upstream, steps, options, expect);
        });
        self.close();
        self.plan_ran();
    }

    /// The branch tip as the status now reads it, fed in by the page on
    /// every report. A tip other than `expect_head` puts the plan away
    /// (core checks again at run time).
    ///
    /// Two strikes: a status read that began before a tip move can land
    /// after the open carrying the older oid. Reads are single-flight, so
    /// only one such stale report can land — a second disagreement is fresh.
    #[qslot]
    fn note_head(&mut self, oid_hex: String) {
        if !self.active || oid_hex.is_empty() {
            return;
        }
        if oid_hex == self.expect_head {
            self.head_suspect = false;
            return;
        }
        if !self.head_suspect {
            self.head_suspect = true;
            return;
        }
        self.close();
        self.stale_plan();
    }

    /// The operation the status now reads as standing (`worktree.opText`,
    /// empty = none); one under an open plan puts it away. `noteHead`
    /// alone misses it — a merge stopped on a conflict leaves HEAD where it
    /// was — and core refusing the run does not take the button away.
    ///
    /// One strike: a report naming an operation was taken while one stood,
    /// so there is no stale reading to wait out.
    #[qslot]
    fn note_op(&mut self, op_text: String) {
        if !self.active || op_text.is_empty() {
            return;
        }
        self.close();
        self.standing_op();
    }
}
