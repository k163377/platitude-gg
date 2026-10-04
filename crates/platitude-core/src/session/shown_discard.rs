//! The discard log's picked entry on the graph (破棄記録仕様.md): the walk
//! takes the entry's tips as more starting points, and the commits only
//! those tips reach come out provisional — drawn for the entry, not
//! because the repository holds them ([`LogRow::provisional`]).
//!
//! The commits are the reader's answer (`discards::Discard::lost`), handed
//! in with the tips: the walk marks them as it meets them, and a pass laid
//! again off its record keeps them marked (`WalkedRow`).

use std::collections::{HashMap, HashSet};

use super::*;
use crate::discards::Stands;

/// One picked entry: where its parts start, and the commits only they
/// reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShownDiscard {
    /// Each part's tip — a branch's old tip, a copy of thrown-away work, a
    /// dropped stash, the commit a name stood on.
    pub tips: Vec<Oid>,
    pub lost: HashSet<Oid>,
    /// The tips that are stash-shaped (破棄記録仕様.md §2.1): sifted as a
    /// stash is, keeping only the commit the work stood on, so their index
    /// and untracked parents draw no rows.
    pub stashlike: HashSet<Oid>,
    /// The stash-shaped tips whose base is a commit made for them, by tip:
    /// drawn on the commit that base was made on, the base no row.
    pub stands: HashMap<Oid, Stands>,
}

impl RepoSession {
    /// Puts the entry on the graph, or takes the one there off (`None`), and
    /// walks again in place — the reader's place is kept, as a refresh keeps
    /// it (`refresh_log`). Nothing shown asked to show nothing walks no more.
    ///
    /// The walk's answer is told once it lands
    /// ([`SessionEvent::DiscardWalked`]), however it went: a tip past the
    /// window draws no row, so the pass can find the picture unchanged and
    /// send nothing of its own. So an entry equal to the one standing walks
    /// again too — another pick with the same tips (a branch and its remote
    /// deleted at one commit) waits on that answer like any other.
    pub fn show_discard(self: &Arc<Self>, shown: Option<ShownDiscard>) {
        let tip = shown.as_ref().and_then(|shown| shown.tips.first().copied());
        {
            let mut standing = relock(&self.shown_discard);
            if standing.is_none() && shown.is_none() {
                return;
            }
            *standing = shown.map(Arc::new);
        }
        let task = self.refresh_log_tracked();
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            if s.graph_answer(task).await.landed() {
                s.sink.event(SessionEvent::DiscardWalked { tip });
            }
        });
    }

    /// The entry on the graph, for a pass to take with it.
    pub(super) fn shown_discard(&self) -> Option<Arc<ShownDiscard>> {
        relock(&self.shown_discard).clone()
    }

    /// The tips the picked entry adds to the walk, none while nothing is
    /// picked.
    pub(super) fn shown_tips(&self) -> Vec<Oid> {
        self.shown_discard()
            .map(|shown| shown.tips.clone())
            .unwrap_or_default()
    }
}
