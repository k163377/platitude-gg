//! The synthetic dashed edges (WIP to HEAD, stash to base) and what
//! real history may not take from them.

use super::testkit::{commit, oid};
use super::*;
use crate::model::StrPool;
use crate::oid::Oid;

#[test]
fn virtual_wip_row_takes_lane_zero_and_dashes_its_edge() {
    let mut pool = StrPool::new();
    let mut b = GraphBuilder::new();
    let head = oid(1);
    let wip = b.push_virtual(&Oid::zero_like(&head), &head);
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
    let wip = b.push_virtual(&Oid::zero_like(&head), &head);
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

/// The everyday shape that gave this away: a stash taken on a branch
/// that was later merged. The merge's second parent is exactly the
/// stash's base, so its fork edge wants the leash's lane — and taking
/// it would draw the stash's whole run down to the base as solid,
/// committed history.
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
    b.push_virtual(&Oid::zero_like(&head), &head);

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
