//! The rules a press on a ref answers to: where it lands, what stands in
//! its way, and what leaving that costs ([`super::moves`]).

use super::*;
use crate::integrate::InProgress;
use crate::opstate::OpState;
use crate::status::Counts;

#[test]
fn switch_lands_where_the_kind_and_the_local_branch_say() {
    use SwitchAction::*;
    assert_eq!(switch_action("branch", "main", "main", "", ""), None);
    assert_eq!(switch_action("branch", "feat", "main", "", ""), Switch);
    assert_eq!(
        switch_action("branch", "feat", "main", "C:/work/other", ""),
        OpenHolder
    );
    assert_eq!(switch_action("remote", "feat", "main", "", ""), Materialize);
    assert_eq!(
        switch_action("remote", "feat", "main", "", "abc123"),
        MoveBranch
    );
    assert_eq!(
        switch_action("remote", "feat", "main", "C:/work/other", "abc123"),
        OpenHolder
    );
    // Landing the current branch on its remote ref is the one move that
    // is not a no-op for a remote: git answers what it would cost.
    assert_eq!(
        switch_action("remote", "main", "main", "", "abc123"),
        MoveBranch
    );
    // A tag could only detach HEAD, and the two markers name no branch:
    // none of them moves.
    assert_eq!(switch_action("tag", "v1.0", "main", "", ""), None);
    assert_eq!(switch_action("head", "", "main", "", ""), None);
    assert_eq!(switch_action("worktree", "rig", "main", "", ""), None);
    assert_eq!(
        switch_action("", "", "", "", ""),
        None,
        "detached on nothing"
    );
}

#[test]
fn a_move_has_landed_only_once_head_and_the_listing_both_say_so() {
    // The gap the guard exists for: the write has answered and the
    // status behind it already has HEAD on the new branch, but the refs
    // are still being read, so the listing has no commit for it yet.
    assert!(!move_landed("feat", "feat", ""));
    assert!(!move_landed("feat", "main", "abc123"));
    assert!(move_landed("feat", "feat", "abc123"));
    // Nothing is on its way, so nothing is waiting to arrive.
    assert!(!move_landed("", "", ""));
    assert!(!move_landed("", "main", "abc123"));
}

#[test]
fn every_standing_operation_and_an_unmerged_index_block_moves() {
    let clean = Counts::default();
    let conflicted = Counts {
        conflicted: 1,
        ..Counts::default()
    };
    assert!(!moves_blocked(&OpState::default(), &clean));
    assert!(moves_blocked(&OpState::default(), &conflicted));
    for ops in [
        OpState {
            rebasing: true,
            ..OpState::default()
        },
        OpState {
            merging: true,
            ..OpState::default()
        },
        OpState {
            cherry_picking: true,
            ..OpState::default()
        },
        OpState {
            reverting: true,
            ..OpState::default()
        },
        OpState {
            bisecting: true,
            ..OpState::default()
        },
    ] {
        assert!(moves_blocked(&ops, &clean), "{ops:?}");
    }
}

#[test]
fn only_a_rebase_costs_something_to_leave() {
    assert!(leaving_undoes(Some(InProgress::Rebase)));
    assert_eq!(leave_code(Some(InProgress::Rebase)), "rebase --abort");
    for op in [
        Some(InProgress::Merge),
        Some(InProgress::CherryPick),
        Some(InProgress::Revert),
        None,
    ] {
        assert!(!leaving_undoes(op), "{op:?}");
        assert_eq!(leave_code(op), "stash", "{op:?}");
    }
}

#[test]
fn a_clean_tree_under_a_stop_is_the_emptied_commit_and_skip_is_free() {
    assert!(skip_is_free(&Counts::default(), false));
    let untracked = Counts {
        untracked: 4,
        ..Counts::default()
    };
    assert!(
        skip_is_free(&untracked, false),
        "untracked files say nothing about the stopped commit"
    );
    assert!(
        !skip_is_free(&Counts::default(), true),
        "an edit stop is just as clean, and skipping it takes the commit out"
    );
    for dirty in [
        Counts {
            conflicted: 1,
            ..Counts::default()
        },
        Counts {
            staged: 1,
            ..Counts::default()
        },
        Counts {
            unstaged: 1,
            ..Counts::default()
        },
    ] {
        assert!(!skip_is_free(&dirty, false), "{dirty:?}");
    }
}
