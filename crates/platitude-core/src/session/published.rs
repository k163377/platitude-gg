//! Which of the walked commits a remote already has — the answer every
//! "this rewrites published history" warning rests on, taken off the walk
//! already drawing them, so a menu reads it from the row instead of a
//! process (デザイン規約 §メニュー wants it in hand as the card opens).
//!
//! "Published" is what `--not --remotes` means in [`crate::publish`]:
//! reachable from a remote-tracking *branch*, so a tag a remote carries
//! marks nothing. Both write the same warning, so they have to agree.

use std::collections::HashSet;

use tokio_util::sync::CancellationToken;

use super::{RepoSession, relock};
use crate::model::CommitMeta;
use crate::oid::Oid;

/// The commits remote-tracking branches point at, sorted for lookup.
///
/// Built once per pass and searched per row: the walk runs across awaits
/// and cannot hold the label index's lock (CLAUDE.md §性能予算).
#[derive(Debug, Default, Clone)]
pub(super) struct RemoteTips {
    oids: Vec<Oid>,
}

impl RemoteTips {
    /// Sorts tip ids into the index, dropping duplicates (many branches
    /// can stand on one commit).
    pub(super) fn new(mut oids: Vec<Oid>) -> Self {
        oids.sort_unstable();
        oids.dedup();
        oids.shrink_to_fit();
        Self { oids }
    }

    fn holds(&self, oid: &Oid) -> bool {
        self.oids.binary_search(oid).is_ok()
    }

    /// Whether any remote-tracking branch was read; with none, nothing is
    /// published and the marking looks nothing up.
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
/// **Exact for every row the walk emits**: `--date-order` never emits a
/// parent before all its children (the stash sifting in `rows.rs` rests
/// on the same), so the rows are a topological prefix — a remote tip a
/// row is behind was emitted earlier, inside the window, and every child
/// that could mark a row has been through here when it is emitted.
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
    /// **Asked exactly once per commit, in walk order**; nothing goes back
    /// to fix an earlier answer — the prefix property above makes that
    /// sound.
    pub(super) fn mark(&mut self, commit: &CommitMeta) -> bool {
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
    /// **Off the refs snapshot, not `LabelIndex`**: the drawing index folds
    /// a remote branch into its local counterpart's chip (`build_label_map`
    /// — `joins.folded`), so a remote read from there goes missing exactly
    /// when the two names agree.
    ///
    /// **Asked of git when no snapshot has landed yet**: at open the walk
    /// can beat the refs read, and its rows would stay unpublished — the
    /// first refs read is not a *move*, so it rebuilds nothing
    /// (`publish_refs`). One narrow listing (`refs::remote_tips`) settles it.
    pub(super) async fn remote_tips(
        &self,
        workdir: &std::path::Path,
        cancel: &CancellationToken,
    ) -> RemoteTips {
        if let Some(snapshot) = relock(&self.last_snapshot).clone() {
            return RemoteTips::new(snapshot.remotes.iter().map(|b| b.oid).collect());
        }
        // A failed listing leaves the marks empty and the pass running;
        // the next pass asks again.
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
