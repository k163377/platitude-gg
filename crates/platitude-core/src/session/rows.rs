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

/// Filters a parsed batch for display: stash commits keep only their
/// first-parent edge (the base commit), and their synthetic index /
/// untracked parent commits are recorded and dropped when they arrive
/// later (the walk shows no parent before all of its children, so the
/// stash row always streams first).
pub(super) fn sift_batch(
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

/// Builds one display row from a commit (labels attached by the caller).
/// `dashed_edge` draws the first-parent edge dashed (stash rows).
pub(super) fn make_row(
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
