//! Everything QML sees of the plan: the header fields, the feed it
//! drains, the verbs and reorders the rows take, and the run.
//!
//! One `#[qobject]` block, and it cannot be split further — QMetaInfo is
//! built per file (app-ui.md).

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
    qproperty!("publishRange", Member = publish_range, Notify = changed);
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
    fn changed(&mut self);

    /// The plan could not be opened, before anything was touched: `kind`
    /// is `merge` (the range holds one) or `off-branch` (the click was on
    /// another branch's row). The page writes the sentence
    /// (app-ui.md「Rust に文言を置かない」).
    #[qsignal]
    fn refused_plan(&mut self, kind: String);

    /// The branch tip moved while the plan stood open, so it was put
    /// away; nothing was run (the page says so — §答えの要らない報せ).
    #[qsignal]
    fn stale_plan(&mut self);

    /// An operation started under the open plan — anything the badge
    /// names, from a terminal or another session — so it was put away;
    /// nothing was run. Its own signal rather than
    /// `stalePlan`: the tip has not moved and saying it did would send a
    /// reader looking for a rewrite nobody made (§答えの要らない報せ).
    #[qsignal]
    fn standing_op(&mut self);

    /// The plan was handed over and put away: from here the replay is
    /// out. Said at the press rather than left to the write's own answer
    /// — `replaying` rises when the queue *starts* the write, not when
    /// the button was let go, and the screen must not come back to life
    /// in between (`RepoPage.planRunSeq`).
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
    /// The same commit asked for again while its answer is still out is
    /// the same question, and is left to the answer already coming
    /// ([`Self::already_asking`]) — the read walks the whole range, and a
    /// second click on the row the menu just closed over is the one click
    /// a reader who has been given nothing to look at will make. Another
    /// commit replaces the ask outright: core cancels the read behind it
    /// and the screen waits on the newer one.
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
    /// draft, and the one explicit button is the way out
    /// (デザイン規約 §可否・警告の出し場所).
    #[qslot]
    fn cancel_plan(&mut self) {
        self.close();
    }

    /// Whether a fold may stand on `row` right now — what the verb menu
    /// freezes as it opens, so squash / fixup are not offered where they
    /// could not land (nothing but drops below, or nothing at all).
    #[qslot]
    fn can_fold(&mut self, row: i32) -> bool {
        usize::try_from(row).is_ok_and(|index| self.fold_lands(index))
    }

    /// Sets row `row`'s verb. A fold with nowhere to land is refused —
    /// the menu does not offer one there (`canFold`), and the guard holds
    /// for a caller that never went through the menu. Leaving `reword`
    /// takes the typed draft with it: a plan whose rows are all pick
    /// again must read clean, and a draft kept invisibly would resurface
    /// on the next reword.
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
        step.shown = Self::shown_of(&step.action, &step.msg_subject, &step.subject);
        // A drop can strand a fold above it (nothing left below to land
        // in), so the fold rule walks the rows after every verb change.
        let mut touched = self.demote_orphan_folds();
        touched.push(index);
        touched.sort_unstable();
        self.notify_runs(touched.into_iter().map(|i| (i, i)));
        self.settle();
        self.changed();
    }

    /// Types the reword for row `row` — the right pane's two boxes write
    /// here, and the row's shown subject mirrors the summary as it is
    /// typed (デザイン規約 §コミットメッセージの 2 つの枠).
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
        step.shown = Self::shown_of(&step.action, &step.msg_subject, &step.subject);
        self.notify_runs([(index, index)]);
        self.settle();
        self.changed();
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

    /// Moves row `from` to sit at `to` (display indices). One Qt move, so
    /// the delegate under the hand stays alive; a fold the reorder
    /// stranded on the oldest row goes back to `pick` on the spot, and
    /// the selection follows the row it was on.
    #[qslot]
    fn move_step(&mut self, from: i32, to: i32) {
        let (Ok(from_i), Ok(to_i)) = (usize::try_from(from), usize::try_from(to)) else {
            return;
        };
        if from_i == to_i || from_i >= self.steps.len() || to_i >= self.steps.len() {
            return;
        }
        self.move_notified(from_i, to_i);
        if self.selected_row == from {
            self.selected_row = to;
        } else if from < self.selected_row && to >= self.selected_row {
            self.selected_row -= 1;
        } else if from > self.selected_row && to <= self.selected_row {
            self.selected_row += 1;
        }
        let demoted = self.demote_orphan_folds();
        self.notify_runs(demoted.into_iter().map(|i| (i, i)));
        self.settle();
        self.changed();
    }

    /// Runs the plan: one queued write, pinned to the tip the plan was
    /// composed against (`RepoSession::rebase_interactive`), and the
    /// draft is put away the moment it is handed over — from here the
    /// badge, the WIP landing and the exit card carry the story
    /// (デザイン規約 §進行中の操作から出る).
    #[qslot]
    fn run_plan(&mut self) {
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

    /// The branch tip as the status now reads it. The page feeds every
    /// report in; a tip that is not the one the plan was composed against
    /// puts the plan away — composing over a history that moved deepens a
    /// draft that can no longer run (実行時の照合はその上にもう 1 枚、
    /// core が持つ).
    ///
    /// **Two strikes, not one.** A status read that began before a tip
    /// move and landed after the plan opened carries the *older* oid, and
    /// one strike would discard a plan composed on exactly the tip that
    /// exists. Status reads are single-flight, so at most one such stale
    /// report can land — a second disagreeing report is always fresh.
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

    /// The operation the status now reads as standing (`workTree.opText`,
    /// empty = none). One that stands under an open plan puts it away:
    /// the plan was composed over a repository with nothing running, and
    /// a merge stopped on a conflict leaves HEAD exactly where it was —
    /// so `noteHead` sees nothing wrong and would leave a doomed draft on
    /// screen with its run button live. The run itself is refused by core
    /// either way (`RepoSession::rebase_interactive`); this is what keeps
    /// the screen from offering it.
    ///
    /// **One strike, unlike `noteHead`.** That rule is for a status read
    /// that began before a tip move and carries the older oid, where a
    /// single strike would discard a plan composed on the tip that
    /// exists. There is no such reading here: a report naming an
    /// operation was taken while one stood, and one that stood after the
    /// plan opened is reason enough to put the draft away whether or not
    /// it is still standing now.
    #[qslot]
    fn note_op(&mut self, op_text: String) {
        if !self.active || op_text.is_empty() {
            return;
        }
        self.close();
        self.standing_op();
    }
}

impl RebasePlanModel {
    fn take(&mut self, msg: PlanMsg) {
        match msg {
            PlanMsg::Loaded { preview } => {
                if !self.loading || preview.from != self.asked_from {
                    return;
                }
                let avatars = crate::hub::AvatarUrls::current();
                let count = preview.rows.len();
                self.reset_rows(
                    preview
                        .rows
                        .into_iter()
                        .rev()
                        .map(|row| PlanStepItem {
                            avatar: crate::encode::avatar_code(&row.author_name),
                            avatar_url: avatars.url_of(&row.author_email),
                            shown: row.subject.clone(),
                            oid_hex: row.oid,
                            author: row.author_name,
                            author_email: row.author_email,
                            subject: row.subject,
                            action: "pick".to_string(),
                            msg_subject: String::new(),
                            msg_body: String::new(),
                        })
                        .collect(),
                );
                self.initial = self.steps.iter().map(|s| s.oid_hex.clone()).collect();
                self.from_oid = preview.from;
                self.onto_oid = preview.upstream.clone();
                self.root = preview.root;
                self.expect_head = self
                    .steps
                    .first()
                    .map(|s| s.oid_hex.clone())
                    .unwrap_or_default();
                // The very range the rebase replays, spelled by core once
                // — the rewrite warning asks about this string, so it
                // cannot drift from what the run touches.
                self.publish_range = preview.range;
                let onto = preview.onto.unwrap_or_default();
                self.onto_subject = onto.subject;
                self.onto_author = onto.author_name.clone();
                self.onto_avatar = crate::encode::avatar_code(&onto.author_name);
                self.onto_avatar_url = avatars.url_of(&onto.author_email);
                self.onto_email = onto.author_email;
                self.onto_ref = preview.onto_ref;
                self.loading = false;
                // An empty range cannot arise from a click on a commit
                // (the row itself is in it), but an answer is an answer.
                // The one question both lines below answer, so a plan
                // cannot stand with nothing selected or a selection be
                // left standing in a plan that never opened.
                let has_rows = count > 0;
                // An open plan already has its newest row selected, and
                // the model is where that is set: every property here
                // notifies on the one `changed()`, so selecting from the
                // page's `planActive` handler would fire `active`'s own
                // notify inside the binding that is still delivering it —
                // a binding loop Qt reports and refuses to re-evaluate.
                self.selected_row = if has_rows { 0 } else { -1 };
                self.head_suspect = false;
                self.active = has_rows;
                self.settle();
                self.changed();
            }
            PlanMsg::Refused { from, refusal } => {
                if !self.loading || from != self.asked_from {
                    return;
                }
                self.loading = false;
                self.changed();
                let kind = match refusal {
                    platitude_core::rebase_plan::PlanRefusal::AcrossMerge => "merge",
                    platitude_core::rebase_plan::PlanRefusal::OffBranch => "off-branch",
                };
                self.refused_plan(kind.to_string());
            }
            // The read failed; the error itself is on the shared surface
            // already, and this puts the waiting state down so nothing
            // keys on `loading` forever.
            PlanMsg::Failed { from } => {
                if !self.loading || from != self.asked_from {
                    return;
                }
                self.loading = false;
                self.asked_from = String::new();
                self.changed();
            }
        }
    }

    /// Replaces every row under one model reset: the plan opens whole,
    /// not row by row (`QListModelBase::reset` installs what
    /// `reset_unnotified` finds staged).
    fn reset_rows(&mut self, rows: Vec<PlanStepItem>) {
        self.pending_rows = Some(rows);
        self.reset();
    }

    fn close(&mut self) {
        if !self.active && !self.loading && self.steps.is_empty() {
            return;
        }
        self.active = false;
        self.loading = false;
        self.asked_from = String::new();
        self.selected_row = -1;
        self.head_suspect = false;
        self.reset_rows(Vec::new());
        self.initial = Vec::new();
        self.settle();
        self.changed();
    }
}
