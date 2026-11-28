//! The branch a stopped rebase or bisect keeps while HEAD is on none:
//! the rows that move it stand, greyed for the operation off HEAD's own
//! reading, its own row's deletes and `worktree add` grey for the
//! operation that keeps it, and the worktree the tab shows is brought into
//! nothing.

use super::testkit::*;
use crate::menu::{Row, RowWorktree, Subject, Why};
use crate::offers::{Leads, Tracks, TreeStanding};
use crate::worktrees::Keeper;

/// A stopped rebase or bisect keeps HEAD detached and keeps the branch it
/// moves once finished: the rows that move that branch stand, greyed for
/// the operation, on the graph and in the left pane — and the kept
/// branch's own row brings nothing into itself.
#[test]
fn a_stopped_operation_keeps_its_branchs_rows_greyed() {
    let rebasing = TreeStanding {
        head_oid: HEAD.into(),
        ..TreeStanding::detached()
            .standing("REBASE 1/3")
            .keeping("main", Keeper::Rebase)
    };
    let graph = said(&on_graph(row(None, "", &rebasing)));
    for row in [
        Row::CherryPick,
        Row::Revert,
        Row::Squash,
        Row::Drop,
        Row::RebaseInteractive,
        Row::Merge,
        Row::Rebase,
        Row::ResetSoft,
        Row::ResetMixed,
        Row::ResetHard,
    ] {
        assert_eq!(at(&graph, row), greyed(Why::StoppedOperation), "{row:?}");
    }
    let feat = said(&row(BRANCH, "feat", &rebasing));
    assert_eq!(at(&feat, Row::Merge), greyed(Why::StoppedOperation));
    assert_eq!(at(&feat, Row::Rebase), greyed(Why::StoppedOperation));
    let kept = said(&row(BRANCH, "main", &rebasing));
    assert_eq!(at(&kept, Row::Merge), None, "nothing brought into itself");
    assert_eq!(at(&kept, Row::Reach), LIVE, "HEAD is not on it");
    let begun_detached = TreeStanding::detached().standing("REBASE 1/3");
    let feat = said(&row(BRANCH, "feat", &begun_detached));
    assert_eq!(
        at(&feat, Row::Merge),
        None,
        "a rebase begun detached keeps none"
    );
}

/// HEAD's reading lands ahead of the badge's status: the rows a kept
/// branch makes grey for the operation off that reading alone, as with the
/// badge in — mid-rebase git would write a merge on the detached HEAD.
#[test]
fn a_kept_branch_greys_its_rows_before_the_badge_lands() {
    let unbadged = TreeStanding {
        head_oid: HEAD.into(),
        ..TreeStanding::detached().keeping("main", Keeper::Rebase)
    };
    assert!(!unbadged.op_standing(), "the badge is not in");
    let graph = said(&on_graph(row(None, "", &unbadged)));
    for row in [
        Row::BranchHere,
        Row::CherryPick,
        Row::Revert,
        Row::Squash,
        Row::Drop,
        Row::RebaseInteractive,
        Row::Merge,
        Row::Rebase,
        Row::ResetSoft,
        Row::ResetMixed,
        Row::ResetHard,
        Row::TagHere,
    ] {
        assert_eq!(at(&graph, row), greyed(Why::StoppedOperation), "{row:?}");
    }
    let feat = said(&Subject {
        far: tracking(Tracks::Together, 0),
        ..row(BRANCH, "feat", &unbadged)
    });
    for row in [Row::Merge, Row::Rebase, Row::Pull] {
        assert_eq!(at(&feat, row), greyed(Why::StoppedOperation), "{row:?}");
    }
}

/// The kept branch's pull stands as on the branch HEAD is on, greyed for
/// the operation, whatever it tracks — a branch here too, which only
/// `git pull` takes in, on the branch HEAD is on once the operation is
/// done.
#[test]
fn a_kept_branch_keeps_its_pull_greyed_whatever_it_tracks() {
    let kept = TreeStanding::detached()
        .standing("REBASE 1/3")
        .keeping("main", Keeper::Rebase);
    for tracks in [Tracks::Together, Tracks::Here] {
        let menu = said(&Subject {
            far: tracking(tracks, 0),
            ..row(BRANCH, "main", &kept)
        });
        assert_eq!(
            at(&menu, Row::Pull),
            greyed(Why::StoppedOperation),
            "{tracks:?}"
        );
    }
}

/// The branch a stopped rebase or bisect keeps is out here as git counts
/// it: its deletes and `worktree add` grey, saying which operation keeps
/// it; another branch's delete stays live.
#[test]
fn a_kept_branch_is_neither_deleted_nor_taken_out_again() {
    for (by, badge) in [
        (Keeper::Rebase, "REBASE 1/3"),
        (Keeper::Bisect, "BISECTING"),
    ] {
        let kept = TreeStanding::detached().standing(badge).keeping("main", by);
        let named = |name| {
            said(&Subject {
                far: tracking(Tracks::Together, 0),
                ..row(BRANCH, name, &kept)
            })
        };
        let main = named("main");
        for row in [Row::DeleteBranch, Row::DeleteBoth, Row::WorktreeAdd] {
            assert_eq!(at(&main, row), greyed(Why::KeptBy(by)), "{row:?} {by:?}");
        }
        assert_eq!(at(&named("feat"), Row::DeleteBranch), LIVE, "{by:?}");
    }
}

/// The worktree the tab shows is never brought into the branch it moves:
/// listed on no branch while a stopped operation keeps one, its row has no
/// merge or rebase, as another worktree's has.
#[test]
fn the_worktree_the_tab_shows_is_not_brought_into_its_branch() {
    let kept = TreeStanding::detached()
        .standing("REBASE 1/3")
        .keeping("main", Keeper::Rebase);
    let worktree = |here: bool| {
        said(&Subject {
            worktree: Some(RowWorktree {
                main: here,
                here,
                locked: false,
            }),
            leads: if here { Leads::Nowhere } else { Leads::Itself },
            ..row(WORKTREE, if here { "/w/repo" } else { "/w/side" }, &kept)
        })
    };
    let shown = worktree(true);
    assert_eq!(at(&shown, Row::Merge), None);
    assert_eq!(at(&shown, Row::Rebase), None);
    let other = worktree(false);
    assert_eq!(at(&other, Row::Merge), greyed(Why::StoppedOperation));
    assert_eq!(at(&other, Row::Rebase), greyed(Why::StoppedOperation));
}
