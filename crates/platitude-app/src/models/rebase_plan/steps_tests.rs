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

/// One row dressed the way the two boxes and the verb chip leave it.
fn typed(action: &str, msg_subject: &str, msg_body: &str, subject: &str) -> PlanStepItem {
    PlanStepItem {
        action: action.to_string(),
        msg_subject: msg_subject.to_string(),
        msg_body: msg_body.to_string(),
        ..step("c1", subject)
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
fn an_untouched_plan_is_not_dirty_and_a_real_request_makes_it_so() {
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

    model.steps[2].action = "reword".to_string();
    model.steps[2].msg_subject = "typed".to_string();
    assert!(model.is_dirty(), "a typed reword is a request");
}

/// The verb on its own is not one. `todo_steps` folds a reword with an
/// empty pair back into `pick`, so a plan holding nothing else would run
/// the very todo an untouched plan writes — and the run button, which
/// reads `dirty`, stays shut.
#[test]
fn a_reword_is_a_request_only_once_something_is_typed() {
    let mut model = fresh();
    model.steps[0].action = "reword".to_string();
    assert!(!model.is_dirty(), "the verb alone asks nothing");
    assert!(
        model
            .todo_steps()
            .iter()
            .all(|s| s.action == TodoAction::Pick),
        "the todo it would write is all picks"
    );

    model.steps[0].msg_subject = "  ".to_string();
    assert!(!model.is_dirty(), "whitespace trims away to nothing");

    model.steps[0].msg_subject = "n".to_string();
    assert!(model.is_dirty(), "one character is a request");

    // The description carries a reword just as far as the summary does.
    model.steps[0].msg_subject = String::new();
    model.steps[0].msg_body = "n".to_string();
    assert!(
        model.is_dirty(),
        "typed into the other box, still a request"
    );
}

/// The button binds to the `dirty` *property*, which only [`settle`]
/// moves — so the answer has to survive the trip out of `is_dirty`, not
/// just be right inside it.
///
/// [`settle`]: RebasePlanModel::settle
#[test]
fn settle_carries_the_reword_rule_out_to_the_property() {
    let mut model = fresh();
    model.steps[0].action = "reword".to_string();
    model.settle();
    assert!(!model.dirty, "the verb alone leaves the button shut");

    model.steps[0].msg_subject = "typed".to_string();
    model.settle();
    assert!(model.dirty, "the typing opens it");
}

/// Every verb that is not `pick` is a request on its own — only `reword`
/// waits for a message. Pinned per verb because they now share one
/// mapping (`RebasePlanModel::todo_action_of`), where a guard written
/// for `reword` could reach the others.
#[test]
fn every_other_verb_is_a_request_without_a_message() {
    for action in ["edit", "squash", "fixup", "drop"] {
        let mut model = fresh();
        model.steps[1].action = action.to_string();
        assert!(model.is_dirty(), "{action} asks for something");
    }
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

/// A reorder that stands on its own holds the fold rule the moment it
/// lands — the row it stranded is `pick` before anything else is asked.
#[test]
fn a_reorder_on_its_own_demotes_where_it_lands() {
    let mut model = fresh();
    model.steps[1].action = "squash".to_string();
    assert!(model.reorder(1, 2), "c2 goes to the oldest place");
    assert_eq!(
        model.steps[2].action, "pick",
        "nothing left below to fold into"
    );
}

/// The drag reports every row it crosses, so the rule cannot be walked
/// per crossing: a `squash` carried down past the oldest row and back
/// would come out `pick` under the hand that never dropped it there, and
/// with the order restored the plan would read all-pick and shut its run
/// button on a fold its author is still looking at.
#[test]
fn a_fold_carried_past_the_oldest_row_and_back_survives_the_trip() {
    let mut model = fresh();
    model.steps[1].action = "squash".to_string();

    model.dragging = true;
    assert!(model.reorder(1, 2), "down onto the oldest place");
    assert_eq!(
        model.steps[2].action, "squash",
        "still a fold while the hand carries it"
    );
    assert!(model.reorder(2, 1), "and back where it came from");
    assert!(!model.hold_fold_rule(), "it lands where a fold stands");

    model.dragging = false;
    assert_eq!(model.steps[1].action, "squash");
    assert_eq!(
        model
            .steps
            .iter()
            .map(|s| s.oid_hex.as_str())
            .collect::<Vec<_>>(),
        vec!["c3", "c2", "c1"],
        "the round trip put the order back"
    );
    model.settle();
    assert!(model.dirty, "and the fold is still what the run would ask");
}

/// The other end of the same trip: a fold the hand actually *leaves* on
/// the oldest row is demoted at the release, visibly on the row
/// (デザイン規約 §フル interactive rebase).
#[test]
fn a_fold_dropped_on_the_oldest_row_is_demoted_at_the_release() {
    let mut model = fresh();
    model.steps[1].action = "squash".to_string();

    model.dragging = true;
    assert!(model.reorder(1, 2));
    assert_eq!(
        model.steps[2].action, "squash",
        "not yet — the hand is on it"
    );

    model.dragging = false;
    assert!(model.hold_fold_rule(), "the release is what demotes it");
    assert_eq!(model.steps[2].action, "pick");
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

/// What the entrance does with a second right-click while the first
/// answer is still out (`RebasePlanModel::open`): the same commit is the
/// same question and spends nothing twice, another commit is a question
/// of its own however long the one before it is taking, and a plan that
/// is already open asks again — the answer for it is not out.
#[test]
fn a_second_click_only_stands_down_for_the_commit_already_asked() {
    let mut model = RebasePlanModel::default();
    assert!(!model.already_asking("c3"), "nothing has been asked yet");

    model.loading = true;
    model.asked_from = "c3".to_string();
    assert!(model.already_asking("c3"));
    assert!(!model.already_asking("c2"), "a different commit, a new ask");

    model.loading = false;
    assert!(
        !model.already_asking("c3"),
        "the answer landed; asking again is a fresh question"
    );
}

#[test]
fn the_shown_subject_follows_the_typed_reword_and_falls_back() {
    let shown = |s: PlanStepItem| RebasePlanModel::shown_of(&s);
    assert_eq!(shown(typed("reword", "typed", "", "own")), "typed");
    assert_eq!(shown(typed("reword", "  ", "", "own")), "own");
    assert_eq!(shown(typed("pick", "typed", "", "own")), "own");
    // The description box carries a reword as far as the summary does, so
    // the row has to say so too: git takes the message's first line as
    // the subject, and the row showing the commit's old one would be the
    // plan telling the reader something the run then contradicts.
    assert_eq!(
        shown(typed("reword", "", "from the body\n\nand more", "own")),
        "from the body"
    );
    assert_eq!(
        shown(typed("pick", "", "from the body", "own")),
        "own",
        "no reword verb, nothing rewritten"
    );
}

/// The rule the row is drawn by and the rule the todo is written by are
/// the one mapping ([`RebasePlanModel::todo_action_of`]): whatever the
/// two boxes hold, what the row shows is the subject that message leaves.
#[test]
fn the_row_shows_the_subject_the_todo_would_write() {
    for (msg_subject, msg_body) in [
        ("typed", ""),
        ("", "from the body"),
        ("typed", "and a body"),
        ("", "from the body\n\nand more"),
    ] {
        let step = typed("reword", msg_subject, msg_body, "own");
        let (action, message) = RebasePlanModel::todo_action_of(&step);
        assert_eq!(action, TodoAction::Reword, "{msg_subject:?}/{msg_body:?}");
        assert_eq!(
            RebasePlanModel::shown_of(&step),
            platitude_core::commit::split_message(&message).0,
            "the row says what the run writes ({msg_subject:?}/{msg_body:?})"
        );
    }
}
