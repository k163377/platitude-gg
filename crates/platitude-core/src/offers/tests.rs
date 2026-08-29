use super::*;

/// The inputs of an idle, open repository standing on `main`, which the
/// cases below bend one at a time.
fn offers_on(kind: RefKind, full: &str) -> RefMenuOffers {
    ref_menu(
        kind, full, "abc123", true, 0, "main", false, "", 0, "", "", "origin", "here",
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
        "here",
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
            // A branch goes out through the toolbar, not this menu, and
            // the tag rows name nothing on it.
            push_tag: false,
            delete_remote_tag: false,
            delete_tag_everywhere: false,
            set_upstream: true,
            on_current_branch: false,
        }
    );
    assert_eq!(
        offers.words(),
        "switch branch-here integrate delete delete-remote set-upstream"
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
        "",
        "here",
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
        "origin",
        "here",
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
        "here",
    );
    assert!(offers.switch_to, "the press goes through to the question");
    assert!(offers.switch_asks);
    assert!(!offers.delete, "git refuses the delete too (measured)");
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
        "here",
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
        "here",
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
        "here",
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
        "here",
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
        "here",
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
        "here",
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
        default_remote,
        sides,
    )
}

/// The push row is the tag's alone, and it needs both a destination and
/// something here to send.
#[test]
fn only_a_tag_is_pushed_from_this_menu_and_only_with_a_remote_to_send_it_to() {
    assert!(tag_offers("here", "origin", 0).push_tag);
    for kind in [RefKind::Branch, RefKind::Remote, RefKind::Stash] {
        assert!(
            !ref_menu(
                kind, "feat", "abc123", true, 0, "main", false, "", 0, "", "", "origin", "here"
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

/// The three deletes, one per side the name stands on. **Assembled, not
/// the branch's fixed table**: a row with nothing to name is gone rather
/// than greyed, and a tag always has one that can be pressed.
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
        "branch-here integrate delete push-tag delete-remote-tag delete-tag-everywhere"
    );
}

/// A section that has not answered yet reads as an ordinary local tag:
/// the everyday delete stays, and the rows that need a remote reading
/// wait for one rather than being offered on a guess — the qualified
/// `--delete` git needs does not fail on a name the remote has not got
/// (measured — `remote::delete_remote_tag`).
#[test]
fn an_unread_tag_keeps_its_local_delete_and_offers_no_remote_one() {
    let unread = tag_offers("", "origin", 0);
    assert!(unread.delete);
    assert!(unread.push_tag);
    assert!(!unread.delete_remote_tag);
    assert!(!unread.delete_tag_everywhere);
}

/// With no remote configured there is nowhere for either remote row to
/// reach, whatever a stale reading says.
#[test]
fn no_remote_leaves_the_tag_only_its_local_delete() {
    let offers = tag_offers("both", "", 0);
    assert!(offers.delete);
    assert!(!offers.delete_remote_tag);
    assert!(!offers.delete_tag_everywhere);
}

/// Nothing about where the working tree is stops a tag going out or
/// coming off a remote: it names a commit, and a stopped operation or
/// unmerged files have no bearing on either.
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
        "origin",
        "both",
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
