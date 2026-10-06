//! What a log pass turns a walked batch into: the sifting that decides
//! which commits are shown, and the display row each one becomes.

use super::*;
use crate::discards::Stands;

/// The synthetic row for uncommitted changes on a branch with no commits
/// yet: nothing to reach down to, so the node stands alone where the first
/// commit will.
pub(super) fn wip_root_row(builder: &mut GraphBuilder) -> LogRow {
    let zero = Oid::zero_unsized();
    let g = builder.push_virtual_root();
    // Uncommitted work is on no remote.
    let published = false;
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
        published,
        carried: None,
        // The edges this row draws are leashes, not parents.
        parents: Box::default(),
        provisional: false,
    }
}

/// The synthetic row for uncommitted changes: zero id (what the UI
/// recognizes it by), no author, a dashed edge down to HEAD and one to
/// each side a standing merge brings in (`incoming`, from `MERGE_HEAD`),
/// so the row already draws the fork the merge commit will have.
pub(super) fn wip_row(head: &Oid, incoming: &[Oid], builder: &mut GraphBuilder) -> LogRow {
    let zero = Oid::zero_like(head);
    let g = builder.push_virtual_merging(head, incoming);
    let published = false;
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
        published,
        carried: None,
        // Leashes, not parents.
        parents: Box::default(),
        provisional: false,
    }
}

/// What one completed log pass answers for besides its rows: the counts,
/// and what it read off HEAD's own row.
///
/// The counts differ in both directions: the WIP row is shown but never
/// walked, a stash's synthetic parents are walked but never shown.
#[derive(Clone, Copy, Default)]
pub(super) struct LogTotals {
    /// Rows delivered to the UI.
    pub(super) shown: u32,
    /// Commits the walk emitted — what `--max-count` limits, so this
    /// decides `truncated`.
    pub(super) walked: u32,
    /// The commit the walk started from; `None` on a branch with no
    /// commits yet.
    pub(super) head: Option<Oid>,
    /// Whether a remote already has that commit, as the walk marked its
    /// row (`session::published`); `None` where the window stopped short
    /// of it, the one case asked of git (`RepoSession::settle_head_published`).
    pub(super) head_published: Option<bool>,
}

impl LogTotals {
    pub(super) fn note_row(&mut self, oid: &Oid, published: bool) {
        if self.head == Some(*oid) {
            self.head_published = Some(published);
        }
    }
}

/// One sifted stream entry (stash rows carry their reflog selector).
pub(super) struct StreamItem {
    pub(super) meta: CommitMeta,
    pub(super) stash_ref: Option<String>,
    /// Only the discard log's picked entry reaches it ([`LogRow::provisional`]).
    pub(super) provisional: bool,
}

impl StreamItem {
    /// The one place a row is built from a sifted entry, so the streaming
    /// pass, the buffered pass and the chunk emitter cannot draw a stash
    /// row three ways.
    pub(super) fn row(
        &self,
        pool: &crate::model::StrPool,
        builder: &mut GraphBuilder,
        marks: &mut PublishMarks,
    ) -> LogRow {
        let dashed = self.stash_ref.is_some() || self.provisional;
        let mut row = make_row(&self.meta, pool, builder, marks, dashed);
        if let Some(stash_ref) = &self.stash_ref {
            row.stash_ref = stash_ref.clone();
            // No remote carries a stash, whatever its base.
            row.published = false;
        }
        row.provisional = self.provisional;
        row
    }
}

/// Filters a parsed batch for display: stash commits keep only their
/// first-parent edge, and their synthetic index / untracked parents are
/// dropped when they arrive (the walk shows no parent before its
/// children, so the stash row always comes first). One whose base is a
/// commit a discard's copy made for it draws on the commit that base was
/// made on, and the base draws no row (`discards::Stands`).
fn sift_batch(
    batch: Vec<CommitMeta>,
    stashes: (&HashMap<Oid, String>, &HashMap<Oid, Stands>),
    shown: Option<&ShownDiscard>,
    skip: &mut std::collections::HashSet<Oid>,
    out: &mut Vec<StreamItem>,
) {
    let (stash_refs, stash_stands) = stashes;
    for mut meta in batch {
        if skip.contains(&meta.oid) {
            continue;
        }
        let stash_ref = stash_refs.get(&meta.oid).cloned();
        // A copy of thrown-away work and a dropped stash are stashes in all but name (`ShownDiscard::stashlike`).
        let snapshot = shown.is_some_and(|shown| shown.stashlike.contains(&meta.oid));
        let stands = shown
            .and_then(|shown| shown.stands.get(&meta.oid))
            .or_else(|| stash_stands.get(&meta.oid))
            .filter(|stands| meta.parents.first() == Some(&stands.made));
        if (stash_ref.is_some() || snapshot)
            && let Some(stands) = stands
        {
            skip.extend(meta.parents.iter().copied());
            meta.parents = Box::from([stands.on]);
        } else if (stash_ref.is_some() || snapshot) && meta.parents.len() > 1 {
            for extra in &meta.parents[1..] {
                skip.insert(*extra);
            }
            meta.parents = Box::from(&meta.parents[..1]);
        }
        let provisional = shown.is_some_and(|shown| shown.lost.contains(&meta.oid));
        out.push(StreamItem {
            meta,
            stash_ref,
            provisional,
        });
    }
}

/// The sifting state one pass carries across its batches, and the
/// commits emitted so far ([`LogTotals::walked`]).
///
/// Both passes sift through this, so the skip set and the count cannot
/// come apart from the batches that filled them.
pub(super) struct Sifter<'a> {
    stash_refs: &'a HashMap<Oid, String>,
    stash_stands: &'a HashMap<Oid, Stands>,
    /// The discard log's picked entry, whose commits come out provisional.
    shown: Option<&'a ShownDiscard>,
    skip: std::collections::HashSet<Oid>,
    pub(super) walked: u32,
}

impl<'a> Sifter<'a> {
    pub(super) fn new(
        stash_refs: &'a HashMap<Oid, String>,
        stash_stands: &'a HashMap<Oid, Stands>,
        shown: Option<&'a ShownDiscard>,
    ) -> Self {
        Self {
            stash_refs,
            stash_stands,
            shown,
            skip: std::collections::HashSet::new(),
            walked: 0,
        }
    }

    pub(super) fn take(&mut self, pending: &mut Vec<CommitMeta>) -> Vec<StreamItem> {
        let batch = std::mem::take(pending);
        self.walked += batch.len() as u32;
        let mut items = Vec::with_capacity(batch.len());
        sift_batch(
            batch,
            (self.stash_refs, self.stash_stands),
            self.shown,
            &mut self.skip,
            &mut items,
        );
        items
    }
}

/// Labels are the caller's to attach; `dashed_edge` is for stash rows and
/// provisional ones.
fn make_row(
    commit: &CommitMeta,
    pool: &crate::model::StrPool,
    builder: &mut GraphBuilder,
    marks: &mut PublishMarks,
    dashed_edge: bool,
) -> LogRow {
    let g = builder.push_with_edge_style(commit, dashed_edge);
    // Asked here, where the parents are: the mark is carried down the walk
    // from the rows a remote branch stands on (`session::published`).
    let published = marks.mark(commit);
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
        published,
        // Sifted, so a stash carries the one edge it draws (`sift_batch`).
        parents: commit.parents.clone(),
        carried: None,
        provisional: false,
    }
}

/// The rows for the other worktrees, handed out as the walk reaches
/// the commit each stands on.
///
/// A row stands above every synthetic row on the same commit (uncommitted
/// work is about to become a commit, a stash is not), so it is handed out
/// at whichever arrives first: the commit, or a stash built on it — a
/// stash sorts above its base by date. A worktree whose HEAD is outside the
/// window draws nothing.
pub(super) struct CarriedRows(Vec<crate::session::Carried>);

impl CarriedRows {
    pub(super) fn new(carried: &[crate::session::Carried]) -> Self {
        Self(carried.to_vec())
    }

    /// The rows owed to a stream entry, in listing order; a stash asks
    /// with its base.
    pub(super) fn take_for(
        &mut self,
        item: &StreamItem,
        builder: &mut GraphBuilder,
    ) -> Vec<LogRow> {
        let anchor = match item.stash_ref {
            Some(_) => match item.meta.parents.first() {
                Some(base) => *base,
                None => return Vec::new(),
            },
            None => item.meta.oid,
        };
        self.take_at(&anchor, builder)
    }

    /// The same, for a caller that has worked the anchor out itself
    /// (`session::relay`).
    pub(super) fn take_at(&mut self, anchor: &Oid, builder: &mut GraphBuilder) -> Vec<LogRow> {
        let anchor = *anchor;
        let mut out = Vec::new();
        // `retain`: a worktree handed out here is gone from the set, so a
        // stash and its base cannot both draw it.
        self.0.retain(|wip| {
            if wip.head != anchor {
                return true;
            }
            // No chip (デザイン規約 §未コミット行が名乗るもの): the worktree
            // rides on the row, for its words and for the pane it opens.
            let mut row = wip_row(&wip.head, &[], builder);
            row.carried = Some(wip.clone());
            out.push(row);
            false
        });
        out
    }
}
