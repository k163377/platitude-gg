//! What a stage or unstage leaves on screen at each of its moments — the
//! press, git's answer, and the statuses each reader draws on either side of
//! the write's end — and how long it stands for the writes that wait on it
//! (`StageAhead::unseen`).

use std::collections::HashSet;

use platitude_core::eol::{Eol, Notice};
use platitude_core::session::EolMark;
use platitude_core::status::{StatusItem, WorktreeStatus};

use super::*;

/// A press's id and somebody else's; any two different numbers do.
const OURS: u64 = 7;
const NEXT: u64 = 8;
const SOMEBODY_ELSE: u64 = 9;

/// Status stamps either side of the write's end (`Standing::fence`).
const IN_FLIGHT: u64 = 40;
const FENCE: u64 = 41;
const AFTERWARDS: u64 = 42;

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

fn untracked(path: &str) -> StatusItem {
    StatusItem::Untracked {
        path: path.to_string(),
    }
}

fn status(items: Vec<StatusItem>) -> WorktreeStatus {
    WorktreeStatus {
        branch_head: Some("main".to_string()),
        items,
        ..WorktreeStatus::default()
    }
}

fn paths(names: &[&str]) -> Reach {
    Reach::Paths(
        names
            .iter()
            .map(|n| (*n).to_string())
            .collect::<HashSet<_>>(),
    )
}

/// Nothing on disk is a directory.
fn files(_: &str) -> bool {
    false
}

/// The rows `ahead` draws over `base` read at `looked`: the status itself
/// where the presses move nothing in it.
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

/// Whether `path`'s row waits for git's answer, as the pane reads it.
fn moving(ahead: &StageAhead, path: &str) -> bool {
    ahead.all_out() || ahead.moving_paths().iter().any(|p| p == path)
}

/// One unstaged edit staged, the queue having taken it.
fn staged_a() -> (StageAhead, WorktreeStatus) {
    let mut ahead = StageAhead::default();
    assert!(ahead.asked(Side::Stage, paths(&["a.txt"]), Some(OURS)));
    (ahead, status(vec![tracked('.', 'M', "a.txt")]))
}

#[test]
fn a_press_moves_its_row_before_git_answers() {
    let (mut ahead, base) = staged_a();
    assert_eq!(
        drawn(&mut ahead, IN_FLIGHT, &base),
        [tracked('M', '.', "a.txt")]
    );
    assert!(moving(&ahead, "a.txt"), "the row waits for git's answer");
    assert!(!moving(&ahead, "b.txt"), "other rows do not");
    assert!(ahead.reaches("unstaged", "a.txt") && !ahead.reaches("unstaged", "b.txt"));
    assert!(
        !ahead.reaches("staged", "a.txt"),
        "staging takes nothing off the staged side"
    );
}

#[test]
fn a_press_the_queue_took_nothing_for_moves_nothing() {
    let mut ahead = StageAhead::default();
    let base = status(vec![tracked('.', 'M', "a.txt")]);
    assert!(!ahead.asked(Side::Stage, paths(&["a.txt"]), None));
    assert!(!ahead.asked(Side::Stage, paths(&["a.txt"]), Some(0)));
    assert!(!ahead.asked(Side::Stage, paths(&[]), Some(OURS)));
    assert_eq!(drawn(&mut ahead, IN_FLIGHT, &base), base.items);
    assert!(!ahead.unseen());
}

#[test]
fn a_refusal_puts_the_row_back_at_once() {
    let (mut ahead, base) = staged_a();
    assert!(ahead.answered(OURS, true, FENCE));
    assert_eq!(drawn(&mut ahead, IN_FLIGHT, &base), base.items);
    assert!(!moving(&ahead, "a.txt"));
    assert!(!ahead.unseen());
}

#[test]
fn another_writes_answer_moves_nothing() {
    let (mut ahead, base) = staged_a();
    assert!(!ahead.answered(SOMEBODY_ELSE, true, FENCE));
    assert!(!ahead.answered(SOMEBODY_ELSE, false, FENCE));
    assert_eq!(
        drawn(&mut ahead, IN_FLIGHT, &base),
        [tracked('M', '.', "a.txt")]
    );
    assert!(moving(&ahead, "a.txt"));
}

/// The status in flight as the write ended still holds the old rows; drawn
/// as it is, it would take the moved row back under the hand.
#[test]
fn a_landing_holds_the_row_until_a_status_past_the_fence() {
    let (mut ahead, base) = staged_a();
    assert!(ahead.answered(OURS, false, FENCE));
    assert!(!moving(&ahead, "a.txt"), "git has answered");
    assert_eq!(
        drawn(&mut ahead, IN_FLIGHT, &base),
        [tracked('M', '.', "a.txt")],
        "a status from before the write's end"
    );
    let unmoved = status(vec![tracked('.', 'M', "a.txt")]);
    assert!(
        ahead.picture(FENCE, &unmoved, &[], files).is_none(),
        "the status at the fence is git's own word, a row the press would move too"
    );
}

/// What the status says once it has seen the write wins — whatever happened
/// to the row in between, here somebody unstaging it again in a terminal. The
/// fence itself is the first stamp that has seen it.
#[test]
fn from_the_fence_on_the_status_is_drawn_as_it_is() {
    let (mut ahead, _) = staged_a();
    ahead.answered(OURS, false, FENCE);
    let terminal = status(vec![tracked('.', 'M', "a.txt")]);
    assert_eq!(drawn(&mut ahead, FENCE, &terminal), terminal.items);
    assert_eq!(drawn(&mut ahead, AFTERWARDS, &terminal), terminal.items);
}

/// Each reader is measured against the status it holds, and the press goes
/// only once all four have drawn past it.
#[test]
fn the_press_is_let_go_once_every_reader_has_drawn_past_it() {
    let (mut ahead, base) = staged_a();
    ahead.answered(OURS, false, FENCE);
    ahead.drawn(Reader::Unstaged, AFTERWARDS);
    ahead.drawn(Reader::Staged, AFTERWARDS);
    ahead.drawn(Reader::Counts, AFTERWARDS);
    assert!(
        ahead.unseen(),
        "the conflicts list still holds the old status"
    );
    assert_eq!(
        drawn(&mut ahead, IN_FLIGHT, &base),
        [tracked('M', '.', "a.txt")],
        "and draws it with the press"
    );
    ahead.drawn(Reader::Conflicts, FENCE);
    assert!(!ahead.unseen());
}

/// Two presses on one row compose in the order they were made — the order the
/// queue runs them in — whichever of them git has answered.
#[test]
fn two_presses_on_one_row_compose_in_order() {
    let (mut ahead, base) = staged_a();
    ahead.asked(Side::Unstage, paths(&["a.txt"]), Some(NEXT));
    assert_eq!(
        drawn(&mut ahead, IN_FLIGHT, &base),
        base.items,
        "back again"
    );
    ahead.answered(OURS, false, FENCE);
    let between = status(vec![tracked('M', '.', "a.txt")]);
    assert_eq!(
        drawn(&mut ahead, AFTERWARDS, &between),
        [tracked('.', 'M', "a.txt")],
        "the status after the stage, with the unstage still out"
    );
    ahead.answered(NEXT, false, AFTERWARDS + 1);
    assert_eq!(
        drawn(&mut ahead, AFTERWARDS, &between),
        [tracked('.', 'M', "a.txt")],
        "the unstage landed, and its status has not come"
    );
}

/// The first press refused, the second still draws over what the status says:
/// unstaging a row that is unstaged moves nothing.
#[test]
fn a_refused_press_under_a_later_one_leaves_the_later_one_standing() {
    let (mut ahead, base) = staged_a();
    ahead.asked(Side::Unstage, paths(&["a.txt"]), Some(NEXT));
    ahead.answered(OURS, true, FENCE);
    assert_eq!(drawn(&mut ahead, IN_FLIGHT, &base), base.items);
    assert!(moving(&ahead, "a.txt"), "the unstage is still out");
    assert!(
        ahead.reaches("staged", "a.txt"),
        "the unstage still reaches it"
    );
}

/// A later press still out is drawn over the status the earlier one's
/// landing brought.
#[test]
fn a_later_press_survives_an_earlier_ones_status() {
    let mut ahead = StageAhead::default();
    ahead.asked(Side::Stage, paths(&["a.txt"]), Some(OURS));
    ahead.asked(Side::Stage, paths(&["b.txt"]), Some(NEXT));
    ahead.answered(OURS, false, FENCE);
    let after_a = status(vec![tracked('M', '.', "a.txt"), tracked('.', 'M', "b.txt")]);
    every_reader_drew(&mut ahead, AFTERWARDS);
    assert_eq!(
        drawn(&mut ahead, AFTERWARDS, &after_a),
        [tracked('M', '.', "a.txt"), tracked('M', '.', "b.txt")]
    );
    assert!(!ahead.reaches("unstaged", "a.txt"), "the first is let go");
    assert!(
        ahead.reaches("unstaged", "b.txt"),
        "the second still stands"
    );
}

#[test]
fn staging_moves_each_kind_of_one_sided_change() {
    let mut ahead = StageAhead::default();
    let base = status(vec![
        tracked('.', 'M', "edit.txt"),
        tracked('.', 'D', "gone.txt"),
        tracked('.', 'T', "link"),
        tracked('.', 'A', "intent.txt"),
        untracked("new.txt"),
    ]);
    ahead.asked(Side::Stage, Reach::All, Some(OURS));
    assert_eq!(
        drawn(&mut ahead, IN_FLIGHT, &base),
        [
            tracked('M', '.', "edit.txt"),
            tracked('D', '.', "gone.txt"),
            tracked('T', '.', "link"),
            tracked('A', '.', "intent.txt"),
            tracked('A', '.', "new.txt"),
        ]
    );
}

#[test]
fn unstaging_moves_each_kind_of_one_sided_change_back() {
    let mut ahead = StageAhead::default();
    let base = status(vec![
        tracked('M', '.', "edit.txt"),
        tracked('D', '.', "gone.txt"),
        tracked('T', '.', "link"),
        tracked('A', '.', "new.txt"),
    ]);
    ahead.asked(Side::Unstage, Reach::All, Some(OURS));
    assert_eq!(
        drawn(&mut ahead, IN_FLIGHT, &base),
        [
            tracked('.', 'M', "edit.txt"),
            tracked('.', 'D', "gone.txt"),
            tracked('.', 'T', "link"),
            untracked("new.txt"),
        ]
    );
}

/// The rows whose next state the row alone cannot say wait for the status
/// (`moved`'s list).
#[test]
fn what_the_row_cannot_decide_waits_for_the_status() {
    let renamed = StatusItem::Tracked {
        staged: 'R',
        unstaged: '.',
        path: "new.txt".to_string(),
        orig_path: Some("old.txt".to_string()),
    };
    let conflicted = StatusItem::Unmerged {
        ours: 'U',
        theirs: 'U',
        path: "both.txt".to_string(),
    };
    let base = status(vec![
        tracked('M', 'M', "split.txt"),
        tracked('A', 'D', "came-and-went.txt"),
        renamed,
        conflicted,
        untracked("nest/"),
        // A staged deletion beside an untracked file of the same name.
        tracked('D', '.', "twice.txt"),
        untracked("twice.txt"),
    ]);
    for side in [Side::Stage, Side::Unstage] {
        let mut ahead = StageAhead::default();
        ahead.asked(side, Reach::All, Some(OURS));
        assert!(
            ahead.picture(IN_FLIGHT, &base, &[], files).is_none(),
            "{side:?} moved a row it cannot decide"
        );
    }
}

/// A tracked directory is a gitlink: `git add` stages its commit and none of
/// its files, so a submodule edited inside stays where it is.
#[test]
fn a_gitlink_waits_for_the_status() {
    let base = status(vec![
        tracked('.', 'M', "vendor/nest"),
        tracked('A', '.', "sub"),
    ]);
    let dir = |path: &str| path == "vendor/nest" || path == "sub";
    for side in [Side::Stage, Side::Unstage] {
        let mut ahead = StageAhead::default();
        ahead.asked(side, paths(&["vendor/nest", "sub"]), Some(OURS));
        assert!(ahead.picture(IN_FLIGHT, &base, &[], dir).is_none());
    }
}

/// A press reaching a gitlink does not count its row as leaving: it may stay
/// where it is, so the open diff waits on it for the status. A file the same
/// press names leaves as ever.
#[test]
fn a_gitlink_is_not_reached_once_the_disk_has_said_so() {
    let base = status(vec![
        tracked('.', 'M', "vendor/nest"),
        tracked('.', 'M', "b.txt"),
    ]);
    let mut ahead = StageAhead::default();
    ahead.asked(Side::Stage, paths(&["vendor/nest", "b.txt"]), Some(OURS));
    ahead.picture(IN_FLIGHT, &base, &[], |path| path == "vendor/nest");
    assert!(!ahead.reaches("unstaged", "vendor/nest"));
    assert!(ahead.reaches("unstaged", "b.txt"));
}

/// A whole bucket's press asks the disk nothing — a bucket can be thousands
/// of rows, and the press's turn is to stay short: a gitlink is drawn moved
/// like a file until the status lands (Commit waits on that status anyway).
#[test]
fn a_whole_bucket_press_asks_the_disk_nothing() {
    let base = status(vec![tracked('.', 'M', "vendor/nest")]);
    let mut ahead = StageAhead::default();
    ahead.asked(Side::Stage, Reach::All, Some(OURS));
    let mut asked = 0;
    let picture = ahead.picture(IN_FLIGHT, &base, &[], |_| {
        asked += 1;
        true
    });
    assert_eq!(asked, 0);
    assert_eq!(
        picture.expect("moved").status.items,
        [tracked('M', '.', "vendor/nest")]
    );
}

/// The disk is asked once a path while presses are out — four readers draw
/// every status — a deleted file's too: its path can be a folder now.
#[test]
fn the_disk_is_asked_once_a_path() {
    let base = status(vec![
        tracked('.', 'M', "a.txt"),
        tracked('.', 'D', "gone.txt"),
    ]);
    let mut ahead = StageAhead::default();
    ahead.asked(Side::Stage, paths(&["a.txt", "gone.txt"]), Some(OURS));
    let mut asked = Vec::new();
    for _ in READERS {
        ahead.picture(IN_FLIGHT, &base, &[], |path| {
            asked.push(path.to_string());
            false
        });
    }
    assert_eq!(asked, ["a.txt", "gone.txt"]);
}

/// A deleted file whose path is a folder now: staging the deletion takes the
/// folder's files with it, which no row says.
#[test]
fn a_deletion_whose_path_is_a_folder_now_waits_for_the_status() {
    let base = status(vec![tracked('.', 'D', "was-a-file")]);
    let mut ahead = StageAhead::default();
    ahead.asked(Side::Stage, paths(&["was-a-file"]), Some(OURS));
    assert!(ahead.picture(IN_FLIGHT, &base, &[], |_| true).is_none());
}

/// A whole bucket's press reaches only what was on that side as it is drawn
/// over: a file edited after its landing is unstaged in the status that
/// follows, and stays there.
#[test]
fn a_whole_bucket_press_past_its_fence_leaves_later_edits_alone() {
    let mut ahead = StageAhead::default();
    ahead.asked(Side::Stage, Reach::All, Some(OURS));
    ahead.answered(OURS, false, FENCE);
    let after = status(vec![
        tracked('M', '.', "a.txt"),
        tracked('.', 'M', "edited-since.txt"),
    ]);
    assert_eq!(drawn(&mut ahead, AFTERWARDS, &after), after.items);
}

/// A press over a row somebody changed since answers for the row as it is:
/// edited on both sides now, it waits for the status.
#[test]
fn a_row_changed_since_the_press_is_drawn_as_it_now_is() {
    let (mut ahead, _) = staged_a();
    let edited = status(vec![tracked('M', 'M', "a.txt")]);
    assert_eq!(drawn(&mut ahead, IN_FLIGHT, &edited), edited.items);
}

#[test]
fn the_marks_go_with_their_rows() {
    let mark = |path: &str, staged: bool| EolMark {
        path: path.to_string(),
        notice: Notice::Flipped {
            from: Eol::Lf,
            to: Eol::Crlf,
        },
        staged,
    };
    let (mut ahead, _) = staged_a();
    let base = status(vec![tracked('.', 'M', "a.txt"), tracked('.', 'M', "b.txt")]);
    let marks = [mark("a.txt", false), mark("b.txt", false)];
    let picture = ahead
        .picture(IN_FLIGHT, &base, &marks, files)
        .expect("a.txt moves");
    assert_eq!(picture.marks, [mark("a.txt", true), mark("b.txt", false)]);
}

#[test]
fn the_rows_waiting_are_the_paths_of_the_presses_still_out() {
    let mut ahead = StageAhead::default();
    ahead.asked(Side::Stage, paths(&["b.txt", "a.txt"]), Some(OURS));
    ahead.asked(Side::Unstage, paths(&["a.txt", "c.txt"]), Some(NEXT));
    assert_eq!(ahead.moving_paths(), ["a.txt", "b.txt", "c.txt"]);
    ahead.answered(OURS, false, FENCE);
    assert_eq!(ahead.moving_paths(), ["a.txt", "c.txt"]);
    ahead.asked(Side::Stage, Reach::All, Some(SOMEBODY_ELSE));
    assert_eq!(
        ahead.moving_paths(),
        ["a.txt", "c.txt"],
        "a whole bucket's press is `all_out`'s to say"
    );
}

#[test]
fn a_whole_bucket_press_holds_every_row_and_the_bucket_buttons() {
    let mut ahead = StageAhead::default();
    ahead.asked(Side::Unstage, Reach::All, Some(OURS));
    assert!(ahead.all_out());
    assert!(moving(&ahead, "anything"));
    assert!(ahead.reaches("staged", "anything"));
    assert!(
        !ahead.reaches("unstaged", "anything"),
        "unstaging takes nothing off the unstaged side"
    );
    ahead.answered(OURS, false, FENCE);
    assert!(!ahead.all_out());
    assert!(!moving(&ahead, "anything"));
    assert!(
        ahead.reaches("staged", "anything"),
        "landed, it reaches every row until its status is drawn"
    );
}

/// The next session's first status is the truth, and its stamps count from
/// the start: nothing drawn or out is carried over.
#[test]
fn a_gone_session_takes_everything_with_it() {
    let (mut ahead, base) = staged_a();
    every_reader_drew(&mut ahead, AFTERWARDS);
    ahead.session_gone();
    assert!(!ahead.unseen());
    assert_eq!(drawn(&mut ahead, 1, &base), base.items);
    ahead.asked(Side::Stage, paths(&["a.txt"]), Some(1));
    ahead.answered(1, false, 2);
    ahead.drawn(Reader::Unstaged, 2);
    assert!(ahead.unseen(), "the old stamps are not the new session's");
}

/// Within a session a reader's statuses only grow, but one of a session gone
/// can land after the next has started counting from the start: the last
/// status drawn is what a press is measured against, not the largest.
#[test]
fn a_reader_counts_from_the_status_it_drew_last() {
    let mut ahead = StageAhead::default();
    ahead.session_gone();
    every_reader_drew(&mut ahead, AFTERWARDS);
    every_reader_drew(&mut ahead, 1);
    ahead.asked(Side::Stage, paths(&["a.txt"]), Some(1));
    ahead.answered(1, false, 2);
    assert!(
        ahead.unseen(),
        "no reader has drawn the new session's status after the write"
    );
}

/// Unstaging a gitlink's staged commit is git's to draw too.
#[test]
fn an_unstaged_gitlink_waits_for_the_status() {
    let mut ahead = StageAhead::default();
    ahead.asked(Side::Unstage, paths(&["vendor/nest"]), Some(OURS));
    let base = status(vec![tracked('M', '.', "vendor/nest")]);
    assert!(ahead.picture(IN_FLIGHT, &base, &[], |_| true).is_none());
}

/// An unstaged row's line-ending mark goes back to the worktree side with it.
#[test]
fn a_mark_goes_back_with_its_row_unstaged() {
    let mark = |staged: bool| EolMark {
        path: "a.txt".to_string(),
        notice: Notice::Flipped {
            from: Eol::Lf,
            to: Eol::Crlf,
        },
        staged,
    };
    let mut ahead = StageAhead::default();
    ahead.asked(Side::Unstage, paths(&["a.txt"]), Some(OURS));
    let base = status(vec![tracked('M', '.', "a.txt")]);
    let picture = ahead
        .picture(IN_FLIGHT, &base, &[mark(true)], files)
        .expect("a.txt moves");
    assert_eq!(picture.marks, [mark(false)]);
}

/// What the disk said is about one run of presses: once none is out, the
/// next press asks again (a folder can be replaced by a file between two).
#[test]
fn the_disk_is_asked_again_after_the_presses_are_over() {
    let base = status(vec![tracked('.', 'M', "a.txt")]);
    let mut asked = 0;
    let mut ahead = StageAhead::default();
    ahead.asked(Side::Stage, paths(&["a.txt"]), Some(OURS));
    ahead.picture(IN_FLIGHT, &base, &[], |_| {
        asked += 1;
        false
    });
    ahead.answered(OURS, true, FENCE);
    ahead.asked(Side::Stage, paths(&["a.txt"]), Some(NEXT));
    ahead.picture(IN_FLIGHT, &base, &[], |_| {
        asked += 1;
        false
    });
    assert_eq!(asked, 2);
}

/// Every reader has drawn past the write's end before its answer comes (the
/// statuses won the race): the press goes with the answer.
#[test]
fn a_landing_every_reader_has_drawn_past_lets_the_press_go_at_once() {
    let (mut ahead, _) = staged_a();
    every_reader_drew(&mut ahead, AFTERWARDS);
    assert!(ahead.answered(OURS, false, FENCE));
    assert!(!ahead.unseen());
}

/// A press stands the screen has not caught up with from the press until
/// every reader has drawn the status after it — Commit waits all that time,
/// landed or not — and the tab is told when a reader's drawing lets it go.
#[test]
fn a_press_is_unseen_until_every_reader_has_drawn_past_it() {
    let (mut ahead, base) = staged_a();
    drawn(&mut ahead, IN_FLIGHT, &base);
    assert!(ahead.unseen(), "out at the queue");
    ahead.answered(OURS, false, FENCE);
    assert!(ahead.unseen(), "landed, its status not yet drawn");
    assert!(!ahead.take_notice(), "an answer is the tab's own news");
    ahead.drawn(Reader::Conflicts, AFTERWARDS);
    ahead.drawn(Reader::Unstaged, AFTERWARDS);
    ahead.drawn(Reader::Staged, AFTERWARDS);
    assert!(ahead.unseen(), "the counts still hold the old status");
    assert!(!ahead.take_notice());
    ahead.drawn(Reader::Counts, AFTERWARDS);
    assert!(!ahead.unseen());
    assert!(ahead.take_notice(), "let go while a reader drew");
    assert!(!ahead.take_notice(), "told once");
}

/// A refused press goes with its answer: nothing is left to wait on.
#[test]
fn a_refused_press_is_seen_at_once() {
    let (mut ahead, _) = staged_a();
    ahead.answered(OURS, true, FENCE);
    assert!(!ahead.unseen());
}

/// The diff landed on a file's new side reads there once git has answered
/// the press that took the file off its old one — not before (the side as it
/// was), and not only once the status after it is drawn: still reached then,
/// the row is no longer out.
#[test]
fn a_press_taking_a_row_off_its_side_is_out_until_its_answer() {
    let (mut ahead, _) = staged_a();
    assert!(ahead.leaving_out("unstaged", "a.txt"));
    assert!(
        !ahead.leaving_out("staged", "a.txt"),
        "a stage takes nothing off the staged side"
    );
    assert!(!ahead.leaving_out("unstaged", "b.txt"));
    ahead.answered(OURS, false, FENCE);
    assert!(!ahead.leaving_out("unstaged", "a.txt"));
    assert!(
        ahead.reaches("unstaged", "a.txt"),
        "its status not yet drawn"
    );
}
