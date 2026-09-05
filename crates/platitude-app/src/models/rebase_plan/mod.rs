use std::collections::HashMap;
use std::sync::Arc;

use platitude_core::sequencer::TodoAction;
use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, qobject};

use crate::hub::{Feed, PlanMsg};

use super::qml_register;

mod drain;
mod qobject;
#[cfg(test)]
mod steps_tests;

// ---------------------------------------------------------------------------
// RebasePlanModel: the interactive-rebase screen's plan — the rows of
// `from^..HEAD` newest-first, the verb and reword text each row carries,
// and the run that turns them back into git's oldest-first todo.
// ---------------------------------------------------------------------------

/// One row of the plan, in the screen's order (newest first — the same way
/// the graph reads, and the reverse of the todo file).
#[derive(QModelItem, Default, Clone)]
pub struct PlanStepItem {
    pub(super) oid_hex: String,
    pub(super) author: String,
    pub(super) author_email: String,
    pub(super) avatar: i32,
    pub(super) avatar_url: String,
    /// The commit's own subject — what the todo line carries as comment,
    /// and what the row falls back to showing.
    pub(super) subject: String,
    /// What the row draws: the reword's new subject once one is typed,
    /// the commit's own otherwise. Chosen here rather than in QML — which
    /// text a row shows is a rule, not a look (app-ui.md).
    pub(super) shown: String,
    /// `pick` / `reword` / `edit` / `squash` / `fixup` / `drop`, as the
    /// verb chip spells it (デザイン規約 §git 用語のコード表記).
    pub(super) action: String,
    /// The reword's replacement message, split the way the two boxes hold
    /// it. Empty until typed; an empty pair at run time means the commit
    /// keeps its message, so the step goes back to a plain `pick`.
    pub(super) msg_subject: String,
    pub(super) msg_body: String,
}

/// The six verbs a row can carry — the todo's own vocabulary.
pub(super) const ACTIONS: [&str; 6] = ["pick", "reword", "edit", "squash", "fixup", "drop"];

/// The two verbs that fold into the row below (the parent). The oldest
/// row has no row below it inside the plan, so neither stands there.
pub(super) fn folds(action: &str) -> bool {
    action == "squash" || action == "fixup"
}

#[derive(Default)]
pub struct RebasePlanModel {
    pub(super) steps: Vec<PlanStepItem>,
    /// Rows staged for the next model reset — `QListModelBase::reset`
    /// calls [`QListModel::reset_unnotified`], which installs these.
    pub(super) pending_rows: Option<Vec<PlanStepItem>>,
    /// The oids in the order the plan opened with — what `dirty` compares
    /// the current order against.
    pub(super) initial: Vec<String>,
    /// A hand is carrying a row. The drag reports every row it crosses,
    /// so the fold rule waits for the release (`end_move`) rather than
    /// walking the rows once per crossing.
    pub(super) dragging: bool,
    pub(super) active: bool,
    pub(super) loading: bool,
    /// The commit the open was asked from; answers for any other click
    /// are stale and dropped.
    pub(super) asked_from: String,
    pub(super) from_oid: String,
    /// Parent of `from_oid`, which the plan replays onto; empty on `root`.
    pub(super) onto_oid: String,
    pub(super) root: bool,
    pub(super) onto_subject: String,
    pub(super) onto_author: String,
    pub(super) onto_email: String,
    pub(super) onto_avatar: i32,
    pub(super) onto_avatar_url: String,
    /// A local branch standing on the base, said first with the id as the
    /// fallback (デザイン規約: onto はブランチ名優先). Empty when none.
    pub(super) onto_ref: String,
    /// The branch tip the plan was composed against (= the newest row).
    /// The run pins it, and a tip that moves under the open plan closes
    /// it (`note_head`).
    pub(super) expect_head: String,
    /// The very range the plan replays, as core spelled it — what the
    /// rewrite warning's count is about, and what `refreshPushed` asks
    /// about again. Empty while no plan stands.
    pub(super) publish_range: String,
    /// How many of the plan's rows a remote already has — the `rewrites
    /// pushed commits` warning's number (`RebasePlanRunBar`). Counted
    /// by core as the plan opens (`PlanPreview::published`) and counted
    /// again when the refs move under the open plan (`PlanMsg::Published`),
    /// so the plan carries its own answer rather than reading one off
    /// the tab that another question could overwrite.
    pub(super) pushed_count: i32,
    /// [`Self::is_dirty`] as a property, settled by every mutation — the
    /// run button's `enabled:` has to follow it, and a binding on a slot
    /// freezes at its first answer (app-ui.md).
    pub(super) dirty: bool,
    /// Rows the plan leaves out, settled the same way: with the tip held
    /// by nothing else, what makes the run a hold (§履歴を合流させる).
    pub(super) drops: i32,
    /// How many rows the plan holds, for the run button's own phrase.
    pub(super) step_count: i32,
    /// The row the page's selection sits on (-1 = none), and its verb —
    /// what the right pane's boxes read to know a reword is on screen.
    /// Held here rather than in QML so a reorder cannot leave the two
    /// disagreeing (the move remaps it).
    pub(super) selected_row: i32,
    pub(super) selected_action: String,
    /// That row's commit, so the page can refuse to route typing whose
    /// details pane shows some other commit (a selection moved by
    /// anything that is not a plan row click).
    pub(super) selected_oid: String,
    /// The selected row's stored reword, so the boxes can reopen on the
    /// draft rather than on the commit's own message when the row is
    /// revisited — saving over the original would silently revert it.
    pub(super) selected_msg_subject: String,
    pub(super) selected_msg_body: String,
    /// One head report that disagreed with `expect_head` is a suspect,
    /// not a verdict: a status read that began before a tip move and
    /// landed after the plan opened carries the *older* oid, and closing
    /// on it would discard a plan composed on exactly the tip that
    /// exists. Reads are single-flight, so at most one stale report can
    /// land after the open — the second strike is always fresh.
    pub(super) head_suspect: bool,
    pub(super) feed: Option<Arc<Feed<PlanMsg>>>,
    pub(super) tab_id: i32,
}

impl QListModel for RebasePlanModel {
    type Item = PlanStepItem;

    fn len(&self) -> usize {
        self.steps.len()
    }
    fn get(&self, index: usize) -> Option<&PlanStepItem> {
        self.steps.get(index)
    }
    fn reset_unnotified(&mut self) {
        self.steps = self.pending_rows.take().unwrap_or_default();
    }
}

impl RebasePlanModel {
    /// Whether the plan asks for anything at all: rows out of their
    /// original order, or a row the run would write as something other
    /// than `pick`. The run button only opens on this — an untouched
    /// plan replays every commit onto the parent it already sits on, and
    /// a button that runs it would be one that visibly does nothing
    /// (デザイン規約 §可否・警告の出し場所).
    ///
    /// The verb is read through [`Self::todo_action_of`] rather than off
    /// `action`, so what opens the button is what the todo actually
    /// carries: a row turned to `reword` with nothing typed goes to git
    /// as a plain `pick`, and counting it here would open a run whose
    /// todo is all picks.
    pub(super) fn is_dirty(&self) -> bool {
        self.steps.len() != self.initial.len()
            || self
                .steps
                .iter()
                .zip(&self.initial)
                .any(|(step, oid)| step.oid_hex != *oid)
            || self
                .steps
                .iter()
                .any(|step| Self::todo_action_of(step).0 != TodoAction::Pick)
    }

    /// Whether the answer to this very commit is already on its way, so
    /// asking again would only spend the walk twice
    /// (`RebasePlanModel::open`). Any other commit is a different
    /// question, however long the one before it is taking.
    pub(super) fn already_asking(&self, from: &str) -> bool {
        self.loading && self.asked_from == from
    }

    pub(super) fn drop_count(&self) -> i32 {
        let drops = self.steps.iter().filter(|s| s.action == "drop").count();
        i32::try_from(drops).unwrap_or(i32::MAX)
    }

    /// What the row should show for its subject: the subject the run
    /// would actually leave on the commit.
    ///
    /// Read through [`Self::todo_action_of`] for the same reason
    /// [`Self::is_dirty`] is — the row and the todo must not be able to
    /// say different things. Typed into the description box alone, a
    /// `reword` still rewrites the commit, and git's subject is that
    /// message's first line; taking the summary box on its own would
    /// leave the row showing the old subject while the run replaced it.
    pub(super) fn shown_of(step: &PlanStepItem) -> String {
        let (action, message) = Self::todo_action_of(step);
        if action == TodoAction::Reword {
            platitude_core::commit::split_message(&message).0
        } else {
            step.subject.clone()
        }
    }

    /// The todo verb one row stands for, with the message a `reword`
    /// would carry. The single place the chips' vocabulary turns into
    /// git's, so the todo the run writes and the button
    /// [`Self::is_dirty`] opens cannot drift apart: reword only where a
    /// message was actually typed (an empty box means the commit keeps
    /// its message, which is a plain `pick`).
    pub(super) fn todo_action_of(step: &PlanStepItem) -> (TodoAction, String) {
        let message = platitude_core::commit::join_message(&step.msg_subject, &step.msg_body);
        let action = match step.action.as_str() {
            "reword" if !message.is_empty() => TodoAction::Reword,
            "reword" => TodoAction::Pick,
            "edit" => TodoAction::Edit,
            "squash" => TodoAction::Squash,
            "fixup" => TodoAction::Fixup,
            "drop" => TodoAction::Drop,
            _ => TodoAction::Pick,
        };
        (action, message)
    }

    /// The steps as git's todo wants them: oldest first, each row's verb
    /// as [`Self::todo_action_of`] settles it.
    pub(super) fn todo_steps(&self) -> Vec<platitude_core::sequencer::RebaseStep> {
        use platitude_core::sequencer::RebaseStep;
        self.steps
            .iter()
            .rev()
            .map(|step| {
                let (action, message) = Self::todo_action_of(step);
                RebaseStep {
                    message: (action == TodoAction::Reword).then_some(message),
                    action,
                    oid: step.oid_hex.clone(),
                    subject: step.subject.clone(),
                }
            })
            .collect()
    }

    /// Re-derives the summaries every mutation moves
    /// ([`Self::is_dirty`] / [`Self::drop_count`] / the selection's verb).
    pub(super) fn settle(&mut self) {
        self.dirty = self.is_dirty();
        self.drops = self.drop_count();
        self.step_count = i32::try_from(self.steps.len()).unwrap_or(i32::MAX);
        let selected = usize::try_from(self.selected_row)
            .ok()
            .and_then(|row| self.steps.get(row));
        self.selected_action = selected.map(|step| step.action.clone()).unwrap_or_default();
        self.selected_oid = selected
            .map(|step| step.oid_hex.clone())
            .unwrap_or_default();
        self.selected_msg_subject = selected
            .map(|step| step.msg_subject.clone())
            .unwrap_or_default();
        self.selected_msg_body = selected
            .map(|step| step.msg_body.clone())
            .unwrap_or_default();
    }

    /// Whether a fold may stand on `row`: something below it has to
    /// remain in the history for it to fold into — a drop is not it, and
    /// the oldest row has nothing below at all. What the verb menu
    /// freezes as it opens; [`Self::demote_orphan_folds`] holds the same
    /// line against reorders and later drops.
    pub(super) fn fold_lands(&self, row: usize) -> bool {
        self.steps
            .get(row + 1..)
            .is_some_and(|below| below.iter().any(|step| step.action != "drop"))
    }

    /// Rows the fold rule holds down after any mutation: a fold folds
    /// into the nearest row below that stays in the history, so one with
    /// nothing but drops under it — moved there, or stranded by a later
    /// drop — goes back to `pick` on the spot, visibly on the row itself,
    /// rather than as a todo git rejects into a broken stop
    /// (`cannot 'squash' without a previous commit`, measured).
    /// Walked bottom-up: a fold that stays is itself something a fold
    /// above can land in.
    pub(super) fn demote_orphan_folds(&mut self) -> Vec<usize> {
        let mut demoted = Vec::new();
        let mut lands = false;
        for row in (0..self.steps.len()).rev() {
            let step = &mut self.steps[row];
            if folds(&step.action) && !lands {
                step.action = "pick".to_string();
                step.shown = Self::shown_of(step);
                demoted.push(row);
            }
            lands |= self.steps[row].action != "drop";
        }
        demoted
    }

    /// Walks [`Self::demote_orphan_folds`] over the rows and redraws the
    /// ones it sent back to `pick`. Answers whether it moved anything, so
    /// a caller with no other reason to notify can stand down.
    pub(super) fn hold_fold_rule(&mut self) -> bool {
        let demoted = self.demote_orphan_folds();
        let moved = !demoted.is_empty();
        self.notify_runs(demoted.into_iter().map(|i| (i, i)));
        moved
    }

    /// Moves row `from` to sit at `to` (display indices), carrying the
    /// selection with the row it was on. One Qt move, so the delegate
    /// under the hand stays alive. Answers whether anything moved.
    ///
    /// The fold rule is held here only when nothing is carrying the row.
    /// A drag reports every row it crosses, and walking the rule per
    /// crossing demotes a fold the hand is merely *passing* through the
    /// oldest place — see `end_move`, which holds it once at the release.
    pub(super) fn reorder(&mut self, from: i32, to: i32) -> bool {
        let (Ok(from_i), Ok(to_i)) = (usize::try_from(from), usize::try_from(to)) else {
            return false;
        };
        if from_i == to_i || from_i >= self.steps.len() || to_i >= self.steps.len() {
            return false;
        }
        self.move_notified(from_i, to_i);
        if self.selected_row == from {
            self.selected_row = to;
        } else if from < self.selected_row && to >= self.selected_row {
            self.selected_row -= 1;
        } else if from > self.selected_row && to <= self.selected_row {
            self.selected_row += 1;
        }
        if !self.dragging {
            self.hold_fold_rule();
        }
        true
    }
}

super::impl_move_notified!(RebasePlanModel, steps);
super::impl_notify_runs!(RebasePlanModel);

qml_register!(RebasePlanModel, "RebasePlanModel", singleton = false);
