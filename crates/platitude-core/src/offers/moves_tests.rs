//! The rules a press on a ref answers to: where it lands, what stands in
//! its way, and what leaving that costs ([`super::moves`]).

use super::*;
use crate::integrate::InProgress;
use crate::opstate::OpState;
use crate::status::Counts;

#[test]
fn switch_lands_where_the_letter_and_the_local_branch_say() {
    use SwitchAction::*;
    assert_eq!(switch_action("L", "main", "main", "", ""), None);
    assert_eq!(switch_action("L", "feat", "main", "", ""), Switch);
    assert_eq!(
        switch_action("L", "feat", "main", "C:/work/other", ""),
        OpenHolder
    );
    assert_eq!(switch_action("R", "feat", "main", "", ""), Materialize);
    assert_eq!(switch_action("R", "feat", "main", "", "abc123"), MoveBranch);
    assert_eq!(
        switch_action("R", "feat", "main", "C:/work/other", "abc123"),
        OpenHolder
    );
    // Landing the current branch on its remote ref is the one move that
    // is not a no-op for `R`: git answers what it would cost.
    assert_eq!(switch_action("R", "main", "main", "", "abc123"), MoveBranch);
    // A tag could only detach HEAD, and the detached marker names no
    // branch: neither moves.
    assert_eq!(switch_action("T", "v1.0", "main", "", ""), None);
    assert_eq!(switch_action("H", "", "main", "", ""), None);
    assert_eq!(
        switch_action("", "", "", "", ""),
        None,
        "detached on nothing"
    );
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
    assert!(skip_is_free(&Counts::default()));
    let untracked = Counts {
        untracked: 4,
        ..Counts::default()
    };
    assert!(
        skip_is_free(&untracked),
        "untracked files say nothing about the stopped commit"
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
        assert!(!skip_is_free(&dirty), "{dirty:?}");
    }
}
