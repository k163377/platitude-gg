//! What a graph row's menu says beside its name's: its commit's rows,
//! a stash's drop alone, and another worktree's changes brought here.

use std::collections::BTreeMap;

use super::testkit::*;
use crate::menu::{Can, History, On, Row, Seen, Subject, Why, lines};
use crate::offers::{RefKind, Tracks, TreeStanding};

/// A chip naming a worktree the listing has not read yet — the two reads
/// land apart —: the graph row stands on its commit until it is read,
/// rather than opening nothing; the left pane's row offers nothing.
#[test]
fn a_worktree_chip_not_read_yet_stands_on_its_commit() {
    let tree = on_main();
    let unread = row(WORKTREE, "/w/side", &tree);
    let graph = said(&on_graph(unread.clone()));
    assert_eq!(at(&graph, Row::CherryPick), LIVE);
    assert_eq!(at(&graph, Row::RemoveWorktree), None);
    assert!(said(&unread).is_empty());
}

#[test]
fn a_graph_row_naming_no_ref_has_its_commits_rows_alone() {
    let tree = on_main();
    let menu = said(&on_graph(row(None, "", &tree)));
    assert_eq!(
        menu.keys().copied().collect::<Vec<_>>(),
        [
            Row::BranchHere,
            Row::CherryPick,
            Row::Revert,
            Row::Squash,
            Row::Drop,
            Row::RebaseInteractive,
            Row::Merge,
            Row::Rebase,
            Row::ResetSoft,
            Row::ResetMixed,
            Row::ResetHard,
            Row::WorktreeHere,
            Row::TagHere,
        ]
    );
    assert!(menu.values().all(|&can| can == Can::Yes), "{menu:?}");
}

/// A graph row whose chip draws a branch carries its commit's rows beside
/// the name's: copied and undone, rewritten, brought in and taken to.
#[test]
fn a_graph_rows_commit_brings_its_rows_beside_the_names() {
    let tree = on_main();
    let feat = Subject {
        far: tracking(Tracks::Together, 0),
        ..row(BRANCH, "feat", &tree)
    };
    let listed = said(&feat);
    let graph = said(&on_graph(feat));
    for row in [
        Row::CherryPick,
        Row::Revert,
        Row::Squash,
        Row::Drop,
        Row::RebaseInteractive,
        Row::ResetSoft,
        Row::ResetMixed,
        Row::ResetHard,
    ] {
        assert_eq!(at(&listed, row), None, "{row:?}: not in the left pane");
        assert_eq!(at(&graph, row), LIVE, "{row:?}");
    }
    for (row, can) in &listed {
        assert_eq!(graph.get(row), Some(can), "{row:?}: the name's as it is");
    }
}

/// On HEAD's own commit `merge`, `rebase` and `cherry-pick` bring nothing
/// in, and `reset` takes the branch nowhere: they stand greyed, as on a
/// name on HEAD's commit, each saying its own; the rest act on it.
#[test]
fn heads_own_row_greys_what_would_move_nothing() {
    let tree = on_main();
    let menu = said(&on_graph(Subject {
        commit: Some(oid(HEAD)),
        history: Some(History::Straight),
        ..row(BRANCH, "main", &tree)
    }));
    for row in [Row::Merge, Row::Rebase, Row::CherryPick] {
        assert_eq!(at(&menu, row), greyed(Why::SameCommit), "{row:?}");
    }
    for row in [Row::ResetSoft, Row::ResetMixed, Row::ResetHard] {
        assert_eq!(at(&menu, row), greyed(Why::AlreadyThere), "{row:?}");
    }
    for row in [Row::Revert, Row::Squash, Row::Drop, Row::RebaseInteractive] {
        assert_eq!(at(&menu, row), LIVE, "{row:?}");
    }
}

/// A commit behind HEAD on its branch is one the branch has already: a
/// `merge` brings nothing in (`Already up to date`), and a `rebase` onto it
/// moves nothing where the branch runs down to it straight (`Current branch
/// … is up to date`), so they stand greyed saying so, on a graph row and on
/// a name alike. Past a merge the rebase lays the branch again in one line,
/// and stays live. A `cherry-pick` stays live either way — what git makes of
/// it is in the lines since: a commit again where they undid it, a
/// conflict where they wrote over it. `reset` takes the branch back to it
/// and the rewrites reach it; where the drawn rows cannot tell, git answers
/// at the press.
#[test]
fn a_commit_the_branch_has_already_greys_what_would_bring_it_in() {
    let tree = on_main();
    let behind = |subject: Subject<'_>, history| {
        said(&Subject {
            commit: Some(oid(OTHER)),
            history,
            ..subject
        })
    };
    let graph = behind(on_graph(row(None, "", &tree)), Some(History::Straight));
    let name = behind(row(TAG, "v1", &tree), Some(History::Straight));
    for menu in [&graph, &name] {
        for row in [Row::Merge, Row::Rebase] {
            assert_eq!(at(menu, row), greyed(Why::InHistory), "{row:?}");
        }
    }
    for row in [
        Row::CherryPick,
        Row::Revert,
        Row::Squash,
        Row::Drop,
        Row::RebaseInteractive,
        Row::ResetSoft,
        Row::ResetMixed,
        Row::ResetHard,
    ] {
        assert_eq!(at(&graph, row), LIVE, "{row:?}");
    }
    let past_merge = behind(on_graph(row(None, "", &tree)), Some(History::PastMerge));
    let name_past_merge = behind(row(TAG, "v1", &tree), Some(History::PastMerge));
    for menu in [&past_merge, &name_past_merge] {
        assert_eq!(at(menu, Row::Merge), greyed(Why::InHistory));
        assert_eq!(at(menu, Row::Rebase), LIVE, "laid again in one line");
    }
    assert_eq!(at(&past_merge, Row::CherryPick), LIVE);
    let unknown = behind(on_graph(row(None, "", &tree)), None);
    for row in [Row::Merge, Row::Rebase, Row::CherryPick] {
        assert_eq!(at(&unknown, row), LIVE, "{row:?}: git answers");
    }
}

/// Detached with nothing stopped, no branch moves: the commit is copied or
/// undone on HEAD, and nothing is brought in, rewritten or taken anywhere.
#[test]
fn detached_a_commit_is_copied_or_undone_alone() {
    let tree = TreeStanding::detached();
    let menu = said(&on_graph(row(None, "", &tree)));
    assert_eq!(at(&menu, Row::CherryPick), LIVE);
    assert_eq!(at(&menu, Row::Revert), LIVE);
    for row in [
        Row::Squash,
        Row::Drop,
        Row::RebaseInteractive,
        Row::Merge,
        Row::Rebase,
        Row::ResetSoft,
        Row::ResetMixed,
        Row::ResetHard,
    ] {
        assert_eq!(at(&menu, row), None, "{row:?}");
    }
}

/// The discard log's entry has only what adds and moves no ref, aimed at no
/// name; what stays greys as anywhere.
#[test]
fn the_discard_logs_entry_has_only_what_adds() {
    let tree = on_main();
    let standing = on_main().standing("MERGING");
    let entry = |kind: Option<RefKind>, name: &'static str, tree| {
        said(&Subject {
            seen: Seen::Graph { provisional: true },
            ..row(kind, name, tree)
        })
    };
    for menu in [entry(None, "", &tree), entry(BRANCH, "feat", &tree)] {
        assert_eq!(
            menu.keys().copied().collect::<Vec<_>>(),
            [
                Row::BranchHere,
                Row::CherryPick,
                Row::WorktreeHere,
                Row::TagHere
            ]
        );
    }
    assert_eq!(
        at(&entry(None, "", &standing), Row::CherryPick),
        greyed(Why::StoppedOperation)
    );
}

/// A stash's row has the left pane's rows on the graph too: brought back
/// (`apply` / `pop`) or dropped. A stash write waits on the queue alone: no
/// operation greys it.
#[test]
fn a_stash_has_its_apply_pop_and_drop_on_the_graph_too() {
    let tree = on_main();
    let menu = said(&on_graph(row(STASH, "stash@{0}", &tree)));
    let rows = [Row::ApplyStash, Row::PopStash, Row::DropStash];
    assert_eq!(menu.keys().copied().collect::<Vec<_>>(), rows);
    let rebasing = TreeStanding::detached().standing("REBASE 1/3");
    let menu = said(&on_graph(row(STASH, "stash@{0}", &rebasing)));
    for row in rows {
        assert_eq!(at(&menu, row), LIVE, "{row:?}");
    }
}

/// Another worktree's uncommitted row leads to that worktree and brings its
/// changes here, greyed while a conflict stands on either side — here
/// first —, and while the tab opens.
#[test]
fn another_worktrees_changes_come_here_but_over_a_conflict() {
    let said = |open: bool, here: bool, there: bool| -> BTreeMap<Row, Can> {
        let on = On::Carried {
            conflicts_here: here,
            conflicts_there: there,
        };
        lines(open, &on)
            .unwrap_or_default()
            .into_iter()
            .map(|line| (line.row, line.can))
            .collect()
    };
    let carried = |here: bool, there: bool| said(true, here, there);
    let menu = carried(false, false);
    assert_eq!(
        menu.keys().copied().collect::<Vec<_>>(),
        [Row::Reach, Row::ApplyChanges]
    );
    assert!(menu.values().all(|&can| can == Can::Yes), "{menu:?}");
    let opening = said(false, false, false);
    assert!(
        opening.values().all(|&can| can == Can::No(Why::Opening)),
        "{opening:?}"
    );
    assert_eq!(
        at(&carried(true, false), Row::ApplyChanges),
        greyed(Why::ConflictsHere)
    );
    assert_eq!(
        at(&carried(false, true), Row::ApplyChanges),
        greyed(Why::ConflictsThere)
    );
    assert_eq!(
        at(&carried(true, true), Row::ApplyChanges),
        greyed(Why::ConflictsHere)
    );
}
