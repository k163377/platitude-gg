//! What a log pass turns a walked batch into: the sifting that decides
//! which commits are shown, and the display row each one becomes.

use super::*;

/// Builds the synthetic row for uncommitted changes: zero id, no author,
/// one dashed edge running down to HEAD. The UI recognizes the all-zero
/// id and renders the dashed empty node and the WIP subject.
pub(super) fn wip_row(head: &Oid, builder: &mut GraphBuilder) -> LogRow {
    let zero = Oid::zero_like(head);
    let g = builder.push_virtual(&zero, head);
    LogRow {
        row: g.row,
        oid_hex: zero.to_hex(),
        short_sha: zero.short_hex(8),
        author: String::new(),
        author_email: String::new(),
        co_authors: Vec::new(),
        time: 0,
        subject: String::new(),
        body: String::new(),
        node_lane: g.node_lane,
        node_color: g.node_color,
        width: g.width,
        segments: g.segments,
        labels: Vec::new(),
        stash_ref: String::new(),
    }
}

/// Row counts of one completed log pass. They differ in both directions:
/// the synthetic WIP row is shown but never walked, and a stash's
/// synthetic index/untracked parents are walked but never shown.
#[derive(Clone, Copy, Default)]
pub(super) struct LogTotals {
    /// Rows delivered to the UI.
    pub(super) shown: u32,
    /// Commits the walk emitted — what `--max-count` limits, so this is
    /// what decides `truncated`.
    pub(super) walked: u32,
}

/// One sifted stream entry (stash rows carry their reflog selector).
pub(super) struct StreamItem {
    pub(super) meta: CommitMeta,
    pub(super) stash_ref: Option<String>,
}

impl StreamItem {
    /// This entry as a display row. The stash selector is what asks for
    /// the dashed first-parent edge, and is put back onto the row after
    /// the build — a row carries the selector the operations act on.
    ///
    /// The one place a row is built from a sifted entry: the streaming
    /// pass, the buffered pass and the chunk emitter all come through
    /// here, and the three of them drawing a stash row differently is the
    /// bug this shape is here to make impossible.
    pub(super) fn row(&self, pool: &crate::model::StrPool, builder: &mut GraphBuilder) -> LogRow {
        let mut row = make_row(&self.meta, pool, builder, self.stash_ref.is_some());
        if let Some(stash_ref) = &self.stash_ref {
            row.stash_ref = stash_ref.clone();
        }
        row
    }
}

/// Filters a parsed batch for display: stash commits keep only their
/// first-parent edge (the base commit), and their synthetic index /
/// untracked parent commits are recorded and dropped when they arrive
/// later (the walk shows no parent before all of its children, so the
/// stash row always streams first).
fn sift_batch(
    batch: Vec<CommitMeta>,
    stash_refs: &HashMap<Oid, String>,
    skip: &mut std::collections::HashSet<Oid>,
    out: &mut Vec<StreamItem>,
) {
    for mut meta in batch {
        if skip.contains(&meta.oid) {
            continue;
        }
        let stash_ref = stash_refs.get(&meta.oid).cloned();
        if stash_ref.is_some() && meta.parents.len() > 1 {
            for extra in &meta.parents[1..] {
                skip.insert(*extra);
            }
            meta.parents = Box::from(&meta.parents[..1]);
        }
        out.push(StreamItem { meta, stash_ref });
    }
}

/// What one pass sifts its batches against, and what they have cost so
/// far: the stash oids whose extra parents are folded away, the synthetic
/// parents already spoken for, and the commits the walk has emitted
/// ([`LogTotals::walked`]).
///
/// Both passes drive the sifting through this — the streaming one when a
/// chunk's worth has piled up, the buffered one on every callback — so
/// the skip set and the count cannot come apart from the batches that
/// filled them.
pub(super) struct Sifter<'a> {
    stash_refs: &'a HashMap<Oid, String>,
    skip: std::collections::HashSet<Oid>,
    /// Commits the walk emitted, batch by batch.
    pub(super) walked: u32,
}

impl<'a> Sifter<'a> {
    pub(super) fn new(stash_refs: &'a HashMap<Oid, String>) -> Self {
        Self {
            stash_refs,
            skip: std::collections::HashSet::new(),
            walked: 0,
        }
    }

    /// Takes everything the parser has produced so far and sifts it for
    /// display, leaving `pending` empty for the next batch.
    pub(super) fn take(&mut self, pending: &mut Vec<CommitMeta>) -> Vec<StreamItem> {
        let batch = std::mem::take(pending);
        self.walked += batch.len() as u32;
        let mut items = Vec::with_capacity(batch.len());
        sift_batch(batch, self.stash_refs, &mut self.skip, &mut items);
        items
    }
}

/// Builds one display row from a commit (labels attached by the caller).
/// `dashed_edge` draws the first-parent edge dashed (stash rows).
fn make_row(
    commit: &CommitMeta,
    pool: &crate::model::StrPool,
    builder: &mut GraphBuilder,
    dashed_edge: bool,
) -> LogRow {
    let g = builder.push_with_edge_style(commit, dashed_edge);
    LogRow {
        row: g.row,
        oid_hex: commit.oid.to_hex(),
        short_sha: commit.oid.short_hex(8),
        author: pool.get(commit.author).to_string(),
        author_email: pool.get(commit.author_email).to_string(),
        co_authors: commit
            .co_authors
            .iter()
            .map(|(name, email)| crate::details::CoAuthor {
                name: pool.get(*name).to_string(),
                email: pool.get(*email).to_string(),
            })
            .collect(),
        time: commit.time,
        subject: commit.subject.to_string(),
        body: commit.body.to_string(),
        node_lane: g.node_lane,
        node_color: g.node_color,
        width: g.width,
        segments: g.segments,
        labels: Vec::new(),
        stash_ref: String::new(),
    }
}
