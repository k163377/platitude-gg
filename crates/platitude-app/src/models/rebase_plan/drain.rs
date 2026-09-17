//! What the plan does with what its feed hands it, and the two moves
//! that reset the rows.
//!
//! A plain `impl`: the Qt-facing face has to be one file
//! (app-ui.md), and what it does does not
//! (structure.md §分割 Qt).

use super::*;

impl RebasePlanModel {
    pub(super) fn take(&mut self, msg: PlanMsg) {
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
                // — the rewrite warning's count is about this string, so
                // it cannot drift from what the run touches — and the
                // count itself, taken in the same read.
                self.publish_range = preview.range;
                self.pushed_count = i32::try_from(preview.published).unwrap_or(i32::MAX);
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
                    platitude_core::rebase_plan::PlanRefusal::AcrossMerge => "across-merge",
                    platitude_core::rebase_plan::PlanRefusal::OffBranch => "off-branch",
                    platitude_core::rebase_plan::PlanRefusal::UnfetchedBase => "unfetched-base",
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
            // The count asked again after the refs moved (`refreshPushed`).
            // Matched by range: a plan put away and another opened since
            // is a different range, and the old answer is dropped on the
            // name.
            PlanMsg::Published { range, published } => {
                if !self.active || range != self.publish_range || published == self.pushed_count {
                    return;
                }
                self.pushed_count = published;
                self.changed();
            }
        }
    }

    /// Replaces every row under one model reset: the plan opens whole
    /// (`QListModelBase::reset` installs what
    /// `reset_unnotified` finds staged).
    ///
    /// Every delegate goes with the reset, one under a hand included, so
    /// no release is coming for it — the drag is put down here
    /// (`end_move`).
    fn reset_rows(&mut self, rows: Vec<PlanStepItem>) {
        self.dragging = false;
        self.pending_rows = Some(rows);
        self.reset();
    }

    pub(super) fn close(&mut self) {
        if !self.active && !self.loading && self.steps.is_empty() {
            return;
        }
        self.active = false;
        self.loading = false;
        self.asked_from = String::new();
        self.selected_row = -1;
        self.head_suspect = false;
        self.publish_range = String::new();
        self.pushed_count = 0;
        self.reset_rows(Vec::new());
        self.initial = Vec::new();
        self.settle();
        self.changed();
    }
}
