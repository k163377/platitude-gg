use super::*;

/// A working copy's row on an idle repository on `main`: `holder` is the
/// copy's own path where it is another copy, empty where the tab stands in
/// it.
fn copy_row(holder: &str) -> RefMenuOffers {
    ref_menu(
        RefKind::Worktree,
        "C:/work/topic",
        "abc123",
        true,
        0,
        "main",
        false,
        "",
        0,
        holder,
        "",
        false,
        "origin",
        "",
        "",
    )
}

#[test]
fn another_copy_leads_to_itself_and_offers_what_its_commit_does() {
    let offers = copy_row("C:/work/topic");
    assert!(offers.switch_to, "the row opens that copy");
    assert!(!offers.switch_asks, "opening another copy asks nothing");
    assert!(offers.branch_here);
    assert!(offers.integrate_from);
    assert!(!offers.pull);
    assert!(!offers.delete, "its delete is the WORKTREE card's");
    assert!(!offers.set_upstream);
    assert!(!offers.push_tag);
}

#[test]
fn the_copy_the_tab_stands_in_leads_nowhere() {
    assert!(!copy_row("").switch_to);
}

#[test]
fn the_repositorys_own_copy_has_nothing_to_remove() {
    assert_eq!(
        worktree_card(true, false, false, 0),
        WorktreeCardOffers::default()
    );
    assert!(worktree_card(true, false, true, 0).words().is_empty());
}

#[test]
fn a_copy_nothing_holds_is_removed_as_it_stands() {
    let card = worktree_card(false, false, false, 0);
    assert_eq!(card.words(), vec!["remove"]);
}

/// Every reason keeps the row; the lock goes first, then the tab's own
/// copy, then a running write.
#[test]
fn what_stands_in_the_way_greys_the_row_and_says_which() {
    assert_eq!(
        worktree_card(false, true, true, 1).words(),
        vec!["remove", "out-locked"]
    );
    assert_eq!(
        worktree_card(false, false, true, 1).words(),
        vec!["remove", "out-here"]
    );
    assert_eq!(
        worktree_card(false, false, false, 1).words(),
        vec!["remove", "out-busy"]
    );
}

#[test]
fn the_kind_word_is_the_chips() {
    assert_eq!(RefKind::from_word("worktree"), Some(RefKind::Worktree));
}

/// The two rows that make a copy, asked of an idle repository on `main`
/// with no copy holding anything.
fn making(kind: Option<RefKind>, full: &str) -> CopyOffers {
    copy_rows(kind, full, "abc123", true, 0, "main", "", false)
}

#[test]
fn a_free_branch_goes_out_as_it_is_and_any_commit_takes_a_new_one() {
    let offers = making(Some(RefKind::Branch), "feature/login");
    assert_eq!(offers.words(), vec!["here", "checkout-branch"]);
    assert_eq!(making(None, "").words(), vec!["here"], "a commit row");
    assert_eq!(making(Some(RefKind::Tag), "v1").words(), vec!["here"]);
    assert_eq!(
        making(Some(RefKind::Worktree), "C:/work/detached").words(),
        vec!["here"],
        "a branchless copy has no branch to take out"
    );
}

/// git keeps a branch to one copy: the tree's own and another's offer
/// only a new branch.
#[test]
fn a_branch_already_out_offers_only_a_new_one() {
    assert_eq!(making(Some(RefKind::Branch), "main").words(), vec!["here"]);
    let held = copy_rows(
        Some(RefKind::Branch),
        "feature/topic-a",
        "abc123",
        true,
        0,
        "main",
        "C:/work/topic",
        false,
    );
    assert_eq!(held.words(), vec!["here"]);
}

/// On a detached HEAD every local branch is free.
#[test]
fn a_detached_tree_holds_no_branch() {
    let offers = copy_rows(
        Some(RefKind::Branch),
        "main",
        "abc123",
        true,
        0,
        "",
        "",
        false,
    );
    assert_eq!(offers.checkout, Some(CopyCheckout::Branch));
}

/// A remote branch makes its local one — unless that is there already,
/// when the local branch's own row is the way.
#[test]
fn a_remote_branch_makes_its_local_one_only_where_there_is_none() {
    assert_eq!(
        making(Some(RefKind::Remote), "origin/feature/login").words(),
        vec!["here", "checkout-track"]
    );
    let local_there = copy_rows(
        Some(RefKind::Remote),
        "origin/feature/login",
        "abc123",
        true,
        0,
        "main",
        "",
        true,
    );
    assert_eq!(local_there.words(), vec!["here"]);
}

/// Not even a stopped operation keeps a new copy from being made: nothing
/// here is touched. A write out, a stash, a closed tab and a row with no
/// commit do.
#[test]
fn only_a_write_out_or_no_commit_keeps_a_copy_from_being_made() {
    assert!(
        copy_rows(None, "", "abc123", true, 1, "main", "", false)
            .words()
            .is_empty()
    );
    assert!(
        copy_rows(None, "", "abc123", false, 0, "main", "", false)
            .words()
            .is_empty()
    );
    assert!(
        copy_rows(None, "", "", true, 0, "main", "", false)
            .words()
            .is_empty()
    );
    assert!(
        copy_rows(
            Some(RefKind::Stash),
            "stash@{0}",
            "abc123",
            true,
            0,
            "main",
            "",
            false
        )
        .words()
        .is_empty()
    );
}
