use super::*;

/// An idle, open repository on `main`; the cases below bend one input at
/// a time.
fn offers_on(kind: RefKind, full: &str) -> RefMenuOffers {
    ref_menu(
        kind, full, "abc123", true, 0, "main", false, "", 0, "", "", false, "origin", "here", "",
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
        false,
        "origin",
        "here",
        "",
    );
    assert_eq!(
        offers,
        RefMenuOffers {
            switch_to: true,
            switch_asks: false,
            branch_here: true,
            integrate_from: true,
            // git pulls into the branch it is run on, not this one.
            pull: false,
            delete: true,
            delete_remote: true,
            // A branch goes out through the toolbar; the tag rows are a tag's.
            push_tag: false,
            delete_remote_tag: false,
            delete_tag_everywhere: false,
            set_upstream: true,
            on_current_branch: false,
        }
    );
    assert_eq!(
        offers.words(),
        [
            "switch",
            "branch-here",
            "integrate",
            "delete",
            "delete-remote",
            "set-upstream"
        ]
    );
}

#[test]
fn only_a_local_branch_is_measured_against_anything() {
    assert!(offers_on(RefKind::Branch, "feat").set_upstream);
    assert!(
        offers_on(RefKind::Branch, "main").set_upstream,
        "the branch the tree is on has an upstream like any other"
    );
    for kind in [RefKind::Remote, RefKind::Tag, RefKind::Stash] {
        assert!(
            !offers_on(kind, "origin/feat").set_upstream,
            "{kind:?} carries no upstream of its own"
        );
    }
}

#[test]
fn there_is_nothing_to_be_measured_against_without_a_remote() {
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
        "",
        false,
        "",
        "here",
        "",
    );
    assert!(!offers.set_upstream, "no remote is no question to ask");
    let busy = ref_menu(
        RefKind::Branch,
        "feat",
        "abc123",
        true,
        1,
        "main",
        false,
        "",
        0,
        "",
        "",
        false,
        "origin",
        "here",
        "",
    );
    assert!(!busy.set_upstream, "another git is still running");
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

/// [`offers_on`], with `main` tracking `upstream`.
fn offers_tracked(kind: RefKind, full: &str, upstream: &str) -> RefMenuOffers {
    ref_menu(
        kind, full, "abc123", true, 0, "main", false, "", 0, "", "", false, "origin", "here",
        upstream,
    )
}

#[test]
fn a_pull_is_offered_on_the_two_ends_of_the_trees_own_comparison() {
    assert!(
        offers_tracked(RefKind::Branch, "main", "origin/main").pull,
        "the branch the tree is on"
    );
    assert!(
        offers_tracked(RefKind::Remote, "origin/main", "origin/main").pull,
        "and the upstream it is measured against"
    );
    assert!(
        !offers_tracked(RefKind::Remote, "origin/feat", "origin/main").pull,
        "a remote nothing here tracks would move a branch this row does not name"
    );
    assert!(
        !offers_tracked(RefKind::Branch, "main", "").pull,
        "git turns a plain pull down with no tracking information"
    );
    assert!(
        !offers_tracked(RefKind::Branch, "feat", "origin/main").pull,
        "a pull moves the branch it is run on, not the row it was asked from"
    );
    for kind in [RefKind::Tag, RefKind::Stash] {
        assert!(
            !offers_tracked(kind, "v1.0", "origin/main").pull,
            "{kind:?} is no branch's upstream"
        );
    }
    assert_eq!(
        offers_tracked(RefKind::Remote, "origin/main", "origin/main").words(),
        ["switch", "branch-here", "integrate", "pull", "delete"]
    );
}

/// The upstream's own row, bent by the three things that hold a write
/// back.
fn pull_on_the_upstream(busy: i32, op_text: &str, detached: bool) -> RefMenuOffers {
    ref_menu(
        RefKind::Remote,
        "origin/main",
        "abc123",
        true,
        busy,
        if detached { "" } else { "main" },
        detached,
        op_text,
        0,
        "",
        "",
        false,
        "origin",
        "here",
        "origin/main",
    )
}

#[test]
fn a_pull_asks_the_same_standing_the_rows_that_integrate_ask() {
    assert!(
        pull_on_the_upstream(0, "", false).pull,
        "with nothing in the way it is on offer"
    );
    assert!(
        !pull_on_the_upstream(1, "", false).pull,
        "another git is still running"
    );
    assert!(
        !pull_on_the_upstream(0, "REBASE 1/3", false).pull,
        "an operation is standing in the way"
    );
    assert!(
        !pull_on_the_upstream(0, "", true).pull,
        "detached there is no branch to pull into"
    );
}

#[test]
fn a_branch_out_in_another_worktree_keeps_switch_and_asks_nothing() {
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
        false,
        "origin",
        "here",
        "",
    );
    assert!(offers.switch_to, "the press goes through to that worktree");
    assert!(
        !offers.switch_asks,
        "it leads somewhere instead of asking, and the row's words say where"
    );
    assert!(!offers.delete, "git refuses the delete too (measured)");
}

#[test]
fn a_branch_out_in_another_worktree_asks_nothing_while_an_operation_stands() {
    // The press opens that worktree and leaves the stopped operation alone
    // (デザイン規約 §進行中の操作から出る「`!` は着ない」).
    for (op, conflicts) in [("CHERRY-PICKING", 0), ("REBASE 1/3", 2), ("", 1)] {
        for (kind, full) in [(RefKind::Branch, "feat"), (RefKind::Remote, "origin/feat")] {
            let asks = |held: &str| {
                ref_menu(
                    kind, full, "abc123", true, 0, "main", false, op, conflicts, held, "", false,
                    "origin", "here", "",
                )
                .switch_asks
            };
            assert!(
                asks(""),
                "{full} with {op:?} / {conflicts}: a plain row asks"
            );
            assert!(
                !asks("C:/work/other"),
                "{full} with {op:?} / {conflicts}: a held row leads to its worktree instead"
            );
        }
    }
}

#[test]
fn a_remote_rows_delete_ignores_who_holds_the_local_branch() {
    // `push --delete` touches nothing on this machine, so a local hold
    // does not bar it.
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
        false,
        "origin",
        "here",
        "",
    );
    assert!(offers.delete);
    assert!(
        !offers.switch_asks,
        "the move lands on the held branch, which is a road to that worktree and not a question"
    );
}

/// A drifted branch and its remote reading stand on two rows, and only the
/// one under the pointer is this menu's to delete (デザイン規約
/// §左メニューの所作 の削除の表).
#[test]
fn a_drifted_reading_keeps_the_local_delete_and_loses_the_remote_one() {
    let drifted = ref_menu(
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
        true,
        "origin",
        "here",
        "",
    );
    assert!(drifted.delete, "the branch itself is standing right here");
    assert!(!drifted.delete_remote);
    assert!(
        drifted.set_upstream,
        "which reading it is measured against is what answers a drift"
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
        false,
        "origin",
        "here",
        "",
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
        false,
        "origin",
        "here",
        "",
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
        false,
        "origin",
        "here",
        "",
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
        false,
        "origin",
        "here",
        "",
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
        false,
        "origin",
        "here",
        "",
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

/// A tag row on an idle repository, bent by the two things that decide
/// its own rows: where the pushes go, and which sides the name stands on.
fn tag_offers(sides: &str, default_remote: &str, busy: i32) -> RefMenuOffers {
    ref_menu(
        RefKind::Tag,
        "v1.0",
        "abc123",
        true,
        busy,
        "main",
        false,
        "",
        0,
        "",
        "",
        false,
        default_remote,
        sides,
        "",
    )
}

#[test]
fn only_a_tag_is_pushed_from_this_menu_and_only_with_a_remote_to_send_it_to() {
    assert!(tag_offers("here", "origin", 0).push_tag);
    for kind in [RefKind::Branch, RefKind::Remote, RefKind::Stash] {
        assert!(
            !ref_menu(
                kind, "feat", "abc123", true, 0, "main", false, "", 0, "", "", false, "origin",
                "here", ""
            )
            .push_tag,
            "a branch goes out through the toolbar; a stash goes nowhere"
        );
    }
    assert!(
        !tag_offers("here", "", 0).push_tag,
        "no remote is no destination"
    );
    assert!(
        !tag_offers("here", "origin", 2).push_tag,
        "another git command is still running"
    );
    assert!(
        !tag_offers("remote", "origin", 0).push_tag,
        "a name only a remote has is nothing this repository can send"
    );
}

/// One delete per side the name stands on — assembled, not the branch's
/// fixed table: a row with nothing to name is gone rather than greyed.
#[test]
fn each_delete_row_needs_the_side_it_names() {
    let here = tag_offers("here", "origin", 0);
    assert!(here.delete, "made here, so `tag --delete` names it");
    assert!(!here.delete_remote_tag, "no remote is known to carry it");
    assert!(!here.delete_tag_everywhere);

    let over_there = tag_offers("remote", "origin", 0);
    assert!(
        !over_there.delete,
        "`tag --delete` would have nothing to name"
    );
    assert!(over_there.delete_remote_tag);
    assert!(
        !over_there.delete_tag_everywhere,
        "there is no local half to take with it"
    );

    let both = tag_offers("both", "origin", 0);
    assert!(both.delete && both.delete_remote_tag && both.delete_tag_everywhere);
    assert_eq!(
        both.words(),
        [
            "branch-here",
            "integrate",
            "delete",
            "push-tag",
            "delete-remote-tag",
            "delete-tag-everywhere"
        ]
    );
}

/// A remote holding the name on another commit: both rows that reach it
/// are out, and the push stays as the leased overwrite (デザイン規約
/// §相手の履歴を置き換える).
#[test]
fn a_tag_the_remote_has_elsewhere_loses_both_rows_that_reach_it() {
    let drifted = ref_menu(
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
        true,
        "origin",
        "both",
        "",
    );
    assert!(drifted.delete);
    assert!(drifted.push_tag);
    assert!(!drifted.delete_remote_tag);
    assert!(!drifted.delete_tag_everywhere);
}

/// An unread section reads as a local tag: the remote rows wait for a
/// reading, which is what their delete is leased to
/// (`remote::delete_remote_tag`).
#[test]
fn an_unread_tag_keeps_its_local_delete_and_offers_no_remote_one() {
    let unread = tag_offers("", "origin", 0);
    assert!(unread.delete);
    assert!(unread.push_tag);
    assert!(!unread.delete_remote_tag);
    assert!(!unread.delete_tag_everywhere);
}

/// No remote configured outweighs a stale reading.
#[test]
fn no_remote_leaves_the_tag_only_its_local_delete() {
    let offers = tag_offers("both", "", 0);
    assert!(offers.delete);
    assert!(!offers.delete_remote_tag);
    assert!(!offers.delete_tag_everywhere);
}

/// A tag names a commit, so a stopped operation or unmerged files do not
/// hold its remote rows back.
#[test]
fn a_stopped_operation_does_not_hold_a_tag_back() {
    let offers = ref_menu(
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
        false,
        "origin",
        "both",
        "",
    );
    assert!(offers.push_tag);
    assert!(offers.delete_remote_tag);
    assert!(offers.delete_tag_everywhere);
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
