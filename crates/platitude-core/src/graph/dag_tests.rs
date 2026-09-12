//! One property over arbitrary generated DAGs: every edge leaving a
//! row's bottom continues at the next row's top, unchanged.

use std::collections::BTreeMap;

use super::testkit::{commit, oid, render};
use super::*;
use crate::model::{CommitMeta, StrPool};
use crate::oid::Oid;

/// Tiny deterministic PRNG (xorshift64*), keeping the test free of
/// dependencies.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// Lanes touching one horizontal edge of a row — its bottom (through
/// and out-of-node) or its top (through and into-node) — with the
/// color and dash of the segment drawn there. Two segments sharing a
/// lane (a merge edge joining a through lane) must agree on both, or
/// one would be drawn over the other.
fn boundary_lanes(row: &GraphRow, bottom: bool) -> BTreeMap<u16, (u8, bool)> {
    let mut lanes = BTreeMap::new();
    for s in &row.segments {
        let touches = match s.kind {
            SegmentKind::Through => true,
            SegmentKind::OutOfNode => bottom,
            SegmentKind::IntoNode => !bottom,
        };
        if !touches {
            continue;
        }
        if let Some(seen) = lanes.insert(s.lane, (s.color, s.dashed)) {
            assert_eq!(
                seen,
                (s.color, s.dashed),
                "row {}: segments on lane {} disagree",
                row.row,
                s.lane
            );
        }
    }
    lanes
}

/// Every edge leaving a row's bottom must continue at the next row's
/// top on the same lane, in the same color and the same dash — across
/// arbitrary DAGs (merges, octopus and duplicate parents, extra roots,
/// orphan tips) with WIP and stash rows mixed in, the WIP row leashing
/// the sides of a standing merge as well as HEAD. A leash lane is a
/// leash for its whole run: no row may turn it solid, and no solid
/// lane may turn dashed.
#[test]
fn edges_are_continuous_across_rows_on_random_dags() {
    // A property test is worth what its generator produces: this counts
    // the WIP rows that really did leave on more than one leash, so a
    // change to the random walk cannot quietly stop covering them.
    let mut merging_rows = 0usize;
    for seed in 1..=300u64 {
        let mut rng = Rng(seed);
        let mut pool = StrPool::new();
        let n = 2 + rng.below(28) as u8;
        // ids n..=1, children before parents (parents have smaller
        // ids), so the stream is topo-ordered by construction.
        let mut commits: Vec<CommitMeta> = Vec::new();
        for id in (1..=n).rev() {
            let mut parents: Vec<u8> = Vec::new();
            if id > 1 && rng.below(12) != 0 {
                let extra = match rng.below(10) {
                    0..=6 => 0,
                    7..=8 => 1,
                    _ => 2,
                };
                for _ in 0..=extra {
                    parents.push(1 + rng.below(u64::from(id) - 1) as u8);
                }
            }
            commits.push(commit(&mut pool, id, &parents));
        }

        let mut b = GraphBuilder::new();
        let mut rows: Vec<GraphRow> = Vec::new();
        // Half the seeds open like a dirty repository with one stash:
        // a WIP leash plus a dashed stash tip, both leading to HEAD.
        // Some of those are stopped mid-merge, where the WIP row holds a
        // leash per side as well as HEAD's — several lanes leaving one
        // row, which nothing else here produces. The sides are ids from
        // the walk, so HEAD and repeats turn up among them and each one
        // has a node further down to land on.
        if seed % 2 == 0 {
            let head = oid(n);
            let sides: Vec<Oid> = (0..rng.below(4))
                .map(|_| oid(1 + rng.below(u64::from(n)) as u8))
                .collect();
            let wip = b.push_virtual_merging(&head, &sides);
            if wip
                .segments
                .iter()
                .filter(|s| s.kind == SegmentKind::OutOfNode)
                .count()
                > 1
            {
                merging_rows += 1;
            }
            rows.push(wip);
            let stash = commit(&mut pool, n + 1, &[n]);
            rows.push(b.push_with_edge_style(&stash, true));
        }
        rows.extend(commits.iter().map(|c| b.push(c)));

        for pair in rows.windows(2) {
            let bottom = boundary_lanes(&pair[0], true);
            let top = boundary_lanes(&pair[1], false);
            assert_eq!(
                bottom.keys().collect::<Vec<_>>(),
                top.keys().collect::<Vec<_>>(),
                "seed {seed}: lanes must continue from row {} to row {}\n{}",
                pair[0].row,
                pair[1].row,
                render(&rows)
            );
            for (lane, (color, dashed)) in &bottom {
                let (top_color, top_dashed) = top[lane];
                assert_eq!(
                    *color,
                    top_color,
                    "seed {seed}: lane {lane} changes color between rows {} and {}\n{}",
                    pair[0].row,
                    pair[1].row,
                    render(&rows)
                );
                assert_eq!(
                    *dashed,
                    top_dashed,
                    "seed {seed}: lane {lane} changes dash between rows {} and {}\n{}",
                    pair[0].row,
                    pair[1].row,
                    render(&rows)
                );
            }
        }
    }
    assert!(
        merging_rows >= 50,
        "only {merging_rows} of 300 seeds put a standing merge above the graph"
    );
}
