//! Which answer is one press's own, at each of its moments — the press,
//! the answer, and the notify that carried neither.
//!
//! No Qt and no repository: whose press an answer was is the
//! application's own question, and this is the whole of what decides it.

use super::*;

/// The id the queue is pretending to have accepted this press under, and
/// one it gave somebody else. Any two different numbers would do — what
/// these are about is that the answer is found by the number rather than
/// by its turn.
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

// The fetch running behind the press answers in the middle of it, and
// git says nothing about which of the two was asked for here.
#[test]
fn an_answer_to_somebody_elses_write_is_not_taken() {
    let mut out = pressed();
    assert!(!out.answered(SOMEBODY_ELSE, 0));
    assert_eq!(out.answer(), None);
}

// …and the press's own is still found when it answers behind that one:
// where it stands in the list is not read into.
#[test]
fn the_press_is_found_wherever_its_answer_stands() {
    let mut out = pressed();
    assert!(!out.answered(SOMEBODY_ELSE, 0));
    assert!(out.answered(OURS, 1));
    assert_eq!(out.answer(), Some(1));
}

// An answer arriving with nothing pressed is a page built over a tab
// whose write was already out — the press was another page's — or one
// the queue accepted nothing for. Either way nothing here is held open
// waiting for it.
#[test]
fn an_answer_nobody_here_pressed_for_is_not_taken() {
    let mut out = Press::default();
    assert!(!out.answered(OURS, 0));
    assert_eq!(out.answer(), None);
}

// The queue takes nothing once the session is closed. Waiting on an
// answer that is never coming would hold the press open for good, so
// nothing is waited on at all — and the id the *next* press gets cannot
// be mistaken for this one's.
#[test]
fn a_press_the_queue_took_nothing_for_waits_for_nothing() {
    let mut out = Press::default();
    out.asked(None);
    assert!(!out.answered(OURS, 0));
    out.asked(Some(0));
    assert!(!out.answered(0, 0));
}

// Answered once. The answer belongs to the notify that carried it, and
// the next drain says this press was told nothing — otherwise a second
// notify would act on the one answer all over again, over boxes somebody
// has since typed into and rows they have since put back.
#[test]
fn the_answer_goes_with_the_notify_that_carried_it() {
    let mut out = pressed();
    assert!(out.answered(OURS, 0));
    out.new_notify();
    assert_eq!(out.answer(), None);
}

// And the wait is spent with it: the same id cannot answer a second
// time, so a number the queue handed out again reaches a press that is
// already over.
#[test]
fn one_press_is_answered_once() {
    let mut out = pressed();
    assert!(out.answered(OURS, 0));
    out.new_notify();
    assert!(!out.answered(OURS, 0));
    assert_eq!(out.answer(), None);
}

// A second press while the first is still out waits for the second: what
// the screen is holding is what that press sent, and its answer is the
// one that says what became of it.
#[test]
fn the_press_that_came_last_is_the_one_waited_for() {
    let mut out = pressed();
    out.asked(Some(SOMEBODY_ELSE));
    assert!(!out.answered(OURS, 0));
    assert!(out.answered(SOMEBODY_ELSE, 1));
}
