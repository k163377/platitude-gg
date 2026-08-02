//! Commit-graph lane assignment (gitk-style greedy allocation).
//!
//! Consumes commits in `--topo-order` (children before parents) and emits,
//! per row, everything the UI needs to draw that row in isolation: the node
//! position/color plus straight and curved lane segments. QML only draws;
//! no layout decisions happen on the UI side (実装計画 §2.4).
//!
//! The engine is incremental: rows stream out while `git log` is still
//! running, so the first chunk can be painted immediately.

use std::collections::HashMap;

use crate::model::CommitMeta;
use crate::oid::Oid;

/// Nominal number of distinct chain colors; must match the `graphLane`
/// palette in the design tokens (internal-docs/デザイン規約.md). The engine
/// only hands out palette indices.
pub const GRAPH_PALETTE_SIZE: u32 = 8;

/// How a segment participates in its row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentKind {
    /// Vertical line spanning the full row height at `lane`.
    Through,
    /// Curve from the top edge at `lane` into the node center.
    IntoNode,
    /// Curve from the node center to the bottom edge at `lane`.
    OutOfNode,
}

/// One drawable lane segment of a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub kind: SegmentKind,
    pub lane: u16,
    /// Palette index (< [`GRAPH_PALETTE_SIZE`]).
    pub color: u8,
}

/// Draw data for one commit row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphRow {
    /// Row index (0-based, equals the commit's position in the stream).
    pub row: u32,
    pub node_lane: u16,
    /// Palette index of the node's chain.
    pub node_color: u8,
    pub segments: Vec<Segment>,
    /// Lanes needed to draw this row (max referenced lane + 1).
    pub width: u16,
}

/// Occupied-lane state. The id a lane waits for lives only in
/// [`GraphBuilder::expects`] (single source of truth).
#[derive(Debug, Clone)]
struct LaneState {
    color: u8,
}

/// Incremental lane allocator.
#[derive(Debug, Default)]
pub struct GraphBuilder {
    lanes: Vec<Option<LaneState>>,
    /// Lane indices currently expecting a given commit id.
    expects: HashMap<Oid, Vec<u16>>,
    /// Emitted commit id → row index (also serves the out-of-order guard).
    rows: HashMap<Oid, u32>,
    next_row: u32,
    next_color: u32,
    /// Widest row seen so far (for sizing the graph column).
    max_width: u16,
    /// Whether an out-of-order parent has been reported already.
    warned_out_of_order: bool,
}

impl GraphBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Row index of an already-processed commit.
    pub fn row_of(&self, oid: &Oid) -> Option<u32> {
        self.rows.get(oid).copied()
    }

    pub fn row_count(&self) -> u32 {
        self.next_row
    }

    /// Widest row emitted so far.
    pub fn max_width(&self) -> u16 {
        self.max_width
    }

    /// Processes the next commit of the topo-ordered stream.
    pub fn push(&mut self, commit: &CommitMeta) -> GraphRow {
        let row = self.next_row;
        self.next_row += 1;

        // Lanes whose edge terminates at this node.
        let joins = self.expects.remove(&commit.oid).unwrap_or_default();

        let (node_lane, node_color, fresh) = match joins.iter().min() {
            Some(min) => {
                let lane = *min;
                let color = self.lane_color(lane);
                (lane, color, false)
            }
            None => {
                // A tip (or orphan): starts a new chain in a free lane. The
                // lane is not occupied yet, so it draws no Through segment.
                let lane = self.find_free_lane();
                (lane, self.take_color(), true)
            }
        };

        let mut segments = Vec::new();
        for (i, state) in self.lanes.iter().enumerate() {
            let Some(state) = state else { continue };
            let lane = i as u16;
            let kind = if joins.contains(&lane) {
                SegmentKind::IntoNode
            } else {
                SegmentKind::Through
            };
            segments.push(Segment {
                kind,
                lane,
                color: state.color,
            });
        }
        debug_assert!(fresh || joins.contains(&node_lane));

        // Release everything that terminated here, then hand the node lane
        // to the first parent (chain continues, same color).
        for &m in &joins {
            self.lanes[m as usize] = None;
        }

        let mut parents = commit.parents.iter();
        if let Some(p0) = parents.next() {
            if self.already_emitted(p0, commit) {
                // Out-of-order stream: the edge cannot be drawn; leave the
                // lane free rather than leaking it forever.
            } else {
                self.occupy(node_lane, *p0, node_color);
                segments.push(Segment {
                    kind: SegmentKind::OutOfNode,
                    lane: node_lane,
                    color: node_color,
                });
            }
        }
        for p in parents {
            if let Some(existing) = self.expects.get(p).and_then(|v| v.iter().min().copied()) {
                // Another edge already waits for this parent: merge into it
                // immediately (keeps the graph narrow).
                segments.push(Segment {
                    kind: SegmentKind::OutOfNode,
                    lane: existing,
                    color: self.lane_color(existing),
                });
            } else if self.already_emitted(p, commit) {
                // Out-of-order stream: skip, as above.
            } else {
                let lane = self.find_free_lane();
                let color = self.take_color();
                self.occupy(lane, *p, color);
                segments.push(Segment {
                    kind: SegmentKind::OutOfNode,
                    lane,
                    color,
                });
            }
        }

        while matches!(self.lanes.last(), Some(None)) {
            self.lanes.pop();
        }

        self.rows.insert(commit.oid, row);

        let mut width = node_lane + 1;
        for s in &segments {
            width = width.max(s.lane + 1);
        }
        self.max_width = self.max_width.max(width);

        GraphRow {
            row,
            node_lane,
            node_color,
            segments,
            width,
        }
    }

    fn lane_color(&self, lane: u16) -> u8 {
        self.lanes
            .get(lane as usize)
            .and_then(|s| s.as_ref())
            .map_or(0, |s| s.color)
    }

    fn take_color(&mut self) -> u8 {
        let c = (self.next_color % GRAPH_PALETTE_SIZE) as u8;
        self.next_color += 1;
        c
    }

    fn find_free_lane(&self) -> u16 {
        self.lanes
            .iter()
            .position(Option::is_none)
            .unwrap_or(self.lanes.len()) as u16
    }

    fn occupy(&mut self, lane: u16, expects: Oid, color: u8) {
        let idx = lane as usize;
        if idx >= self.lanes.len() {
            self.lanes.resize(idx + 1, None);
        }
        self.lanes[idx] = Some(LaneState { color });
        self.expects.entry(expects).or_default().push(lane);
    }

    fn already_emitted(&mut self, parent: &Oid, child: &CommitMeta) -> bool {
        let emitted = self.rows.contains_key(parent);
        if emitted && !self.warned_out_of_order {
            self.warned_out_of_order = true;
            tracing::warn!(
                child = %child.oid,
                parent = %parent,
                "log stream is not topologically ordered; some edges will not be drawn"
            );
        }
        emitted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::StrPool;

    fn oid(n: u8) -> Oid {
        let hex = format!("{n:02x}").repeat(20);
        // Test-only helper; the input is always valid hex.
        #[allow(clippy::unwrap_used)]
        Oid::from_hex_str(&hex).unwrap()
    }

    fn commit(pool: &mut StrPool, id: u8, parents: &[u8]) -> CommitMeta {
        CommitMeta {
            oid: oid(id),
            parents: parents.iter().map(|p| oid(*p)).collect(),
            author: pool.intern("Test"),
            time: 1_700_000_000 + i64::from(id),
            subject: format!("commit {id:02x}").into_boxed_str(),
        }
    }

    /// Renders rows into a deterministic ASCII grid for snapshots:
    /// `*` node, `|` through, `/` into-node, `\` out-of-node, `x` both.
    fn render(rows: &[GraphRow]) -> String {
        let width = rows.iter().map(|r| r.width).max().unwrap_or(0) as usize;
        let mut out = String::new();
        for r in rows {
            let mut cells = vec![' '; width];
            for s in &r.segments {
                let c = &mut cells[s.lane as usize];
                let mark = match s.kind {
                    SegmentKind::Through => '|',
                    SegmentKind::IntoNode => '/',
                    SegmentKind::OutOfNode => '\\',
                };
                *c = match (*c, mark) {
                    (' ', m) => m,
                    ('/', '\\') | ('\\', '/') => 'x',
                    ('|', m) | (m, '|') if m != ' ' => m,
                    _ => 'x',
                };
            }
            cells[r.node_lane as usize] = '*';
            let line: String = cells.into_iter().collect();
            out.push_str(&format!(
                "{:>3} {} lane={} color={}\n",
                r.row,
                line.trim_end_matches(' '),
                r.node_lane,
                r.node_color
            ));
        }
        out
    }

    fn build(commits: &[CommitMeta]) -> (Vec<GraphRow>, GraphBuilder) {
        let mut b = GraphBuilder::new();
        let rows = commits.iter().map(|c| b.push(c)).collect();
        (rows, b)
    }

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
        // lane 1 immediately instead of opening lane 2. First parents, by
        // contrast, keep their own lane until the parent row (gitk-style),
        // which is why 3 -> [1] still flows down lane 0.
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
        // The dangling edge is dropped instead of waiting forever.
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
}
