use super::super::testkit::{standing, target};
use super::super::*;

/// A worktree's row on an idle repository on `main`, leading to itself or,
/// where the tab stands in it, nowhere.
fn worktree_row(leads: Leads) -> RefMenuOffers {
    let main = TreeStanding::on("main");
    let row = RowStanding {
        leads,
        ..standing(Some(target(RefKind::Worktree, "C:/work/topic")), &main)
    };
    ref_menu(&row, FarSide::Unread)
}

#[test]
fn another_worktree_leads_to_itself_and_offers_what_its_commit_does() {
    let offers = worktree_row(Leads::Itself);
    assert!(offers.switch_to, "the row opens that worktree");
    assert!(!offers.switch_asks, "opening another worktree asks nothing");
    assert!(offers.branch_here);
    assert!(offers.integrate_from);
    assert!(!offers.delete, "its delete is the WORKTREE card's");
    assert!(!offers.set_upstream);
    assert!(!offers.push_tag);
}

#[test]
fn the_worktree_the_tab_stands_in_leads_nowhere() {
    assert!(!worktree_row(Leads::Nowhere).switch_to);
}

#[test]
fn the_repositorys_own_worktree_has_nothing_to_remove() {
    assert_eq!(
        worktree_card(true, false, false),
        WorktreeCardOffers::default()
    );
    assert_eq!(
        worktree_card(true, false, true),
        WorktreeCardOffers::default()
    );
}

#[test]
fn a_worktree_nothing_holds_is_removed_as_it_stands() {
    assert_eq!(
        worktree_card(false, false, false),
        WorktreeCardOffers {
            remove: true,
            remove_out: None,
        }
    );
}

/// Every reason keeps the row; the lock goes first, then the tab's own
/// worktree.
#[test]
fn what_stands_in_the_way_greys_the_row_and_says_which() {
    let out = |locked: bool, here: bool| {
        let card = worktree_card(false, locked, here);
        (card.remove, card.remove_out.map(RemoveOut::word))
    };
    assert_eq!(out(true, true), (true, Some("locked")));
    assert_eq!(out(false, true), (true, Some("here")));
}

#[test]
fn the_kind_word_is_the_chips() {
    assert_eq!(RefKind::from_word("worktree"), Some(RefKind::Worktree));
}

/// The two rows' answer: the new branch's row, and the row's own branch out
/// as `checkout`, greyed for `out`.
fn rows(
    here: bool,
    checkout: Option<WorktreeCheckout>,
    out: Option<WorktreeAddOut>,
) -> WorktreeOffers {
    WorktreeOffers {
        here,
        checkout,
        checkout_out: out,
    }
}

/// The two rows that make a worktree, asked of an idle repository on `main`
/// with no worktree holding anything.
fn making(kind: Option<RefKind>, full: &str) -> WorktreeOffers {
    let main = TreeStanding::on("main");
    worktree_rows(
        &standing(kind.map(|kind| target(kind, full)), &main),
        false,
        false,
    )
}

#[test]
fn a_free_branch_goes_out_as_it_is_and_any_commit_takes_a_new_one() {
    assert_eq!(
        making(Some(RefKind::Branch), "feature/login"),
        rows(true, Some(WorktreeCheckout::Branch), None)
    );
    assert_eq!(making(None, ""), rows(true, None, None), "a commit row");
    assert_eq!(making(Some(RefKind::Tag), "v1"), rows(true, None, None));
    assert_eq!(
        making(Some(RefKind::Worktree), "C:/work/detached"),
        rows(true, None, None),
        "a branchless worktree has no branch to take out"
    );
}

/// git keeps a branch to one worktree: on the tree's own and on another's,
/// `worktree add` stands greyed and says which.
#[test]
fn a_branch_already_out_greys_its_worktree_add() {
    assert_eq!(
        making(Some(RefKind::Branch), "main"),
        rows(
            true,
            Some(WorktreeCheckout::Branch),
            Some(WorktreeAddOut::Current)
        )
    );
    let main = TreeStanding::on("main");
    let held = worktree_rows(
        &RowStanding {
            leads: Leads::Holder,
            ..standing(Some(target(RefKind::Branch, "feature/topic-a")), &main)
        },
        false,
        false,
    );
    assert_eq!(
        held,
        rows(
            true,
            Some(WorktreeCheckout::Branch),
            Some(WorktreeAddOut::Held)
        )
    );
    assert_eq!(
        making(Some(RefKind::Branch), "feature/login").checkout_out,
        None,
        "a free branch goes out"
    );
}

/// A rebase or bisect stopped here leaves the tree detached, and the
/// branch it names is still out here — the one listing no worktree as its
/// holder.
#[test]
fn a_branch_a_stopped_operation_here_holds_greys_its_worktree_add() {
    let rebasing = TreeStanding::detached().standing("REBASING");
    let stopped = worktree_rows(
        &standing(Some(target(RefKind::Branch, "feature/clash")), &rebasing),
        true,
        false,
    );
    assert_eq!(
        stopped,
        rows(
            true,
            Some(WorktreeCheckout::Branch),
            Some(WorktreeAddOut::Stopped)
        )
    );
}

/// On a detached HEAD every local branch is free.
#[test]
fn a_detached_tree_holds_no_branch() {
    let detached = TreeStanding::detached();
    let offers = worktree_rows(
        &standing(Some(target(RefKind::Branch, "main")), &detached),
        false,
        false,
    );
    assert_eq!(offers.checkout, Some(WorktreeCheckout::Branch));
}

/// A remote branch makes its local one — unless that is there already,
/// when the local branch's own row is the way.
#[test]
fn a_remote_branch_makes_its_local_one_only_where_there_is_none() {
    assert_eq!(
        making(Some(RefKind::Remote), "origin/feature/login"),
        rows(true, Some(WorktreeCheckout::Track), None)
    );
    let main = TreeStanding::on("main");
    let local_there = worktree_rows(
        &standing(Some(target(RefKind::Remote, "origin/feature/login")), &main),
        false,
        true,
    );
    assert_eq!(local_there, rows(true, None, None));
}

/// Not even a stopped operation keeps a new worktree from being made: nothing
/// here is touched. A stash, a closed tab and a row with no commit do.
#[test]
fn only_a_closed_tab_or_no_commit_keeps_a_worktree_from_being_made() {
    let main = TreeStanding::on("main");
    let closed = RowStanding {
        open: false,
        ..standing(None, &main)
    };
    assert_eq!(
        worktree_rows(&closed, false, false),
        WorktreeOffers::default()
    );
    let no_commit = RowStanding {
        has_commit: false,
        ..standing(None, &main)
    };
    assert_eq!(
        worktree_rows(&no_commit, false, false),
        WorktreeOffers::default()
    );
    assert_eq!(
        worktree_rows(
            &standing(Some(target(RefKind::Stash, "stash@{0}")), &main),
            false,
            false
        ),
        WorktreeOffers::default()
    );
}

/// A worktree on no branch: the box opens on its folder, but on the
/// repository's own worktree nothing happens, and a lock or a gone folder
/// says why instead — the lock first.
#[test]
fn a_folder_is_renamed_unless_git_would_keep_it() {
    assert_eq!(worktree_rename(false, false, false), WorktreeRename::Box);
    assert_eq!(worktree_rename(true, false, false), WorktreeRename::Never);
    assert_eq!(worktree_rename(false, true, false), WorktreeRename::Locked);
    assert_eq!(worktree_rename(false, false, true), WorktreeRename::Gone);
    assert_eq!(worktree_rename(false, true, true), WorktreeRename::Locked);
    assert_eq!(WorktreeRename::Never.word(), "");
}
