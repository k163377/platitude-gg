//! The greedy allocator itself: which lane a commit takes, which
//! lanes stay open behind it, and what that draws.

use std::collections::HashMap;

use crate::model::CommitMeta;
use crate::oid::Oid;

use super::{GRAPH_PALETTE_SIZE, GraphRow, Segment, SegmentKind};

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

/// The lane bookkeeping, which outlives every row it drew: `expects` and
/// `rows` are keyed by commit id and keep growing with the walk.
impl crate::mem::Footprint for GraphBuilder {
    fn heap_bytes(&self) -> usize {
        self.lanes.capacity() * size_of::<Option<LaneState>>()
            + self.expects.heap_bytes()
            + self.rows.heap_bytes()
    }
}

impl GraphBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Commit ids the builder is holding — the count beside its bytes.
    pub fn tracked_oids(&self) -> usize {
        self.rows.len() + self.expects.len()
    }

    /// Row index of an already-processed commit.
    pub fn row_of(&self, oid: &Oid) -> Option<u32> {
        self.rows.get(oid).copied()
    }

    #[cfg(test)]
    pub fn row_count(&self) -> u32 {
        self.next_row
    }

    #[cfg(test)]
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
        self.push_virtual_merging(oid, parent, &[])
    }

    /// Like [`GraphBuilder::push_virtual`], but the row also reaches the
    /// sides a standing merge is bringing in (`MERGE_HEAD` — more than
    /// one of them for an octopus).
    ///
    /// The commit this row is about to become has those parents, so the
    /// row draws them: the same fork the graph will show once it is
    /// committed, on leashes, because nothing here is a commit yet.
    ///
    /// Same feeding rule as [`GraphBuilder::push_virtual`] — first, before
    /// any real commit — so HEAD keeps lane 0 and each incoming side takes
    /// the lane beside it that its own tip will arrive on.
    pub fn push_virtual_merging(&mut self, oid: &Oid, parent: &Oid, incoming: &[Oid]) -> GraphRow {
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
        // One leash per commit: git accepts a merge that names the same
        // side twice, and two lanes waiting for one id would draw the
        // second as an edge arriving from nowhere.
        let mut leashed = vec![*parent];
        for side in incoming {
            if leashed.contains(side) {
                continue;
            }
            leashed.push(*side);
            let side_lane = self.find_free_lane_near(lane);
            let side_color = self.take_color();
            self.occupy(side_lane, *side, side_color, true);
            segments.push(Segment {
                kind: SegmentKind::OutOfNode,
                lane: side_lane,
                color: side_color,
                dashed: true,
            });
        }
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
