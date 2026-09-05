//! Commit-graph lane assignment (gitk-style greedy allocation).
//!
//! Consumes commits in `--date-order` (children before parents) and emits,
//! per row, everything the UI needs to draw that row in isolation: the node
//! position/color plus straight and curved lane segments. QML only draws;
//! no layout decisions happen on the UI side (実装計画 §2).
//!
//! The engine is incremental: rows stream out while `git log` is still
//! running, so the first chunk can be painted immediately.

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

/// Nominal number of distinct chain colors; must match the `graphLane`
/// palette in the design tokens (internal-docs/デザイン規約.md). The engine
/// only hands out palette indices.
pub const GRAPH_PALETTE_SIZE: u32 = 8;

/// Whether a status leaves the synthetic working-tree row standing at the
/// head of the graph.
///
/// **The one place the rule is written.** The walk asks it of the record a
/// status read left behind (`session::RepoSession::pending_commit`), and a
/// reader that has the status in hand asks it here — a second reader
/// spelling out "dirty, or an operation with a card" would be a rule that
/// can drift from the one the rows are actually built by.
///
/// The row stands while the tree is dirty *or* an operation is: the `edit`
/// stop and the emptied-commit stop both leave the tree clean while the
/// exit card waits under the row. "An operation" is one with an exit card
/// to land — a bisect also flips `OpState::any()`, and it has no card and
/// changes nothing about the tree
/// (デザイン規約 §進行中の操作から出る「着地は WIP 行」).
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
