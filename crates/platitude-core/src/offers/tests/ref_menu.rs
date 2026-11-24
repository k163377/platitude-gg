use super::super::testkit::{standing, target};
use super::super::*;

/// The rows of `full`'s menu on an idle, open repository on `main` that no
/// other worktree holds, reading `far` over there; the cases below bend
/// one input at a time.
fn reading(kind: RefKind, full: &str, far: FarSide) -> RefMenuOffers {
    ref_menu(
        &standing(Some(target(kind, full)), &TreeStanding::on("main")),
        far,
    )
}

/// The row's own menu, which reads nothing over there.
fn offers_on(kind: RefKind, full: &str) -> RefMenuOffers {
    reading(kind, full, FarSide::Unread)
}

/// A local branch's far side in a repository with a remote: `reading`
/// where it stands, and an upstream exactly where there is a reading.
fn upstream(reading: Reading) -> FarSide {
    FarSide::Branch(BranchFar {
        reading,
        tracked: reading != Reading::None,
        has_remote: true,
        ahead: 0,
    })
}

/// A tag's far side standing on `sides`, nothing held back, its push going
/// to `origin` — or nowhere without a remote.
fn tag_far(sides: Option<TagSides>, has_remote: bool) -> FarSide {
    FarSide::Tag(TagFar {
        sides,
        push_remote: pushed_to(has_remote),
        held_back: TagDeleteHeld::No,
    })
}

fn pushed_to(has_remote: bool) -> String {
    if has_remote { "origin" } else { "" }.to_string()
}

#[test]
fn a_branch_someone_else_is_not_on_offers_everything() {
    let offers = reading(RefKind::Branch, "feat", upstream(Reading::Together));
    assert_eq!(
        offers,
        RefMenuOffers {
            switch_to: true,
            switch_asks: false,
            branch_here: true,
            integrate_from: true,
            standing: false,
            delete: true,
            delete_remote: true,
            // A branch goes out through the toolbar; the tag rows are a tag's.
            push_tag: false,
            delete_remote_tag: Seat::Gone,
            delete_tag_everywhere: Seat::Gone,
            set_upstream: true,
            unset_upstream: true,
            on_current_branch: false,
        }
    );
}

/// The rows that reach over there are the cards': the row's own menu reads
/// nothing there and offers none of them, and keeps the everyday delete on
/// a tag, as an unread section does.
#[test]
fn a_rows_own_menu_reads_nothing_over_there() {
    let branch = offers_on(RefKind::Branch, "feat");
    assert!(!branch.delete_remote);
    assert!(!branch.set_upstream);
    assert!(!branch.unset_upstream);
    let tag = offers_on(RefKind::Tag, "v1.0");
    assert!(tag.delete, "read as a tag here");
    assert!(!tag.push_tag);
    assert_eq!(tag.delete_remote_tag, Seat::Gone);
    assert_eq!(tag.delete_tag_everywhere, Seat::Gone);
}

/// The far side is the model's record and the kind the menu's word, handed
/// on apart: a far side on a row of another kind is read as nothing there.
#[test]
fn a_far_side_of_another_kind_reads_as_nothing_over_there() {
    let branch = upstream(Reading::Together);
    let tag = tag_far(Some(TagSides::Both), true);
    for (kind, full, far) in [
        (RefKind::Tag, "v1.0", branch.clone()),
        (RefKind::Remote, "origin/feat", branch.clone()),
        (RefKind::Stash, "stash@{0}", branch.clone()),
        (RefKind::Worktree, "C:/work/topic", branch.clone()),
        (RefKind::Branch, "feat", tag.clone()),
        (RefKind::Remote, "origin/feat", tag.clone()),
        (RefKind::Stash, "stash@{0}", tag.clone()),
    ] {
        assert_eq!(
            reading(kind, full, far.clone()),
            offers_on(kind, full),
            "{kind:?} reading {far:?}"
        );
    }
}

/// Only a reading on the branch's own commit is this card's to delete; a
/// drifted one goes from its own row, and with none there is nothing.
#[test]
fn only_a_reading_beside_the_branch_is_deleted_from_its_card() {
    let delete_remote = |at: Reading| reading(RefKind::Branch, "feat", upstream(at)).delete_remote;
    assert!(delete_remote(Reading::Together));
    assert!(!delete_remote(Reading::Drifted));
    assert!(!delete_remote(Reading::None));
}

/// `tracked` alone decides it: a gone upstream leaves no reading to
/// name, and one that is a branch here needs no remote.
#[test]
fn only_a_branch_with_an_upstream_has_one_to_take_off() {
    let on = |kind, tracked, has_remote| {
        reading(
            kind,
            "feat",
            FarSide::Branch(BranchFar {
                reading: Reading::None,
                tracked,
                has_remote,
                ahead: 0,
            }),
        )
        .unset_upstream
    };
    assert!(on(RefKind::Branch, true, true));
    assert!(!on(RefKind::Branch, false, true), "nothing to take off");
    assert!(
        on(RefKind::Branch, true, false),
        "an upstream here needs no remote"
    );
    for kind in [
        RefKind::Remote,
        RefKind::Tag,
        RefKind::Stash,
        RefKind::Worktree,
    ] {
        assert!(
            !on(kind, true, true),
            "{kind:?} carries no upstream of its own"
        );
    }
}

#[test]
fn the_current_branch_and_a_held_one_take_their_upstream_off_too() {
    let current = reading(RefKind::Branch, "main", upstream(Reading::Together));
    assert!(
        current.unset_upstream,
        "this writes configuration, not the branch"
    );
    let held = ref_menu(
        &RowStanding {
            leads: Leads::Holder,
            ..standing(
                Some(target(RefKind::Branch, "feat")),
                &TreeStanding::on("main"),
            )
        },
        upstream(Reading::Together),
    );
    assert!(
        held.unset_upstream,
        "another worktree holding it changes nothing"
    );
}

#[test]
fn only_a_local_branch_is_measured_against_anything() {
    let measured = |kind, full| reading(kind, full, upstream(Reading::None)).set_upstream;
    assert!(measured(RefKind::Branch, "feat"));
    assert!(
        measured(RefKind::Branch, "main"),
        "the branch the tree is on has an upstream like any other"
    );
    for kind in [RefKind::Remote, RefKind::Tag, RefKind::Stash] {
        assert!(
            !measured(kind, "origin/feat"),
            "{kind:?} carries no upstream of its own"
        );
    }
}

#[test]
fn there_is_nothing_to_be_measured_against_without_a_remote() {
    let offers = reading(
        RefKind::Branch,
        "feat",
        FarSide::Branch(BranchFar::default()),
    );
    assert!(!offers.set_upstream, "no remote is no question to ask");
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
fn a_branch_out_in_another_worktree_keeps_switch_and_asks_nothing() {
    let offers = ref_menu(
        &RowStanding {
            leads: Leads::Holder,
            ..standing(
                Some(target(RefKind::Branch, "feat")),
                &TreeStanding::on("main"),
            )
        },
        FarSide::Unread,
    );
    assert!(offers.switch_to, "the press goes through to that worktree");
    assert!(
        !offers.switch_asks,
        "it leads somewhere instead of asking, and the row's words say where"
    );
    assert!(!offers.delete, "git refuses the delete too (measured)");
}

#[test]
fn a_branch_out_in_another_worktree_asks_nothing_while_moves_are_blocked() {
    // The press opens that worktree and leaves the stopped operation alone
    // and wears no `!` (`SwitchAction::OpenHolder`).
    let main = TreeStanding::on("main");
    for (blocker, tree) in [
        ("a cherry-pick", main.clone().standing("CHERRY-PICKING")),
        ("a rebase", main.clone().standing("REBASE 1/3")),
        (
            "unmerged paths alone",
            TreeStanding {
                moves_blocked: true,
                ..main
            },
        ),
    ] {
        for (kind, full) in [(RefKind::Branch, "feat"), (RefKind::Remote, "origin/feat")] {
            let asks = |held: bool| {
                ref_menu(
                    &RowStanding {
                        leads: if held { Leads::Holder } else { Leads::Nowhere },
                        ..standing(Some(target(kind, full)), &tree)
                    },
                    FarSide::Unread,
                )
                .switch_asks
            };
            assert!(asks(false), "{full} under {blocker}: a plain row asks");
            assert!(
                !asks(true),
                "{full} under {blocker}: a held row leads to its worktree instead"
            );
        }
    }
}

#[test]
fn a_remote_rows_delete_ignores_who_holds_the_local_branch() {
    // `push --delete` touches nothing on this machine, so a local hold
    // does not bar it.
    let offers = ref_menu(
        &RowStanding {
            leads: Leads::Holder,
            ..standing(
                Some(target(RefKind::Remote, "origin/feat")),
                &TreeStanding::on("main"),
            )
        },
        FarSide::Unread,
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
    let drifted = reading(RefKind::Branch, "feat", upstream(Reading::Drifted));
    assert!(drifted.delete, "the branch itself is standing right here");
    assert!(!drifted.delete_remote);
    assert!(
        drifted.set_upstream,
        "which reading it is measured against is what answers a drift"
    );
}

#[test]
fn a_standing_operation_asks_and_greys_new_history_but_not_deletes() {
    let standing = |detached: bool| {
        ref_menu(
            &standing(
                Some(target(RefKind::Branch, "feat")),
                &if detached {
                    TreeStanding::detached().standing("REBASE 1/3")
                } else {
                    TreeStanding::on("main").standing("MERGING")
                },
            ),
            FarSide::Unread,
        )
    };
    let merging = standing(false);
    assert!(merging.switch_asks);
    assert!(merging.standing);
    assert!(merging.branch_here, "kept, greyed");
    assert!(merging.integrate_from, "kept, greyed");
    assert!(merging.delete, "a delete moves no working tree");
    let rebasing = standing(true);
    assert!(rebasing.standing);
    assert!(rebasing.branch_here, "kept, greyed");
    assert!(
        !rebasing.integrate_from,
        "a stopped rebase leaves the tree detached: no branch for a row to name"
    );
}

/// Unmerged paths with no operation standing (what `--quit` leaves).
#[test]
fn a_blocked_move_asks_with_no_operation_standing() {
    let offers = ref_menu(
        &standing(
            Some(target(RefKind::Branch, "feat")),
            &TreeStanding {
                moves_blocked: true,
                ..TreeStanding::on("main")
            },
        ),
        FarSide::Unread,
    );
    assert!(offers.switch_asks);
    assert!(!offers.standing, "nothing to grey the new history rows for");
    assert!(offers.branch_here, "conflicts gate moves, not new branches");
}

/// The `!` is what the press does (`RepoPage.standsInTheWay`): a badge a
/// replay's tick put up ahead of the status asks nothing until the status
/// says the move is blocked.
#[test]
fn the_mark_follows_the_blocked_move_and_not_the_badge() {
    let under_the_badge = |moves_blocked: bool| {
        ref_menu(
            &standing(
                Some(target(RefKind::Branch, "feat")),
                &TreeStanding {
                    moves_blocked,
                    ..TreeStanding::detached().standing("REBASING")
                },
            ),
            FarSide::Unread,
        )
    };
    let ticked = under_the_badge(false);
    assert!(!ticked.switch_asks, "the press would not ask yet");
    assert!(
        ticked.standing,
        "the badge still greys the new history rows"
    );
    assert!(
        under_the_badge(true).switch_asks,
        "the status has caught up"
    );
}

#[test]
fn detached_or_unborn_has_no_branch_to_integrate_into() {
    let on = |tree: &TreeStanding| {
        ref_menu(
            &standing(Some(target(RefKind::Branch, "feat")), tree),
            FarSide::Unread,
        )
    };
    let detached = on(&TreeStanding::detached());
    assert!(!detached.integrate_from);
    assert!(detached.switch_to, "the way back out of detached");
    assert!(!on(&TreeStanding::default()).integrate_from);
}

#[test]
fn a_tag_moves_nowhere_but_starts_branches_and_deletes() {
    let offers = offers_on(RefKind::Tag, "v1.0");
    assert!(!offers.switch_to);
    assert!(offers.branch_here);
    assert!(offers.delete);
}

/// A tag's card on an idle repository, bent by the two things that decide
/// its own rows: whether there is a remote to go to, and which sides the
/// name stands on.
fn tag_offers(sides: Option<TagSides>, has_remote: bool) -> RefMenuOffers {
    reading(RefKind::Tag, "v1.0", tag_far(sides, has_remote))
}

#[test]
fn only_a_tag_is_pushed_from_this_menu_and_only_with_a_remote_to_send_it_to() {
    assert!(tag_offers(Some(TagSides::Here), true).push_tag);
    for kind in [RefKind::Branch, RefKind::Remote, RefKind::Stash] {
        assert!(
            !reading(kind, "feat", tag_far(Some(TagSides::Here), true)).push_tag,
            "a branch goes out through the toolbar; a stash goes nowhere"
        );
    }
    assert!(
        !tag_offers(Some(TagSides::Here), false).push_tag,
        "no remote is no destination"
    );
    assert!(
        !tag_offers(Some(TagSides::Remote), true).push_tag,
        "a name only a remote has is nothing this repository can send"
    );
}

/// One delete per side the name stands on — assembled, not the branch's
/// fixed table: a row with nothing to name is gone rather than greyed.
#[test]
fn each_delete_row_needs_the_side_it_names() {
    let here = tag_offers(Some(TagSides::Here), true);
    assert!(here.delete, "made here, so `tag --delete` names it");
    assert_eq!(
        here.delete_remote_tag,
        Seat::Gone,
        "no remote is known to carry it"
    );
    assert_eq!(here.delete_tag_everywhere, Seat::Gone);

    let over_there = tag_offers(Some(TagSides::Remote), true);
    assert!(
        !over_there.delete,
        "`tag --delete` would have nothing to name"
    );
    assert_eq!(over_there.delete_remote_tag, Seat::Live);
    assert_eq!(
        over_there.delete_tag_everywhere,
        Seat::Gone,
        "there is no local half to take with it"
    );

    let both = tag_offers(Some(TagSides::Both), true);
    assert!(both.delete);
    assert_eq!(both.delete_remote_tag, Seat::Live);
    assert_eq!(both.delete_tag_everywhere, Seat::Live);
}

/// A remote holding the name on another commit, or several carrying it
/// with none named: both rows that reach over there keep their seats
/// greyed, and the push stays — as the leased overwrite where it is a drift
/// (デザイン規約 §相手の履歴を置き換える).
#[test]
fn a_tag_held_back_over_there_greys_both_rows_that_reach_it() {
    let held = |sides: TagSides, has_remote: bool, held_back: TagDeleteHeld| {
        reading(
            RefKind::Tag,
            "v1.0",
            FarSide::Tag(TagFar {
                sides: Some(sides),
                push_remote: pushed_to(has_remote),
                held_back,
            }),
        )
    };
    for held_back in [TagDeleteHeld::Drifted, TagDeleteHeld::Unnamed] {
        let both = held(TagSides::Both, true, held_back);
        assert!(both.delete, "{held_back:?}");
        assert!(both.push_tag, "{held_back:?}");
        assert_eq!(both.delete_remote_tag, Seat::Greyed, "{held_back:?}");
        assert_eq!(both.delete_tag_everywhere, Seat::Greyed, "{held_back:?}");
        assert_eq!(
            held(TagSides::Remote, true, held_back).delete_tag_everywhere,
            Seat::Gone,
            "{held_back:?}: the pair needs a name here to hold back"
        );
        assert_eq!(
            held(TagSides::Both, false, held_back).delete_remote_tag,
            Seat::Greyed,
            "{held_back:?}: the hold names a reading left from before, and says so"
        );
    }
}

/// An unread section reads as a local tag: the remote rows wait for a
/// reading, which is what their delete is leased to
/// (`remote::delete_remote_tag`).
#[test]
fn an_unread_tag_keeps_its_local_delete_and_offers_no_remote_one() {
    let unread = tag_offers(None, true);
    assert!(unread.delete);
    assert!(unread.push_tag);
    assert_eq!(unread.delete_remote_tag, Seat::Gone);
    assert_eq!(unread.delete_tag_everywhere, Seat::Gone);
}

/// No remote configured outweighs a stale reading.
#[test]
fn no_remote_leaves_the_tag_only_its_local_delete() {
    let offers = tag_offers(Some(TagSides::Both), false);
    assert!(offers.delete);
    assert_eq!(offers.delete_remote_tag, Seat::Gone);
    assert_eq!(offers.delete_tag_everywhere, Seat::Gone);
}

/// A tag names a commit, so a stopped operation or unmerged files do not
/// hold its remote rows back.
#[test]
fn a_stopped_operation_does_not_hold_a_tag_back() {
    let offers = ref_menu(
        &standing(
            Some(target(RefKind::Tag, "v1.0")),
            &TreeStanding::on("main").standing("REBASING"),
        ),
        tag_far(Some(TagSides::Both), true),
    );
    assert!(offers.push_tag);
    assert_eq!(offers.delete_remote_tag, Seat::Live);
    assert_eq!(offers.delete_tag_everywhere, Seat::Live);
}

#[test]
fn a_stash_is_nobodys_history_to_branch_from() {
    assert!(!offers_on(RefKind::Stash, "stash@{0}").branch_here);
}

#[test]
fn a_row_on_no_commit_starts_nothing_on_it() {
    let main = TreeStanding::on("main");
    let row = RowStanding {
        has_commit: false,
        ..standing(Some(target(RefKind::Branch, "feat")), &main)
    };
    assert!(!ref_menu(&row, FarSide::Unread).branch_here);
}

#[test]
fn a_row_that_names_no_ref_offers_none_of_a_refs_rows() {
    let main = TreeStanding::on("main");
    assert_eq!(
        ref_menu(&standing(None, &main), FarSide::Unread),
        RefMenuOffers::default()
    );
}

/// The colour the upstream's name wears: a remote reading, a branch here,
/// or the one HEAD is on — told by the tree the menu stands on, a gone
/// one by where it was configured.
#[test]
fn an_upstream_stands_where_its_configuration_and_the_tree_say() {
    let main = TreeStanding::on("main");
    assert_eq!(upstream_place("", false, &main), UpstreamPlace::None);
    assert_eq!(
        upstream_place("origin/feat", false, &main),
        UpstreamPlace::Remote
    );
    assert_eq!(upstream_place("main", true, &main), UpstreamPlace::Current);
    assert_eq!(upstream_place("release", true, &main), UpstreamPlace::Local);
    assert_eq!(
        upstream_place("main", true, &TreeStanding::detached()),
        UpstreamPlace::Local,
        "detached, no branch is the one HEAD is on"
    );
    assert_eq!(
        upstream_place("origin/main", true, &main),
        UpstreamPlace::Local,
        "a branch here may be called like a remote one"
    );
}

#[test]
fn every_lead_reads_back_as_its_word() {
    for leads in [Leads::Nowhere, Leads::Holder, Leads::Itself] {
        assert_eq!(Leads::from_word(leads.word()), Some(leads));
    }
    assert_eq!(Leads::from_word("elsewhere"), None);
}

#[test]
fn a_row_is_a_kind_and_a_name() {
    assert_eq!(
        RefTarget::new(RefKind::Branch, ""),
        None,
        "no name is no row"
    );
    assert_eq!(RefTarget::from_words("HEAD", "main"), None);
    assert_eq!(RefTarget::from_words("branch", ""), None);
    assert_eq!(
        RefTarget::from_words("remote", "origin/feat"),
        RefTarget::new(RefKind::Remote, "origin/feat")
    );
}

#[test]
fn a_kind_word_nothing_answers_to_offers_nothing() {
    assert_eq!(RefKind::from_word("HEAD"), None);
    assert_eq!(RefKind::from_word(""), None);
    assert_eq!(RefKind::from_word("branch"), Some(RefKind::Branch));
}
