//! What the plan does with what its feed hands it, and the two moves
//! that reset the rows — a plain `impl` beside the one-file `#[qobject]`
//! (structure.md §分割).

use super::*;

/// The name the page reads a refused preview under — all it has to write
/// both lines from, since git was never asked. The spellings are the row
/// menu's (`repo_tab::drain`), so one `Words.rewriteRefusedWhy` answers
/// both doors.
pub(in crate::models) fn refusal_kind(
    refusal: platitude_core::rebase_plan::PlanRefusal,
) -> &'static str {
    use platitude_core::rebase_plan::PlanRefusal as Refusal;
    match refusal {
        Refusal::AcrossMerge => "across-merge",
        Refusal::OffBranch => "off-branch",
        Refusal::UnfetchedBase => "unfetched-base",
    }
}

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
                // The range the rebase replays, spelled once by core, and
                // its published count from the same read — so the rewrite
                // warning counts exactly what the run touches.
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
                // A click cannot bring an empty range (its row is in it),
                // but an answer is an answer. One answer for both lines
                // below: no open plan without a selection, no selection
                // without a plan.
                let has_rows = count > 0;
                // Selected here, not in the page's `planActive` handler:
                // every property notifies on the one `changed()`, and
                // selecting there re-fires `active`'s notify inside its own
                // binding — a loop Qt refuses to re-evaluate.
                self.selected_row = if has_rows { 0 } else { -1 };
                self.head_suspect = false;
                self.active = has_rows;
                self.settle();
                self.changed();
            }
            PlanMsg::Refused { from, refusal } => {
                let Some(kind) = self.refused(&from, refusal) else {
                    return;
                };
                self.changed();
                self.refused_plan(kind.to_string());
            }
            // The error is already on the shared surface; this only puts
            // `loading` down.
            PlanMsg::Failed { from } => {
                if !self.loading || from != self.asked_from {
                    return;
                }
                self.loading = false;
                self.asked_from = String::new();
                self.changed();
            }
            // The count re-asked after the refs moved (`refreshPushed`),
            // matched by range so an answer for a replaced plan drops.
            PlanMsg::Published { range, published } => {
                if !self.active || range != self.publish_range || published == self.pushed_count {
                    return;
                }
                self.pushed_count = published;
                self.changed();
            }
        }
    }

    /// A refused preview: puts `loading` down and names the refusal for
    /// the page, or `None` for a click the screen has left behind
    /// (`asked_from`). All the model's side is here, testable without a
    /// QObject; the arm above only emits the signals.
    pub(super) fn refused(
        &mut self,
        from: &str,
        refusal: platitude_core::rebase_plan::PlanRefusal,
    ) -> Option<&'static str> {
        if !self.loading || from != self.asked_from {
            return None;
        }
        self.loading = false;
        Some(refusal_kind(refusal))
    }

    /// Replaces every row under one model reset (`QListModelBase::reset`
    /// installs what `reset_unnotified` finds staged). The dragged
    /// delegate goes too and no release will come, so the drag ends here
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
