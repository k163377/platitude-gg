//! The rules a press on a ref answers to: where it lands, what stands in
//! its way, and what leaving that costs ([`super::super::moves`]).

use super::super::*;
use crate::integrate::InProgress;
use crate::opstate::OpState;
use crate::status::Counts;

#[test]
fn switch_lands_where_the_kind_and_the_local_branch_say() {
    use SwitchAction::*;
    let main = TreeStanding::on("main");
    // The press as the page asks it: the chip's word, the local branch,
    // whether another worktree holds that branch, and whether it is
    // listed.
    let lands = |kind: &str, local: &str, held: bool, listed: bool| {
        switch_action(SwitchTarget::from_words(kind, local), &main, held, listed)
    };
    assert_eq!(lands("branch", "main", false, true), None);
    assert_eq!(lands("branch", "feat", false, true), Switch);
    assert_eq!(lands("branch", "feat", true, true), OpenHolder);
    assert_eq!(lands("remote", "feat", false, false), Materialize);
    assert_eq!(lands("remote", "feat", false, true), MoveBranch);
    assert_eq!(lands("remote", "feat", true, true), OpenHolder);
    // Landing the current branch on its remote ref is the one move that
    // is not a no-op for a remote: git answers what it would cost.
    assert_eq!(lands("remote", "main", false, true), MoveBranch);
    // A tag could only detach HEAD, and the two markers name no branch.
    assert_eq!(lands("tag", "v1.0", false, false), None);
    assert_eq!(lands("head", "", false, false), None);
    assert_eq!(lands("worktree", "rig", false, false), None);
    assert_eq!(
        switch_action(
            SwitchTarget::from_words("", ""),
            &TreeStanding::detached(),
            false,
            false
        ),
        None,
        "detached on nothing"
    );
    // A press that names no branch lands on none, whichever row it was on.
    assert_eq!(lands("branch", "", false, false), None);
    assert_eq!(lands("remote", "", false, false), None);
}

/// The checkout each move sends: a `switch` onto the row's own branch, or
/// the remote branch's local one made off the remote row — the row's name
/// as the ref, the landing as the branch made.
#[test]
fn a_move_sends_the_checkout_its_kind_says() {
    use crate::branch::CheckoutTarget;
    use SwitchAction::*;
    assert_eq!(
        Switch.checkout("feat", "feat"),
        Some(CheckoutTarget::Branch {
            name: "feat".into()
        })
    );
    assert_eq!(
        Materialize.checkout("origin/feat", "feat"),
        Some(CheckoutTarget::Track {
            remote_ref: "origin/feat".into(),
            local: "feat".into(),
        })
    );
    for sends_none in [None, OpenHolder, MoveBranch] {
        assert_eq!(
            sends_none.checkout("origin/feat", "feat"),
            Option::None,
            "{sends_none:?}"
        );
    }
}

#[test]
fn a_move_has_landed_only_once_head_and_the_listing_both_say_so() {
    let feat = TreeStanding::on("feat");
    let main = TreeStanding::on("main");
    // The gap the guard exists for: status has HEAD on the new branch,
    // and the listing has no commit for it yet.
    assert!(!move_landed("feat", &feat, false));
    assert!(!move_landed("feat", &main, true));
    assert!(move_landed("feat", &feat, true));
    assert!(!move_landed("", &TreeStanding::detached(), false));
    assert!(!move_landed("", &main, true));
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
