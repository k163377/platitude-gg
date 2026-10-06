//! Which commits a delete takes off the graph, and how what stays is laid
//! out without them.

use std::collections::HashSet;
use std::sync::Arc;

use super::leaving::{Holders, Walked, doomed, footer_without, lay_walked};
use super::model::LabelIndex;
use super::relay::Standing;
use super::{BranchItem, Footer, LabelKind, LogRow, RefLabel, RefsSnapshot, RelaidRow, RowPrint};
use crate::graph::GraphBuilder;
use crate::oid::Oid;

fn oid(n: u8) -> Oid {
    Oid::from_hex_str(&format!("{n:02x}").repeat(20)).unwrap()
}

/// Rows as a walk gives them, children first: `(id, parents, stash)`.
fn walk(rows: &[(u8, &[u8], bool)]) -> Vec<LogRow> {
    let mut builder = GraphBuilder::new();
    rows.iter()
        .map(|(id, parents, stash)| {
            let mine = oid(*id);
            let parents: Box<[Oid]> = parents.iter().map(|p| oid(*p)).collect();
            let g = builder.push_ids(&mine, &parents, *stash);
            LogRow {
                row: g.row,
                oid_hex: mine.to_hex(),
                short_sha: mine.short_hex(8),
                author: "Test".to_string(),
                author_email: "test@example.com".to_string(),
                co_authors: Vec::new(),
                time: 1_700_000_000 + i64::from(*id),
                subject: format!("commit {id:02x}"),
                body: String::new(),
                node_lane: g.node_lane,
                node_color: g.node_color,
                width: g.width,
                segments: g.segments,
                labels: Vec::new(),
                stash_ref: if *stash {
                    "stash@{0}".to_string()
                } else {
                    String::new()
                },
                published: false,
                carried: None,
                parents,
                provisional: false,
            }
        })
        .collect()
}

/// main: 1 ─ 2 ─ 3; topic forks at 2: 4 ─ 5. The walk puts the topic's tip
/// first.
fn forked() -> Vec<(u8, &'static [u8], bool)> {
    vec![
        (5, &[4], false),
        (3, &[2], false),
        (4, &[2], false),
        (2, &[1], false),
        (1, &[], false),
    ]
}

fn ids(set: &HashSet<Oid>) -> Vec<u8> {
    let mut out: Vec<u8> = (1..=9).filter(|n| set.contains(&oid(*n))).collect();
    out.sort_unstable();
    out
}

fn gone(rows: &[(u8, &[u8], bool)], tips: &[u8], held: &[u8]) -> Vec<u8> {
    let walked = Walked::of(&walk(rows));
    let tips: Vec<Oid> = tips.iter().map(|n| oid(*n)).collect();
    let held: HashSet<Oid> = held.iter().map(|n| oid(*n)).collect();
    ids(&doomed(&walked, &tips, |oid, stash| {
        stash || held.contains(oid)
    }))
}

#[test]
fn a_branch_of_its_own_goes_down_to_where_it_forked() {
    assert_eq!(gone(&forked(), &[5], &[3]), [4, 5]);
}

#[test]
fn a_commit_another_name_stands_on_stays_and_so_does_all_below_it() {
    assert_eq!(gone(&forked(), &[5], &[3, 5]), Vec::<u8>::new());
    assert_eq!(
        gone(&forked(), &[5], &[3, 4]),
        [5],
        "only the tip went: a name holds its parent"
    );
}

/// A branch stacked on the one going holds it through its own commits,
/// though no name stands on the commits in between.
#[test]
fn a_commit_a_kept_branch_reaches_stays() {
    let mut stacked = vec![(6, &[5][..], false)];
    stacked.extend(forked());
    assert_eq!(gone(&stacked, &[5], &[3, 6]), Vec::<u8>::new());
}

/// A stash is a starting point of the walk: taken on the branch going, it
/// holds the commit it was taken on.
#[test]
fn a_stash_holds_the_commit_it_was_taken_on() {
    let mut stashed = vec![(7, &[4][..], true)];
    stashed.extend(forked());
    assert_eq!(gone(&stashed, &[5], &[3]), [5]);
}

/// Two deletes out at once each take their own.
#[test]
fn two_names_leaving_take_both_runs() {
    assert_eq!(gone(&forked(), &[5, 3], &[]), [1, 2, 3, 4, 5]);
    assert_eq!(gone(&forked(), &[5, 3], &[1]), [2, 3, 4, 5]);
}

fn label(text: &str, kind: LabelKind) -> RefLabel {
    RefLabel {
        text: text.into(),
        kind,
        has_remote: false,
        is_head: false,
        here: true,
        remote: String::new(),
        held_elsewhere: false,
        locked: false,
    }
}

/// A local branch as the snapshot lists it, measured against `upstream`.
fn branch(short: &str, upstream: &str, drifted: bool) -> BranchItem {
    BranchItem {
        short: short.into(),
        full: format!("refs/heads/{short}").into(),
        oid: oid(5),
        has_remote: !upstream.is_empty(),
        is_head: false,
        upstream: upstream.into(),
        upstream_gone: crate::Name::default(),
        upstream_oid: None,
        upstream_drifted: drifted,
        held_elsewhere: false,
        tracked_by: crate::Name::default(),
        ahead: 0,
        behind: 0,
    }
}

fn holders(locals: Vec<BranchItem>, tags: bool) -> Holders {
    Holders {
        snapshot: Some(Arc::new(RefsSnapshot {
            locals,
            ..RefsSnapshot::default()
        })),
        head_tip: None,
        incoming: Vec::new(),
        detached: Vec::new(),
        shown_tips: Vec::new(),
        tags,
    }
}

/// Whether commit 5, wearing `chips`, stays while `leaving` goes.
fn held(holders: &Holders, chips: Vec<RefLabel>, leaving: &[&str]) -> bool {
    let labels = LabelIndex::from_pairs(chips.into_iter().map(|chip| (oid(5), chip)).collect());
    holders.hold(&oid(5), false, &labels, &leaving.iter().copied().collect())
}

/// A remote branch on the same commit rides on the local chip, and holds
/// the commit when only the local branch goes.
#[test]
fn a_remote_branch_folded_into_the_chip_holds_until_it_goes_too() {
    let chip = || {
        vec![RefLabel {
            has_remote: true,
            ..label("topic", LabelKind::LocalBranch)
        }]
    };
    let folded = holders(vec![branch("topic", "origin/topic", false)], true);
    assert!(held(&folded, chip(), &["refs/heads/topic"]));
    assert!(
        !held(
            &folded,
            chip(),
            &["refs/heads/topic", "refs/remotes/origin/topic"]
        ),
        "Delete both takes both"
    );
    // Drifted, the remote branch is a chip on a row of its own.
    let drifted = holders(vec![branch("topic", "origin/topic", true)], true);
    assert!(!held(&drifted, chip(), &["refs/heads/topic"]));
    assert!(held(
        &drifted,
        vec![label("origin/topic", LabelKind::RemoteBranch)],
        &["refs/heads/topic"]
    ));
}

/// A tag starts the walk only where this repository holds it and the
/// graph walks tags at all.
#[test]
fn a_tag_holds_only_where_the_walk_starts_from_it() {
    let shown = holders(Vec::new(), true);
    assert!(held(&shown, vec![label("v1", LabelKind::Tag)], &[]));
    assert!(!held(
        &shown,
        vec![label("v1", LabelKind::Tag)],
        &["refs/tags/v1"]
    ));
    let theirs = RefLabel {
        here: false,
        ..label("v1", LabelKind::Tag)
    };
    assert!(!held(&shown, vec![theirs], &[]), "a tag only a remote has");
    assert!(
        !held(
            &holders(Vec::new(), false),
            vec![label("v1", LabelKind::Tag)],
            &[]
        ),
        "the tags are not walked"
    );
}

#[test]
fn head_a_stash_a_merge_side_and_a_worktree_hold_what_they_stand_on() {
    let none = LabelIndex::default();
    let nothing = HashSet::new();
    let plain = holders(Vec::new(), true);
    assert!(!plain.hold(&oid(5), false, &none, &nothing));
    assert!(plain.hold(&oid(5), true, &none, &nothing), "a stash");
    assert!(held(&plain, vec![label("HEAD", LabelKind::Head)], &[]));
    assert!(held(
        &plain,
        vec![label("worktree", LabelKind::Worktree)],
        &[]
    ));
    for standing in [
        Holders {
            head_tip: Some(oid(5)),
            ..holders(Vec::new(), true)
        },
        Holders {
            incoming: vec![oid(5)],
            ..holders(Vec::new(), true)
        },
        Holders {
            detached: vec![oid(5)],
            ..holders(Vec::new(), true)
        },
        Holders {
            shown_tips: vec![oid(5)],
            ..holders(Vec::new(), true)
        },
    ] {
        assert!(standing.hold(&oid(5), false, &none, &nothing));
    }
}

#[test]
fn the_footer_counts_fewer_only_where_the_window_held_every_commit() {
    let gone: HashSet<Oid> = [oid(4), oid(5)].into();
    let whole = Footer {
        walked: 7,
        truncated: false,
    };
    let full = Footer {
        walked: 2000,
        truncated: true,
    };
    assert_eq!(footer_without(whole, &gone).walked, 5);
    assert_eq!(footer_without(full, &gone), full);
}

fn no_standing() -> Standing {
    Standing {
        pending: None,
        carried: Arc::new(Vec::new()),
    }
}

/// What stays is laid as a walk that never saw the commits would lay it:
/// the lanes a fresh builder gives the same rows.
#[test]
fn what_stays_is_laid_as_if_the_commits_had_never_been_walked() {
    let walked = Walked::of(&walk(&forked()));
    let gone: HashSet<Oid> = [oid(5), oid(4)].into();

    let laid = lay_walked(
        &walked,
        &no_standing(),
        None,
        &gone,
        &LabelIndex::default(),
        true,
    );

    let fresh = walk(&[(3, &[2], false), (2, &[1], false), (1, &[], false)]);
    let moved: Vec<_> = laid
        .rows
        .iter()
        .map(|row| match row {
            RelaidRow::Moved { oid, lanes, .. } => (
                oid.to_hex(),
                lanes.row,
                lanes.node_lane,
                lanes.node_color,
                lanes.segments.clone(),
            ),
            RelaidRow::Made(row) => panic!("nothing stands to be made: {row:?}"),
        })
        .collect();
    let expected: Vec<_> = fresh
        .iter()
        .map(|row| {
            (
                row.oid_hex.clone(),
                row.row,
                row.node_lane,
                row.node_color,
                row.segments.clone(),
            )
        })
        .collect();
    assert_eq!(moved, expected);
    assert_eq!(
        laid.prints,
        fresh.iter().map(RowPrint::of).collect::<Vec<_>>(),
        "the walk behind the delete would find this a changed picture"
    );
}

/// Nothing going lays out what the walk drew, to the print: the delete
/// refused puts back exactly the screen it took from.
#[test]
fn nothing_going_lays_out_the_walk_itself() {
    let rows = walk(&forked());
    let laid = lay_walked(
        &Walked::of(&rows),
        &no_standing(),
        None,
        &HashSet::new(),
        &LabelIndex::default(),
        true,
    );
    assert_eq!(
        laid.prints,
        rows.iter().map(RowPrint::of).collect::<Vec<_>>()
    );
}

/// This window's uncommitted row is made again on top, where the walk
/// puts it, whatever went from under it.
#[test]
fn the_uncommitted_row_is_made_again_on_top() {
    let walked = Walked::of(&walk(&forked()));
    let laid = lay_walked(
        &walked,
        &Standing {
            pending: Some(Vec::new()),
            carried: Arc::new(Vec::new()),
        },
        Some(oid(3)),
        &[oid(5), oid(4)].into(),
        &LabelIndex::default(),
        true,
    );
    assert!(
        matches!(&laid.rows[0], RelaidRow::Made(row) if Oid::hex_is_zero(&row.oid_hex)),
        "{:?}",
        laid.rows[0]
    );
    assert_eq!(laid.rows.len(), 4, "the row and the three commits left");
}
