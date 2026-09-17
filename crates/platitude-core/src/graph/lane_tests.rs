//! Which lane a row takes, and when a lane is freed for reuse.

use super::testkit::{build, commit, oid, render};
use super::*;
use crate::model::StrPool;

#[test]
fn linear_history_stays_in_lane_zero() {
    let mut pool = StrPool::new();
    let commits = vec![
        commit(&mut pool, 3, &[2]),
        commit(&mut pool, 2, &[1]),
        commit(&mut pool, 1, &[]),
    ];
    let (rows, b) = build(&commits);
    assert!(rows.iter().all(|r| r.node_lane == 0 && r.width == 1));
    assert!(
        rows.iter().all(|r| r.node_color == 0),
        "one chain, one color"
    );
    assert_eq!(b.max_width(), 1);
    // Middle commit: chain in from above, chain out below.
    assert_eq!(rows[1].segments.len(), 2);
}

#[test]
fn branch_and_merge_uses_two_lanes() {
    // 4 = merge(3, 2); 3 and 2 both children of 1.
    let mut pool = StrPool::new();
    let commits = vec![
        commit(&mut pool, 4, &[3, 2]),
        commit(&mut pool, 3, &[1]),
        commit(&mut pool, 2, &[1]),
        commit(&mut pool, 1, &[]),
    ];
    let (rows, b) = build(&commits);
    assert_eq!(rows[0].node_lane, 0);
    // Merge row forks into lane 0 (first parent) and lane 1.
    assert_eq!(
        rows[0]
            .segments
            .iter()
            .filter(|s| s.kind == SegmentKind::OutOfNode)
            .count(),
        2
    );
    // Side chain gets its own color.
    let side_out = rows[0]
        .segments
        .iter()
        .find(|s| s.kind == SegmentKind::OutOfNode && s.lane == 1)
        .unwrap();
    assert_ne!(side_out.color, rows[0].node_color);
    // Both edges converge on commit 1: two IntoNode segments.
    assert_eq!(
        rows[3]
            .segments
            .iter()
            .filter(|s| s.kind == SegmentKind::IntoNode)
            .count(),
        2
    );
    // After the merge point the width shrinks back.
    assert_eq!(rows[3].node_lane, 0);
    assert_eq!(b.max_width(), 2);
    insta::assert_snapshot!(render(&rows));
}

#[test]
fn octopus_merge_forks_three_ways() {
    let mut pool = StrPool::new();
    let commits = vec![
        commit(&mut pool, 5, &[2, 3, 4]),
        commit(&mut pool, 2, &[1]),
        commit(&mut pool, 3, &[1]),
        commit(&mut pool, 4, &[1]),
        commit(&mut pool, 1, &[]),
    ];
    let (rows, _) = build(&commits);
    assert_eq!(
        rows[0]
            .segments
            .iter()
            .filter(|s| s.kind == SegmentKind::OutOfNode)
            .count(),
        3
    );
    // Final ancestor gathers three edges.
    assert_eq!(
        rows[4]
            .segments
            .iter()
            .filter(|s| s.kind == SegmentKind::IntoNode)
            .count(),
        3
    );
    insta::assert_snapshot!(render(&rows));
}

#[test]
fn second_parent_merges_into_existing_edge() {
    // 6 -> [5, 1] opens lane 1 waiting for 1. Later 4 -> [3, 1]: its
    // second parent is already awaited by lane 1, so the edge joins
    // lane 1 immediately. First parents, by contrast, keep their
    // own lane until the parent row (gitk-style), which is why
    // 3 -> [1] still flows down lane 0.
    let mut pool = StrPool::new();
    let commits = vec![
        commit(&mut pool, 6, &[5, 1]),
        commit(&mut pool, 5, &[4]),
        commit(&mut pool, 4, &[3, 1]),
        commit(&mut pool, 3, &[1]),
        commit(&mut pool, 1, &[]),
    ];
    let (rows, b) = build(&commits);
    assert_eq!(b.max_width(), 2, "4's second parent must reuse lane 1");
    let row4 = &rows[2];
    let outs: Vec<u16> = row4
        .segments
        .iter()
        .filter(|s| s.kind == SegmentKind::OutOfNode)
        .map(|s| s.lane)
        .collect();
    assert_eq!(outs, vec![0, 1], "fork into own lane and the waiting lane");
    // Both surviving edges converge on the common ancestor.
    assert_eq!(
        rows[4]
            .segments
            .iter()
            .filter(|s| s.kind == SegmentKind::IntoNode)
            .count(),
        2
    );
    insta::assert_snapshot!(render(&rows));
}

#[test]
fn independent_roots_occupy_separate_lanes() {
    let mut pool = StrPool::new();
    let commits = vec![
        commit(&mut pool, 4, &[2]),
        commit(&mut pool, 3, &[1]),
        commit(&mut pool, 2, &[]),
        commit(&mut pool, 1, &[]),
    ];
    let (rows, _) = build(&commits);
    assert_eq!(rows[0].node_lane, 0);
    assert_eq!(rows[1].node_lane, 1, "orphan tip takes the next lane");
    assert_ne!(rows[0].node_color, rows[1].node_color);
    // Root of the first chain frees lane 0 while chain 2 passes through.
    assert_eq!(rows[2].node_lane, 0);
    insta::assert_snapshot!(render(&rows));
}

#[test]
fn fork_prefers_the_nearest_free_lane() {
    // Node at lane 2 forks its second parent while lane 0 is free:
    // crossing lane 1 to reach it would be worse than opening the
    // adjacent lane 3.
    let mut pool = StrPool::new();
    let commits = vec![
        commit(&mut pool, 9, &[1]),    // lane 0
        commit(&mut pool, 8, &[2]),    // lane 1
        commit(&mut pool, 7, &[3]),    // lane 2
        commit(&mut pool, 1, &[]),     // root: frees lane 0
        commit(&mut pool, 3, &[4, 5]), // node lane 2, forks parent 5
        commit(&mut pool, 2, &[]),
        commit(&mut pool, 4, &[]),
        commit(&mut pool, 5, &[]),
    ];
    let (rows, _) = build(&commits);
    let fork = &rows[4];
    assert_eq!(fork.node_lane, 2);
    let outs: Vec<u16> = fork
        .segments
        .iter()
        .filter(|s| s.kind == SegmentKind::OutOfNode)
        .map(|s| s.lane)
        .collect();
    assert_eq!(outs, vec![2, 3], "second parent takes the adjacent lane");
}

#[test]
fn lanes_are_reused_after_a_root_ends() {
    let mut pool = StrPool::new();
    let commits = vec![
        commit(&mut pool, 4, &[2]),
        commit(&mut pool, 3, &[1]),
        commit(&mut pool, 2, &[]), // chain in lane 0 ends here
        commit(&mut pool, 1, &[]), // still in lane 1
        commit(&mut pool, 5, &[]), // new orphan tip: lane 0 is free again
    ];
    let (rows, _) = build(&commits);
    assert_eq!(rows[4].node_lane, 0);
}

#[test]
fn out_of_order_parent_does_not_leak_a_lane() {
    let mut pool = StrPool::new();
    // Parent 2 appears before its child 3 (violates topo order).
    let commits = vec![
        commit(&mut pool, 2, &[1]),
        commit(&mut pool, 1, &[]),
        commit(&mut pool, 3, &[2]),
    ];
    let (rows, b) = build(&commits);
    // The dangling edge is dropped.
    assert_eq!(rows[2].segments.len(), 0);
    assert_eq!(b.max_width(), 1);
}

#[test]
fn duplicate_parent_reuses_the_first_edge() {
    let mut pool = StrPool::new();
    let commits = vec![commit(&mut pool, 2, &[1, 1]), commit(&mut pool, 1, &[])];
    let (rows, b) = build(&commits);
    assert_eq!(b.max_width(), 1, "duplicate parent must not open a lane");
    assert_eq!(
        rows[0]
            .segments
            .iter()
            .filter(|s| s.kind == SegmentKind::OutOfNode)
            .count(),
        2
    );
}

#[test]
fn row_lookup_matches_emitted_order() {
    let mut pool = StrPool::new();
    let commits = vec![commit(&mut pool, 2, &[1]), commit(&mut pool, 1, &[])];
    let (_, b) = build(&commits);
    assert_eq!(b.row_of(&oid(2)), Some(0));
    assert_eq!(b.row_of(&oid(1)), Some(1));
    assert_eq!(b.row_of(&oid(9)), None);
    assert_eq!(b.row_count(), 2);
}
