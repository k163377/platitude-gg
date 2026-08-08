//! Commit-graph lane assignment (gitk-style greedy allocation).
//!
//! Consumes commits in `--date-order` (children before parents) and emits,
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
    /// Drawn with a dashed stroke (a synthetic leash: WIP → HEAD or
    /// stash → base). Real history always draws solid.
    pub dashed: bool,
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
    /// The lane carries a synthetic leash (WIP → HEAD, stash → base) and
    /// draws dashed for its whole run: real edges never join it.
    dashed: bool,
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

    /// Processes the next commit of the stream (children before parents).
    pub fn push(&mut self, commit: &CommitMeta) -> GraphRow {
        self.push_with_edge_style(commit, false)
    }

    /// Like [`GraphBuilder::push`], but the first-parent edge leaving the
    /// node draws dashed (stash rows: not part of committed history
    /// proper).
    pub fn push_with_edge_style(&mut self, commit: &CommitMeta, dashed_edge: bool) -> GraphRow {
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
                dashed: state.dashed,
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
                self.occupy(node_lane, *p0, node_color, dashed_edge);
                segments.push(Segment {
                    kind: SegmentKind::OutOfNode,
                    lane: node_lane,
                    color: node_color,
                    dashed: dashed_edge,
                });
            }
        }
        for p in parents {
            // Another edge already waits for this parent: merge into the
            // nearest waiting lane (keeps the graph narrow and the
            // horizontal jog short).
            if let Some(existing) = self.waiting_lane(p, node_lane) {
                segments.push(Segment {
                    kind: SegmentKind::OutOfNode,
                    lane: existing,
                    color: self.lane_color(existing),
                    dashed: false,
                });
            } else if self.already_emitted(p, commit) {
                // Out-of-order stream: skip, as above.
            } else {
                let lane = self.find_free_lane_near(node_lane);
                let color = self.take_color();
                self.occupy(lane, *p, color, false);
                segments.push(Segment {
                    kind: SegmentKind::OutOfNode,
                    lane,
                    color,
                    dashed: false,
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

    /// Emits the synthetic working-tree (WIP) row: a childless tip whose
    /// only edge — drawn dashed — runs down to `parent` (HEAD). Feed it
    /// before the first real commit so the current chain keeps lane 0 and
    /// other tips shift right, exactly like a real commit would.
    pub fn push_virtual(&mut self, oid: &Oid, parent: &Oid) -> GraphRow {
        let row = self.next_row;
        self.next_row += 1;
        let lane = self.find_free_lane();
        let color = self.take_color();
        let mut segments = Vec::new();
        for (i, state) in self.lanes.iter().enumerate() {
            let Some(state) = state else { continue };
            segments.push(Segment {
                kind: SegmentKind::Through,
                lane: i as u16,
                color: state.color,
                dashed: state.dashed,
            });
        }
        self.occupy(lane, *parent, color, true);
        segments.push(Segment {
            kind: SegmentKind::OutOfNode,
            lane,
            color,
            dashed: true,
        });
        self.rows.insert(*oid, row);
        let mut width = lane + 1;
        for s in &segments {
            width = width.max(s.lane + 1);
        }
        self.max_width = self.max_width.max(width);
        GraphRow {
            row,
            node_lane: lane,
            node_color: color,
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

    /// Lane a fork edge to `parent` merges into: the lane already waiting
    /// for it that sits nearest to `near` (ties prefer the left side).
    ///
    /// A leash lane is not one of them. Real history sharing it would draw
    /// solid over the leash's whole run down to the shared parent, leaving
    /// the stash (or WIP) hanging from a line that reads as committed —
    /// which is what happens whenever the stash's base is also a merge's
    /// second parent. The real edge opens its own lane instead, and both
    /// arrive at the parent as separate curves.
    fn waiting_lane(&self, parent: &Oid, near: u16) -> Option<u16> {
        self.expects
            .get(parent)?
            .iter()
            .filter(|l| !self.is_leash(**l))
            .min_by_key(|l| (l.abs_diff(near), **l))
            .copied()
    }

    /// Whether a lane carries a synthetic leash rather than real history.
    fn is_leash(&self, lane: u16) -> bool {
        self.lanes
            .get(lane as usize)
            .and_then(Option::as_ref)
            .is_some_and(|s| s.dashed)
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

    /// Free lane nearest to `near` (ties prefer the left side); appending
    /// a rightmost lane competes under the same distance rule. Keeping a
    /// fork's target lane close to its node shortens the horizontal run,
    /// which is the main source of avoidable edge crossings.
    fn find_free_lane_near(&self, near: u16) -> u16 {
        let mut best: Option<(u16, u16)> = None; // (distance, lane)
        for (i, s) in self.lanes.iter().enumerate() {
            if s.is_none() {
                let lane = i as u16;
                let candidate = (lane.abs_diff(near), lane);
                if best.is_none_or(|b| candidate < b) {
                    best = Some(candidate);
                }
            }
        }
        let append = self.lanes.len() as u16;
        match best {
            Some((dist, lane)) if dist <= append.abs_diff(near) => lane,
            _ => append,
        }
    }

    fn occupy(&mut self, lane: u16, expects: Oid, color: u8, dashed: bool) {
        let idx = lane as usize;
        if idx >= self.lanes.len() {
            self.lanes.resize(idx + 1, None);
        }
        self.lanes[idx] = Some(LaneState { color, dashed });
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
    use std::collections::BTreeMap;

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
            author_email: pool.intern("test@example.com"),
            co_authors: Box::new([]),
            body: "".into(),
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
    /// orphan tips) with WIP and stash rows mixed in. A leash lane is a
    /// leash for its whole run: no row may turn it solid, and no solid
    /// lane may turn dashed.
    #[test]
    fn edges_are_continuous_across_rows_on_random_dags() {
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
            if seed % 2 == 0 {
                let head = oid(n);
                rows.push(b.push_virtual(&Oid::zero_like(&head), &head));
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
    }
}
