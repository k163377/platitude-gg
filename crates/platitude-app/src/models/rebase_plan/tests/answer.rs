//! What a confirmed press takes from the plan and when it gives it
//! (`answer.rs`): the content fixed at the press, given once to its own
//! ticket, and gone for good with a plan put away.

use super::super::answer::Answer;
use super::super::{PlanStepItem, RebasePlanModel};

fn step(oid: &str, action: &str) -> PlanStepItem {
    PlanStepItem {
        oid_hex: oid.to_string(),
        subject: oid.to_string(),
        shown: oid.to_string(),
        action: action.to_string(),
        ..PlanStepItem::default()
    }
}

/// The tip the plan below was composed against.
fn tip() -> platitude_core::Oid {
    platitude_core::Oid::from_hex_str("a9771664a8219f845abfd8ce6f5d7af521247539")
        .expect("an object name")
}

/// An open plan, newest first, asking to leave `b` out.
fn standing() -> RebasePlanModel {
    let mut model = RebasePlanModel {
        active: true,
        onto_oid: "base".to_string(),
        expect_head: Some(tip()),
        ..RebasePlanModel::default()
    };
    model.steps = vec![step("c", "pick"), step("b", "drop"), step("a", "pick")];
    model.initial = model.steps.iter().map(|s| s.oid_hex.clone()).collect();
    model.settle();
    model
}

/// `b` dropped at the press, then the rows turned round on the floor —
/// what runs is what was pressed for.
#[test]
fn a_run_is_taken_as_the_plan_stood_at_the_press() {
    let mut model = standing();
    let pressed = model.todo_steps();
    let ticket = model.accept(true);
    assert_ne!(ticket, 0);
    assert!(
        model.locked,
        "the plan takes no edit while the answer waits"
    );

    model.steps[1].action = "pick".to_string();
    model.steps[2].action = "drop".to_string();
    model.steps.swap(0, 2);
    model.settle();

    assert_eq!(
        model.take_accepted(ticket),
        Some(Answer::Run {
            upstream: "base".to_string(),
            steps: pressed,
            expect: tip(),
            root: false,
        })
    );
    assert!(!model.locked);
}

/// A plan with no tip to pin takes no run: one taken would be given to
/// nothing, and the plan put away over a write never queued.
#[test]
fn a_plan_with_no_tip_to_pin_takes_no_run() {
    let mut model = RebasePlanModel {
        expect_head: None,
        ..standing()
    };
    assert_eq!(model.accept(true), 0);
    assert!(!model.locked, "the plan stays open to edits");
    assert_ne!(model.accept(false), 0, "and can still be put away");
}

#[test]
fn an_answer_is_given_once_and_only_to_its_own_ticket() {
    let mut model = standing();
    let ticket = model.accept(false);
    assert_eq!(
        model.take_accepted(ticket + 1),
        None,
        "another press's ticket takes nothing"
    );
    assert!(model.locked, "and leaves the answer standing");
    assert_eq!(model.take_accepted(ticket), Some(Answer::Discard));
    assert_eq!(model.take_accepted(ticket), None, "given once");
}

/// The tip moved, an operation stood, the tab left its worktree: the plan
/// is put away, and the answer with it — no plan opened after brings it
/// back, nor hands it the old ticket.
#[test]
fn a_plan_put_away_takes_its_answer_with_it_for_good() {
    let mut model = standing();
    let old = model.accept(true);
    // What `close` does to the state, short of the model reset.
    model.forget();
    assert!(!model.active);
    assert!(!model.locked);
    assert_eq!(model.take_accepted(old), None);

    // A plan open again.
    model.active = true;
    model.settle();
    let new = model.accept(true);
    assert_ne!(new, old, "a new press gets a ticket of its own");
    assert_eq!(
        model.take_accepted(old),
        None,
        "so the old one never takes the new answer"
    );
    assert!(model.take_accepted(new).is_some());
}

#[test]
fn nothing_is_taken_over_a_standing_answer_or_without_a_plan_to_run() {
    let mut model = standing();
    assert_ne!(model.accept(true), 0);
    assert_eq!(model.accept(false), 0, "one answer at a time");

    let mut shut = standing();
    shut.active = false;
    assert_eq!(shut.accept(false), 0, "no plan");

    let mut replacing = standing();
    replacing.loading = true;
    assert_eq!(replacing.accept(true), 0, "a plan about to be replaced");

    let mut clean = standing();
    clean.steps[1].action = "pick".to_string();
    clean.settle();
    assert_eq!(clean.accept(true), 0, "a run that asks nothing");
    assert_ne!(
        clean.accept(false),
        0,
        "while a clean draft can still be put away"
    );
}
