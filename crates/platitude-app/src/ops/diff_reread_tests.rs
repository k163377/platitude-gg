//! Which status a re-read of the open file already answers for.
//!
//! No Qt and no repository: telling the status a write published from
//! one that was in flight when it ended is the application's own
//! question, and a stamp is the whole of what decides it.

use super::*;

/// Reports of HEAD either side of the write's end — the fence its answer
/// named, a read that was already in flight when it ended, and the one
/// that came after.
const IN_FLIGHT: u64 = 40;
const FENCE: u64 = 41;
const AFTERWARDS: u64 = 42;

fn read_for_a_write() -> DiffReread {
    let mut read = DiffReread::default();
    read.read_after(FENCE);
    read
}

#[test]
fn the_status_the_write_published_is_the_one_the_read_answers_for() {
    let mut read = read_for_a_write();
    assert!(read.taken(FENCE));
}

// A report from above the fence answers just as well: the page reads
// whichever of the two comes first, and a later one still looked after
// the write.
#[test]
fn a_later_report_answers_for_it_too() {
    let mut read = read_for_a_write();
    assert!(read.taken(AFTERWARDS));
}

// A read already out when the write ended describes the repository as it
// was. Taken for the write's own, it would leave the shown file as a
// picture of a file that is gone.
#[test]
fn a_report_from_before_the_write_answers_for_nothing() {
    let mut read = read_for_a_write();
    assert!(!read.taken(IN_FLIGHT));
    assert!(
        read.taken(FENCE),
        "and the one that does answer is still coming"
    );
}

// Spent by the report it answers for, so the next status to move the
// tree — somebody editing the file in another window — is news again.
#[test]
fn one_read_answers_for_one_status() {
    let mut read = read_for_a_write();
    assert!(read.taken(FENCE));
    assert!(!read.taken(AFTERWARDS));
}

// A status with no read standing behind it is news, which is every
// status that does not follow a write of this window's.
#[test]
fn a_status_nothing_was_read_for_is_news() {
    let mut read = DiffReread::default();
    assert!(!read.taken(AFTERWARDS));
}

// A write that named no report has nothing to measure a status against,
// so the status behind it reads the file once more rather than skipping
// a read nobody can prove was already made.
#[test]
fn a_write_that_named_no_report_claims_no_status() {
    let mut read = DiffReread::default();
    read.read_after(0);
    assert!(!read.taken(AFTERWARDS));
}

// The read standing is the one made last: two answers in one notify are
// read for once, and the fence is the later write's.
#[test]
fn the_read_made_last_is_the_one_standing() {
    let mut read = read_for_a_write();
    read.read_after(AFTERWARDS);
    assert!(!read.taken(FENCE));
    assert!(read.taken(AFTERWARDS));
}
