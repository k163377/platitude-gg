use std::collections::HashMap;
use std::sync::Arc;

use platitude_core::sequencer::TodoAction;
use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, qobject};

use crate::hub::{Feed, PlanMsg};

use super::qml_register;

mod drain;
#[cfg(test)]
mod drain_tests;
mod qobject;
#[cfg(test)]
mod steps_tests;

/// For `repo_tab::drain_report_tests`, which holds both doors to one
/// spelling per refused history — test-only, since nothing else imports it.
#[cfg(test)]
pub(in crate::models) use drain::refusal_kind;

// RebasePlanModel: the interactive-rebase screen's plan — the rows of
// `from^..HEAD` newest-first, and the run that turns them back into git's
// oldest-first todo.

/// One row of the plan, in the screen's order (newest first — the reverse
/// of the todo file).
#[derive(QModelItem, Default, Clone)]
pub struct PlanStepItem {
    pub(super) oid_hex: String,
    pub(super) author: String,
    pub(super) author_email: String,
    pub(super) avatar: i32,
    pub(super) avatar_url: String,
    /// The commit's own subject — the todo line's comment, and what the
    /// row falls back to showing.
    pub(super) subject: String,
    /// What the row draws (`RebasePlanModel::shown_of`).
    pub(super) shown: String,
    /// One of `ACTIONS`, as the verb chip spells it.
    pub(super) action: String,
    /// The reword's replacement message, as the two boxes hold it. An
    /// empty pair runs as a plain `pick` (`RebasePlanModel::todo_action_of`).
    pub(super) msg_subject: String,
    pub(super) msg_body: String,
}

/// The verbs a row can carry — the todo's own vocabulary.
pub(super) const ACTIONS: [&str; 6] = ["pick", "reword", "edit", "squash", "fixup", "drop"];

/// The verbs that fold into the row below (the parent).
pub(super) fn folds(action: &str) -> bool {
    action == "squash" || action == "fixup"
}

#[derive(Default)]
pub struct RebasePlanModel {
    pub(super) steps: Vec<PlanStepItem>,
    /// Rows staged for the next model reset, installed by
    /// [`QListModel::reset_unnotified`].
    pub(super) pending_rows: Option<Vec<PlanStepItem>>,
    /// The oids in the order the plan opened with — what `dirty` compares
    /// the current order against.
    pub(super) initial: Vec<String>,
    /// A hand is carrying a row; the fold rule waits for the release
    /// (`end_move`).
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
    /// A local branch standing on the base, named in place of the id
    /// (デザイン規約 §フル interactive rebase「onto の名はブランチ優先」).
    /// Empty when none.
    pub(super) onto_ref: String,
    /// The branch tip the plan was composed against (= the newest row).
    /// The run pins it, and a tip that moves under the open plan closes
    /// it (`note_head`).
    pub(super) expect_head: String,
    /// The range the plan replays, as core spelled it — what
    /// `refreshPushed` asks the pushed count of again. Empty while no
    /// plan stands.
    pub(super) publish_range: String,
    /// How many of the plan's rows a remote already has (the run bar's
    /// warning). Counted as the plan opens (`PlanPreview::published`) and
    /// again when the refs move (`PlanMsg::Published`).
    pub(super) pushed_count: i32,
    /// [`Self::is_dirty`] as a property, settled by every mutation — the
    /// run button binds to it, and bindings follow properties only
    /// (app-ui.md).
    pub(super) dirty: bool,
    /// Rows the plan leaves out, settled the same way: with the tip held
    /// by nothing else, what makes the run a hold (§履歴を合流させる).
    pub(super) drops: i32,
    /// How many rows the plan holds, for the run button's own phrase.
    pub(super) step_count: i32,
    /// The row the page's selection sits on (-1 = none), and its verb —
    /// what the right pane's boxes read to know a reword is on screen.
    /// Held here so a reorder remaps it with the rows.
    pub(super) selected_row: i32,
    pub(super) selected_action: String,
    /// That row's commit, so the page can refuse to route typing while
    /// the details pane shows another commit.
    pub(super) selected_oid: String,
    /// The selected row's stored reword, so the boxes can reopen on the
    /// draft when the row is revisited — saving over the original would
    /// silently revert it.
    pub(super) selected_msg_subject: String,
    pub(super) selected_msg_body: String,
    /// One head report disagreed with `expect_head` — the first of the two
    /// strikes `note_head` waits for.
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
    /// original order, or a row the todo carries as something other than
    /// `pick` ([`Self::todo_action_of`] — counting the chip's verb would
    /// open the run button on an all-pick todo). An untouched plan would
    /// run a rebase that visibly does nothing.
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

    /// Whether the answer to this very commit is already on its way
    /// (`RebasePlanModel::open`).
    pub(super) fn already_asking(&self, from: &str) -> bool {
        self.loading && self.asked_from == from
    }

    pub(super) fn drop_count(&self) -> i32 {
        let drops = self.steps.iter().filter(|s| s.action == "drop").count();
        i32::try_from(drops).unwrap_or(i32::MAX)
    }

    /// The subject the run would leave on the commit, read through
    /// [`Self::todo_action_of`]: a reword typed into the description box
    /// alone still rewrites, and git's subject is that message's first
    /// line — reading the summary box alone would show the old subject.
    pub(super) fn shown_of(step: &PlanStepItem) -> String {
        let (action, message) = Self::todo_action_of(step);
        if action == TodoAction::Reword {
            platitude_core::commit::split_message(&message).0
        } else {
            step.subject.clone()
        }
    }

    /// The todo verb one row stands for, with the message a `reword` would
    /// carry — the one place the chips' vocabulary turns into git's, so the
    /// run and [`Self::is_dirty`] cannot drift apart. A `reword` with
    /// nothing typed is a plain `pick`.
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

    /// Whether a fold may stand on `row`: some row below it has to stay in
    /// the history (not a drop). [`Self::demote_orphan_folds`] holds the
    /// same line against reorders and later drops.
    pub(super) fn fold_lands(&self, row: usize) -> bool {
        self.steps
            .get(row + 1..)
            .is_some_and(|below| below.iter().any(|step| step.action != "drop"))
    }

    /// Sends every fold with nothing but drops (or nothing) below it back
    /// to `pick`, and answers their rows — git stops broken on such a todo
    /// (`cannot 'squash' without a previous commit`). Walked bottom-up: a
    /// fold that stays is itself a landing for the fold above.
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

    /// [`Self::demote_orphan_folds`], with the demoted rows redrawn.
    /// Answers whether it moved anything.
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
    /// The fold rule is held here only when no hand is carrying the row
    /// (`end_move` says why).
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
