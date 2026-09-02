//! The rules a commit row answers to: the rows its own menu offers.
//!
//! Beside [`super::tests`] rather than in it: the ref menu's rows and
//! these share nothing but the module they are declared from
//! (structure.md §分割).

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
