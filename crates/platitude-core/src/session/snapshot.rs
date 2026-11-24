//! What the sidebar is handed: one sorted reading of the repository's
//! refs, and the rows it lists them as.

use super::*;
use crate::offers::{BranchFar, Reading, TagDeleteHeld, TagFar, TagSides, TreeStanding};
use crate::remote::{PushFacts, PushRoute, RemoteTagRef, Upstream};

/// Sidebar-ready refs snapshot (sorted).
///
/// Keyed by name as well as listed: the lookups ([`Self::local_named`] and
/// the two beside it) answer a menu as it opens, where a walk over every
/// row per question is what the budget rules out (internal-docs/性能.md §設計規則).
/// Branches are sorted by name; tags are sorted newest first and carry a
/// second order for the lookup.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefsSnapshot {
    pub locals: Vec<BranchItem>,
    pub remotes: Vec<BranchItem>,
    pub tags: Vec<TagItem>,
    /// `tags` in name order, as indices into it.
    pub tags_by_name: Vec<u32>,
    /// `locals` and the tags held here in name order with letter case set
    /// aside, as indices — what a name box searches for the names git would
    /// refuse beside them ([`Self::local_clash`]).
    pub locals_folded: Vec<u32>,
    pub tags_folded: Vec<u32>,
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
    /// The marks the remotes read kept ([`crate::remote::KeptMarks`]): which
    /// remote this repository calls origin, by both its keys — the push's
    /// says where a push goes when no branch says otherwise, `None` where
    /// unset, and then a branch's upstream decides, else the remote called
    /// `origin` — and every branch's own push mark. The one HEAD is on is
    /// among them, as the read found it, and read by nothing here: its
    /// push is the toolbar's, off the status tick's own read of its marks
    /// (`Self::head_push`), and its row answers nothing
    /// (`Self::branch_push`). The branches' marks are a run of their own,
    /// as `tag_drifts` is: marks are rare.
    pub marks: crate::remote::KeptMarks,
    /// The branches a mark sends to another remote than their upstream's,
    /// counted against the tracking ref of where they go
    /// ([`crate::remote::push_tracks`]), in branch name order — but the one
    /// HEAD is on, counted by the status tick. Only those with such a ref
    /// here; read only where some branch goes elsewhere.
    pub push_tracks: Vec<(String, crate::remote::PushTrack)>,
    /// Local branches renamed while the window stood — `git branch -m`,
    /// outside or the window's own — as `(old, new)` by short name: a rename
    /// told apart from a branch deleted and
    /// another made on its commit by the new name's reflog
    /// ([`crate::branch::renamed_from`]) — and a rename of a rename folded to
    /// the last name. What a menu standing on the old name follows
    /// ([`Self::renamed_to`]). Only names that still answer: an old name made
    /// again is a branch of its own, and a new name gone takes its entry
    /// with it (`session::renames`).
    pub renamed: Vec<(crate::Name, crate::Name)>,
}

/// What the BRANCH card of one local branch reads of its upstream
/// ([`RefsSnapshot::branch_upstream`]), all off the branch's one row.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BranchUpstreamFacts {
    /// What the card's rows read over there (`offers::ref_menu`).
    pub far: BranchFar,
    /// The remote reading the branch speaks for, by short name — what the
    /// card's remote deletes name; empty where none is listed.
    pub counterpart: String,
    /// That reading's commit, which those deletes are leased to.
    pub counterpart_oid: Option<Oid>,
    /// The upstream it is configured with, listed or gone, a branch here
    /// too ([`BranchItem::upstream_named`]) — what taking it off names.
    pub upstream: String,
    /// That upstream is a branch of this repository
    /// ([`BranchItem::upstream_here`]) — where its name stands.
    pub upstream_here: bool,
    /// The commit that upstream stands on, whichever kind it is — what
    /// `branch --delete` measures the tip against
    /// ([`BranchItem::upstream_oid`]).
    pub upstream_oid: Option<Oid>,
}

impl RefsSnapshot {
    /// The local branch called `short`, by binary search over the sorted
    /// list.
    pub fn local_named(&self, short: &str) -> Option<&BranchItem> {
        Self::branch_named(&self.locals, short)
    }

    /// The name the local branch once called `old` answers to now, renamed
    /// while the window stood ([`Self::renamed`]); `None` where it was not.
    pub fn renamed_to(&self, old: &str) -> Option<&str> {
        self.renamed
            .iter()
            .find(|(from, _)| from.as_str() == old)
            .map(|(_, to)| to.as_str())
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
        let tag = self.tag_named(short);
        let here = tag.filter(|tag| tag.here).map(|tag| tag.oid);
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
            let target = RemoteTagRef { remote, tag: short };
            here.and_then(|oid| {
                self.remote_tags
                    .commit_on(target)
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
            far: TagFar {
                sides: tag.map(TagItem::sides),
                push_remote: push_remote.to_string(),
                held_back,
            },
            lease: if push_remote.is_empty() {
                None
            } else {
                elsewhere(push_remote)
            },
            row_goes: here.is_none() && carriers.as_slice() == [reach],
            reach: reach.to_string(),
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

    /// What the toolbar's push stands on: the push of the branch HEAD is
    /// on, as the worktree holds it ([`crate::remote::HeadPush::facts`]),
    /// read against this snapshot's remotes — their names, and the commit
    /// an overwrite leases to as the remote branches list it, the one hand
    /// a branch's row leases by too ([`Self::branch_push`]). Never this
    /// snapshot's marks: both are the status's, read with the counts under
    /// them (`remote::StatusPush::push_default`). `fallback` is where a
    /// branch that tracks nothing, marked nowhere, goes; a tab that is not
    /// `open` stands closed.
    pub fn head_push(
        &self,
        head: &crate::remote::HeadPush,
        fallback: &str,
        open: bool,
    ) -> PushFacts {
        head.facts(
            self.remote_names.iter().map(String::as_str),
            fallback,
            |measured| self.remote_named(measured).map(|remote| remote.oid),
        )
        .closed_unless(open)
    }

    /// What a menu's `push` row on the local branch `short` stands on: the
    /// toolbar's reading (`remote::PushFacts::read`, the one resolver), fed
    /// this branch's row, its own mark and the counts of where that mark
    /// sends it. `fallback` is the remote a branch that tracks nothing,
    /// marked nowhere, goes to; a tab that is not `open` stands closed.
    ///
    /// For a branch the worktree is not on: the one HEAD is on — as the
    /// tree has it, or as this listing does — answers nothing here (closed,
    /// so the menu keeps no row for it): its push's one door is the toolbar
    /// (デザイン規約 §メニュー「そこにしか入口が無いもの」), off the status
    /// tick, and this snapshot keeps none of its counts. A branch
    /// measured against another branch here (`branch.<n>.remote = .`) that
    /// is listed stands as the toolbar's would (`PushStanding::StaysHere`,
    /// or a mark's destination and its counts); one whose upstream, of
    /// either kind, is gone asks where it goes, as git's `[gone]` does
    /// anywhere.
    pub fn branch_push(
        &self,
        short: &str,
        fallback: &str,
        open: bool,
        tree: &TreeStanding,
    ) -> PushFacts {
        let Some(branch) = self.local_named(short) else {
            return PushFacts::default();
        };
        if branch.is_head || tree.is_current(short) {
            return PushFacts::default();
        }
        // With whether its ref is in the listing: a remote branch's reading
        // fills `upstream`, a branch here's commit `upstream_oid`
        // (`RefJoins::upstream_commit`). An upstream fetched outside the
        // remote branches is in neither, so a row reads its branch as never
        // sent (`Publish`) while the toolbar, whose counts are git's own
        // (`status`), offers the push: the two doors part in that
        // configuration, which gets no mechanism of its own (user decision
        // — ask before changing).
        let upstream = Upstream::of_listing(
            &branch.upstream,
            branch.upstream_here,
            &branch.upstream_gone,
        );
        let counted = if branch.upstream_here {
            branch.upstream_oid.is_some()
        } else {
            !branch.upstream.is_empty()
        };
        let upstream_counts = counted.then_some(branch.counts);
        let route = PushRoute::new(
            upstream,
            self.marks.marked(short),
            self.remote_names.iter().map(String::as_str),
        );
        let track = self
            .push_tracks
            .binary_search_by(|(name, _)| name.as_str().cmp(short))
            .ok()
            .map(|at| &self.push_tracks[at].1);
        PushFacts::read(route, short, fallback, upstream_counts, track, |measured| {
            self.remote_named(measured).map(|remote| remote.oid)
        })
        .closed_unless(open)
    }

    /// What the card of local branch `short` reads of its upstream
    /// (`offers::ref_menu`): where the reading it speaks for stands,
    /// whether it has an upstream at all, and whether this repository has a
    /// remote to name one on. A name this snapshot does not list reads with
    /// no upstream; the remotes are there all the same.
    pub fn branch_far(&self, short: &str) -> BranchFar {
        let branch = self.local_named(short);
        BranchFar {
            reading: branch.map_or(Reading::None, BranchItem::reading),
            tracked: branch.is_some_and(|branch| !branch.upstream_named().is_empty()),
            has_remote: !self.remote_names.is_empty(),
            ahead: branch.map_or(0, |branch| branch.counts.ahead),
        }
    }

    /// What the BRANCH card of local branch `short` reads of its upstream:
    /// [`Self::branch_far`], and the reading, its commit and the configured
    /// upstream the card's rows name. A name this snapshot does not list
    /// names none of them.
    pub fn branch_upstream(&self, short: &str) -> BranchUpstreamFacts {
        let branch = self.local_named(short);
        let reading = branch.filter(|branch| branch.reading() != Reading::None);
        BranchUpstreamFacts {
            far: self.branch_far(short),
            counterpart: reading
                .map_or("", |branch| branch.upstream.as_str())
                .to_string(),
            counterpart_oid: reading.and_then(|branch| branch.upstream_oid),
            upstream: branch.map_or("", BranchItem::upstream_named).to_string(),
            upstream_here: branch.is_some_and(|branch| branch.upstream_here),
            upstream_oid: branch.and_then(|branch| branch.upstream_oid),
        }
    }

    fn branch_named<'a>(sorted: &'a [BranchItem], short: &str) -> Option<&'a BranchItem> {
        sorted
            .binary_search_by(|branch| branch.short.as_str().cmp(short))
            .ok()
            .and_then(|at| sorted.get(at))
    }

    /// Why git would not make a local branch called `name` beside the
    /// ones here ([`NameClash`]); `except` is the branch being renamed,
    /// which steps out of the way of its own new name.
    pub fn local_clash(&self, name: &str, except: &str) -> Option<NameClash> {
        name_clash(
            |at| {
                self.locals
                    .get(at as usize)
                    .map(|branch| branch.short.as_str())
            },
            &self.locals_folded,
            name,
            except,
        )
    }

    /// The same for a tag.
    pub fn tag_clash(&self, name: &str, except: &str) -> Option<NameClash> {
        name_clash(
            |at| self.tags.get(at as usize).map(|tag| tag.short.as_str()),
            &self.tags_folded,
            name,
            except,
        )
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
        // The tags this repository holds: a name only a remote has is no
        // ref here, and git makes a tag of it.
        self.tags_folded = folded_order(
            self.tags
                .iter()
                .enumerate()
                .filter(|(_, tag)| tag.here)
                .map(|(at, tag)| (at as u32, tag.short.as_str())),
        );
    }

    /// The same order with letter case set aside, over `locals`; whoever
    /// last sorted them calls it.
    pub fn index_locals(&mut self) {
        self.locals_folded = folded_order(
            self.locals
                .iter()
                .enumerate()
                .map(|(at, branch)| (at as u32, branch.short.as_str())),
        );
    }
}

/// Why git would refuse a new name for a branch or a tag beside the ones
/// it already has — what a name box says before the press
/// (デザイン規約 §左メニューの所作「名前の衝突」).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameClash {
    /// One of that very name.
    Taken,
    /// `a` and `a/b` cannot both be: git keeps a ref as a file, and a
    /// file cannot be a folder too. The name would be the folder of the
    /// one carried — `a` typed beside `a/b` — and the two are told apart
    /// because the box says which of them would have to be the folder.
    Holds(String),
    /// The same, the other way: the name would go inside the one carried,
    /// taken as a folder — `a/b` typed beside `a`.
    Under(String),
    /// One that differs only in letter case: the same file to a disk
    /// that ignores case, which a repository may be opened from at any
    /// time, so it clashes on every disk. There git turns the loose ref
    /// down naming a name nobody has (`a branch named 'C3' already exists`,
    /// 2.55.0.windows.3). Carries the one in the way.
    Case(String),
}

/// The characters a name is compared by with letter case set aside.
fn folded(name: &str) -> impl Iterator<Item = char> + '_ {
    name.chars().flat_map(char::to_lowercase)
}

fn folded_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    folded(a).cmp(folded(b))
}

/// Positions of `names` in their order with letter case set aside: what
/// [`name_clash`] searches, so a keystroke costs a few binary searches
/// whatever the number of refs (internal-docs/性能.md §設計規則).
/// `names` are the positions to index and the names at them. Each name is
/// folded once: a comparator folding both sides on every step would fold
/// tens of thousands of tags many times over, on every read of the refs.
/// The folded strings order as [`folded_cmp`] does (UTF-8 keeps the order
/// of the characters).
fn folded_order<'a>(names: impl Iterator<Item = (u32, &'a str)>) -> Vec<u32> {
    let mut named: Vec<(u32, &str)> = names.collect();
    named.sort_by_cached_key(|(_, name)| folded(name).collect::<String>());
    named.into_iter().map(|(at, _)| at).collect()
}

/// [`NameClash`] over one section's names, `order` being their
/// [`folded_order`]. Three questions, each a binary search: is the name
/// there in any case, is any folder above it a name, is it the folder of
/// some name. `except` steps aside from all three but one: git refuses a
/// rename that changes only the letter case where the ref is a file, so
/// that one is a clash with itself.
fn name_clash<'a>(
    name_at: impl Fn(u32) -> Option<&'a str>,
    order: &[u32],
    name: &str,
    except: &str,
) -> Option<NameClash> {
    let run_from = |key: &str| {
        order.partition_point(|at| {
            name_at(*at).is_some_and(|there| folded_cmp(there, key) == std::cmp::Ordering::Less)
        })
    };
    // Collected: the run is a name and its case variants, a handful at most.
    let same = |key: &str| -> Vec<&'a str> {
        order[run_from(key)..]
            .iter()
            .filter_map(|at| name_at(*at))
            .take_while(|there| folded_cmp(there, key) == std::cmp::Ordering::Equal)
            .collect()
    };
    let mut case = None;
    for there in same(name) {
        if there == name && there != except {
            return Some(NameClash::Taken);
        }
        if there != name {
            case = Some(NameClash::Case(there.to_string()));
        }
    }
    if case.is_some() {
        return case;
    }
    for (cut, _) in name.match_indices('/') {
        if let Some(there) = same(&name[..cut])
            .into_iter()
            .find(|there| *there != except)
        {
            return Some(NameClash::Under(there.to_string()));
        }
    }
    let folder = format!("{name}/");
    let key: Vec<char> = folded(&folder).collect();
    order[run_from(&folder)..]
        .iter()
        .filter_map(|at| name_at(*at))
        .take_while(|there| folded(there).take(key.len()).eq(key.iter().copied()))
        .find(|there| *there != except)
        .map(|there| NameClash::Holds(there.to_string()))
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
    /// The branch here this one is configured against (`remote = .`)
    /// while the listing has it, by short name. With [`Self::upstream`]
    /// and [`Self::upstream_gone`], at most one of the three is filled —
    /// which one is where the configured upstream stands.
    pub upstream_local: crate::Name,
    /// The configured upstream is a branch here (`branch.<n>.remote = .`),
    /// listed or gone ([`crate::refs::RefEntry::upstream_is_here`]). The
    /// short names above cannot say so: a branch here may be called
    /// `origin/main`.
    pub upstream_here: bool,
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
    /// Another worktree has this branch checked out, so git refuses
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
    /// ([`crate::refs::RefEntry::counts`]); both zero draws nothing, level
    /// or no upstream.
    ///
    /// On a remote-tracking ref, the counts of the branch in
    /// [`Self::tracked_by`], drawn on that line only (デザイン規約
    /// §左メニューの所作).
    pub counts: crate::AheadBehind,
}

impl BranchItem {
    /// The upstream this branch is configured with, wherever it stands —
    /// the remote reading, a branch here, or one gone ([`Self::upstream`] /
    /// [`Self::upstream_local`] / [`Self::upstream_gone`], at most one of
    /// them filled); empty where it has none. What the menu takes off.
    pub fn upstream_named(&self) -> &str {
        [&self.upstream, &self.upstream_local, &self.upstream_gone]
            .into_iter()
            .find(|named| !named.is_empty())
            .map_or("", |named| named.as_str())
    }

    /// Where the remote reading this branch speaks for stands.
    pub fn reading(&self) -> Reading {
        if self.upstream.is_empty() {
            Reading::None
        } else if self.upstream_drifted {
            Reading::Drifted
        } else {
            Reading::Together
        }
    }
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

impl TagItem {
    /// Which sides the name stands on: one row carries both, a tag having
    /// no namespace.
    pub fn sides(&self) -> TagSides {
        match (self.here, self.has_remote) {
            (false, _) => TagSides::Remote,
            (true, true) => TagSides::Both,
            (true, false) => TagSides::Here,
        }
    }
}

/// What a menu on one tag stands on ([`RefsSnapshot::tag_menu`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TagMenuFacts {
    /// What the card's rows read over there (`offers::ref_menu`): which
    /// sides the name stands on — `None` where this snapshot lists no tag
    /// of the name —, where `push` sends it, and why the delete rows stand
    /// greyed, if they do.
    pub far: TagFar,
    /// Where that remote has the name when that is not where it stands
    /// here — what the held overwrite is leased to; `None` for the plain
    /// push.
    pub lease: Option<Oid>,
    /// The remote the delete rows reach; empty where none can be named.
    pub reach: String,
    /// Every remote carrying the name, in name order — what the greyed
    /// rows name when no one of them can be reached.
    pub carriers: Vec<String>,
    /// Deleting from `reach` leaves no holder of the name, so its row goes
    /// with it.
    pub row_goes: bool,
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

#[cfg(test)]
mod tests {
    use super::{NameClash, folded_order, name_clash};

    const NAMES: [&str; 6] = [
        "main",
        "feature/topic-a",
        "C3",
        "Release/1.0",
        "x/y",
        "feat",
    ];

    fn clash(name: &str, except: &str) -> Option<NameClash> {
        let order = folded_order(
            NAMES
                .iter()
                .enumerate()
                .map(|(at, name)| (at as u32, *name)),
        );
        name_clash(|at| NAMES.get(at as usize).copied(), &order, name, except)
    }

    fn holds(with: &str) -> Option<NameClash> {
        Some(NameClash::Holds(with.to_string()))
    }

    fn under(with: &str) -> Option<NameClash> {
        Some(NameClash::Under(with.to_string()))
    }

    fn case(with: &str) -> Option<NameClash> {
        Some(NameClash::Case(with.to_string()))
    }

    #[test]
    fn a_name_already_there_is_taken_in_its_own_case_only() {
        assert_eq!(clash("main", ""), Some(NameClash::Taken));
        assert_eq!(clash("Main", ""), case("main"));
        assert_eq!(clash("c3", ""), case("C3"));
        assert_eq!(clash("new", ""), None);
    }

    #[test]
    fn a_name_cannot_be_a_folder_of_another_nor_have_one_above_it() {
        assert_eq!(clash("feature", ""), holds("feature/topic-a"));
        assert_eq!(clash("main/x", ""), under("main"));
        assert_eq!(clash("feature/topic-a/more", ""), under("feature/topic-a"));
        assert_eq!(
            clash("release/1.0/fix", ""),
            under("Release/1.0"),
            "with letter case set aside, as a disk that ignores it reads the folders"
        );
        assert_eq!(clash("FEATURE", ""), holds("feature/topic-a"));
        assert_eq!(clash("featured", ""), None, "a longer name is not a folder");
        assert_eq!(clash("feature/topic-b", ""), None, "beside a name is free");
    }

    /// git renames `a` to `a/b` and back (measured on 2.55), so the name
    /// being renamed is out of its own way — all but its letter case,
    /// which git refuses where the ref is a file.
    #[test]
    fn the_name_being_renamed_steps_aside_but_for_its_case() {
        assert_eq!(clash("main/x", "main"), None);
        assert_eq!(clash("x", "x/y"), None);
        assert_eq!(clash("main", "main"), None, "unchanged");
        assert_eq!(clash("c3", "C3"), case("C3"));
        assert_eq!(clash("main", "C3"), Some(NameClash::Taken));
    }

    /// A name only a remote has is no ref here: git makes a tag of it, so
    /// the box lets it through, whatever its case or folders.
    #[test]
    fn a_tag_only_a_remote_has_is_in_nobodys_way() {
        let tag = |short: &str, here: bool| super::TagItem {
            short: short.into(),
            oid: crate::oid::Oid::from_hex(&[b'a'; 40]).expect("an oid"),
            annotated: false,
            created_unix: 0,
            has_remote: true,
            here,
        };
        let mut snapshot = super::RefsSnapshot {
            tags: vec![tag("v1.0", true), tag("v2.0", false)],
            ..Default::default()
        };
        snapshot.index_tags();
        assert_eq!(snapshot.tag_clash("v1.0", ""), Some(NameClash::Taken));
        assert_eq!(snapshot.tag_clash("v2.0", ""), None);
        assert_eq!(snapshot.tag_clash("V2.0", ""), None);
        assert_eq!(snapshot.tag_clash("v2.0/fix", ""), None);
    }
}
