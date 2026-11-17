//! A bucket list draws the status it holds with the stage presses still out
//! over it (`ops::StageAhead`, the hub's): the rows a press moves at once,
//! drawn again as the presses move (`restage_files`), and the reading the
//! list draws written down for a press to be let go on (`take_status_feed`).
//! A hub is installed on the test's thread with one tab and no session — no
//! disk to ask whether a path is a folder, so the presses here are whole
//! buckets', which ask none (`ops::stage_ahead::picture_of`).

use std::sync::Arc;

use super::super::testkit::*;
use super::*;
use crate::ops::{Reach, Reader, Side};

const STAGE_ALL: u64 = 7;
/// Status stamps either side of the write's end.
const FENCE: u64 = 41;
const AFTERWARDS: u64 = 42;

/// A file changed on one side, one on both, one untracked.
fn pending() -> platitude_core::status::WorktreeStatus {
    platitude_core::status::WorktreeStatus {
        items: vec![
            tracked('.', 'M', "b.txt"),
            tracked('M', 'M', "c.txt"),
            platitude_core::status::StatusItem::Untracked {
                path: "u.txt".to_string(),
            },
        ],
        ..Default::default()
    }
}

/// The same after `git add --all`.
fn all_staged() -> platitude_core::status::WorktreeStatus {
    platitude_core::status::WorktreeStatus {
        items: vec![
            tracked('M', '.', "b.txt"),
            tracked('M', '.', "c.txt"),
            tracked('A', '.', "u.txt"),
        ],
        ..Default::default()
    }
}

fn unstaged_list(tab: i32) -> NavSectionModel {
    let mut model = worktree(
        "unstaged",
        Source::files(platitude_core::status::WorktreeStatus::default()),
    );
    model.tab_id = tab;
    model
}

fn status_msg(
    status: platitude_core::status::WorktreeStatus,
    looked: u64,
) -> crate::hub::StatusMsg {
    crate::hub::StatusMsg {
        status: status.into(),
        head_seq: 1,
        looked,
        op_state: platitude_core::opstate::OpState::default(),
        progress: None,
        sides: platitude_core::conflict::Sides::default(),
        op_message: String::new(),
        merge_tool: String::new(),
        push_remote: String::new(),
        upstream_here: false,
        push_track: platitude_core::remote::PushTrack::default(),
        eol_marks: Arc::new(Vec::new()),
        lfs_needed: 0,
        not_copied: Arc::new(Vec::new()),
        stop: platitude_core::integrate::RebaseStop::default(),
    }
}

/// The press moves the rows only the row decides on the list's next
/// drawing, a file changed on both sides waiting for the status; refused,
/// the press goes and the rows come back on the drawing after.
#[test]
fn a_list_draws_a_bucket_press_over_its_status_and_again_as_it_goes() {
    let tab = crate::hub::testkit::installed_tab();
    let mut unstaged = unstaged_list(tab);
    unstaged.draw_status(pending().into(), Arc::new(Vec::new()));
    assert!(unstaged.holds("unstaged", "b.txt"));

    assert!(crate::hub::stage_ahead(tab, |ahead| {
        ahead.asked(Side::Stage, Reach::All, Some(STAGE_ALL))
    }));
    assert!(unstaged.restage_files(), "the press moved rows");
    assert!(!unstaged.holds("unstaged", "b.txt"));
    assert!(unstaged.holds("staged", "b.txt"));
    assert!(
        unstaged.holds("staged", "u.txt"),
        "an untracked file is added"
    );
    assert!(
        unstaged.holds("unstaged", "c.txt"),
        "a file changed on both sides waits for the status"
    );

    assert!(crate::hub::stage_ahead(tab, |ahead| {
        ahead.answered(STAGE_ALL, true, 0)
    }));
    assert!(unstaged.restage_files(), "refused, the rows come back");
    assert!(unstaged.holds("unstaged", "b.txt"));
    assert!(!unstaged.holds("staged", "b.txt"));
}

/// A landed press is let go once every reader has drawn a status past the
/// write — this list's drawing is one of them, written down as it takes
/// the status off its feed. That status is drawn as git reported it.
#[test]
fn the_reading_a_list_takes_is_what_lets_a_landed_press_go() {
    let tab = crate::hub::testkit::installed_tab();
    let mut unstaged = unstaged_list(tab);
    let feed = Arc::new(crate::hub::Feed::default());
    unstaged.status_feed = Some(Arc::clone(&feed));
    feed.push(status_msg(pending(), FENCE - 1));
    assert!(unstaged.take_status_feed());

    crate::hub::stage_ahead(tab, |ahead| {
        ahead.asked(Side::Stage, Reach::All, Some(STAGE_ALL))
    });
    crate::hub::stage_ahead(tab, |ahead| ahead.answered(STAGE_ALL, false, FENCE));
    for reader in [Reader::Conflicts, Reader::Staged, Reader::Counts] {
        crate::hub::status_drawn(tab, reader, AFTERWARDS);
    }
    assert!(
        crate::hub::stage_ahead(tab, |ahead| ahead.unseen()),
        "this list has not drawn the status after the write"
    );

    feed.push(status_msg(all_staged(), AFTERWARDS));
    assert!(unstaged.take_status_feed());
    assert!(
        !crate::hub::stage_ahead(tab, |ahead| ahead.unseen()),
        "let go on this list's drawing"
    );
    assert!(
        unstaged.holds("staged", "c.txt"),
        "git's own word on the rows"
    );
    assert!(!unstaged.holds("unstaged", "c.txt"));
}
