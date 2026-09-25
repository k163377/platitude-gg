//! Commit-graph lane assignment (gitk-style greedy allocation).
//!
//! Consumes commits in `--date-order` (children before parents) and emits,
//! per row, everything needed to draw that row in isolation; QML makes no
//! layout decisions (実装計画 §2). Rows stream out while `git log` is still
//! running.

mod builder;

#[cfg(test)]
mod dag_tests;
#[cfg(test)]
mod lane_tests;
#[cfg(test)]
mod leash_tests;
#[cfg(test)]
mod testkit;

pub use builder::GraphBuilder;

/// Number of chain colors; must match the length of the `graphLane`
/// palette (デザイン規約).
pub const GRAPH_PALETTE_SIZE: u32 = 8;

/// Whether a status leaves the synthetic working-tree row standing at the
/// head of the graph. The one place the rule is written: every reader asks
/// here, so the window cannot drift from the rows the walk builds.
///
/// The row stands while the tree is dirty or an operation with an exit card
/// is in progress (the `edit` and emptied-commit stops leave the tree clean
/// with the card under the row). A bisect flips `OpState::any()` but has no
/// card, so it does not count (デザイン規約 §進行中の操作から出る「出口は WIP ペインに立つ」).
pub fn wip_row_stands(
    status: &crate::status::WorkTreeStatus,
    op: &crate::opstate::OpState,
) -> bool {
    status.is_dirty() || crate::integrate::InProgress::from_state(op).is_some()
}

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
    /// A synthetic leash (WIP → HEAD, stash → base); real history is
    /// always solid.
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
