//! Which of the walked commits a remote already has.
//!
//! The answer every "this rewrites published history" warning rests on,
//! taken off the walk that is already drawing those commits. A menu
//! opened on a row reads it from the row (デザイン規約 §メニュー wants the
//! answer in hand as the card opens, and a `git rev-list` is a whole
//! process away).
//!
//! "Published" is what `--not --remotes` means in [`crate::publish`]:
//! reachable from some remote-tracking *branch*. Tags are not in
//! `--remotes`, so a tag a remote also carries marks nothing here — the
//! two answers have to agree, because the same warning is written from
//! both.

use std::collections::HashSet;

use tokio_util::sync::CancellationToken;

use super::{RepoSession, relock};
use crate::model::CommitMeta;
use crate::oid::Oid;

/// The commits remote-tracking branches point at, sorted for lookup.
///
/// Built once per pass and searched once per row: the walk runs across
/// awaits and cannot hold the label index's lock
/// (CLAUDE.md 性能予算 — refs は 5 万本
/// ありうるので索引を 1 本作ってから回す).
#[derive(Debug, Default, Clone)]
pub(super) struct RemoteTips {
    oids: Vec<Oid>,
}

impl RemoteTips {
    /// Sorts loose tip ids into the index, dropping the duplicates many
    /// branches standing on one commit would otherwise leave.
    pub(super) fn new(mut oids: Vec<Oid>) -> Self {
        oids.sort_unstable();
        oids.dedup();
        oids.shrink_to_fit();
        Self { oids }
    }

    fn holds(&self, oid: &Oid) -> bool {
        self.oids.binary_search(oid).is_ok()
    }

    /// Whether any remote-tracking branch was read at all. A repository
    /// with none has nothing published, which is the answer the marking
    /// then gives without looking anything up.
    pub(super) fn is_empty(&self) -> bool {
        self.oids.is_empty()
    }
}

impl crate::mem::Footprint for RemoteTips {
    fn heap_bytes(&self) -> usize {
        self.oids.capacity() * size_of::<Oid>()
    }
}

/// Carries "a remote has this" down the walk, from the rows a remote
/// branch stands on to the commits behind them.
///
/// **This is exact for every row the walk emits.** The walk is
/// `--date-order`, which never emits a parent before all of its
/// children (the stash sifting in `rows.rs` already
/// rests on the same guarantee), so the rows are a topological prefix: if
/// an emitted row is an ancestor of a remote tip, that tip is a
/// descendant and was emitted earlier — inside the window too. A row can
/// therefore be answered for the moment it is emitted, because every
/// child that could have marked it has already been through here.
#[derive(Debug, Default)]
pub(super) struct PublishMarks {
    tips: RemoteTips,
    /// Parents of commits a remote has, waiting for the walk to reach
    /// them. Bounded by the open lanes: an id goes in when its child is
    /// emitted and comes out when it is.
    pending: HashSet<Oid>,
}

impl PublishMarks {
    pub(super) fn new(tips: RemoteTips) -> Self {
        Self {
            tips,
            pending: HashSet::new(),
        }
    }

    /// Commit ids being held — the count beside its bytes, the way the
    /// graph builder reports its own.
    pub(super) fn tracked_oids(&self) -> usize {
        self.tips.oids.len() + self.pending.len()
    }

    /// Answers for one commit as it is emitted, and remembers its parents
    /// when the answer is yes.
    ///
    /// **Asked exactly once per commit, in walk order.** A later call
    /// cannot change an earlier answer, and nothing here goes back to fix
    /// one — the prefix property above is what makes that sound.
    pub(super) fn mark(&mut self, commit: &CommitMeta) -> bool {
        // Nothing is published where no remote-tracking branch was read,
        // so the walk pays no lookup at all in a repository without one.
        if self.tips.is_empty() {
            return false;
        }
        let published = self.pending.remove(&commit.oid) || self.tips.holds(&commit.oid);
        if published {
            self.pending.extend(commit.parents.iter().copied());
        }
        published
    }
}

impl RepoSession {
    /// Where the remote-tracking branches stand, for the pass about to
    /// walk.
    ///
    /// **Off the refs snapshot.** `LabelIndex` is the drawing index
    /// and folds a remote branch into its local counterpart's chip
    /// (`build_label_map` — `joins.folded`), so a remote read from
    /// there goes missing exactly when the two names agree. What is
    /// wanted here is where the refs are, which is the snapshot's
    /// half.
    ///
    /// **Asked of git when no snapshot has landed yet.** The refs read and
    /// the graph walk are independent, and at open the walk can be first
    /// — its rows would then all be marked unpublished, and nothing would
    /// come back to correct them (the first refs read is not a *move*, so
    /// it rebuilds nothing: `publish_refs`). One narrow listing settles it
    /// (`refs::remote_tips`), and only ever on the pass that beat the
    /// refs read.
    pub(super) async fn remote_tips(
        &self,
        workdir: &std::path::Path,
        cancel: &CancellationToken,
    ) -> RemoteTips {
        if let Some(snapshot) = relock(&self.last_snapshot).clone() {
            return RemoteTips::new(snapshot.remotes.iter().map(|b| b.oid).collect());
        }
        // A listing that failed leaves the marks empty and the
        // pass running: no warning is the same answer this gave
        // before anything was read, and the next pass asks again.
        match crate::refs::remote_tips(&self.executor, workdir, cancel).await {
            Ok(oids) => RemoteTips::new(oids),
            Err(e) => {
                tracing::warn!(error = %e, "could not read where the remotes are");
                RemoteTips::default()
            }
        }
    }
}

impl crate::mem::Footprint for PublishMarks {
    fn heap_bytes(&self) -> usize {
        self.tips.heap_bytes() + self.pending.heap_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(n: u8) -> Oid {
        let hex = format!("{n:02x}").repeat(20);
        // Test-only helper; the input is always valid hex.
        Oid::from_hex_str(&hex).unwrap()
    }

    fn commit(id: u8, parents: &[u8]) -> CommitMeta {
        CommitMeta {
            oid: oid(id),
            parents: parents.iter().map(|p| oid(*p)).collect(),
            author: 0,
            author_email: 0,
            co_authors: Box::new([]),
            body: "".into(),
            time: 1_700_000_000 + i64::from(id),
            subject: format!("commit {id:02x}").into_boxed_str(),
        }
    }

    #[test]
    fn a_remote_tip_and_everything_behind_it_is_published() {
        let mut marks = PublishMarks::new(RemoteTips::new(vec![oid(3)]));
        // Walk order: children before parents.
        assert!(!marks.mark(&commit(1, &[2])), "above the tip");
        assert!(!marks.mark(&commit(2, &[3])), "above the tip");
        assert!(marks.mark(&commit(3, &[4])), "the tip itself");
        assert!(marks.mark(&commit(4, &[5])), "behind the tip");
        assert!(marks.mark(&commit(5, &[])), "behind the tip");
    }

    #[test]
    fn a_merge_publishes_both_of_its_sides() {
        let mut marks = PublishMarks::new(RemoteTips::new(vec![oid(1)]));
        assert!(marks.mark(&commit(1, &[2, 3])));
        assert!(marks.mark(&commit(2, &[])), "first parent");
        assert!(marks.mark(&commit(3, &[])), "second parent");
    }

    #[test]
    fn a_side_no_remote_reaches_stays_unpublished() {
        let mut marks = PublishMarks::new(RemoteTips::new(vec![oid(2)]));
        // 1 is a local commit on top of the remote's tip; 9 is a local
        // branch of its own that no remote points into.
        assert!(!marks.mark(&commit(1, &[2])));
        assert!(!marks.mark(&commit(9, &[8])));
        assert!(marks.mark(&commit(2, &[3])));
        assert!(marks.mark(&commit(3, &[])));
        assert!(!marks.mark(&commit(8, &[])));
    }

    #[test]
    fn without_remotes_nothing_is_published() {
        let mut marks = PublishMarks::new(RemoteTips::default());
        assert!(!marks.mark(&commit(1, &[2])));
        assert!(!marks.mark(&commit(2, &[])));
    }

    #[test]
    fn many_branches_on_one_commit_index_once() {
        let tips = RemoteTips::new(vec![oid(4), oid(4), oid(4), oid(2)]);
        assert_eq!(tips.oids, vec![oid(2), oid(4)]);
    }
}
