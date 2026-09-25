//! Whose push an answer was, and which branch it was about — at the
//! press, at the answer, and across the presses that follow.

use super::*;

/// This push's id and somebody else's — a remote branch's replace, which
/// also answers as a push, or the fetch running behind the press.
const OURS: u64 = 7;
const SOMEBODY_ELSE: u64 = 8;

fn pressed(branch: &str) -> PushOut {
    let mut out = PushOut::default();
    out.asked(Some(OURS), branch.into());
    out
}

#[test]
fn the_answer_to_the_press_is_the_presss_own_and_names_its_branch() {
    let mut out = pressed("topic");
    assert!(out.answered(OURS, 0));
    assert_eq!(out.answer(), Some(0));
    assert_eq!(out.branch(), "topic");
}

#[test]
fn an_answer_to_somebody_elses_push_is_not_taken() {
    let mut out = pressed("topic");
    assert!(!out.answered(SOMEBODY_ELSE, 0));
    assert_eq!(out.answer(), None);
}

// The reader switched branches and pushed again before the first answered.
#[test]
fn a_second_press_is_its_own_and_the_first_is_no_longer_waited_for() {
    let mut out = pressed("topic");
    out.asked(Some(SOMEBODY_ELSE), "other".into());
    assert!(
        !out.answered(OURS, 0),
        "the first press was overtaken by the second and is nobody's now"
    );
    assert!(out.answered(SOMEBODY_ELSE, 1));
    assert_eq!(out.answer(), Some(1));
    assert_eq!(out.branch(), "other");
}

// The branch of the press before is not left standing as this one's.
#[test]
fn a_press_the_queue_took_nothing_for_waits_for_nothing() {
    let mut out = pressed("topic");
    out.asked(None, "other".into());
    assert!(!out.answered(OURS, 0));
    assert_eq!(out.branch(), "other");
}

#[test]
fn the_answers_place_is_put_down_with_the_notify() {
    let mut out = pressed("topic");
    assert!(out.answered(OURS, 2));
    out.new_notify();
    assert_eq!(out.answer(), None);
}
