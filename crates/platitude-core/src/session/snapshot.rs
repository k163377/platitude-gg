//! What the sidebar is handed: one sorted reading of the repository's
//! refs, and the rows it lists them as.

use super::*;

/// Sidebar-ready refs snapshot (sorted).
///
/// Keyed by name as well as listed: the lookups ([`Self::local_named`] and
/// the two beside it) answer a menu as it opens, where a walk over every
/// row per question is what the budget rules out (CLAUDE.md 性能予算).
/// Branches are sorted by name; tags are sorted newest first and carry a
/// second order for the lookup.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefsSnapshot {
    pub locals: Vec<BranchItem>,
    pub remotes: Vec<BranchItem>,
    pub tags: Vec<TagItem>,
    /// `tags` in name order, as indices into it.
    pub tags_by_name: Vec<u32>,
    /// Tags this repository holds that a remote carries on some other
    /// commit. A run of its own rather than a field on every tag: drift is
    /// rare, and tens of thousands of tags would each pay for the field.
    pub tag_drifts: Vec<TagDrift>,
    /// What the remotes last advertised under `refs/tags/`, shared with
    /// the session that built this — a pointer, not a copy
    /// ([`Self::tag_remotes`]).
    ///
    /// Writing the carriers into every tag row as well would cost
    /// megabytes on tens of thousands of tags
    /// (ci/baseline/code-costs-windows-x64.md §メモリの形); one row is open
    /// at a time, so the reading searches one name's run.
    ///
    /// Two snapshots sharing one index compare in constant time (`Arc`
    /// over an `Eq` type answers on the pointer), so
    /// `RepoSession::share_snapshot` pays nothing for this field while the
    /// remotes have not been read again.
    pub remote_tags: std::sync::Arc<RemoteTagIndex>,
    pub head: Option<HeadState>,
    /// Names of the configured remotes, sorted — the list offered to a
    /// branch with no upstream.
    pub remote_names: Vec<String>,
    /// Their fetch URLs, in the same order.
    pub remote_urls: Vec<String>,
    /// Which of them a push goes to when no branch says otherwise, and
    /// whether this repository's own config says so. `None` where nothing
    /// is marked — then a branch's upstream decides, else the remote
    /// called `origin`.
    pub push_default: Option<crate::remote::PushDefault>,
    /// The remote `checkout.defaultRemote` names — the other key marking a
    /// remote as origin ([`crate::remote::OriginMarks`]). `None` where unset.
    pub checkout_default: Option<String>,
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

    /// The remotes carrying the tag called `short`, each once and in name
    /// order, with whether it stands apart from the right reading, which
    /// `against` decides where it carries the name — empty where no remote
    /// has the name or nothing has read the remotes yet
    /// ([`RemoteTagIndex::carriers_against`]).
    ///
    /// The row's cloud ([`TagItem::has_remote`]) says somebody has it; the
    /// row opened says who, from this (デザイン規約 §左メニューの所作).
    pub fn tag_remotes(&self, short: &str, against: &str) -> Vec<(&str, bool)> {
        self.remote_tags.carriers_against(short, against)
    }

    /// Whether the reading of tag `short` standing on `commit` stands apart
    /// from the right one ([`RemoteTagIndex::apart_at`]) — asked with this
    /// repository's own commit for the tag here, and with a graph row's
    /// commit for the reading that row carries. The copy here is weighed
    /// like any remote's and never decides.
    pub fn tag_apart_at(&self, short: &str, commit: Oid, against: &str) -> bool {
        self.remote_tags.apart_at(short, commit, against)
    }

    /// Who the right reading of tag `short` belongs to, as the sentence of
    /// a holder apart from it names them ([`RemoteTagIndex::weighed_against`]);
    /// empty where the remotes disagree and nobody decides.
    pub fn tag_weighed_against(&self, short: &str, against: &str) -> Vec<&str> {
        self.remote_tags.weighed_against(short, against)
    }

    /// The remote a tag menu's delete rows reach when nobody named one
    /// ([`RemoteTagIndex::delete_target`]).
    pub fn tag_delete_target(&self, short: &str, against: &str) -> Option<&str> {
        self.remote_tags.delete_target(short, against)
    }

    /// What a menu on tag `short` stands on, worked out as it opens
    /// (デザイン規約 §タグを作る・送る / §左メニューの所作 の削除の表).
    /// `against` is the remote this repository's tag rows act on; `aim` a
    /// remote the reader named on its own — a carrier's line of the open
    /// row, a chip drawing that one remote's reading — or empty.
    ///
    /// Named on its own, a remote takes every row that reaches over there,
    /// drifted or not: the reader is pointing at that reading itself.
    /// Unnamed, the pushes go to `against` and the deletes to
    /// [`RemoteTagIndex::delete_target`]; those stand greyed where no
    /// remote can be named, or where the one reached has the name on
    /// another commit than here (the reading is deleted from its own line).
    pub fn tag_menu(&self, short: &str, against: &str, aim: &str) -> TagMenuFacts {
        let here = self
            .tag_named(short)
            .filter(|tag| tag.here)
            .map(|tag| tag.oid);
        let carriers: Vec<&str> = self
            .remote_tags
            .carriers_against(short, against)
            .into_iter()
            .map(|(remote, _)| remote)
            .collect();
        let push_remote = if aim.is_empty() { against } else { aim };
        let reach = if aim.is_empty() {
            self.remote_tags.delete_target(short, against).unwrap_or("")
        } else {
            aim
        };
        let elsewhere = |remote: &str| {
            here.and_then(|oid| {
                self.remote_tags
                    .commit_on(short, remote)
                    .filter(|there| *there != oid)
            })
        };
        let held_back = if !aim.is_empty() {
            TagDeleteHeld::No
        } else if reach.is_empty() && !carriers.is_empty() {
            TagDeleteHeld::Unnamed
        } else if !reach.is_empty() && elsewhere(reach).is_some() {
            TagDeleteHeld::Drifted
        } else {
            TagDeleteHeld::No
        };
        TagMenuFacts {
            push_remote: push_remote.to_string(),
            lease: if push_remote.is_empty() {
                None
            } else {
                elsewhere(push_remote)
            },
            row_goes: here.is_none() && carriers.as_slice() == [reach],
            reach: reach.to_string(),
            held_back,
            carriers: carriers
                .iter()
                .map(|remote| (*remote).to_string())
                .collect(),
        }
    }

    /// The remote a menu opened on the chip of tag `short` at `commit`
    /// names on its own: that chip draws exactly one remote's reading, and
    /// not the copy here — which, standing on the same commit, is what the
    /// chip is. `None` for the copy here and for a reading several remotes
    /// share.
    pub fn tag_aim_at(&self, short: &str, commit: Oid) -> Option<&str> {
        if self
            .tag_named(short)
            .is_some_and(|tag| tag.here && tag.oid == commit)
        {
            return None;
        }
        match self.remote_tags.carriers_at(short, commit).as_slice() {
            [only] => Some(*only),
            _ => None,
        }
    }

    fn branch_named<'a>(sorted: &'a [BranchItem], short: &str) -> Option<&'a BranchItem> {
        sorted
            .binary_search_by(|branch| branch.short.as_str().cmp(short))
            .ok()
            .and_then(|at| sorted.get(at))
    }

    /// Builds the name-ordered index over `tags`; whoever last sorted the
    /// tags calls it.
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
/// The commit is kept as an [`Oid`] and spelled out where it is shown: a
/// hex string per row costs megabytes on tens of thousands of refs
/// (ci/baseline/code-costs-windows-x64.md §メモリの形).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchItem {
    pub short: crate::Name,
    pub full: crate::Name,
    pub oid: Oid,
    pub has_remote: bool,
    pub is_head: bool,
    /// For a local branch, the remote branch it speaks for (`origin/main`),
    /// wherever the two stand. Empty when it speaks for none.
    pub upstream: crate::Name,
    /// The upstream this branch is configured for while nothing in the
    /// listing answers to the name (git's `[gone]`), by short name. Never
    /// filled together with [`Self::upstream`].
    pub upstream_gone: crate::Name,
    /// The commit the configured upstream stands on (under `remote = .` a
    /// branch here); `None` where there is none or it is not in the
    /// listing. The reference point `branch --delete` measures the tip
    /// against (git's `branch_merged`: the upstream where it resolves, HEAD
    /// otherwise), so a menu answers that check off the drawn rows
    /// (`publish::reaches`).
    pub upstream_oid: Option<Oid>,
    /// That reading stands on another commit, so the pair is two rows on
    /// the graph ([`crate::refs::RemoteBranches::folded_into_local`]).
    /// False where there is no reading.
    pub upstream_drifted: bool,
    /// Another working copy has this branch checked out, so git refuses
    /// `switch` and `branch --delete` for it — locked or not (a lock stops
    /// a different set of commands). A `bool`: it rides on every branch.
    ///
    /// Local branches only: for a remote row the menu asks the worktree
    /// list about the local branch of the same name.
    pub held_elsewhere: bool,
    /// The other side of [`Self::upstream`]: on a remote-tracking ref, the
    /// local branch configured against it. Empty on local branches and on
    /// a reading nothing names.
    ///
    /// git's config decides ([`crate::refs::RemoteBranches::spoken_for`]):
    /// a local branch merely carrying the same name is a different branch.
    /// Where two branches name one reading, the first in listing order is
    /// carried.
    pub tracked_by: crate::Name,
    /// How far this branch stands from its upstream as of the last fetch
    /// ([`crate::refs::RefEntry::ahead`]); both zero draws nothing, level
    /// or no upstream.
    ///
    /// On a remote-tracking ref, the counts of the branch in
    /// [`Self::tracked_by`], drawn on that line only (デザイン規約
    /// §左メニューの所作).
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
    /// Creator date (unix seconds), the newest-first sort key. Zero for a
    /// tag only a remote has: an advertisement carries no date.
    pub created_unix: i64,
    /// A remote carries this name too (cloud badge).
    pub has_remote: bool,
    /// Whether this repository holds the tag; false lists a name only a
    /// remote has, which no graph row shows.
    pub here: bool,
}

/// What a menu on one tag stands on ([`RefsSnapshot::tag_menu`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TagMenuFacts {
    /// Where `push` sends the name.
    pub push_remote: String,
    /// Where that remote has the name when that is not where it stands
    /// here — what the held overwrite is leased to; `None` for the plain
    /// push.
    pub lease: Option<Oid>,
    /// The remote the delete rows reach; empty where none can be named.
    pub reach: String,
    /// Why those rows stand greyed, if they do.
    pub held_back: TagDeleteHeld,
    /// Every remote carrying the name, in name order — what the greyed
    /// rows name when no one of them can be reached.
    pub carriers: Vec<String>,
    /// Deleting from `reach` leaves no holder of the name, so its row goes
    /// with it.
    pub row_goes: bool,
}

/// Why a tag menu's rows that delete over there stand greyed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TagDeleteHeld {
    /// They do not.
    #[default]
    No,
    /// The remote reached has the name on another commit than here.
    Drifted,
    /// Several remotes carry the name, none of them the reference, and
    /// nobody named one.
    Unnamed,
}

/// One remote holding one tag on a commit this repository does not have
/// it on.
///
/// A plain push of that name is refused, so the menu's push row becomes
/// the leased overwrite, pinned to [`Self::commit`] (デザイン規約
/// §相手の履歴を置き換える).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagDrift {
    pub name: crate::Name,
    pub remote: crate::Name,
    /// Where that remote has it, peeled the same way the local reading is.
    pub commit: Oid,
}
