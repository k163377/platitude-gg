//! Which answer the standing card's delete is, and what the card is
//! left holding at each of its moments.

use super::*;

/// The ids of the plain delete and of the fetch running behind it.
const OURS: u64 = 7;
const SOMEBODY_ELSE: u64 = 8;

fn pressed() -> BranchDeleteOut {
    let mut out = BranchDeleteOut::default();
    out.asked("feature", Some(OURS));
    out
}

#[test]
fn a_delete_git_took_names_the_branch_for_the_card_to_go_on() {
    let mut out = pressed();
    assert!(out.answered(OURS, 0, false, false));
    assert_eq!(out.landed(), "feature");
    assert_eq!(out.refused(), "");
    assert_eq!(out.answer(), Some(0));
}

#[test]
fn a_delete_git_refused_names_the_branch_for_the_row_to_turn_on() {
    let mut out = pressed();
    assert!(out.answered(OURS, 0, true, false));
    assert_eq!(out.refused(), "feature");
    assert_eq!(out.landed(), "");
}

#[test]
fn a_refusal_with_something_to_report_turns_no_row() {
    let mut out = pressed();
    assert!(out.answered(OURS, 0, true, true));
    assert_eq!(out.refused(), "");
    assert_eq!(out.landed(), "");
    assert_eq!(out.answer(), Some(0), "and the page still has to say it");
}

// The fetch behind the press answers in the same drain under its own id;
// matched by turn, the card would go on a branch git never touched.
#[test]
fn somebody_elses_answer_leaves_the_card_where_it_is() {
    let mut out = pressed();
    assert!(!out.answered(SOMEBODY_ELSE, 0, false, false));
    assert_eq!(out.landed(), "");
    assert_eq!(out.refused(), "");
    assert_eq!(out.answer(), None);
    // …and the card's own answer still finds it, wherever it stands.
    assert!(out.answered(OURS, 1, false, false));
    assert_eq!(out.landed(), "feature");
    assert_eq!(out.answer(), Some(1));
}

// `-D` and `Delete both` arm nothing.
#[test]
fn a_branch_answer_nobody_stayed_up_for_names_no_card() {
    let mut out = BranchDeleteOut::default();
    assert!(!out.answered(OURS, 0, false, false));
    assert_eq!(out.landed(), "");
}

#[test]
fn the_answer_stands_but_its_place_in_the_notify_does_not() {
    let mut out = pressed();
    assert!(out.answered(OURS, 0, true, false));
    out.new_notify();
    assert_eq!(out.refused(), "feature", "the card has still to read it");
    assert_eq!(out.answer(), None, "but this notify carried none of it");
}

// A re-made branch of the same name reads its own answer as a change.
#[test]
fn the_next_press_takes_the_last_answer_down_with_it() {
    let mut out = pressed();
    assert!(out.answered(OURS, 0, false, false));
    out.asked("feature", Some(SOMEBODY_ELSE));
    assert_eq!(out.landed(), "");
    assert_eq!(out.refused(), "");
}

// Not even the last press's id answers it any more.
#[test]
fn a_press_the_queue_took_nothing_for_waits_for_no_answer() {
    let mut out = pressed();
    assert!(out.answered(OURS, 0, false, false));
    out.asked("other", None);
    assert_eq!(out.landed(), "");
    assert!(!out.answered(OURS, 0, false, false));
    assert!(!out.answered(SOMEBODY_ELSE, 0, false, false));
}
