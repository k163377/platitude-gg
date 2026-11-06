//! The rows a commit row's own menu offers — apart from the ref menu's
//! [`super::tests`], as the two share nothing (structure.md §分割).

use super::*;

/// The commit-menu inputs of the same idle repository, `oid` under the
/// pointer.
fn commit_offers(oid: &str, head: &str) -> CommitMenuOffers {
    commit_menu(true, "main", false, "", oid, head, "")
}

#[test]
fn an_ordinary_commit_away_from_head_offers_the_whole_menu() {
    let offers = commit_offers("abc123", "def456");
    assert_eq!(
        offers,
        CommitMenuOffers {
            cherry_pick: true,
            revert: true,
            integrate: true,
            edit_history: true,
            move_branch: true,
            branch_here: true,
            standing: false,
            stash_write: false,
        }
    );
    assert_eq!(
        offers.words(),
        [
            "cherry-pick",
            "revert",
            "integrate",
            "edit-history",
            "move-branch",
            "branch-here"
        ]
    );
}

#[test]
fn a_commit_the_discard_log_draws_offers_only_what_adds() {
    let offers = commit_offers("abc123", "def456").on_provisional();
    assert_eq!(offers.words(), ["cherry-pick", "branch-here"]);
    // HEAD's own commit drawn for an entry that took a name off it: the rows
    // that would rewrite it are as out as anywhere.
    assert_eq!(
        commit_offers("abc123", "abc123").on_provisional().words(),
        ["cherry-pick", "branch-here"]
    );
    assert_eq!(
        commit_menu(true, "main", false, "MERGING", "abc123", "def456", "")
            .on_provisional()
            .words(),
        ["cherry-pick", "branch-here", "standing"],
        "what stays greys under a standing operation as anywhere"
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
    let offers = commit_menu(true, "", true, "", "abc123", "def456", "");
    assert!(
        offers.cherry_pick && offers.revert,
        "cherry-pick and revert run detached"
    );
    assert!(offers.branch_here, "the way back out of detached");
    assert!(!offers.integrate);
    assert!(!offers.edit_history);
    assert!(!offers.move_branch);
}

#[test]
fn a_standing_operation_greys_the_whole_menu_in_its_seats() {
    let offers = commit_menu(true, "main", false, "MERGING", "abc123", "def456", "");
    assert_eq!(
        offers,
        CommitMenuOffers {
            standing: true,
            ..commit_offers("abc123", "def456")
        },
        "every row the idle menu has, kept"
    );
    assert_eq!(
        offers.words(),
        [
            "cherry-pick",
            "revert",
            "integrate",
            "edit-history",
            "move-branch",
            "branch-here",
            "standing"
        ]
    );
    let rebasing = commit_menu(true, "", true, "REBASE 1/3", "abc123", "def456", "");
    assert_eq!(
        rebasing,
        CommitMenuOffers {
            standing: true,
            ..commit_menu(true, "", true, "", "abc123", "def456", "")
        },
        "a stopped rebase leaves the tree detached: the rows that need a branch have none to name"
    );
}

#[test]
fn a_stash_row_offers_only_its_own_writes() {
    let offers = commit_menu(true, "main", false, "", "abc123", "def456", "stash@{1}");
    assert_eq!(
        offers,
        CommitMenuOffers {
            stash_write: true,
            ..CommitMenuOffers::default()
        }
    );
    // A stash write waits only on the queue — an operation standing does
    // not gate it.
    assert!(commit_menu(true, "main", false, "REBASING", "abc123", "", "stash@{1}").stash_write);
}
