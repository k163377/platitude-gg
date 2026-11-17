//! Which answer is one press's own, at each of its moments — the press,
//! the answer, and the notify that carried neither.

use super::*;

/// This press's id and somebody else's; any two different numbers do.
const OURS: u64 = 7;
const SOMEBODY_ELSE: u64 = 8;

fn pressed() -> Press {
    let mut out = Press::default();
    out.asked(Some(OURS));
    out
}

#[test]
fn the_answer_to_the_press_is_the_presss_own() {
    let mut out = pressed();
    assert!(out.answered(OURS, 0));
    assert_eq!(out.answer(), Some(0));
}

// The fetch running behind the press answers in the middle of it.
#[test]
fn an_answer_to_somebody_elses_write_is_not_taken() {
    let mut out = pressed();
    assert!(!out.answered(SOMEBODY_ELSE, 0));
    assert_eq!(out.answer(), None);
}

#[test]
fn the_press_is_found_wherever_its_answer_stands() {
    let mut out = pressed();
    assert!(!out.answered(SOMEBODY_ELSE, 0));
    assert!(out.answered(OURS, 1));
    assert_eq!(out.answer(), Some(1));
}

// A page built over a tab whose write was already out: the press was
// another page's.
#[test]
fn an_answer_nobody_here_pressed_for_is_not_taken() {
    let mut out = Press::default();
    assert!(!out.answered(OURS, 0));
    assert_eq!(out.answer(), None);
}

// The session is closed (`None`), and zero is never an accepted id.
#[test]
fn a_press_the_queue_took_nothing_for_waits_for_nothing() {
    let mut out = Press::default();
    out.asked(None);
    assert!(!out.answered(OURS, 0));
    out.asked(Some(0));
    assert!(!out.answered(0, 0));
}

// Otherwise a second notify would act on the one answer again, over boxes
// somebody has since typed into.
#[test]
fn the_answer_goes_with_the_notify_that_carried_it() {
    let mut out = pressed();
    assert!(out.answered(OURS, 0));
    out.new_notify();
    assert_eq!(out.answer(), None);
}

// A number the queue hands out again reaches a press that is already over.
#[test]
fn one_press_is_answered_once() {
    let mut out = pressed();
    assert!(out.answered(OURS, 0));
    out.new_notify();
    assert!(!out.answered(OURS, 0));
    assert_eq!(out.answer(), None);
}

// What the screen holds is what the second press sent.
#[test]
fn the_press_that_came_last_is_the_one_waited_for() {
    let mut out = pressed();
    out.asked(Some(SOMEBODY_ELSE));
    assert!(!out.answered(OURS, 0));
    assert!(out.answered(SOMEBODY_ELSE, 1));
}
