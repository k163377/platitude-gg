//! Whose doing an empty working tree is, at each of the moments that
//! decide it — the press, the answer, and the status whose counts can
//! speak for that answer.
//!
//! No Qt and no repository: what a tree emptying means is the
//! application's own question, and this is the whole of what decides it.

use super::*;

/// The id the queue is pretending to have accepted this window's stash
/// under, and one it gave somebody else — a pop pressed from the details
/// band, or the fetch running behind the press.
const OURS: u64 = 7;
const SOMEBODY_ELSE: u64 = 8;

/// Reports of HEAD either side of the write's end: a status read before
/// it (whose counts are the tree as it was), the fence its answer named,
/// and one from after.
const IN_FLIGHT: u64 = 40;
const FENCE: u64 = 41;
const AFTERWARDS: u64 = 42;

/// What the tree looks like to [`StashOut::read`].
const EMPTIED: bool = true;
const STILL_DIRTY: bool = false;

fn pressed() -> StashOut {
    let mut out = StashOut::default();
    out.asked(Some(OURS));
    out
}

fn landed() -> StashOut {
    let mut out = pressed();
    assert!(out.answered(OURS, false, FENCE));
    out
}

#[test]
fn the_tree_that_empties_after_our_own_stash_is_ours() {
    let mut out = landed();
    assert!(out.read(FENCE, EMPTIED));
}

// The answer stands from the write until a status that can speak for it
// arrives, because the two travel feeds of their own: a fetch coming
// back in between says nothing about whose stash emptied the tree.
#[test]
fn somebody_elses_answer_in_between_leaves_the_landing_standing() {
    let mut out = landed();
    assert!(!out.answered(SOMEBODY_ELSE, false, AFTERWARDS));
    assert!(out.ours(), "the fetch answered for itself and nothing else");
    assert!(out.read(FENCE, EMPTIED));
}

// A read already out when the write ended still shows the tree as it
// was. Taken as the answer, it settles the press on a dirty tree and the
// empty one that follows is nobody's — the reader is left standing over
// a pane that no longer describes anything.
#[test]
fn a_status_from_before_the_write_settles_nothing() {
    let mut out = landed();
    assert!(!out.read(IN_FLIGHT, STILL_DIRTY));
    assert!(out.ours(), "the status that can answer is still coming");
    assert!(out.read(FENCE, EMPTIED));
}

// …and it is the answer to *this* press that claims it: one arriving
// before ours has nothing to do with the tree we took away.
#[test]
fn an_answer_to_somebody_elses_write_claims_nothing() {
    let mut out = pressed();
    assert!(!out.answered(SOMEBODY_ELSE, false, FENCE));
    assert!(!out.ours());
}

// git would not make the entry, so nothing was taken away. A tree that
// empties after that emptied for some other reason, and the reader
// writing in the message box stays where they are.
#[test]
fn a_stash_git_refused_took_no_tree_away() {
    let mut out = pressed();
    assert!(out.answered(OURS, true, FENCE));
    assert!(!out.read(FENCE, EMPTIED));
}

// A tree that still has something in it settles the press too: the stash
// plainly did not take that away — half of one never does
// (`stash push -- paths`) — and a later emptying is somebody else's.
#[test]
fn a_tree_the_stash_did_not_empty_settles_the_press_all_the_same() {
    let mut out = landed();
    assert!(!out.read(FENCE, STILL_DIRTY));
    assert!(!out.ours());
    assert!(!out.read(AFTERWARDS, EMPTIED));
}

// Once: the landing is spent by the status it answers for, so the next
// tree to empty — somebody committing in a terminal — is not read as
// this window's doing all over again.
#[test]
fn one_press_empties_one_tree() {
    let mut out = landed();
    assert!(out.read(FENCE, EMPTIED));
    assert!(!out.read(AFTERWARDS, EMPTIED));
}

// A second press before the first tree was read is the one the landing
// belongs to: the tree emptying next is the work of the stash asked for
// last.
#[test]
fn the_press_that_came_last_is_the_one_the_tree_answers_for() {
    let mut out = landed();
    out.asked(Some(SOMEBODY_ELSE));
    assert!(!out.ours(), "the standing landing went with the new press");
    assert!(out.answered(SOMEBODY_ELSE, false, AFTERWARDS));
    assert!(!out.read(FENCE, EMPTIED), "that press named a later report");
    assert!(out.read(AFTERWARDS, EMPTIED));
}

// The queue takes nothing once the session is closed, so no answer is
// coming — and a tree that empties afterwards was emptied by somebody
// else.
#[test]
fn a_press_the_queue_took_nothing_for_claims_no_tree() {
    let mut out = landed();
    out.asked(None);
    assert!(!out.answered(OURS, false, FENCE));
    assert!(!out.read(AFTERWARDS, EMPTIED));
}

// A tree emptying with nothing of ours out at all — somebody committed
// these changes in another window.
#[test]
fn a_tree_nobody_here_emptied_is_not_claimed() {
    let mut out = StashOut::default();
    assert!(!out.answered(OURS, false, FENCE));
    assert!(!out.read(AFTERWARDS, EMPTIED));
}
