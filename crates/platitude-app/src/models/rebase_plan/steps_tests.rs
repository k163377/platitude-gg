//! The plan's own arithmetic: what counts as dirty, how display rows turn
//! back into git's todo, and the fold the oldest row cannot carry.

use super::{PlanStepItem, RebasePlanModel};
use platitude_core::sequencer::TodoAction;

fn step(oid: &str, subject: &str) -> PlanStepItem {
    PlanStepItem {
        oid_hex: oid.to_string(),
        subject: subject.to_string(),
        shown: subject.to_string(),
        action: "pick".to_string(),
        ..PlanStepItem::default()
    }
}

/// Three rows in display order (newest first), untouched.
fn fresh() -> RebasePlanModel {
    let mut model = RebasePlanModel::default();
    model.steps = vec![
        step("c3", "third"),
        step("c2", "second"),
        step("c1", "first"),
    ];
    model.initial = model.steps.iter().map(|s| s.oid_hex.clone()).collect();
    model.settle();
    model
}

#[test]
fn an_untouched_plan_is_not_dirty_and_every_touch_makes_it_so() {
    let mut model = fresh();
    assert!(
        !model.is_dirty(),
        "all-pick in the opened order asks nothing"
    );

    model.steps[1].action = "drop".to_string();
    assert!(model.is_dirty());
    model.steps[1].action = "pick".to_string();

    model.steps.swap(0, 1);
    assert!(model.is_dirty(), "a reorder alone is a request");
    model.steps.swap(0, 1);

    model.steps[2].msg_subject = "typed".to_string();
    assert!(model.is_dirty(), "a typed reword is a request");
}

#[test]
fn the_todo_runs_oldest_first_and_a_reword_left_empty_is_a_pick() {
    let mut model = fresh();
    model.steps[0].action = "reword".to_string();
    model.steps[0].msg_subject = "new words".to_string();
    model.steps[1].action = "reword".to_string(); // nothing typed
    model.steps[2].action = "drop".to_string();

    let todo = model.todo_steps();
    assert_eq!(
        todo.iter().map(|s| s.oid.as_str()).collect::<Vec<_>>(),
        vec!["c1", "c2", "c3"],
        "display order turns round into git's"
    );
    assert_eq!(todo[0].action, TodoAction::Drop);
    assert_eq!(
        todo[1].action,
        TodoAction::Pick,
        "an empty reword keeps the message"
    );
    assert_eq!(todo[1].message, None);
    assert_eq!(todo[2].action, TodoAction::Reword);
    assert_eq!(todo[2].message.as_deref(), Some("new words"));
}

#[test]
fn a_fold_stranded_on_the_oldest_row_goes_back_to_pick() {
    let mut model = fresh();
    model.steps[2].action = "squash".to_string();
    let demoted = model.demote_orphan_folds();
    assert_eq!(demoted, vec![2]);
    assert_eq!(model.steps[2].action, "pick");

    // Anywhere else a fold stands as it is.
    model.steps[1].action = "fixup".to_string();
    assert!(model.demote_orphan_folds().is_empty());
    assert_eq!(model.steps[1].action, "fixup");
}

#[test]
fn a_fold_with_nothing_but_drops_below_goes_back_to_pick() {
    let mut model = fresh();
    model.steps[0].action = "squash".to_string();
    model.steps[1].action = "drop".to_string();
    model.steps[2].action = "drop".to_string();
    let demoted = model.demote_orphan_folds();
    assert_eq!(
        demoted,
        vec![0],
        "everything under it is leaving the history"
    );
    assert_eq!(model.steps[0].action, "pick");
}

#[test]
fn a_demoted_fold_is_itself_a_landing_for_the_fold_above() {
    let mut model = fresh();
    model.steps[0].action = "squash".to_string();
    model.steps[1].action = "squash".to_string();
    model.steps[2].action = "drop".to_string();
    let demoted = model.demote_orphan_folds();
    assert_eq!(
        demoted,
        vec![1],
        "the middle one lands in nothing; the top one lands in it"
    );
    assert_eq!(model.steps[0].action, "squash");
    assert_eq!(model.steps[1].action, "pick");
}

#[test]
fn fold_lands_answers_for_the_menu_what_the_demotion_enforces() {
    let mut model = fresh();
    assert!(model.fold_lands(0));
    assert!(model.fold_lands(1));
    assert!(!model.fold_lands(2), "the oldest row has nothing below");
    model.steps[2].action = "drop".to_string();
    assert!(model.fold_lands(0), "c2 still stays");
    assert!(!model.fold_lands(1), "only a drop is left below");
}

#[test]
fn settle_carries_the_selected_rows_stored_reword_out() {
    let mut model = fresh();
    model.steps[1].msg_subject = "typed".to_string();
    model.steps[1].msg_body = "and more".to_string();
    model.selected_row = 1;
    model.settle();
    assert_eq!(model.selected_msg_subject, "typed");
    assert_eq!(model.selected_msg_body, "and more");
    model.selected_row = 0;
    model.settle();
    assert_eq!(
        model.selected_msg_subject, "",
        "the newest row holds no draft"
    );
}

#[test]
fn the_shown_subject_follows_the_typed_reword_and_falls_back() {
    assert_eq!(RebasePlanModel::shown_of("reword", "typed", "own"), "typed");
    assert_eq!(RebasePlanModel::shown_of("reword", "  ", "own"), "own");
    assert_eq!(RebasePlanModel::shown_of("pick", "typed", "own"), "own");
}
