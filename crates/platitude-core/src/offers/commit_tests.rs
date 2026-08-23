//! The rules a commit row answers to: its own menu, the move a press on
//! a ref adds up to, and what leaving a stopped operation costs.
//!
//! Beside [`super::tests`] rather than in it: the ref menu's rows and
//! these share nothing but the module they are declared from, and one
//! file of every rule this module holds outgrew the ceiling.

use super::*;

/// The commit-menu inputs of the same idle repository, `oid` under the
/// pointer.
fn commit_offers(oid: &str, head: &str) -> CommitMenuOffers {
    commit_menu(true, 0, "main", false, "", oid, head, "")
}

#[test]
fn an_ordinary_commit_away_from_head_offers_the_whole_menu() {
    let offers = commit_offers("abc123", "def456");
    assert_eq!(
        offers,
        CommitMenuOffers {
            sequence: true,
            integrate: true,
            edit_history: true,
            move_branch: true,
            branch_here: true,
            stash_write: false,
        }
    );
    assert_eq!(
        offers.words(),
        "sequence integrate edit-history move-branch branch-here"
    );
}

#[test]
fn the_commit_head_stands_on_still_rewrites_but_integrates_nothing() {
    let offers = commit_offers("abc123", "abc123");
    assert!(offers.edit_history, "the newest commit is fair game");
    assert!(!offers.integrate);
    assert!(!offers.move_branch);
    assert!(offers.branch_here);
}

#[test]
fn detached_keeps_only_the_rows_that_ask_nothing_of_a_branch() {
    let offers = commit_menu(true, 0, "", true, "", "abc123", "def456", "");
    assert!(offers.sequence, "cherry-pick and revert run detached");
    assert!(offers.branch_here, "the way back out of detached");
    assert!(!offers.integrate);
    assert!(!offers.edit_history);
    assert!(!offers.move_branch);
}

#[test]
fn a_standing_operation_closes_the_commit_menu_down() {
    let offers = commit_menu(true, 0, "main", false, "MERGING", "abc123", "def456", "");
    assert_eq!(offers, CommitMenuOffers::default());
}

#[test]
fn a_stash_row_offers_only_its_own_writes() {
    let offers = commit_menu(true, 0, "main", false, "", "abc123", "def456", "stash@{1}");
    assert_eq!(
        offers,
        CommitMenuOffers {
            stash_write: true,
            ..CommitMenuOffers::default()
        }
    );
    // A stash write waits only on the queue — an operation standing does
    // not gate it.
    assert!(
        commit_menu(
            true,
            0,
            "main",
            false,
            "REBASING",
            "abc123",
            "",
            "stash@{1}"
        )
        .stash_write
    );
    assert!(!commit_menu(true, 1, "main", false, "", "abc123", "", "stash@{1}").stash_write);
}

#[test]
fn a_running_command_closes_the_commit_menu_down() {
    let offers = commit_menu(true, 2, "main", false, "", "abc123", "def456", "");
    assert_eq!(offers, CommitMenuOffers::default());
}

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
