//! A local branch's upstream: set and taken off, where it stands — the
//! listing's word, and for the branch HEAD is on git's own count —, and
//! what its pull runs ([`crate::menu::pull_runs`]).

use super::testkit::*;
use crate::menu::{Can, Facts, Row, Subject, UpstreamShape, Why, pull_runs};
use crate::offers::{BranchFar, FarSide, Leads, PullRuns, Tracks, TreeStanding};
use crate::worktrees::Keeper;

/// An upstream configured is a row to take it off, listed or gone; with
/// none, git has no far side to go to. A gone one is no far side to pull
/// from either, and a branch here is pulled into the branch HEAD is on
/// alone: the fast-forward aside reaches a remote only.
#[test]
fn only_a_branch_with_an_upstream_has_one_to_take_off_or_pull_from() {
    let tree = on_main();
    let bare = said(&row(BRANCH, "feat", &tree));
    for row in [
        Row::UnsetUpstream,
        Row::Pull,
        Row::DeleteRemoteBranch,
        Row::DeleteBoth,
    ] {
        assert_eq!(at(&bare, row), None, "{row:?}");
    }
    let gone = said(&Subject {
        far: tracking(Tracks::Gone, 0),
        ..row(BRANCH, "feat", &tree)
    });
    assert_eq!(at(&gone, Row::UnsetUpstream), LIVE);
    assert_eq!(
        at(&gone, Row::Pull),
        None,
        "nothing over there to pull from"
    );
    let here = said(&Subject {
        far: tracking(Tracks::Here, 0),
        ..row(BRANCH, "feat", &tree)
    });
    assert_eq!(at(&here, Row::UnsetUpstream), LIVE);
    assert_eq!(at(&here, Row::Pull), None, "no remote to fast-forward from");
    assert_eq!(
        at(&gone, Row::DeleteRemoteBranch),
        None,
        "no reading to delete"
    );
}

/// Configuration, not the branch: the branch the tab is on and one another
/// worktree has out take their upstream too.
#[test]
fn the_current_branch_and_a_held_one_take_their_upstream_too() {
    let tree = on_main();
    for menu in [
        said(&Subject {
            far: tracking(Tracks::Together, 0),
            ..row(BRANCH, "main", &tree)
        }),
        said(&Subject {
            far: tracking(Tracks::Together, 0),
            leads: Leads::Holder,
            ..row(BRANCH, "feat", &tree)
        }),
    ] {
        assert_eq!(at(&menu, Row::SetUpstream), LIVE);
        assert_eq!(at(&menu, Row::UnsetUpstream), LIVE);
    }
}

#[test]
fn there_is_nothing_to_be_measured_against_without_a_remote() {
    let tree = on_main();
    let menu = said(&Subject {
        far: FarSide::Branch(BranchFar::default()),
        ..row(BRANCH, "feat", &tree)
    });
    assert_eq!(at(&menu, Row::SetUpstream), greyed(Why::NoRemote));
}

#[test]
fn only_a_local_branch_is_measured_against_anything() {
    let tree = on_main();
    for subject in [
        row(REMOTE, "origin/feat", &tree),
        row(TAG, "v1", &tree),
        row(STASH, "stash@{0}", &tree),
        on_graph(row(None, "", &tree)),
    ] {
        let kind = subject.kind;
        let menu = said(&Subject {
            far: tracking(Tracks::Together, 0),
            ..subject
        });
        assert_eq!(at(&menu, Row::SetUpstream), None, "{kind:?}");
        assert_eq!(at(&menu, Row::UnsetUpstream), None, "{kind:?}");
    }
}

/// The branch HEAD is on pulls by the upstream the tree reads — the far
/// end of the window's ↑↓, which the remote branch's row reads too —, so
/// a listing a tick behind it moves neither row's `pull`. Another branch
/// pulls by the upstream the listing says it has.
#[test]
fn the_branch_head_is_on_pulls_by_the_trees_upstream() {
    let tracked_by_tree = on_main();
    let untracked_by_tree = TreeStanding {
        upstream: String::new(),
        ..on_main()
    };
    let untracked = FarSide::Branch(BranchFar::default());
    let pull = |tree: &TreeStanding, far: FarSide, name| {
        at(
            &said(&Subject {
                far,
                ..row(BRANCH, name, tree)
            }),
            Row::Pull,
        )
    };
    assert_ne!(
        pull(&tracked_by_tree, untracked.clone(), "main"),
        None,
        "the tree says main tracks origin/main"
    );
    assert_eq!(
        pull(&untracked_by_tree, tracking(Tracks::Together, 0), "main"),
        None,
        "the tree says main tracks nothing, whatever the listing still says"
    );
    assert_ne!(
        pull(&untracked_by_tree, tracking(Tracks::Together, 0), "topic"),
        None,
        "another branch, by the listing"
    );
    assert_eq!(pull(&tracked_by_tree, untracked.clone(), "topic"), None);
}

/// Where an upstream stands is the listing's where it has a row for it: a
/// branch here only where the listing names one. The branch HEAD is on,
/// with no row to answer for its upstream — one the listing has not read,
/// or one fetched outside the remote branches —, stands on git's own count
/// on the tree: counted, the ref is there; not, it is gone — unless it has
/// no commit yet, which git counts nothing against. Another branch has the
/// listing's word alone.
#[test]
fn where_an_upstream_stands_is_the_listings_or_gits_count() {
    let counted = on_main();
    let uncounted = TreeStanding {
        upstream_counted: false,
        ..on_main()
    };
    let shape = |far: FarSide, name, tree: &TreeStanding| {
        Facts::of_subject(&Subject {
            far,
            ..row(BRANCH, name, tree)
        })
        .shape
        .upstream
    };
    let unlisted = FarSide::Branch(BranchFar::default());
    for far in [unlisted.clone(), tracking(Tracks::Gone, 0)] {
        assert_eq!(
            shape(far.clone(), "main", &counted),
            Some(UpstreamShape::Counted)
        );
        assert_eq!(shape(far, "main", &uncounted), Some(UpstreamShape::Gone));
    }
    let unborn = TreeStanding {
        head_oid: String::new(),
        ..uncounted.clone()
    };
    assert_eq!(
        shape(unlisted.clone(), "main", &unborn),
        Some(UpstreamShape::Counted),
        "no commit to count with: not a sign the ref is gone"
    );
    assert_eq!(shape(unlisted, "topic", &counted), None);
    assert_eq!(
        shape(tracking(Tracks::Gone, 0), "topic", &counted),
        Some(UpstreamShape::Gone)
    );
    for name in ["main", "topic"] {
        for tree in [&counted, &uncounted] {
            assert_eq!(
                shape(tracking(Tracks::Here, 0), name, tree),
                Some(UpstreamShape::Here)
            );
            assert_eq!(
                shape(tracking(Tracks::Drifted, 0), name, tree),
                Some(UpstreamShape::Read)
            );
        }
    }
}

/// `git pull` on the two ends of the comparison the tab shows: its branch
/// and the remote branch it tracks — no other remote row, where a pull
/// would move a branch the row does not name. Greyed while an operation
/// stands, then while the two have both moved.
#[test]
fn git_pull_runs_on_the_two_ends_of_the_tabs_own_comparison() {
    let ends = |tree: &TreeStanding| -> Vec<Option<Can>> {
        let current = Subject {
            commit: Some(oid(HEAD)),
            far: tracking(Tracks::Together, 0),
            ..row(BRANCH, "main", tree)
        };
        let upstream = row(REMOTE, "origin/main", tree);
        [current, upstream]
            .iter()
            .map(|end| {
                assert_eq!(pull_runs(&Facts::of_subject(end).shape), PullRuns::Here);
                at(&said(end), Row::Pull)
            })
            .collect()
    };
    assert_eq!(ends(&on_main()), [LIVE, LIVE]);
    let diverged = TreeStanding {
        diverged: true,
        ..on_main()
    };
    assert_eq!(ends(&diverged), [greyed(Why::Diverged); 2]);
    assert_eq!(
        ends(&stopped(diverged)),
        [greyed(Why::StoppedOperation); 2],
        "the operation is said first"
    );
    let tree = on_main();
    assert_eq!(
        at(&said(&row(REMOTE, "origin/feat", &tree)), Row::Pull),
        None
    );
}

/// The tab's own pull with no reading of its upstream: from a branch here
/// it is `git pull`'s merge, greyed only while an operation stands; from a
/// gone one — git counts nothing against it — there is none, as on any
/// other branch: as of the last fetch there is nothing to pull from
/// (`pull_integration`'s periodic tests hold what git does with each). One
/// fetched outside the remote branches, which no row of the listing answers
/// to either, git counts against: its `git pull` stands.
#[test]
fn the_tabs_own_pull_is_one_from_a_ref_git_counts_and_none_from_one_gone() {
    fn current(tree: &TreeStanding, far: FarSide) -> Subject<'_> {
        Subject {
            commit: Some(oid(HEAD)),
            far,
            ..row(BRANCH, "main", tree)
        }
    }
    let pull = |subject: &Subject<'_>| at(&said(subject), Row::Pull);
    let tree = on_main();
    assert_eq!(pull(&current(&tree, tracking(Tracks::Here, 0))), LIVE);
    let halted = stopped(on_main());
    assert_eq!(
        pull(&current(&halted, tracking(Tracks::Here, 0))),
        greyed(Why::StoppedOperation)
    );
    let gone = TreeStanding {
        upstream_counted: false,
        ..on_main()
    };
    assert_eq!(pull(&current(&gone, tracking(Tracks::Gone, 0))), None);
    let mirrored = current(&tree, tracking(Tracks::Gone, 0));
    assert_eq!(pull(&mirrored), LIVE);
    // With no commit yet, no row of the branch listing and no count: the
    // pull is how the branch takes its upstream's first commits.
    let unborn = TreeStanding {
        head_oid: String::new(),
        ..gone
    };
    let first = Subject {
        commit: None,
        far: FarSide::Branch(BranchFar::default()),
        ..row(BRANCH, "main", &unborn)
    };
    assert_eq!(pull(&first), LIVE);
    assert_eq!(
        pull_runs(&Facts::of_subject(&mirrored).shape),
        PullRuns::Here
    );
}

/// Every other local branch with an upstream is fast-forwarded aside — a
/// tab tracking nothing too —, greyed where it has commits its upstream
/// lacks.
#[test]
fn every_other_branch_with_an_upstream_is_fast_forwarded_aside() {
    for tree in [
        on_main(),
        TreeStanding {
            upstream: String::new(),
            ..on_main()
        },
        TreeStanding {
            diverged: true,
            ..on_main()
        },
    ] {
        let feat = Subject {
            far: tracking(Tracks::Together, 0),
            ..row(BRANCH, "feat", &tree)
        };
        assert_eq!(pull_runs(&Facts::of_subject(&feat).shape), PullRuns::Aside);
        assert_eq!(at(&said(&feat), Row::Pull), LIVE, "{tree:?}");
    }
    let tree = on_main();
    let ahead = |leads: Leads| {
        let menu = said(&Subject {
            far: tracking(Tracks::Together, 2),
            leads,
            ..row(BRANCH, "feat", &tree)
        });
        at(&menu, Row::Pull)
    };
    assert_eq!(ahead(Leads::Nowhere), greyed(Why::OwnCommits));
    assert_eq!(
        ahead(Leads::Holder),
        greyed(Why::CheckedOutElsewhere),
        "the holder is said first: the branch is not switched to from here"
    );
}

/// A stopped rebase leaves HEAD on no branch, and the window compares
/// nothing: the branch it keeps is pulled on its own row, greyed for the
/// operation, and no remote row is an end of a comparison — the status
/// names no upstream while HEAD is on none, and a reading that named the
/// kept branch's still would not make one. That remote row is brought into
/// the kept branch, greyed for the operation, as any.
#[test]
fn a_stopped_rebase_leaves_its_branch_aside() {
    let rebasing = TreeStanding::detached()
        .standing("REBASE 1/3")
        .keeping("main", Keeper::Rebase);
    let main = Subject {
        far: tracking(Tracks::Together, 0),
        ..row(BRANCH, "main", &rebasing)
    };
    assert_eq!(pull_runs(&Facts::of_subject(&main).shape), PullRuns::Aside);
    assert_eq!(at(&said(&main), Row::Pull), greyed(Why::StoppedOperation));
    for upstream in ["", "origin/main"] {
        let tree = TreeStanding {
            upstream: upstream.into(),
            ..rebasing.clone()
        };
        let remote = said(&row(REMOTE, "origin/main", &tree));
        assert_eq!(at(&remote, Row::Pull), None, "{upstream:?}");
        assert_eq!(
            at(&remote, Row::Merge),
            greyed(Why::StoppedOperation),
            "{upstream:?}"
        );
    }
}
