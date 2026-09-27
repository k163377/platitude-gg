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
