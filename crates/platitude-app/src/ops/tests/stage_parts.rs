//! A part of a diff and a discard of rows pressed ahead of git: what each
//! leaves on screen at the press, at git's answer, and at the reads either
//! side of the write's end — the status readers' (stamped) and the open
//! diff's (by the bytes it was made on).

use platitude_core::eol::{Eol, Notice};
use platitude_core::parts::{Moves, Part as CorePart};
use platitude_core::session::EolMark;
use platitude_core::stage::DiscardSide;
use platitude_core::status::{StatusItem, WorktreeStatus};

use super::*;

const OURS: u64 = 7;
const NEXT: u64 = 8;

/// Status stamps either side of the write's end (`Standing::fence`).
const IN_FLIGHT: u64 = 40;
const FENCE: u64 = 41;

/// The diff on screen at the press, and the same file read again after it.
const KEY: &str = "unstaged:a.txt";
const PRESSED_ON: u64 = 0xaaaa;
const READ_AFTER: u64 = 0xbbbb;

const READERS: [Reader; 4] = [
    Reader::Conflicts,
    Reader::Unstaged,
    Reader::Staged,
    Reader::Counts,
];

fn tracked(staged: char, unstaged: char, path: &str) -> StatusItem {
    StatusItem::Tracked {
        staged,
        unstaged,
        path: path.to_string(),
        orig_path: None,
    }
}

fn status(items: Vec<StatusItem>) -> WorktreeStatus {
    WorktreeStatus {
        branch_head: Some("main".to_string()),
        items,
        ..WorktreeStatus::default()
    }
}

fn files(_: &str) -> bool {
    false
}

fn drawn(ahead: &mut StageAhead, looked: u64, base: &WorktreeStatus) -> Vec<StatusItem> {
    ahead
        .picture(looked, base, &[], files)
        .map_or_else(|| base.items.clone(), |p| p.status.items)
}

fn every_reader_drew(ahead: &mut StageAhead, looked: u64) {
    for reader in READERS {
        ahead.drawn(reader, looked);
    }
}

/// Hunk 1 of `a.txt`'s diff, pressed on the bytes on screen.
fn part(verb: PartVerb, whole: bool) -> Part {
    Part {
        verb,
        path: "a.txt".to_string(),
        whole,
        spot: DiffSpot {
            key: KEY.to_string(),
            fingerprint: PRESSED_ON,
            hunk: 1,
            line: None,
        },
    }
}

fn hunk_staged() -> (StageAhead, WorktreeStatus) {
    let mut ahead = StageAhead::default();
    assert!(ahead.asked_part(part(PartVerb::Stage, false), Some(OURS)));
    (ahead, status(vec![tracked('.', 'M', "a.txt")]))
}

#[test]
fn a_hunk_staged_out_of_several_shows_the_file_on_both_sides_at_once() {
    let (mut ahead, base) = hunk_staged();
    assert_eq!(
        drawn(&mut ahead, IN_FLIGHT, &base),
        [tracked('M', 'M', "a.txt")]
    );
    assert!(
        !ahead.reaches("unstaged", "a.txt"),
        "a change is left on the unstaged side: the pane stays on it"
    );
    assert_eq!(
        ahead.moving_paths(),
        ["a.txt"],
        "the row's marks wait for git"
    );
    assert!(
        ahead.index_unseen(),
        "Commit waits: git is writing the index"
    );
    assert_eq!(
        ahead.diff_parts(KEY, PRESSED_ON),
        [DiffPart {
            id: OURS,
            part: CorePart {
                hunk: 1,
                line: None,
                moves: Moves::Old,
            },
            landed: false,
        }],
        "the open diff draws the hunk taken out"
    );
    assert!(
        ahead.diff_parts(KEY, READ_AFTER).is_empty(),
        "nothing over other bytes: the press's places do not address them"
    );
}

#[test]
fn the_last_change_staged_moves_the_row_as_a_whole_file_would() {
    let mut ahead = StageAhead::default();
    assert!(ahead.asked_part(part(PartVerb::Stage, true), Some(OURS)));
    let base = status(vec![tracked('.', 'M', "a.txt"), tracked('A', 'M', "b.txt")]);
    assert_eq!(
        drawn(&mut ahead, IN_FLIGHT, &base),
        [tracked('M', '.', "a.txt"), tracked('A', 'M', "b.txt")]
    );
    assert!(ahead.reaches("unstaged", "a.txt"), "the pane walks on");
    assert!(!ahead.reaches("staged", "a.txt"));
}

#[test]
fn a_row_whose_staged_half_is_a_comparison_of_contents_waits_for_the_status() {
    let mut ahead = StageAhead::default();
    assert!(ahead.asked_part(part(PartVerb::Stage, true), Some(OURS)));
    let base = status(vec![tracked('M', 'M', "a.txt")]);
    assert_eq!(drawn(&mut ahead, IN_FLIGHT, &base), base.items);
    assert!(
        ahead.reaches("unstaged", "a.txt"),
        "its unstaged row leaves all the same: the pane walks on"
    );
}

#[test]
fn a_part_unstaged_from_a_staged_edit_shows_it_on_both_sides() {
    let mut ahead = StageAhead::default();
    assert!(ahead.asked_part(part(PartVerb::Unstage, false), Some(OURS)));
    let base = status(vec![tracked('M', '.', "a.txt")]);
    assert_eq!(
        drawn(&mut ahead, IN_FLIGHT, &base),
        [tracked('M', 'M', "a.txt")]
    );
    let mut whole = StageAhead::default();
    assert!(whole.asked_part(part(PartVerb::Unstage, true), Some(OURS)));
    assert_eq!(
        drawn(&mut whole, IN_FLIGHT, &base),
        [tracked('.', 'M', "a.txt")]
    );
    assert!(whole.reaches("staged", "a.txt"));
    assert_eq!(
        whole.diff_parts(KEY, PRESSED_ON)[0].part.moves,
        Moves::New,
        "an unstage gives the new side back"
    );
}

#[test]
fn a_refused_part_comes_back_in_the_rows_and_the_diff_on_the_answer() {
    let (mut ahead, base) = hunk_staged();
    assert!(ahead.answered(OURS, true, 0));
    assert_eq!(drawn(&mut ahead, IN_FLIGHT, &base), base.items);
    assert!(ahead.diff_parts(KEY, PRESSED_ON).is_empty());
    assert!(!ahead.ahead_unseen());
}

#[test]
fn a_hunk_discarded_whole_takes_the_row_and_leaves_the_index_alone() {
    let mut ahead = StageAhead::default();
    assert!(ahead.asked_part(part(PartVerb::Discard, true), Some(OURS)));
    let base = status(vec![tracked('.', 'M', "a.txt"), tracked('M', 'M', "b.txt")]);
    assert_eq!(
        drawn(&mut ahead, IN_FLIGHT, &base),
        [tracked('M', 'M', "b.txt")]
    );
    assert!(ahead.reaches("unstaged", "a.txt"));
    assert!(
        !ahead.index_unseen(),
        "the disk alone: Commit and the stash do not wait"
    );
    assert!(ahead.unseen() && ahead.ahead_unseen());

    let mut kept = StageAhead::default();
    let mut on_both = part(PartVerb::Discard, true);
    on_both.path = "b.txt".to_string();
    assert!(kept.asked_part(on_both, Some(OURS)));
    assert_eq!(
        drawn(&mut kept, IN_FLIGHT, &base),
        [tracked('.', 'M', "a.txt"), tracked('M', '.', "b.txt")],
        "what is staged stays"
    );
}

fn discard(chosen: &[(&str, DiscardSide)]) -> StageAhead {
    let chosen: Vec<(String, DiscardSide)> = chosen
        .iter()
        .map(|(path, side)| ((*path).to_string(), *side))
        .collect();
    let mut ahead = StageAhead::default();
    assert!(ahead.asked_discard(&chosen, Some(OURS)));
    ahead
}

#[test]
fn discarded_rows_go_at_the_press_each_by_the_side_it_was_chosen_on() {
    let base = status(vec![
        tracked('.', 'M', "edit.txt"),
        tracked('M', 'M', "both.txt"),
        tracked('M', 'M', "staged.txt"),
        StatusItem::Untracked {
            path: "new.txt".to_string(),
        },
        tracked('.', 'M', "kept.txt"),
    ]);
    let mut ahead = discard(&[
        ("edit.txt", DiscardSide::Unstaged),
        ("both.txt", DiscardSide::Unstaged),
        ("staged.txt", DiscardSide::Staged),
        ("new.txt", DiscardSide::Untracked),
    ]);
    assert_eq!(
        drawn(&mut ahead, IN_FLIGHT, &base),
        [tracked('M', '.', "both.txt"), tracked('.', 'M', "kept.txt")]
    );
    assert!(
        ahead.index_unseen(),
        "a staged row goes back to HEAD: git writes the index"
    );
    assert!(ahead.reaches("staged", "staged.txt") && ahead.reaches("unstaged", "staged.txt"));
    assert!(ahead.reaches("untracked", "new.txt"));
    assert!(
        !ahead.reaches("staged", "both.txt"),
        "its staged half stays"
    );
    let mut moving = ahead.moving_paths();
    moving.sort();
    assert_eq!(moving, ["both.txt", "edit.txt", "new.txt", "staged.txt"]);
}

#[test]
fn a_discard_of_the_disk_alone_holds_no_write_of_the_index() {
    let ahead = discard(&[("edit.txt", DiscardSide::Unstaged)]);
    assert!(!ahead.index_unseen());
    assert!(ahead.unseen());
}

#[test]
fn a_folder_and_an_intent_to_add_file_are_left_to_the_status() {
    let base = status(vec![
        tracked('.', 'M', "lib"),
        tracked('.', 'A', "ita.txt"),
        StatusItem::Untracked {
            path: "nest/".to_string(),
        },
    ]);
    let mut ahead = discard(&[
        ("lib", DiscardSide::Unstaged),
        ("ita.txt", DiscardSide::Unstaged),
        ("nest/", DiscardSide::Untracked),
    ]);
    let rows = ahead
        .picture(IN_FLIGHT, &base, &[], |path| path == "lib")
        .map_or_else(|| base.items.clone(), |p| p.status.items);
    assert_eq!(rows, base.items);
    assert!(
        !ahead.reaches("unstaged", "lib"),
        "a gitlink's checkout stays: the pane waits on it for the status"
    );
    assert!(
        !ahead.reaches("untracked", "nest/"),
        "`git clean` skips a repository"
    );
}

#[test]
fn a_discarded_row_takes_its_line_ending_mark_with_it() {
    let base = status(vec![tracked('.', 'M', "a.txt"), tracked('.', 'M', "b.txt")]);
    let mark = |path: &str| EolMark {
        path: path.to_string(),
        notice: Notice::Flipped {
            from: Eol::Lf,
            to: Eol::Crlf,
        },
        staged: false,
    };
    let mut ahead = discard(&[("a.txt", DiscardSide::Unstaged)]);
    let picture = ahead
        .picture(IN_FLIGHT, &base, &[mark("a.txt"), mark("b.txt")], files)
        .expect("a row went");
    assert_eq!(picture.marks, [mark("b.txt")]);
}

#[test]
fn a_discard_refused_puts_its_rows_back_and_one_landed_waits_for_the_status() {
    let base = status(vec![tracked('.', 'M', "a.txt")]);
    let mut refused = discard(&[("a.txt", DiscardSide::Unstaged)]);
    assert!(refused.answered(OURS, true, 0));
    assert_eq!(drawn(&mut refused, IN_FLIGHT, &base), base.items);

    let mut landed = discard(&[("a.txt", DiscardSide::Unstaged)]);
    assert!(landed.answered(OURS, false, FENCE));
    every_reader_drew(&mut landed, IN_FLIGHT);
    assert_eq!(
        drawn(&mut landed, IN_FLIGHT, &base),
        [],
        "a status in flight as the write ended still holds the row"
    );
    every_reader_drew(&mut landed, FENCE);
    assert!(!landed.unseen());
}

#[test]
fn a_part_behind_a_refused_one_leaves_the_drawing_with_it() {
    let (mut ahead, base) = hunk_staged();
    let mut behind = part(PartVerb::Stage, false);
    behind.spot.hunk = 0;
    assert!(ahead.asked_part(behind, Some(NEXT)));
    assert_eq!(ahead.diff_parts(KEY, PRESSED_ON).len(), 2);
    assert!(ahead.answered(OURS, true, 0));
    assert!(ahead.diff_parts(KEY, PRESSED_ON).is_empty());
    assert!(ahead.unseen(), "the one behind still waits for git");
    assert_eq!(
        drawn(&mut ahead, IN_FLIGHT, &base),
        [tracked('M', 'M', "a.txt")],
        "its row is drawn by its own press until it is answered"
    );
}
