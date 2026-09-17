//! What a log pass turns a walked batch into: the sifting that decides
//! which commits are shown, and the display row each one becomes.

use super::*;

/// Builds the synthetic row for uncommitted changes: zero id, no author,
/// one dashed edge running down to HEAD. The UI recognizes the all-zero
/// id and renders the dashed empty node and the WIP subject.
///
/// `incoming` is what a standing merge is bringing in (`MERGE_HEAD`, empty
/// otherwise): each side gets a dashed edge of its own, so the row already
/// draws the fork the merge commit will have.
/// The same row on a branch with no commits yet: nothing to reach down
/// to, so the node stands alone where the first commit will.
pub(super) fn wip_root_row(builder: &mut GraphBuilder) -> LogRow {
    let zero = Oid::zero_unsized();
    let g = builder.push_virtual_root();
    // Uncommitted work is on no remote, and there is no commit to ask about.
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
        // The edges this row draws are leashes.
        parents: Box::default(),
    }
}

pub(super) fn wip_row(head: &Oid, incoming: &[Oid], builder: &mut GraphBuilder) -> LogRow {
    let zero = Oid::zero_like(head);
    let g = builder.push_virtual_merging(head, incoming);
    // Uncommitted work is on no remote, and there is no commit to ask about.
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
        // As above: the dashed edges to HEAD and to each incoming side
        // are drawn and no more.
        parents: Box::default(),
    }
}

/// What one completed log pass has to answer for besides its rows: the
/// counts, and what it read off HEAD's own row.
///
/// The counts differ in both directions: the synthetic WIP row is shown
/// but never walked, and a stash's synthetic index/untracked parents are
/// walked but never shown.
#[derive(Clone, Copy, Default)]
pub(super) struct LogTotals {
    /// Rows delivered to the UI.
    pub(super) shown: u32,
    /// Commits the walk emitted — what `--max-count` limits, so this is
    /// what decides `truncated`.
    pub(super) walked: u32,
    /// The commit the walk started from; `None` on a branch with no
    /// commits yet.
    pub(super) head: Option<Oid>,
    /// Whether a remote already has that commit, as the walk marked its
    /// row (`session::published`); `None` where the window stopped short
    /// of it — which is the one case that has to be asked of git
    /// (`RepoSession::settle_head_published`).
    pub(super) head_published: Option<bool>,
}

impl LogTotals {
    /// Keeps the mark a row carries where the row is HEAD's own.
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
    pub(super) fn row(
        &self,
        pool: &crate::model::StrPool,
        builder: &mut GraphBuilder,
        marks: &mut PublishMarks,
    ) -> LogRow {
        let mut row = make_row(&self.meta, pool, builder, marks, self.stash_ref.is_some());
        if let Some(stash_ref) = &self.stash_ref {
            row.stash_ref = stash_ref.clone();
            // A stash is off to one side of every branch and no remote
            // carries it, whatever the commit it was built on.
            row.published = false;
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
    marks: &mut PublishMarks,
    dashed_edge: bool,
) -> LogRow {
    let g = builder.push_with_edge_style(commit, dashed_edge);
    // Asked here because this is where the parents are: the mark is
    // carried down the walk from the rows a remote branch stands on
    // (`session::published`).
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
    }
}

/// The rows the other working copies get, handed out as the walk reaches
/// what each of them is standing on.
///
/// **A row stands above every synthetic row landing on the same commit**
/// — uncommitted work is what is about to become a commit and a stash is
/// not, so it is asked for at whichever arrives first: the
/// commit itself, or a stash built on it. A stash on the same
/// commit sorts above it by date, so the commit alone is the
/// wrong anchor.
///
/// **A copy whose commit the walk never reaches draws nothing.** The rows
/// are handed out by the walk, so a HEAD outside the window is a row that
/// simply never comes.
pub(super) struct CarriedRows(Vec<crate::session::Carried>);

impl CarriedRows {
    pub(super) fn new(carried: &[crate::session::Carried]) -> Self {
        Self(carried.to_vec())
    }

    /// The rows owed to a stream entry, in the order the listing had them.
    /// Each copy is handed out once: the anchor is the commit a row lands
    /// on, and a stash asks with the base it was built on.
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

    /// The same, for a caller that has worked the anchor out itself — a
    /// graph being laid out again has the rows, and the anchor is the
    /// same commit either way
    /// (`session::relay`).
    pub(super) fn take_at(&mut self, anchor: &Oid, builder: &mut GraphBuilder) -> Vec<LogRow> {
        let anchor = *anchor;
        let mut out = Vec::new();
        // `retain`: a copy handed out here is gone from the set, so a
        // stash and its base cannot both draw it.
        self.0.retain(|wip| {
            if wip.head != anchor {
                return true;
            }
            let mut row = wip_row(&wip.head, &[], builder);
            row.labels = vec![RefLabel {
                text: wip.name.clone(),
                kind: LabelKind::Worktree,
                has_remote: false,
                is_head: false,
                here: true,
                remote: String::new(),
                held_elsewhere: false,
            }];
            row.carried = Some(wip.clone());
            out.push(row);
            false
        });
        out
    }
}
