//! A part of the open diff over the reads the pane draws, in the orders a
//! press, git's write, its answer, the pane's reads and somebody else's edit
//! can come in: a read the pane asked once git had answered the part is git's
//! word on it; one asked before keeps the part drawn over it, moved onto the
//! read where the read is what the parts in front leave.

use std::sync::Arc;

use platitude_core::parse::diff::{FilePatch, parse_patch};
use platitude_core::parts::{Moves, Part as Placed, after};

use super::super::*;

const KEY: &str = "unstaged:a.txt";
const HEAD: &str =
    "diff --git a/a.txt b/a.txt\nindex 1111111..2222222 100644\n--- a/a.txt\n+++ b/a.txt\n";
/// The diff as read at the press: a line added near the top, one rewritten
/// further down.
const PRESSED: &str = "@@ -1,4 +1,5 @@\n one\n+new1\n two\n three\n four\n\
@@ -10,4 +11,4 @@ heading\n ten\n-eleven\n+ELEVEN\n twelve\n thirteen\n";

const A: u64 = 7;
const B: u64 = 8;

fn read(body: &str) -> Arc<Vec<FilePatch>> {
    Arc::new(parse_patch(format!("{HEAD}{body}").as_bytes()))
}

/// What git prints once the parts are done: `after` holds it against git in
/// core (`stage_integration::pressed_on`).
fn done(parts: &[Placed]) -> Arc<Vec<FilePatch>> {
    Arc::new(after(&read(PRESSED), parts).patches)
}

const FIRST: Placed = Placed {
    hunk: 0,
    line: None,
    moves: Moves::Old,
};

/// Hunk 0 staged, pressed on the read drawn as `print`.
fn hunk(print: u64) -> super::super::Part {
    super::super::Part {
        verb: PartVerb::Stage,
        path: "a.txt".to_string(),
        whole: false,
        spot: DiffSpot {
            key: KEY.to_string(),
            fingerprint: print,
            hunk: 0,
            line: None,
        },
    }
}

/// The pane drawing the read at the press, asked as `asked`, and `id`
/// pressed on it.
fn pressed(id: u64) -> StageAhead {
    let mut ahead = StageAhead::default();
    ahead.diff_asked(1);
    ahead.diff_drawn(Some((KEY.to_string(), 100)), Some(read(PRESSED)), 1);
    assert!(ahead.asked_part(hunk(100), Some(id)));
    ahead
}

fn drawn_over(ahead: &StageAhead, print: u64) -> Vec<u64> {
    ahead.diff_parts(KEY, print).iter().map(|p| p.id).collect()
}

/// P → git writes → answer → a read asked after it: git's word, the part goes.
#[test]
fn a_read_asked_after_the_answer_is_drawn_as_it_is() {
    let mut ahead = pressed(A);
    assert!(ahead.answered(A, false, 50));
    ahead.diff_asked(2);
    ahead.diff_drawn(Some((KEY.to_string(), 200)), Some(done(&[FIRST])), 2);
    assert!(drawn_over(&ahead, 200).is_empty());
    assert!(!ahead.part_on_drawn(A));
}

/// P → a read asked (a status from outside came in) → git writes → answer →
/// that read lands, holding the bytes from before the write: the part stays
/// drawn over it, and goes with the read asked after the answer.
#[test]
fn a_read_asked_before_the_answer_keeps_the_part_drawn_over_it() {
    let mut ahead = pressed(A);
    ahead.diff_asked(2);
    assert!(ahead.answered(A, false, 50));
    ahead.diff_drawn(Some((KEY.to_string(), 100)), Some(read(PRESSED)), 2);
    assert_eq!(
        drawn_over(&ahead, 100),
        [A],
        "drawn without it, the hunk would come back under the hand"
    );
    ahead.diff_asked(3);
    ahead.diff_drawn(Some((KEY.to_string(), 200)), Some(done(&[FIRST])), 3);
    assert!(drawn_over(&ahead, 200).is_empty());
}

/// Hunk after hunk: the second pressed while the first is out; the first's
/// answer asks a read that git makes before the second runs. That read is
/// git's word on the first, and the second is moved onto it where it stood —
/// then goes with the read asked after its own answer.
#[test]
fn the_part_still_out_is_moved_onto_the_read_after_the_one_in_front() {
    let mut ahead = pressed(A);
    let mut second = hunk(100);
    second.spot.hunk = 0;
    assert!(ahead.asked_part(second, Some(B)));
    assert!(ahead.answered(A, false, 50));
    ahead.diff_asked(2);
    let first_done = done(&[FIRST]);
    ahead.diff_drawn(Some((KEY.to_string(), 200)), Some(first_done), 2);
    assert_eq!(
        drawn_over(&ahead, 200),
        [B],
        "the second stands on git's read"
    );
    let (read, in_front) = ahead.pressed_on(KEY, 200).expect("drawn");
    assert_eq!(
        in_front.len(),
        1,
        "a third press is placed behind the second"
    );
    assert_eq!(read.len(), 1);
    assert!(ahead.answered(B, false, 60));
    ahead.diff_asked(3);
    ahead.diff_drawn(Some((KEY.to_string(), 300)), Some(done(&[FIRST, FIRST])), 3);
    assert!(drawn_over(&ahead, 300).is_empty());
}

/// Both written before the read the first's answer asked: it already holds
/// the second, which is drawn over nothing.
#[test]
fn a_read_that_already_holds_the_part_still_out_draws_nothing_over_it() {
    let mut ahead = pressed(A);
    assert!(ahead.asked_part(hunk(100), Some(B)));
    assert!(ahead.answered(A, false, 50));
    ahead.diff_asked(2);
    ahead.diff_drawn(Some((KEY.to_string(), 300)), Some(done(&[FIRST, FIRST])), 2);
    assert!(drawn_over(&ahead, 300).is_empty());
}

/// P → git writes → somebody puts the file back → answer → the read asked
/// after it holds the very bytes of the press: git's word all the same — the
/// hunk is back, and drawn.
#[test]
fn a_read_asked_after_the_answer_wins_even_on_the_bytes_of_the_press() {
    let mut ahead = pressed(A);
    assert!(ahead.answered(A, false, 50));
    ahead.diff_asked(2);
    ahead.diff_drawn(Some((KEY.to_string(), 100)), Some(read(PRESSED)), 2);
    assert!(drawn_over(&ahead, 100).is_empty());
}

/// A read asked before the answer on bytes somebody changed since: the part's
/// place names nothing in it, and it leaves the drawing — git turns it down
/// as moved; its row waits on its own answer.
#[test]
fn a_read_of_bytes_no_part_leaves_takes_the_parts_out_of_the_drawing() {
    let mut ahead = pressed(A);
    ahead.diff_asked(2);
    let moved = read("@@ -1,4 +1,5 @@\n one\n+other\n two\n three\n four\n");
    ahead.diff_drawn(Some((KEY.to_string(), 900)), Some(moved), 2);
    assert!(drawn_over(&ahead, 900).is_empty());
    assert!(ahead.unseen(), "the row waits for git");
}

/// The pane on another file: an answered part has nothing left to draw it
/// over; one still out stays written down for its answer.
#[test]
fn another_file_drawn_lets_an_answered_part_go() {
    let mut ahead = pressed(A);
    assert!(ahead.answered(A, false, 50));
    ahead.diff_drawn(None, None, 0);
    assert!(drawn_over(&ahead, 100).is_empty());
}

/// The pane left the file while the part ran (closed, or another file
/// opened): its answer lets it go — no read of the pane's will speak for it,
/// and nothing waits on one once the readers have drawn the status after it.
#[test]
fn a_part_answered_after_the_pane_left_goes_with_its_answer() {
    let mut ahead = pressed(A);
    ahead.diff_drawn(None, None, 0);
    assert!(ahead.answered(A, false, 50));
    for reader in [
        Reader::Conflicts,
        Reader::Unstaged,
        Reader::Staged,
        Reader::Counts,
    ] {
        ahead.drawn(reader, 60);
    }
    assert!(!ahead.ahead_unseen());
}
