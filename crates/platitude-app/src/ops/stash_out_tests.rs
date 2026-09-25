//! Whose doing an empty working tree is, at each of the moments that
//! decide it — the press, the answer, and the status whose counts can
//! speak for that answer, in either order.

use super::*;

/// This stash's id and somebody else's — a pop pressed from the details
/// band, or the fetch running behind the press.
const OURS: u64 = 7;
const SOMEBODY_ELSE: u64 = 8;

/// Reports of HEAD either side of the write's end: a status read before
/// it (whose counts are the tree as it was), the fence its answer named,
/// and one from after.
const IN_FLIGHT: u64 = 40;
const FENCE: u64 = 41;
const AFTERWARDS: u64 = 42;

/// What the counts look like to [`StashOut::taken`].
const EMPTIED: bool = true;
const STILL_DIRTY: bool = false;

fn pressed() -> StashOut {
    let mut out = StashOut::default();
    out.asked(Some(OURS));
    out
}

fn landed() -> StashOut {
    let mut out = pressed();
    assert!(out.answered(OURS, 0, false, FENCE));
    out
}

#[test]
fn the_tree_that_empties_after_our_own_stash_is_ours() {
    let mut out = landed();
    assert!(out.taken(FENCE, EMPTIED));
}

// Dropped when read, the emptying is gone: the next status has no change
// to report, and the press never lands.
#[test]
fn a_tree_read_empty_before_the_answer_is_still_ours() {
    let mut out = pressed();
    assert!(
        !out.taken(FENCE, EMPTIED),
        "nothing to settle on until git answers"
    );
    assert!(out.answered(OURS, 0, false, FENCE));
    assert!(
        out.taken(FENCE, EMPTIED),
        "and the answer settles on the tree already read"
    );
}

#[test]
fn somebody_elses_answer_in_between_leaves_the_landing_standing() {
    let mut out = landed();
    assert!(!out.answered(SOMEBODY_ELSE, 1, false, AFTERWARDS));
    assert!(out.ours(), "the fetch answered for itself and nothing else");
    assert!(out.taken(FENCE, EMPTIED));
}

#[test]
fn a_status_from_before_the_write_settles_nothing() {
    let mut out = landed();
    assert!(!out.taken(IN_FLIGHT, STILL_DIRTY));
    assert!(out.ours(), "the status that can answer is still coming");
    assert!(out.taken(FENCE, EMPTIED));
}

#[test]
fn an_answer_to_somebody_elses_write_claims_nothing() {
    let mut out = pressed();
    assert!(!out.answered(SOMEBODY_ELSE, 0, false, FENCE));
    assert!(!out.ours());
    assert_eq!(out.answer(), None);
}

#[test]
fn a_stash_git_refused_took_no_tree_away() {
    let mut out = pressed();
    assert!(out.answered(OURS, 0, true, FENCE));
    assert!(!out.taken(FENCE, EMPTIED));
    assert_eq!(out.answer(), Some(0), "and the page still has to say it");
}

// Half a stash (`stash push -- paths`) never empties the tree.
#[test]
fn a_tree_the_stash_did_not_empty_settles_the_press_all_the_same() {
    let mut out = landed();
    assert!(!out.taken(FENCE, STILL_DIRTY));
    assert!(!out.ours());
    assert!(!out.taken(AFTERWARDS, EMPTIED));
}

// The next tree to empty — somebody committing in a terminal — is theirs.
#[test]
fn one_press_empties_one_tree() {
    let mut out = landed();
    assert!(out.taken(FENCE, EMPTIED));
    assert!(!out.taken(AFTERWARDS, EMPTIED));
}

#[test]
fn the_press_that_came_last_is_the_one_the_tree_answers_for() {
    let mut out = landed();
    out.asked(Some(SOMEBODY_ELSE));
    assert!(!out.ours(), "the standing landing went with the new press");
    assert!(out.answered(SOMEBODY_ELSE, 1, false, AFTERWARDS));
    assert!(
        !out.taken(FENCE, EMPTIED),
        "that press named a later report"
    );
    assert!(out.taken(AFTERWARDS, EMPTIED));
}

#[test]
fn a_press_the_queue_took_nothing_for_claims_no_tree() {
    let mut out = landed();
    out.asked(None);
    assert!(!out.answered(OURS, 0, false, FENCE));
    assert!(!out.taken(AFTERWARDS, EMPTIED));
}

#[test]
fn a_tree_nobody_here_emptied_is_not_claimed() {
    let mut out = StashOut::default();
    assert!(!out.answered(OURS, 0, false, FENCE));
    assert!(!out.taken(AFTERWARDS, EMPTIED));
}

#[test]
fn the_answers_place_goes_with_its_notify_and_the_wait_does_not() {
    let mut out = landed();
    assert_eq!(out.answer(), Some(0));
    out.new_notify();
    assert_eq!(out.answer(), None);
    assert!(out.ours(), "the tree it took away is still to be read");
    assert!(out.taken(FENCE, EMPTIED));
}
