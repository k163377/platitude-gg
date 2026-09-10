//! What the sidebar is handed: one sorted reading of the repository's
//! refs, and the rows it lists them as.

use super::*;

/// Sidebar-ready refs snapshot (sorted).
///
/// **Keyed by name as well as listed.** The rows are what the sidebar
/// draws; the lookups ([`Self::local_named`] and the two beside it) are
/// what a menu asks as it opens — which remote a branch speaks for, which
/// sides a tag stands on — and a walk over fifty thousand rows per
/// question is what the budget rules out (CLAUDE.md 性能予算 — ref 同士の
/// 突き合わせは索引を 1 本作ってから回す). Branches are sorted by name
/// already; tags are sorted for the eye (newest first) and carry a second
/// order for the lookup.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefsSnapshot {
    pub locals: Vec<BranchItem>,
    pub remotes: Vec<BranchItem>,
    pub tags: Vec<TagItem>,
    /// `tags` in name order, as indices into it — the lookup's half of a
    /// list the eye reads newest-first. Four bytes a tag.
    pub tags_by_name: Vec<u32>,
    /// Tags this repository holds that a remote carries on some other
    /// commit. A run of its own rather than a field on every
    /// [`TagItem`]: a name standing on two commits is rare, and 45,901
    /// tags would each pay for the field (CLAUDE.md 性能予算 — refs の
    /// 本数に比例させない).
    pub tag_drifts: Vec<TagDrift>,
    pub head: Option<HeadState>,
    /// Names of the configured remotes, sorted. A branch with no upstream
    /// has to be told where to go, and this is the list to offer.
    pub remote_names: Vec<String>,
    /// Their fetch URLs, in the same order — what the form that corrects
    /// one opens with already filled in.
    pub remote_urls: Vec<String>,
    /// Which of them a push goes to when no branch says otherwise, and
    /// whether this repository's own config is what says so. `None` where
    /// nothing is marked — then a branch's own upstream decides, and a
    /// branch with none falls back to whichever remote is called `origin`.
    pub push_default: Option<crate::remote::PushDefault>,
}

impl RefsSnapshot {
    /// The local branch called `short`, by binary search over the sorted
    /// list.
    pub fn local_named(&self, short: &str) -> Option<&BranchItem> {
        Self::branch_named(&self.locals, short)
    }

    /// The remote-tracking branch shown as `short` (`origin/main`).
    pub fn remote_named(&self, short: &str) -> Option<&BranchItem> {
        Self::branch_named(&self.remotes, short)
    }

    /// The tag called `short`, through the name-ordered index.
    pub fn tag_named(&self, short: &str) -> Option<&TagItem> {
        self.tags_by_name
            .binary_search_by(|at| {
                self.tags
                    .get(*at as usize)
                    .map_or(std::cmp::Ordering::Less, |tag| {
                        tag.short.as_str().cmp(short)
                    })
            })
            .ok()
            .and_then(|found| self.tags_by_name.get(found))
            .and_then(|at| self.tags.get(*at as usize))
    }

    fn branch_named<'a>(sorted: &'a [BranchItem], short: &str) -> Option<&'a BranchItem> {
        sorted
            .binary_search_by(|branch| branch.short.as_str().cmp(short))
            .ok()
            .and_then(|at| sorted.get(at))
    }

    /// Builds the name-ordered index over `tags`, once the tags are in
    /// their own order. Called by whoever last sorted them — the joins
    /// here, and a test that builds a snapshot by hand.
    pub fn index_tags(&mut self) {
        let mut by_name: Vec<(&str, u32)> = self
            .tags
            .iter()
            .enumerate()
            .map(|(at, tag)| (tag.short.as_str(), at as u32))
            .collect();
        by_name.sort_unstable();
        self.tags_by_name = by_name.into_iter().map(|(_, at)| at).collect();
    }
}

/// One sidebar branch row.
///
/// The commit is kept as an [`Oid`] and spelled out where it is shown.
/// Forty hex characters is a string allocation per row for something no
/// reader ever sees in full — the graph is what it is for — and a
/// repository with tens of thousands of refs pays megabytes for them
/// (ci/baseline/code-costs-windows-x64.md §メモリの形).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchItem {
    pub short: crate::Name,
    pub full: crate::Name,
    pub oid: Oid,
    pub has_remote: bool,
    pub is_head: bool,
    /// For a local branch, the remote branch it speaks for (`origin/main`),
    /// wherever the two stand — the one its badge is about, and the one a
    /// rename offers to carry over. Empty when it speaks for none.
    pub upstream: crate::Name,
    /// The commit the configured upstream stands on — that reading, or
    /// under `remote = .` a branch here — and `None` where the branch has
    /// none or the ref it names is not in the listing. **The reference
    /// point `branch --delete` measures the tip against** (git's
    /// `branch_merged`: the upstream where it resolves, HEAD otherwise),
    /// so a menu can answer the safety valve off the drawn rows
    /// (`publish::reaches`) with exactly git's two halves.
    pub upstream_oid: Option<Oid>,
    /// That reading stands on **another commit**. The pair is then two
    /// rows on the graph rather than one folded chip
    /// ([`crate::refs::RemoteBranches::folded_into_local`]), and the
    /// menu's rows that reach the remote say why instead of running
    /// (デザイン規約 §左メニューの所作 の削除の表). False where there is
    /// no reading to have drifted.
    pub upstream_drifted: bool,
    /// Another working copy has this branch checked out, so git refuses
    /// both `switch` and `branch --delete` for it (measured) —
    /// whether or not that copy is **locked**, which stops a different
    /// set of commands. A `bool` and not the folder: this rides one per
    /// branch, and the reference repository has fifty thousand refs.
    ///
    /// Local branches only. A remote row lands on the local branch of
    /// the same name, and whether *that* one is held is a question the
    /// menu asks of the worktree list by name.
    pub held_elsewhere: bool,
    /// How far this branch stands from its upstream, as of the last fetch
    /// ([`crate::refs::RefEntry::ahead`]) — the pair the sidebar row
    /// draws. **Both zero says nothing to draw**, whether the two are
    /// level or there is no upstream to measure against.
    ///
    /// Local branches only. A remote-tracking ref is the far side of
    /// somebody's measurement, never a side that has one of its own.
    pub ahead: u32,
    pub behind: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagItem {
    pub short: crate::Name,
    /// Peeled commit id (what the graph row is keyed on). Binary, for the
    /// reason [`BranchItem::oid`] is.
    pub oid: Oid,
    pub annotated: bool,
    /// Creator date (unix seconds); the sidebar sorts tags newest-first.
    /// Zero for a tag only a remote has: an advertisement carries the name
    /// and the commit, and no date to sort by.
    pub created_unix: i64,
    /// A remote carries this name too (cloud badge).
    pub has_remote: bool,
    /// Whether this repository holds the tag. False lists a name only a
    /// remote has — the sidebar is where it can be read at all, since no
    /// local ref puts it on a graph row.
    pub here: bool,
}

/// One remote holding one tag on a commit this repository does not have
/// it on.
///
/// What the menu's push row is decided by: a plain push to a name the
/// remote already has elsewhere is refused outright, so that row comes up
/// as the leased overwrite instead — and [`Self::commit`] is the commit
/// the lease is pinned to, which is the one the reader was being shown
/// (デザイン規約 §相手の履歴を置き換える).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagDrift {
    pub name: crate::Name,
    pub remote: crate::Name,
    /// Where that remote has it, peeled the same way the local reading is.
    pub commit: Oid,
}
