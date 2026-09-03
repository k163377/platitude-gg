//! What the graph is walked with and what one row of it carries: the walk
//! options, the chips, and the row the pane draws.

use super::*;

/// What the log stream walks.
///
/// Tags are shown by default (product decision). On tag-heavy repositories
/// they dominate the walk's frontier setup (JetBrains/kotlin: 44k tags cost
/// ~1.7s extra before the first byte even with a commit-graph), which is
/// why the toggle exists.
#[derive(Debug, Clone, Copy)]
pub struct LogOptions {
    pub include_tags: bool,
    /// `None` walks the full history.
    pub limit: Option<u32>,
    /// What one press of the graph's tail adds to `limit`
    /// (`RepoSession::grow_log_window`).
    ///
    /// **Held rather than derived from `limit`**, which grows with every
    /// press: the step is a quarter of the window the graph *opened*
    /// with, so it stays the same size however deep the reader has gone
    /// (`session::log_window_step`).
    pub step: u32,
}

impl Default for LogOptions {
    fn default() -> Self {
        Self {
            include_tags: true,
            limit: Some(DEFAULT_LOG_LIMIT),
            step: log_window_step(DEFAULT_LOG_LIMIT),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LabelKind {
    /// Detached-HEAD marker (synthetic `HEAD` chip).
    Head,
    LocalBranch,
    RemoteBranch,
    Tag,
}

/// One label chip on a graph row (branch / tag / HEAD).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefLabel {
    pub text: crate::Name,
    pub kind: LabelKind,
    /// Cloud badge: this name is on a remote as well. The PR dimension is
    /// wired in Phase 4.
    pub has_remote: bool,
    /// True when HEAD is on this branch (bold chip).
    pub is_head: bool,
    /// Whether this repository holds the ref — the whereabouts the chip
    /// writes in the name's colour (デザイン規約 §ref の種別). False for a
    /// remote branch, and for a tag that is only over there or that points
    /// somewhere this one does not.
    pub here: bool,
    /// Whose reading this is, when it is not this repository's: the remote
    /// names carrying the tag, comma-separated. Empty for everything else.
    ///
    /// A remote branch says it in its own name (`origin/main`); a tag has
    /// no such namespace to say it in, and a drifted one puts the same
    /// bare name on two rows. The hover card is where those two meet, and
    /// this is what tells them apart there.
    pub remote: String,
    /// Another working copy has this branch checked out — the same bit
    /// the sidebar row reads (`BranchItem::held_elsewhere`), so the chip
    /// and the row cannot disagree about where a branch can be gone to.
    /// The chip has no room for a mark of its own and does not need one:
    /// it already has a way of saying "nowhere to go from here".
    pub held_elsewhere: bool,
}

/// The chips every commit carries, as one sorted run.
///
/// **Not a map of vectors.** A repository's refs are almost all singletons
/// — one name on one commit — and `HashMap<Oid, Vec<RefLabel>>` charges
/// twice for that shape: the table's power-of-two buckets, and a separate
/// four-slot `Vec` for every commit, because a `Vec` grown by one `push`
/// asks for four. Measured on `JetBrains/kotlin` (47,715 labelled
/// commits): 4.3MB of table plus 10.7MB of four-slot vectors, to hold
/// 2.7MB of labels.
///
/// Flat, the two questions asked of it stay as cheap. A streamed row looks
/// its own commit up (binary search, once per shown row), and the refresh
/// walks the whole thing in commit order.
#[derive(Debug, Default)]
pub(crate) struct LabelIndex {
    /// `(commit, first label, how many)`, sorted by commit.
    commits: Vec<(Oid, u32, u32)>,
    /// Every chip, grouped by commit and in the order they are drawn.
    labels: Vec<RefLabel>,
}

impl LabelIndex {
    /// Sorts loose `(commit, chip)` pairs into the run.
    ///
    /// The chips of one commit keep the order the row draws them in: the
    /// current branch first, then by kind, then by name.
    pub(crate) fn from_pairs(mut pairs: Vec<(Oid, RefLabel)>) -> Self {
        pairs.sort_by(|(left_oid, left), (right_oid, right)| {
            left_oid.cmp(right_oid).then_with(|| {
                (!left.is_head, left.kind, left.text.as_str()).cmp(&(
                    !right.is_head,
                    right.kind,
                    right.text.as_str(),
                ))
            })
        });
        let mut commits: Vec<(Oid, u32, u32)> = Vec::new();
        let mut labels: Vec<RefLabel> = Vec::with_capacity(pairs.len());
        for (oid, label) in pairs {
            match commits.last_mut() {
                Some((last, _, count)) if *last == oid => *count += 1,
                _ => commits.push((oid, labels.len() as u32, 1)),
            }
            labels.push(label);
        }
        commits.shrink_to_fit();
        Self { commits, labels }
    }

    /// The chips on one commit; empty when it carries none.
    ///
    /// `tags` is whether the graph is drawing tags at all — the TAGS
    /// band's eye. Off, a tag's chip goes with the rows the walk stopped
    /// covering, so a tag standing on a commit a branch also reaches
    /// stops being drawn rather than staying behind on its own.
    pub(crate) fn labels_of(&self, oid: &Oid, tags: bool) -> &[RefLabel] {
        let Ok(at) = self.commits.binary_search_by(|(c, _, _)| c.cmp(oid)) else {
            return &[];
        };
        match self.commits.get(at) {
            Some((_, first, count)) => Self::cut(self.run(*first, *count), tags),
            None => &[],
        }
    }

    /// Every commit carrying chips, in commit order, with them.
    ///
    /// A commit whose only chips were tags carries none once they are cut,
    /// and it is left out rather than yielded empty: what reads this tells
    /// "these chips" from "no chips" by whether the commit is in it.
    pub(crate) fn commits(&self, tags: bool) -> impl Iterator<Item = (Oid, &[RefLabel])> {
        self.commits.iter().filter_map(move |(oid, first, count)| {
            let run = Self::cut(self.run(*first, *count), tags);
            (!run.is_empty()).then_some((*oid, run))
        })
    }

    fn run(&self, first: u32, count: u32) -> &[RefLabel] {
        self.labels
            .get(first as usize..(first + count) as usize)
            .unwrap_or_default()
    }

    /// One commit's chips with the tags taken off the end.
    ///
    /// **A cut rather than a filter**, because `from_pairs` has already
    /// left them there: it sorts by `kind` after the current branch, and
    /// `Tag` is the last kind there is. So the tags of a commit are the
    /// tail of its run, and dropping them is a shorter slice of the same
    /// labels — no second index, and nothing allocated to hide a chip.
    fn cut(run: &[RefLabel], tags: bool) -> &[RefLabel] {
        if tags {
            return run;
        }
        let kept = run
            .iter()
            .take_while(|label| label.kind != LabelKind::Tag)
            .count();
        debug_assert!(
            run[kept..].iter().all(|label| label.kind == LabelKind::Tag),
            "from_pairs leaves a commit's tags at the tail of its run"
        );
        &run[..kept]
    }

    /// Commits carrying chips.
    pub(crate) fn len(&self) -> usize {
        self.commits.len()
    }
}

impl crate::mem::Footprint for LabelIndex {
    fn heap_bytes(&self) -> usize {
        self.commits.capacity() * size_of::<(Oid, u32, u32)>() + self.labels.heap_bytes()
    }
}

/// Display-ready row of the commit graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRow {
    pub row: u32,
    pub oid_hex: String,
    pub short_sha: String,
    pub author: String,
    /// The author's address, lowercased and mailmapped — what a locally
    /// assigned picture is filed under. Empty on the WIP row, which has
    /// no author until it is committed.
    pub author_email: String,
    /// Whoever the message credits alongside the author, in its order.
    /// Empty on the WIP and stash rows.
    pub co_authors: Vec<crate::details::CoAuthor>,
    /// Author time (unix seconds); formatting is presentation.
    pub time: i64,
    pub subject: String,
    /// Everything after the subject, minus the co-author trailers — what
    /// the row's hover reads out. Empty on the WIP and stash rows.
    pub body: String,
    pub node_lane: u16,
    pub node_color: u8,
    pub width: u16,
    pub segments: Vec<Segment>,
    pub labels: Vec<RefLabel>,
    /// Reflog selector (`stash@{n}`) when this row is a stash; empty for
    /// ordinary commits and the WIP row.
    pub stash_ref: String,
    /// Whether a remote-tracking branch already reaches this commit —
    /// what every "this rewrites published history" warning reads
    /// (`session::published`). False on the WIP and stash rows, which no
    /// remote has.
    ///
    /// **On the row rather than asked per question.** The walk that drew
    /// the row already knows it, so a menu opened on the row has the
    /// answer the moment it opens instead of a `git rev-list` later
    /// (デザイン規約 §メニュー).
    pub published: bool,
}
