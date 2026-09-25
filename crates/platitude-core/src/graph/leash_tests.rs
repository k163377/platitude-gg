//! The synthetic dashed edges (WIP to HEAD, stash to base) and what
//! real history may not take from them.

use super::testkit::{commit, oid};
use super::*;
use crate::model::StrPool;
use crate::oid::Oid;
use crate::opstate::OpState;
use crate::status::{StatusItem, WorkTreeStatus};

#[test]
fn the_wip_row_stands_for_a_clean_tree_under_an_operation_but_not_under_a_bisect() {
    let clean = WorkTreeStatus::default();
    let dirty = WorkTreeStatus {
        items: vec![StatusItem::Untracked {
            path: "scratch.txt".into(),
        }],
        ..WorkTreeStatus::default()
    };
    assert!(!wip_row_stands(&clean, &OpState::default()));
    assert!(wip_row_stands(&dirty, &OpState::default()));
    // A rebase stopped at `edit` leaves the tree clean; its exit card
    // lands on the row.
    let rebasing = OpState {
        rebasing: true,
        ..OpState::default()
    };
    assert!(wip_row_stands(&clean, &rebasing));
    // A bisect has no card to land.
    let bisecting = OpState {
        bisecting: true,
        ..OpState::default()
    };
    assert!(!wip_row_stands(&clean, &bisecting));
}

/// A synthetic row carries the all-zero id, which more than one row can
/// share, so it is no address; neither reader of the map (`row_of`, the
/// out-of-order guard for parents) ever asks about one.
#[test]
fn a_row_with_no_object_is_not_in_the_map_from_commit_to_row() {
    let mut b = GraphBuilder::new();
    let head = oid(1);
    b.push_virtual(&head);
    assert_eq!(b.row_of(&Oid::zero_like(&head)), None);
    assert_eq!(b.tracked_oids(), 1, "only the lane waiting for HEAD");

    let mut unborn = GraphBuilder::new();
    unborn.push_virtual_root();
    assert_eq!(unborn.row_of(&Oid::zero_unsized()), None);
    assert_eq!(unborn.tracked_oids(), 0, "nothing to wait for either");
}

/// Another working copy's row, drawn above the commit that copy stands
/// on, may not move any commit's lane or colour: a copy picked up on
/// window focus would re-colour chains under the reader's eyes
/// (P3-確認事項 §別 worktree の未コミット行). Covers a commit a lane
/// already waits for (`02`, HEAD) and tips nothing waits for (`04`, `06`).
#[test]
fn a_row_for_another_copy_moves_no_lane_and_no_colour() {
    fn lanes_and_colours(insert_above: &[u8]) -> Vec<(u8, u16, u8)> {
        let mut pool = StrPool::new();
        let mut b = GraphBuilder::new();
        // `demo-repo basic` in walk order: this window's row, the stash
        // on HEAD, HEAD, the remote-only sibling tip, their shared parent,
        // the topic branch, and the trunk under both.
        b.push_virtual(&oid(2));
        let history = [
            (commit(&mut pool, 1, &[2]), true),
            (commit(&mut pool, 2, &[3]), false),
            (commit(&mut pool, 4, &[3]), false),
            (commit(&mut pool, 3, &[5]), false),
            (commit(&mut pool, 6, &[7]), false),
            (commit(&mut pool, 7, &[5]), false),
            (commit(&mut pool, 5, &[8]), false),
            (commit(&mut pool, 8, &[]), false),
        ];
        let mut out = Vec::new();
        for (c, is_stash) in &history {
            let id = c.oid.as_bytes()[0];
            if insert_above.contains(&id) {
                b.push_virtual(&c.oid);
            }
            let row = b.push_with_edge_style(c, *is_stash);
            out.push((id, row.node_lane, row.node_color));
        }
        out
    }
    let plain = lanes_and_colours(&[]);
    assert_eq!(
        lanes_and_colours(&[4]),
        plain,
        "a row over the sibling tip moved the history under it"
    );
    assert_eq!(
        lanes_and_colours(&[6]),
        plain,
        "a row over the topic tip moved the history under it"
    );
    assert_eq!(
        lanes_and_colours(&[4, 6]),
        plain,
        "two rows moved the history under them"
    );
    assert_eq!(
        lanes_and_colours(&[2]),
        plain,
        "a row over HEAD, where this window's own row already leashes, moved it"
    );
}

#[test]
fn virtual_wip_row_takes_lane_zero_and_dashes_its_edge() {
    let mut pool = StrPool::new();
    let mut b = GraphBuilder::new();
    let head = oid(1);
    let wip = b.push_virtual(&head);
    assert_eq!((wip.row, wip.node_lane), (0, 0));
    assert!(
        wip.segments
            .iter()
            .all(|s| s.kind == SegmentKind::OutOfNode && s.dashed),
        "the WIP edge draws dashed"
    );
    // Another tip streams next: it must shift right of the WIP chain.
    let other = b.push(&commit(&mut pool, 9, &[2]));
    assert_eq!(other.node_lane, 1);
    // HEAD joins the dashed edge and continues the chain normally.
    let head_row = b.push(&commit(&mut pool, 1, &[3]));
    assert_eq!(head_row.node_lane, 0);
    let into = head_row
        .segments
        .iter()
        .find(|s| s.kind == SegmentKind::IntoNode && s.lane == 0)
        .unwrap();
    assert!(into.dashed);
    let out = head_row
        .segments
        .iter()
        .find(|s| s.kind == SegmentKind::OutOfNode && s.lane == 0)
        .unwrap();
    assert!(!out.dashed, "the chain below HEAD is a normal edge");
}

/// A merge whose second parent is HEAD wants the lane the WIP leash
/// reserved. Real history may not share it — the fork edge opens a
/// lane of its own, and the leash keeps dotting all the way to HEAD.
#[test]
fn a_merge_reaching_head_leaves_the_wip_leash_dashed() {
    let mut pool = StrPool::new();
    let mut b = GraphBuilder::new();
    let head = oid(2);
    let wip = b.push_virtual(&head);
    assert!(wip.segments.iter().all(|s| s.dashed));

    // Merge of the branch HEAD sits on: parents are the mainline (3)
    // and HEAD itself, which the WIP lane is already waiting for.
    let merge = b.push(&commit(&mut pool, 1, &[3, 2]));
    assert_eq!(merge.node_lane, 1, "the WIP row keeps lane 0");
    let outs: Vec<u16> = merge
        .segments
        .iter()
        .filter(|s| s.kind == SegmentKind::OutOfNode)
        .map(|s| s.lane)
        .collect();
    assert_eq!(outs, vec![1, 2], "the fork edge takes a fresh lane");
    let through = merge
        .segments
        .iter()
        .find(|s| s.kind == SegmentKind::Through && s.lane == 0)
        .unwrap();
    assert!(through.dashed, "the leash runs on: {:?}", merge.segments);
    assert!(
        merge
            .segments
            .iter()
            .filter(|s| s.lane != 0)
            .all(|s| !s.dashed),
        "the merge's own edges are committed history: {:?}",
        merge.segments
    );

    // HEAD gathers both: the leash arrives dashed, the fork solid, and
    // the chain leaving HEAD is a normal edge.
    let head_row = b.push(&commit(&mut pool, 2, &[4]));
    assert_eq!(head_row.node_lane, 0);
    let ins: Vec<(u16, bool)> = head_row
        .segments
        .iter()
        .filter(|s| s.kind == SegmentKind::IntoNode)
        .map(|s| (s.lane, s.dashed))
        .collect();
    assert_eq!(ins, vec![(0, true), (2, false)]);
    assert!(
        head_row
            .segments
            .iter()
            .filter(|s| s.kind == SegmentKind::OutOfNode)
            .all(|s| !s.dashed)
    );
}

/// A stash taken on a branch that was later merged: the merge's second
/// parent is the stash's base, so its fork edge wants the leash's lane —
/// taking it would draw the stash's run down to the base as solid history.
#[test]
fn a_merge_reaching_a_stash_base_leaves_the_leash_dashed() {
    let mut pool = StrPool::new();
    let mut b = GraphBuilder::new();
    // Branch tip (5) on lane 0, then the stash hanging off base 3.
    b.push(&commit(&mut pool, 5, &[4]));
    let stash = b.push_with_edge_style(&commit(&mut pool, 6, &[3]), true);
    assert_eq!(stash.node_lane, 1);
    // The merge that brought the branch in: mainline (4) and base (3).
    let merge = b.push(&commit(&mut pool, 2, &[4, 3]));
    assert_eq!(
        merge
            .segments
            .iter()
            .filter(|s| s.kind == SegmentKind::OutOfNode)
            .map(|s| s.lane)
            .collect::<Vec<_>>(),
        vec![2, 3],
        "the fork edge steps around the leash: {:?}",
        merge.segments
    );
    let through = merge
        .segments
        .iter()
        .find(|s| s.kind == SegmentKind::Through && s.lane == 1)
        .unwrap();
    assert!(
        through.dashed,
        "the leash passes the merge untouched: {:?}",
        merge.segments
    );

    // Every row between the stash and its base keeps the lane dashed.
    let mainline = b.push(&commit(&mut pool, 4, &[3]));
    assert!(
        mainline
            .segments
            .iter()
            .find(|s| s.kind == SegmentKind::Through && s.lane == 1)
            .is_some_and(|s| s.dashed)
    );
    // The base gathers the leash and the merge's fork edge separately.
    let base = b.push(&commit(&mut pool, 3, &[]));
    let ins: Vec<(u16, bool)> = base
        .segments
        .iter()
        .filter(|s| s.kind == SegmentKind::IntoNode)
        .map(|s| (s.lane, s.dashed))
        .collect();
    assert_eq!(ins, vec![(0, false), (1, true), (3, false)]);
}

/// A duplicate-parent merge sends its extra edge onto the node's own
/// first-parent edge (distance zero beats every waiting lane, and a
/// WIP leash waiting for the same commit is not a candidate at all),
/// so the leash keeps drawing dashed.
#[test]
fn duplicate_parent_merge_keeps_an_unrelated_leash_dashed() {
    let mut pool = StrPool::new();
    let mut b = GraphBuilder::new();
    let head = oid(2);
    b.push_virtual(&head);

    // Merge listing HEAD twice (git accepts duplicate parents).
    let merge = b.push(&commit(&mut pool, 1, &[2, 2]));
    assert_eq!(merge.node_lane, 1, "the WIP leash keeps lane 0");
    assert!(
        merge
            .segments
            .iter()
            .filter(|s| s.kind == SegmentKind::OutOfNode)
            .all(|s| s.lane == 1),
        "both parent edges lie on the node lane: {:?}",
        merge.segments
    );
    let through = merge
        .segments
        .iter()
        .find(|s| s.kind == SegmentKind::Through && s.lane == 0)
        .unwrap();
    assert!(
        through.dashed,
        "no real edge joined the WIP lane: {:?}",
        merge.segments
    );

    // HEAD gathers both the leash and the merge edges; the leash side
    // still arrives dashed.
    let head_row = b.push(&commit(&mut pool, 2, &[]));
    assert_eq!(head_row.node_lane, 0);
    let into = head_row
        .segments
        .iter()
        .find(|s| s.kind == SegmentKind::IntoNode && s.lane == 0)
        .unwrap();
    assert!(into.dashed);
}

/// A merge stopped in the working tree: the row for the uncommitted
/// files is the merge commit it is about to become, so the side being
/// brought in hangs off it too — on a leash of its own, beside HEAD's.
#[test]
fn a_standing_merge_leashes_the_side_it_is_bringing_in() {
    let mut pool = StrPool::new();
    let mut b = GraphBuilder::new();
    let head = oid(1);
    let theirs = oid(9);
    let wip = b.push_virtual_merging(&head, &[theirs]);
    assert_eq!((wip.row, wip.node_lane), (0, 0));
    let outs: Vec<(u16, bool)> = wip
        .segments
        .iter()
        .filter(|s| s.kind == SegmentKind::OutOfNode)
        .map(|s| (s.lane, s.dashed))
        .collect();
    assert_eq!(
        outs,
        vec![(0, true), (1, true)],
        "both sides leave the node dotted: {:?}",
        wip.segments
    );
    assert_eq!(wip.width, 2);

    // The side's tip arrives on the lane its leash reserved, and the
    // chain below it is committed history again.
    let their_tip = b.push(&commit(&mut pool, 9, &[3]));
    assert_eq!(their_tip.node_lane, 1);
    let into = their_tip
        .segments
        .iter()
        .find(|s| s.kind == SegmentKind::IntoNode && s.lane == 1)
        .unwrap();
    assert!(into.dashed, "the leash arrives dotted");
    assert!(
        their_tip
            .segments
            .iter()
            .filter(|s| s.kind == SegmentKind::OutOfNode)
            .all(|s| !s.dashed)
    );

    // HEAD keeps lane 0 the whole way, with its own leash still dotted.
    let head_row = b.push(&commit(&mut pool, 1, &[3]));
    assert_eq!(head_row.node_lane, 0);
    assert!(
        head_row
            .segments
            .iter()
            .find(|s| s.kind == SegmentKind::IntoNode && s.lane == 0)
            .is_some_and(|s| s.dashed)
    );
}

/// An octopus gets a leash per side, each in its own lane and colour.
#[test]
fn an_octopus_leashes_every_side_separately() {
    let mut b = GraphBuilder::new();
    let head = oid(1);
    let wip = b.push_virtual_merging(&head, &[oid(8), oid(9)]);
    let outs: Vec<u16> = wip
        .segments
        .iter()
        .filter(|s| s.kind == SegmentKind::OutOfNode)
        .map(|s| s.lane)
        .collect();
    assert_eq!(outs, vec![0, 1, 2]);
    assert!(wip.segments.iter().all(|s| s.dashed));
    let colors: std::collections::HashSet<u8> = wip.segments.iter().map(|s| s.color).collect();
    assert_eq!(colors.len(), 3, "three chains, three colours");
}

/// One leash per commit, however many times it is named: a side that is
/// where HEAD already stands, and a side named twice, each draw the one
/// edge the graph will have.
#[test]
fn a_side_already_leashed_does_not_get_a_second_lane() {
    let head = oid(1);
    let mut b = GraphBuilder::new();
    let same = b.push_virtual_merging(&head, &[head]);
    assert_eq!(same.width, 1, "the side is HEAD: {:?}", same.segments);

    let mut b = GraphBuilder::new();
    let twice = b.push_virtual_merging(&head, &[oid(9), oid(9)]);
    assert_eq!(
        twice.width, 2,
        "one side, named twice: {:?}",
        twice.segments
    );
}
