//! What the graph is walked with and what one row of it carries: the walk
//! options, the chips, and the row the pane draws.

use super::*;

/// What the log stream walks.
///
/// Tags are shown by default (product decision); on tag-heavy repositories
/// they dominate the walk's frontier setup, which is why the toggle exists
/// (ci/baseline/code-costs-windows-x64.md §git のプロセス代).
#[derive(Debug, Clone, Copy)]
pub struct LogOptions {
    pub include_tags: bool,
    /// `None` walks the full history.
    pub limit: Option<u32>,
    /// What one press of the graph's tail adds to `limit`. Held rather than
    /// derived from `limit`: it is a quarter of the window the graph opened
    /// with (`session::log_window_step`).
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
    /// Another working copy standing on this commit with no branch out — a
    /// synthetic chip like `Head`. A copy with a branch out is said by that
    /// branch's chip instead (`RefLabel::held_elsewhere`).
    ///
    /// Before `Tag`: the tags must stay the tail of a commit's run for the
    /// TAGS eye's cut ([`LabelIndex::cut`]).
    Worktree,
    Tag,
}

/// One label chip on a graph row (branch / tag / HEAD).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefLabel {
    pub text: crate::Name,
    pub kind: LabelKind,
    /// Cloud badge: this name is on a remote as well.
    pub has_remote: bool,
    /// True when HEAD is on this branch (bold chip).
    pub is_head: bool,
    /// Whether this repository holds the ref — the name's colour
    /// (デザイン規約 §ref の種別). False for a remote branch, and for a tag
    /// that is only over there or drifted.
    pub here: bool,
    /// For a tag read off the remotes: the remote names carrying it,
    /// comma-separated; empty otherwise. A tag has no `origin/` namespace,
    /// so this is what tells a drift's two same-named chips apart on the
    /// hover card.
    pub remote: String,
    /// Another working copy has this branch checked out — the same bit as
    /// `BranchItem::held_elsewhere`. The chip draws its green frame off
    /// this (デザイン規約 §ref の種別).
    pub held_elsewhere: bool,
    /// `git worktree lock` is on the copy standing here — the branch's
    /// holder, or the `Worktree` marker's copy. False where no copy stands.
    pub locked: bool,
}

/// The chips every commit carries, as one sorted run.
///
/// Flat rather than `HashMap<Oid, Vec<RefLabel>>`: refs are almost all one
/// name on one commit, and the map's buckets plus a four-slot `Vec` per
/// commit cost several times the labels themselves
/// (ci/baseline/code-costs-windows-x64.md §メモリの形). A streamed row
/// still looks its commit up by binary search.
#[derive(Debug, Default)]
pub(crate) struct LabelIndex {
    /// `(commit, first label, how many)`, sorted by commit.
    commits: Vec<(Oid, u32, u32)>,
    /// Every chip, grouped by commit and in the order they are drawn.
    labels: Vec<RefLabel>,
}

impl LabelIndex {
    /// Sorts loose `(commit, chip)` pairs into the run, each commit's chips
    /// in the order the row draws them.
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

    /// The chips on one commit. `tags` is the TAGS band's eye: off, a
    /// tag's chip goes too, even on a commit a branch reaches.
    pub(crate) fn labels_of(&self, oid: &Oid, tags: bool) -> &[RefLabel] {
        let Ok(at) = self.commits.binary_search_by(|(c, _, _)| c.cmp(oid)) else {
            return &[];
        };
        match self.commits.get(at) {
            Some((_, first, count)) => Self::cut(self.run(*first, *count), tags),
            None => &[],
        }
    }

    /// Every commit carrying chips, in commit order. A commit whose only
    /// chips were cut tags is left out — readers take absence as "no
    /// chips".
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

    /// One commit's chips with the tags taken off the end — a shorter
    /// slice, since `from_pairs` sorts `Tag`, the last kind, to the tail.
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

/// One row of a graph laid out again without walking
/// ([`SessionEvent::LogRelaid`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelaidRow {
    /// A commit the consumer has drawn — on screen now, or taken away by
    /// an earlier relaying — on its new lanes, wearing `labels`. All else
    /// about the row is as the consumer has it.
    Moved {
        oid: Oid,
        lanes: GraphRow,
        labels: Vec<RefLabel>,
    },
    /// A row made fresh from the readings: this window's uncommitted row,
    /// or another working copy's.
    Made(Box<LogRow>),
}

/// Display-ready row of the commit graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRow {
    pub row: u32,
    pub oid_hex: String,
    pub short_sha: String,
    pub author: String,
    /// The author's address, lowercased and mailmapped — what a locally
    /// assigned picture is filed under. Empty on the WIP row.
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
    /// (`session::published`). False on the WIP and stash rows. Carried on
    /// the row so a menu has it the moment it opens (デザイン規約 §メニュー).
    pub published: bool,
    /// The commit's parents as the walk sifted them — a stash keeps only
    /// the base it was built on (`rows::sift_batch`). Empty on the WIP
    /// row. Not for drawing (`segments` is): for asking about a range
    /// ([`crate::publish::range_rewrites_published`]).
    pub parents: Box<[Oid]>,
    /// The other working copy this uncommitted row is about; `None` on
    /// commits, stashes and this window's own uncommitted row. This is the
    /// row's identity, since every uncommitted row carries the all-zero id
    /// (`session::carried`).
    pub carried: Option<super::Carried>,
    /// A commit only the discard log's picked entry reaches — drawn for
    /// that entry, not because the repository holds it (破棄記録仕様.md,
    /// [`super::ShownDiscard`]). Its edges draw dashed. False everywhere
    /// else.
    pub provisional: bool,
}
