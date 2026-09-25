//! Which status a re-read of the open file already answers for.

use super::*;

/// Reports of HEAD either side of the write's end — the fence its answer
/// named, a read that was already in flight when it ended, and the one
/// that came after.
const IN_FLIGHT: u64 = 40;
const FENCE: u64 = 41;
const AFTERWARDS: u64 = 42;

/// A page that has put no status on screen yet.
const NOTHING_READ: u64 = 0;

fn read_for_a_write() -> DiffReread {
    let mut read = DiffReread::default();
    read.read_after(FENCE, NOTHING_READ);
    read
}

#[test]
fn the_status_the_write_published_is_the_one_the_read_answers_for() {
    let mut read = read_for_a_write();
    assert!(read.taken(FENCE));
}

// The page reads whichever comes first; a later report still looked after
// the write.
#[test]
fn a_later_report_answers_for_it_too() {
    let mut read = read_for_a_write();
    assert!(read.taken(AFTERWARDS));
}

// Taken for the write's own, a read in flight when the write ended would
// leave a file that is gone on screen.
#[test]
fn a_report_from_before_the_write_answers_for_nothing() {
    let mut read = read_for_a_write();
    assert!(!read.taken(IN_FLIGHT));
    assert!(
        read.taken(FENCE),
        "and the one that does answer is still coming"
    );
}

// The next status — somebody editing the file elsewhere — is news again.
#[test]
fn one_read_answers_for_one_status() {
    let mut read = read_for_a_write();
    assert!(read.taken(FENCE));
    assert!(!read.taken(AFTERWARDS));
}

#[test]
fn a_status_nothing_was_read_for_is_news() {
    let mut read = DiffReread::default();
    assert!(!read.taken(AFTERWARDS));
}

#[test]
fn a_write_that_named_no_report_claims_no_status() {
    let mut read = DiffReread::default();
    read.read_after(0, NOTHING_READ);
    assert!(!read.taken(AFTERWARDS));
}

// Two answers in one notify are read for once, and the fence is the later
// write's.
#[test]
fn the_read_made_last_is_the_one_standing() {
    let mut read = read_for_a_write();
    read.read_after(AFTERWARDS, NOTHING_READ);
    assert!(!read.taken(FENCE));
    assert!(read.taken(AFTERWARDS));
}

// The status the write published was applied before the page read the
// answer that asks for the file.
#[test]
fn a_status_that_arrived_first_leaves_no_read_standing() {
    let mut read = DiffReread::default();
    assert!(
        !read.taken(FENCE),
        "nothing was waiting for it, and it read the file itself"
    );
    read.read_after(FENCE, FENCE);
    assert!(
        !read.taken(AFTERWARDS),
        "the next status is somebody else's change, and it is news"
    );
}

#[test]
fn a_status_from_before_the_write_leaves_the_read_to_be_made() {
    let mut read = DiffReread::default();
    read.read_after(FENCE, IN_FLIGHT);
    assert!(read.taken(FENCE));
}
