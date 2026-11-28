//! What each row says it can do, read off the type alone: which subjects
//! have it, and the reasons it greys for, in the order its tip says them
//! ([`crate::menu::line`]) — and what a press sends besides
//! ([`Subject::switch_asks`], [`Subject::worktree_checkout`]). A branch's
//! upstream and its pull are `upstream`'s.

use super::testkit::*;
use crate::menu::{
    Can, Facts, History, Nowhere, On, Placing, Row, RowWorktree, Subject, Why, WorktreeOut, lines,
    worktree_left,
};
use crate::offers::{
    BranchFar, FarSide, Leads, TagDeleteHeld, TagSides, Tracks, TreeStanding, WorktreeCheckout,
};
use crate::remote::PushStanding;
use crate::worktrees::Keeper;

#[test]
fn a_branch_the_tab_is_not_on_has_every_row_of_a_branch_live() {
    let tree = on_main();
    let menu = said(&Subject {
        far: tracking(Tracks::Together, 0),
        ..row(BRANCH, "feat", &tree)
    });
    for row in [
        Row::BranchHere,
        Row::Reach,
        Row::Merge,
        Row::Rebase,
        Row::Pull,
        Row::Push,
        Row::DeleteBranch,
        Row::DeleteRemoteBranch,
        Row::DeleteBoth,
        Row::SetUpstream,
        Row::UnsetUpstream,
        Row::WorktreeHere,
        Row::WorktreeAdd,
        Row::TagHere,
    ] {
        assert_eq!(at(&menu, row), LIVE, "{row:?}");
    }
    assert_eq!(menu.len(), 14, "and no other: {menu:?}");
}

#[test]
fn a_far_side_of_another_kind_reads_as_nothing_over_there() {
    let tree = on_main();
    let menu = said(&Subject {
        far: tag_far(TagSides::Both, "origin", TagDeleteHeld::No),
        ..row(BRANCH, "feat", &tree)
    });
    assert_eq!(at(&menu, Row::UnsetUpstream), None, "no upstream");
    assert_eq!(at(&menu, Row::Pull), None);
    assert_eq!(at(&menu, Row::DeleteRemoteBranch), None);
    assert_eq!(
        at(&menu, Row::SetUpstream),
        greyed(Why::NoRemote),
        "nor a remote to name one on"
    );
}

/// The branch HEAD is on, with no commits yet, keeps the rows it would have
/// with some, greyed where they need one: git has no ref to start on,
/// measure or bring in — the branch it is first.
#[test]
fn a_branch_with_no_commits_yet_keeps_its_rows_greyed() {
    let far = FarSide::Branch(BranchFar {
        has_remote: true,
        ..BranchFar::default()
    });
    let unborn = TreeStanding {
        head_oid: String::new(),
        ..TreeStanding::on("main")
    };
    let menu = said(&Subject {
        commit: None,
        far: far.clone(),
        ..row(BRANCH, "main", &unborn)
    });
    let born = TreeStanding::on("main");
    let with_commits = said(&Subject {
        far,
        ..row(BRANCH, "main", &born)
    });
    assert!(
        menu.keys().eq(with_commits.keys()),
        "{menu:?} against {with_commits:?}"
    );
    for row in [
        Row::BranchHere,
        Row::SetUpstream,
        Row::WorktreeHere,
        Row::TagHere,
    ] {
        assert_eq!(at(&menu, row), greyed(Why::NoCommitsYet), "{row:?}");
    }
    assert_eq!(at(&menu, Row::DeleteBranch), greyed(Why::OnThisBranch));
    assert_eq!(at(&menu, Row::WorktreeAdd), greyed(Why::OnThisBranch));
}

/// Another branch with no commits yet — out in another worktree, since one
/// with none is HEAD somewhere: no listing names it (it has no ref), so its
/// push answer is closed for good, and the rows that need a commit say that
/// rather than waiting for a read that will not come.
#[test]
fn a_branch_elsewhere_with_no_commits_yet_greys_for_its_missing_commit() {
    let tree = on_main();
    let menu = said(&Subject {
        commit: None,
        push: PushStanding::Closed,
        ..row(BRANCH, "topic", &tree)
    });
    for row in [Row::Merge, Row::Rebase, Row::Push] {
        assert_eq!(at(&menu, row), greyed(Why::NoCommitsYet), "{row:?}");
    }
}

#[test]
fn the_current_branch_moves_nowhere_and_keeps_its_name() {
    let tree = on_main();
    let menu = said(&Subject {
        commit: Some(oid(HEAD)),
        far: tracking(Tracks::Together, 0),
        ..row(BRANCH, "main", &tree)
    });
    for row in [Row::Reach, Row::Merge, Row::Rebase, Row::Push] {
        assert_eq!(at(&menu, row), None, "{row:?}: the tab is on it");
    }
    assert_eq!(at(&menu, Row::DeleteBranch), greyed(Why::OnThisBranch));
    assert_eq!(at(&menu, Row::DeleteBoth), greyed(Why::OnThisBranch));
    assert_eq!(
        at(&menu, Row::DeleteRemoteBranch),
        LIVE,
        "its reading goes on its own"
    );
    assert_eq!(at(&menu, Row::Pull), LIVE);
}

/// A branch's push row answers its standing arm by arm. Closed is no
/// answer at all — the listing still marks the branch as HEAD's while the
/// tree reads another, or does not name it yet —, so it greys rather than
/// sending with nothing asked of where the branch goes.
#[test]
fn a_push_row_answers_each_standing_and_greys_one_not_read() {
    let tree = on_main();
    for (push, can) in [
        (PushStanding::Closed, greyed(Why::Unread)),
        (PushStanding::Unborn, greyed(Why::NoCommitsYet)),
        (PushStanding::StaysHere, greyed(Why::StaysHere)),
        (PushStanding::Clean, greyed(Why::NothingToSend)),
        (PushStanding::Publish, LIVE),
        (PushStanding::Elsewhere, LIVE),
        (PushStanding::Ready, LIVE),
        (PushStanding::Behind, LIVE),
        (PushStanding::Diverged, LIVE),
    ] {
        let menu = said(&Subject {
            push,
            ..row(BRANCH, "topic", &tree)
        });
        assert_eq!(at(&menu, Row::Push), can, "{push:?}");
    }
}

#[test]
fn a_name_on_heads_commit_greys_merge_and_rebase() {
    let tree = on_main();
    for (kind, name) in [(BRANCH, "feat"), (REMOTE, "origin/feat"), (TAG, "v1")] {
        let menu = said(&Subject {
            commit: Some(oid(HEAD)),
            ..row(kind, name, &tree)
        });
        assert_eq!(at(&menu, Row::Merge), greyed(Why::SameCommit), "{kind:?}");
        assert_eq!(at(&menu, Row::Rebase), greyed(Why::SameCommit), "{kind:?}");
    }
}

/// A tag named as the branch the tab is on is a tag: it brings history in
/// as any tag does.
#[test]
fn a_tag_named_as_the_current_branch_is_a_tag() {
    let tree = on_main();
    let menu = said(&row(TAG, "main", &tree));
    assert_eq!(at(&menu, Row::Merge), LIVE);
    assert_eq!(at(&menu, Row::Rebase), LIVE);
}

#[test]
fn a_branch_out_in_another_worktree_leads_there_and_asks_nothing() {
    let tree = stopped(on_main());
    let held = Subject {
        leads: Leads::Holder,
        far: tracking(Tracks::Together, 0),
        ..row(BRANCH, "feat", &tree)
    };
    let menu = said(&held);
    assert_eq!(at(&menu, Row::Reach), LIVE, "`Open`: the row leads there");
    assert!(
        !held.switch_asks(),
        "opening it asks nothing, moves held back or not"
    );
    assert_eq!(
        at(&menu, Row::DeleteBranch),
        greyed(Why::CheckedOutElsewhere)
    );
    assert_eq!(at(&menu, Row::DeleteBoth), greyed(Why::CheckedOutElsewhere));
    assert_eq!(
        at(&menu, Row::DeleteRemoteBranch),
        LIVE,
        "the reading goes all the same"
    );
    assert_eq!(
        at(&menu, Row::Pull),
        greyed(Why::CheckedOutElsewhere),
        "said before the operation: nothing done here lifts it"
    );
    assert_eq!(
        at(&menu, Row::WorktreeAdd),
        greyed(Why::CheckedOutElsewhere)
    );
}

/// The `!` follows the move held back — unmerged files with no operation
/// too —, not the operation's badge.
#[test]
fn a_move_held_back_asks_first() {
    let blocked = TreeStanding {
        moves_blocked: true,
        ..on_main()
    };
    assert!(row(BRANCH, "feat", &blocked).switch_asks());
    let badge = TreeStanding {
        op_text: "MERGING".into(),
        ..on_main()
    };
    assert!(!row(BRANCH, "feat", &badge).switch_asks());
}

#[test]
fn a_remote_rows_delete_ignores_who_holds_its_local_branch() {
    let tree = on_main();
    let menu = said(&Subject {
        leads: Leads::Holder,
        ..row(REMOTE, "origin/feat", &tree)
    });
    assert_eq!(at(&menu, Row::DeleteRemoteBranch), LIVE);
}

#[test]
fn a_drifted_reading_keeps_the_local_delete_and_greys_the_far_ones() {
    let tree = on_main();
    let menu = said(&Subject {
        far: tracking(Tracks::Drifted, 0),
        ..row(BRANCH, "feat", &tree)
    });
    assert_eq!(at(&menu, Row::DeleteBranch), LIVE);
    assert_eq!(
        at(&menu, Row::DeleteRemoteBranch),
        greyed(Why::ReadingDrifted)
    );
    assert_eq!(at(&menu, Row::DeleteBoth), greyed(Why::ReadingDrifted));
}

#[test]
fn a_stopped_operation_greys_what_starts_history_and_no_delete() {
    let tree = stopped(on_main());
    let feat = Subject {
        far: tracking(Tracks::Together, 0),
        ..row(BRANCH, "feat", &tree)
    };
    let menu = said(&feat);
    for row in [
        Row::BranchHere,
        Row::Merge,
        Row::Rebase,
        Row::Pull,
        Row::TagHere,
    ] {
        assert_eq!(at(&menu, row), greyed(Why::StoppedOperation), "{row:?}");
    }
    assert!(feat.switch_asks(), "the move asks first");
    assert_eq!(
        at(&menu, Row::Reach),
        LIVE,
        "and presses through to its question"
    );
    assert_eq!(at(&menu, Row::DeleteBranch), LIVE);
    assert_eq!(
        at(&menu, Row::WorktreeHere),
        LIVE,
        "nothing in this worktree is touched"
    );
}

/// Detached, or before HEAD is read, no branch moves: nothing is brought
/// in, the tab's comparison has no end, and every branch is aside.
#[test]
fn detached_there_is_no_branch_to_bring_anything_into() {
    for tree in [TreeStanding::detached(), TreeStanding::default()] {
        let menu = said(&Subject {
            far: tracking(Tracks::Together, 0),
            ..row(BRANCH, "main", &tree)
        });
        assert_eq!(at(&menu, Row::Merge), None);
        assert_eq!(at(&menu, Row::Rebase), None);
        assert_eq!(at(&menu, Row::Reach), LIVE, "the way back onto a branch");
        assert_eq!(at(&menu, Row::Pull), LIVE, "pulled aside");
        assert_eq!(at(&menu, Row::WorktreeAdd), LIVE, "no branch is out here");
        let remote = said(&row(REMOTE, "origin/main", &tree));
        assert_eq!(at(&remote, Row::Pull), None);
    }
}

#[test]
fn a_tag_moves_nowhere_but_starts_branches_sends_and_deletes() {
    let tree = on_main();
    let menu = said(&Subject {
        far: tag_far(TagSides::Here, "origin", TagDeleteHeld::No),
        ..row(TAG, "v1", &tree)
    });
    assert_eq!(at(&menu, Row::Reach), None);
    for row in [Row::BranchHere, Row::TagHere, Row::PushTag, Row::DeleteTag] {
        assert_eq!(at(&menu, row), LIVE, "{row:?}");
    }
}

#[test]
fn each_tag_row_needs_the_side_it_names() {
    let tree = on_main();
    let rows = |sides: TagSides| {
        let menu = said(&Subject {
            far: tag_far(sides, "origin", TagDeleteHeld::No),
            ..row(TAG, "v1", &tree)
        });
        [
            Row::PushTag,
            Row::DeleteTag,
            Row::DeleteRemoteTag,
            Row::DeleteTagBoth,
        ]
        .map(|row| at(&menu, row))
    };
    assert_eq!(rows(TagSides::Here), [LIVE, LIVE, None, None]);
    assert_eq!(
        rows(TagSides::Remote),
        [None, None, LIVE, None],
        "a tag only a remote has is not sent"
    );
    assert_eq!(rows(TagSides::Both), [LIVE, LIVE, LIVE, LIVE]);
    let unread = said(&row(TAG, "v1", &tree));
    assert_eq!(
        at(&unread, Row::DeleteTag),
        LIVE,
        "unread reads as a tag here"
    );
    assert_eq!(at(&unread, Row::DeleteRemoteTag), None);
}

#[test]
fn without_a_remote_a_tag_greys_what_reaches_over_there() {
    let tree = on_main();
    let menu = said(&Subject {
        far: tag_far(TagSides::Both, "", TagDeleteHeld::No),
        ..row(TAG, "v1", &tree)
    });
    for row in [Row::PushTag, Row::DeleteRemoteTag, Row::DeleteTagBoth] {
        assert_eq!(at(&menu, row), greyed(Why::NoRemote), "{row:?}");
    }
    assert_eq!(
        at(&menu, Row::DeleteTag),
        LIVE,
        "the name here goes all the same"
    );
    let unread = said(&row(TAG, "v1", &tree));
    assert_eq!(at(&unread, Row::PushTag), greyed(Why::NoRemote));
}

/// Held back over there, both rows that reach it grey and say why; the push
/// is not gated on drift — over a name elsewhere it is the leased overwrite.
#[test]
fn a_tag_held_back_over_there_greys_both_rows_that_reach_it() {
    let tree = on_main();
    for (held_back, why) in [
        (TagDeleteHeld::Drifted, Why::TagDrifted),
        (TagDeleteHeld::Unnamed, Why::TagUnnamed),
    ] {
        let menu = said(&Subject {
            far: tag_far(TagSides::Both, "origin", held_back),
            ..row(TAG, "v1", &tree)
        });
        assert_eq!(at(&menu, Row::DeleteRemoteTag), greyed(why));
        assert_eq!(at(&menu, Row::DeleteTagBoth), greyed(why));
        assert_eq!(at(&menu, Row::DeleteTag), LIVE);
        assert_eq!(at(&menu, Row::PushTag), LIVE);
    }
}

#[test]
fn a_stopped_operation_holds_no_tag_back() {
    let tree = stopped(on_main());
    let menu = said(&Subject {
        far: tag_far(TagSides::Both, "origin", TagDeleteHeld::No),
        ..row(TAG, "v1", &tree)
    });
    for row in [
        Row::PushTag,
        Row::DeleteTag,
        Row::DeleteRemoteTag,
        Row::DeleteTagBoth,
    ] {
        assert_eq!(at(&menu, row), LIVE, "{row:?}");
    }
}

/// What brings a stash back is on its row too: the details pane's band
/// shows the row chosen, not always the one right-clicked (デザイン規約
/// §メニュー).
#[test]
fn a_stash_is_applied_popped_or_dropped() {
    let tree = on_main();
    let menu = said(&row(STASH, "stash@{0}", &tree));
    assert_eq!(
        menu.keys().copied().collect::<Vec<_>>(),
        [Row::ApplyStash, Row::PopStash, Row::DropStash]
    );
}

/// The left pane's rows all name something: one that names nothing has no
/// menu.
#[test]
fn a_listed_row_naming_nothing_has_no_menu() {
    let tree = on_main();
    let named = row(None, "", &tree);
    assert_eq!(lines(named.open, &On::Ref(Facts::of_subject(&named))), None);
}

#[test]
fn a_tab_not_open_greys_every_row_under_one_reason() {
    let tree = on_main();
    let closed = Subject {
        open: false,
        far: tracking(Tracks::Together, 0),
        ..row(BRANCH, "feat", &tree)
    };
    let menu = said(&closed);
    assert_eq!(menu.len(), 14);
    assert!(
        menu.values().all(|&can| can == Can::No(Why::Opening)),
        "{menu:?}"
    );
    assert_eq!(closed.worktree_checkout(), None);
}

/// Another worktree leads to itself and asks nothing; the one the tab shows
/// leads nowhere and is not removed from under it; the lock is said first;
/// the repository's own is never removed.
#[test]
fn a_worktrees_row_leads_to_it_and_removes_it() {
    let blocked = TreeStanding {
        moves_blocked: true,
        ..on_main()
    };
    let other = |here: bool, locked: bool| Subject {
        worktree: Some(RowWorktree {
            main: false,
            here,
            locked,
        }),
        leads: if here { Leads::Nowhere } else { Leads::Itself },
        ..row(WORKTREE, "/w/side", &blocked)
    };
    let elsewhere = other(false, false);
    let menu = said(&elsewhere);
    assert_eq!(at(&menu, Row::Reach), LIVE);
    assert!(!elsewhere.switch_asks());
    assert_eq!(at(&menu, Row::RemoveWorktree), LIVE);
    for row in [
        Row::BranchHere,
        Row::Merge,
        Row::Rebase,
        Row::WorktreeHere,
        Row::TagHere,
    ] {
        assert_eq!(at(&menu, row), LIVE, "{row:?}: as its commit's");
    }
    assert_eq!(at(&menu, Row::DeleteBranch), None);
    let shown = said(&other(true, false));
    assert_eq!(at(&shown, Row::Reach), None);
    assert_eq!(at(&shown, Row::RemoveWorktree), greyed(Why::ShownHere));
    assert_eq!(
        at(&said(&other(true, true)), Row::RemoveWorktree),
        greyed(Why::Locked)
    );
    let own = said(&Subject {
        worktree: Some(RowWorktree {
            main: true,
            here: false,
            locked: false,
        }),
        leads: Leads::Itself,
        ..row(WORKTREE, "/w/repo", &blocked)
    });
    assert_eq!(at(&own, Row::RemoveWorktree), None);
}

/// What acts on a worktree itself, whatever branch it has out: its removal
/// alone — what a worktree's row still offers once the worktree has another
/// branch out than its menu was raised on (`RefRowMenu.standOn`).
#[test]
fn only_a_worktrees_removal_acts_on_the_worktree_itself() {
    let on_worktree: Vec<Row> = Row::ALL
        .iter()
        .copied()
        .filter(|row| row.on_worktree())
        .collect();
    assert_eq!(on_worktree, [Row::RemoveWorktree]);
}

/// The worktree a menu was raised on has left its branch where it has gone,
/// is on no branch, or on a branch of another name — a switch, or a rename
/// read on one listing before the other, in either order: the menu cannot
/// tell the two apart until both land, and holds the branch's rows grey.
#[test]
fn a_worktree_leaves_its_branch_for_any_other_name() {
    let on = WorktreeOut::Branch;
    assert!(!worktree_left("topic", on("topic")), "still out");
    assert!(worktree_left("topic", WorktreeOut::Gone), "removed outside");
    assert!(worktree_left("topic", WorktreeOut::Detached));
    assert!(worktree_left("topic", on("zz-other")), "switched");
    // `topic` renamed `topic-b` outside: the worktrees read first…
    assert!(worktree_left("topic", on("topic-b")));
    // …or the branches, the menu following the name on them…
    assert!(worktree_left("topic-b", on("topic")));
    // …and both read: the menu's branch is out again.
    assert!(!worktree_left("topic-b", on("topic-b")), "both read");
}

/// git keeps a branch to one worktree; the reasons come in order, then where
/// the new one would go.
#[test]
fn worktree_add_takes_out_a_branch_no_worktree_has() {
    let tree = on_main();
    // A stopped rebase leaves the tree detached, and git still counts the
    // branch it keeps as out here.
    let rebasing = TreeStanding::detached()
        .standing("REBASE 1/3")
        .keeping("feat", Keeper::Rebase);
    let free = row(BRANCH, "feat", &tree);
    assert_eq!(free.worktree_checkout(), Some(WorktreeCheckout::Branch));
    assert_eq!(at(&said(&free), Row::WorktreeAdd), LIVE);
    let cases = [
        (row(BRANCH, "main", &tree), Why::OnThisBranch),
        (
            Subject {
                leads: Leads::Holder,
                ..row(BRANCH, "feat", &tree)
            },
            Why::CheckedOutElsewhere,
        ),
        (row(BRANCH, "feat", &rebasing), Why::KeptBy(Keeper::Rebase)),
        (
            Subject {
                placing: Placing::Beside { taken: true },
                ..row(BRANCH, "feat", &tree)
            },
            Why::PlaceTaken,
        ),
    ];
    for (row, why) in cases {
        assert_eq!(at(&said(&row), Row::WorktreeAdd), greyed(why), "{why:?}");
    }
    let taken = said(&Subject {
        placing: Placing::Beside { taken: true },
        ..row(BRANCH, "feat", &tree)
    });
    assert_eq!(
        at(&taken, Row::WorktreeHere),
        LIVE,
        "the box names a folder of its own"
    );
    // Nowhere, each reason its own; before the worktree listing lands, no
    // place is known either way.
    for (placing, why) in [
        (Placing::Nowhere(Nowhere::Bare), Why::Nowhere(Nowhere::Bare)),
        (
            Placing::Nowhere(Nowhere::DriveRoot),
            Why::Nowhere(Nowhere::DriveRoot),
        ),
        (Placing::Unread, Why::WorktreesUnread),
    ] {
        let menu = said(&Subject {
            placing,
            ..row(BRANCH, "feat", &tree)
        });
        for row in [Row::WorktreeHere, Row::WorktreeAdd] {
            assert_eq!(at(&menu, row), greyed(why), "{row:?} {placing:?}");
        }
    }
}

/// Every place goes out in a word of its own and reads back as itself.
#[test]
fn every_placing_word_reads_back_as_what_wrote_it() {
    for placing in [
        Placing::Unread,
        Placing::Nowhere(Nowhere::Bare),
        Placing::Nowhere(Nowhere::DriveRoot),
        Placing::Beside { taken: false },
        Placing::Beside { taken: true },
    ] {
        assert_eq!(Placing::from_word(placing.word()), Some(placing));
    }
    assert_eq!(Placing::from_word(""), None);
    for history in [History::Straight, History::PastMerge, History::Off] {
        assert_eq!(History::from_word(history.word()), Some(history));
    }
    assert_eq!(History::from_word(""), None, "the window cannot say");
}

/// Any row with a commit but a stash makes a new branch out in a new
/// worktree, the current branch's too.
#[test]
fn any_commit_takes_a_new_branch_out_in_a_new_worktree() {
    let tree = on_main();
    for subject in [
        row(BRANCH, "main", &tree),
        row(REMOTE, "origin/feat", &tree),
        row(TAG, "v1", &tree),
        on_graph(row(None, "", &tree)),
    ] {
        let menu = said(&subject);
        assert_eq!(at(&menu, Row::WorktreeHere), LIVE, "{:?}", subject.kind);
    }
    let stash = said(&row(STASH, "stash@{0}", &tree));
    assert_eq!(at(&stash, Row::WorktreeHere), None);
}

/// A commit's row, a tag's and a worktree's on no branch have no branch of
/// their own to take out: `Create worktree here…` alone.
#[test]
fn a_row_with_no_branch_of_its_own_takes_none_out() {
    let tree = on_main();
    let detached_worktree = Subject {
        worktree: Some(RowWorktree {
            main: false,
            here: false,
            locked: false,
        }),
        leads: Leads::Itself,
        ..row(WORKTREE, "/w/detached", &tree)
    };
    for subject in [
        on_graph(row(None, "", &tree)),
        row(TAG, "v1", &tree),
        detached_worktree,
    ] {
        let kind = subject.kind;
        assert_eq!(subject.worktree_checkout(), None, "{kind:?}");
        let menu = said(&subject);
        assert_eq!(at(&menu, Row::WorktreeAdd), None, "{kind:?}");
        assert_eq!(at(&menu, Row::WorktreeHere), LIVE, "{kind:?}");
    }
}

#[test]
fn a_remote_branch_makes_its_local_one_only_where_there_is_none() {
    let tree = on_main();
    let remote = row(REMOTE, "origin/feat", &tree);
    assert_eq!(remote.worktree_checkout(), Some(WorktreeCheckout::Track));
    assert_eq!(at(&said(&remote), Row::WorktreeAdd), LIVE);
    for there in [
        Subject {
            local_listed: true,
            ..row(REMOTE, "origin/feat", &tree)
        },
        Subject {
            leads: Leads::Holder,
            ..row(REMOTE, "origin/feat", &tree)
        },
    ] {
        assert_eq!(there.worktree_checkout(), None, "its own row takes it out");
        assert_eq!(
            at(&said(&there), Row::WorktreeAdd),
            greyed(Why::LocalBranchThere)
        );
    }
}

/// HEAD on a branch with no commits yet has nothing to undo, rebase or
/// rewrite: `rebase` greys on the left pane's rows of another branch, a
/// remote branch and a tag, and a graph row's revert, rebase and rewrites
/// grey alike, while what takes the branch to a commit — cherry-pick,
/// merge, reset — stays live (`integrate_integration`'s periodic tests
/// hold merge's and rebase's answers; revert's, cherry-pick's and reset's
/// were measured on 2.55 — `TreeStanding::head_unborn`).
#[test]
fn an_unborn_head_greys_what_needs_its_history() {
    let unborn = TreeStanding {
        head_oid: String::new(),
        ..TreeStanding::on("fresh")
    };
    for kind in [BRANCH, REMOTE, TAG] {
        let listed = said(&Subject {
            commit: Some(oid(OTHER)),
            ..row(kind, "feat", &unborn)
        });
        assert_eq!(
            at(&listed, Row::Rebase),
            greyed(Why::HeadUnborn),
            "{kind:?}"
        );
        assert_eq!(at(&listed, Row::Merge), LIVE, "{kind:?}");
    }
    let graph = said(&on_graph(Subject {
        commit: Some(oid(OTHER)),
        ..row(None, "", &unborn)
    }));
    for row in [
        Row::Revert,
        Row::Squash,
        Row::Drop,
        Row::RebaseInteractive,
        Row::Rebase,
    ] {
        assert_eq!(at(&graph, row), greyed(Why::HeadUnborn), "{row:?}");
    }
    for row in [
        Row::CherryPick,
        Row::Merge,
        Row::ResetSoft,
        Row::ResetMixed,
        Row::ResetHard,
    ] {
        assert_eq!(at(&graph, row), LIVE, "{row:?}");
    }
}

/// A graph row's commit off the history of the branch HEAD is on has
/// nothing a rewrite can take: squash, drop and `rebase --interactive`
/// grey, while revert and cherry-pick — which act on top of HEAD — stay
/// live. Where the drawn rows cannot say, the rows stay live and git
/// answers.
#[test]
fn a_rewrite_greys_on_a_commit_off_heads_history() {
    let tree = on_main();
    let graph = |history| {
        said(&on_graph(Subject {
            history,
            ..row(None, "", &tree)
        }))
    };
    let rewrites = [Row::Squash, Row::Drop, Row::RebaseInteractive];
    let off = graph(Some(History::Off));
    for row in rewrites {
        assert_eq!(at(&off, row), greyed(Why::OffBranch), "{row:?}");
    }
    assert_eq!(at(&off, Row::Revert), LIVE);
    assert_eq!(at(&off, Row::CherryPick), LIVE);
    for history in [Some(History::Straight), Some(History::PastMerge), None] {
        let menu = graph(history);
        for row in rewrites {
            assert_eq!(at(&menu, row), LIVE, "{row:?} {history:?}");
        }
    }
}
