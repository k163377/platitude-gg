use super::*;

/// The inputs of an idle, open repository standing on `main`, which the
/// cases below bend one at a time.
fn offers_on(kind: RefKind, full: &str) -> RefMenuOffers {
    ref_menu(
        kind, full, "abc123", true, 0, "main", false, "", 0, "", "", "origin",
    )
}

#[test]
fn a_branch_someone_else_is_not_on_offers_everything() {
    let offers = ref_menu(
        RefKind::Branch,
        "feat",
        "abc123",
        true,
        0,
        "main",
        false,
        "",
        0,
        "",
        "origin/feat",
        "origin",
    );
    assert_eq!(
        offers,
        RefMenuOffers {
            switch_to: true,
            switch_asks: false,
            branch_here: true,
            integrate_from: true,
            delete: true,
            delete_remote: true,
            // A branch goes out through the toolbar, not this menu.
            push_tag: false,
            on_current_branch: false,
        }
    );
    assert_eq!(
        offers.words(),
        "switch branch-here integrate delete delete-remote"
    );
}

#[test]
fn the_current_branch_keeps_its_rows_but_moves_and_deletes_nowhere() {
    let offers = offers_on(RefKind::Branch, "main");
    assert!(!offers.switch_to, "the branch the tree is on is not a move");
    assert!(!offers.delete, "git refuses deleting the branch HEAD is on");
    assert!(offers.on_current_branch);
    assert!(!offers.integrate_from, "nothing of itself to bring in");
    assert!(offers.branch_here, "where a branch is most often started");
}

#[test]
fn a_branch_out_in_another_copy_keeps_switch_but_it_asks() {
    let offers = ref_menu(
        RefKind::Branch,
        "feat",
        "abc123",
        true,
        0,
        "main",
        false,
        "",
        0,
        "C:/work/other",
        "",
        "origin",
    );
    assert!(offers.switch_to, "the press goes through to the question");
    assert!(offers.switch_asks);
    assert!(
        !offers.delete,
        "git refuses the delete too (実測 2026-08-21)"
    );
}

#[test]
fn a_remote_rows_delete_ignores_who_holds_the_local_branch() {
    // `push --delete` weighs nothing on this machine — the hold stands in
    // for the refusal git cannot give.
    let offers = ref_menu(
        RefKind::Remote,
        "origin/feat",
        "abc123",
        true,
        0,
        "main",
        false,
        "",
        0,
        "C:/work/other",
        "",
        "origin",
    );
    assert!(offers.delete);
    assert!(
        offers.switch_asks,
        "the move still lands on the held branch"
    );
}

#[test]
fn a_standing_operation_asks_and_holds_back_new_history_but_not_deletes() {
    let offers = ref_menu(
        RefKind::Branch,
        "feat",
        "abc123",
        true,
        0,
        "main",
        false,
        "REBASING",
        0,
        "",
        "",
        "origin",
    );
    assert!(offers.switch_asks);
    assert!(!offers.branch_here);
    assert!(!offers.integrate_from);
    assert!(offers.delete, "a delete moves no working tree");
}

#[test]
fn conflicts_alone_make_the_move_ask() {
    let offers = ref_menu(
        RefKind::Branch,
        "feat",
        "abc123",
        true,
        0,
        "main",
        false,
        "",
        2,
        "",
        "",
        "origin",
    );
    assert!(offers.switch_asks);
    assert!(offers.branch_here, "conflicts gate moves, not new branches");
}

#[test]
fn a_running_command_holds_back_every_write_but_not_the_switch() {
    let offers = ref_menu(
        RefKind::Branch,
        "feat",
        "abc123",
        true,
        3,
        "main",
        false,
        "",
        0,
        "",
        "origin/feat",
        "origin",
    );
    assert!(offers.switch_to);
    assert!(!offers.branch_here);
    assert!(!offers.integrate_from);
    assert!(!offers.delete);
    assert!(!offers.delete_remote);
}

#[test]
fn detached_or_unborn_has_no_branch_to_integrate_into() {
    let detached = ref_menu(
        RefKind::Branch,
        "feat",
        "abc123",
        true,
        0,
        "",
        true,
        "",
        0,
        "",
        "",
        "origin",
    );
    assert!(!detached.integrate_from);
    assert!(detached.switch_to, "the way back out of detached");
    let unborn = ref_menu(
        RefKind::Branch,
        "feat",
        "abc123",
        true,
        0,
        "",
        false,
        "",
        0,
        "",
        "",
        "origin",
    );
    assert!(!unborn.integrate_from);
}

#[test]
fn a_tag_moves_nowhere_but_starts_branches_and_deletes() {
    let offers = offers_on(RefKind::Tag, "v1.0");
    assert!(!offers.switch_to);
    assert!(offers.branch_here);
    assert!(offers.delete);
}

/// The push row is the tag's alone, and it needs somewhere to send it.
#[test]
fn only_a_tag_is_pushed_from_this_menu_and_only_with_a_remote_to_send_it_to() {
    assert!(offers_on(RefKind::Tag, "v1.0").push_tag);
    for kind in [RefKind::Branch, RefKind::Remote, RefKind::Stash] {
        assert!(
            !ref_menu(
                kind, "feat", "abc123", true, 0, "main", false, "", 0, "", "", "origin"
            )
            .push_tag,
            "a branch goes out through the toolbar; a stash goes nowhere"
        );
    }
    assert!(
        !ref_menu(
            RefKind::Tag,
            "v1.0",
            "abc123",
            true,
            0,
            "main",
            false,
            "",
            0,
            "",
            "",
            "",
        )
        .push_tag,
        "no remote is no destination"
    );
    assert!(
        !ref_menu(
            RefKind::Tag,
            "v1.0",
            "abc123",
            true,
            2,
            "main",
            false,
            "",
            0,
            "",
            "",
            "origin",
        )
        .push_tag,
        "another git command is still running"
    );
}

/// Nothing about where the working tree is stops a tag going out: it
/// names a commit, and a stopped operation or unmerged files have no
/// bearing on sending that commit somewhere.
#[test]
fn a_stopped_operation_does_not_hold_a_tag_back() {
    assert!(
        ref_menu(
            RefKind::Tag,
            "v1.0",
            "abc123",
            true,
            0,
            "main",
            false,
            "REBASING",
            3,
            "",
            "",
            "origin",
        )
        .push_tag
    );
}

#[test]
fn a_stash_is_nobodys_history_to_branch_from() {
    assert!(!offers_on(RefKind::Stash, "stash@{0}").branch_here);
}

#[test]
fn a_kind_word_nothing_answers_to_offers_nothing() {
    assert_eq!(RefKind::from_word("HEAD"), None);
    assert_eq!(RefKind::from_word(""), None);
    assert_eq!(RefKind::from_word("branch"), Some(RefKind::Branch));
}

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
