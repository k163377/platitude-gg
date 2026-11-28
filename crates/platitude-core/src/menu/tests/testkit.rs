//! Shared by the test modules beside it: the rows an open tab hands a
//! menu, and what the menu says on each.

use std::collections::BTreeMap;

use crate::Oid;
use crate::menu::{Can, Facts, On, Placing, Row, Seen, Subject, Why, lines};
use crate::offers::{
    BranchFar, FarSide, Leads, RefKind, TagDeleteHeld, TagFar, TagSides, Tracks, TreeStanding,
};
use crate::remote::PushStanding;

pub(super) const HEAD: &str = "2222222222222222222222222222222222222222";
pub(super) const OTHER: &str = "1111111111111111111111111111111111111111";

pub(super) const BRANCH: Option<RefKind> = Some(RefKind::Branch);
pub(super) const REMOTE: Option<RefKind> = Some(RefKind::Remote);
pub(super) const TAG: Option<RefKind> = Some(RefKind::Tag);
pub(super) const STASH: Option<RefKind> = Some(RefKind::Stash);
pub(super) const WORKTREE: Option<RefKind> = Some(RefKind::Worktree);

pub(super) const LIVE: Option<Can> = Some(Can::Yes);

pub(super) fn greyed(why: Why) -> Option<Can> {
    Some(Can::No(why))
}

pub(super) fn oid(hex: &str) -> Oid {
    Oid::from_hex_str(hex).expect("a full object name")
}

/// On `main` at [`HEAD`], tracking `origin/main` and level with it.
pub(super) fn on_main() -> TreeStanding {
    TreeStanding {
        head_oid: HEAD.into(),
        upstream: "origin/main".into(),
        upstream_counted: true,
        ..TreeStanding::on("main")
    }
}

/// `tree` with an operation stopped in it, which holds moves back too.
pub(super) fn stopped(tree: TreeStanding) -> TreeStanding {
    tree.standing("MERGING")
}

/// A local branch's far side, in a repository with a remote: an upstream
/// standing where `tracks` says, with `ahead` commits of its own on top.
pub(super) fn tracking(tracks: Tracks, ahead: u32) -> FarSide {
    FarSide::Branch(BranchFar {
        tracks,
        has_remote: true,
        ahead,
    })
}

/// A tag's far side: the sides holding it, the remote a push goes to (empty
/// for none), and what holds its deletes over there back.
pub(super) fn tag_far(sides: TagSides, remote: &str, held_back: TagDeleteHeld) -> FarSide {
    FarSide::Tag(TagFar {
        sides: Some(sides),
        push_remote: remote.into(),
        held_back,
    })
}

/// The row `name` of `kind` on `tree` in the left pane, as an open tab
/// hands it: on another commit than HEAD's, nothing over there read,
/// nothing holding it, a place for a new worktree known, a push ready to
/// go.
pub(super) fn row<'a>(kind: Option<RefKind>, name: &'a str, tree: &'a TreeStanding) -> Subject<'a> {
    Subject {
        seen: Seen::Listed,
        kind,
        name,
        commit: Some(oid(OTHER)),
        history: None,
        open: true,
        tree,
        leads: Leads::Nowhere,
        far: FarSide::Unread,
        local_listed: false,
        worktree: None,
        placing: Placing::Beside { taken: false },
        push: PushStanding::Ready,
    }
}

/// `row` as a graph row hands it: the commit's rows on it as well as the
/// name's.
pub(super) fn on_graph(row: Subject<'_>) -> Subject<'_> {
    Subject {
        seen: Seen::Graph { provisional: false },
        ..row
    }
}

/// What every row of the menu on `row` says; a row the menu does not have
/// is absent.
pub(super) fn said(row: &Subject<'_>) -> BTreeMap<Row, Can> {
    lines(row.open, &On::Ref(Facts::of_subject(row)))
        .unwrap_or_default()
        .into_iter()
        .map(|line| (line.row, line.can))
        .collect()
}

pub(super) fn at(menu: &BTreeMap<Row, Can>, row: Row) -> Option<Can> {
    menu.get(&row).copied()
}
