//! The ref joins behind the sidebar snapshot and the row chips, and the
//! per-listing lookups they share.
//!
//! Beside `build`: the two halves answer different questions (one
//! moves a working tree around a stash, this one reads a refs
//! listing) and share nothing but the module they sat in
//! (structure.md §分割).

use super::*;

/// The per-listing lookups the two ref joins share, built once so neither
/// of them scans the listing from inside a loop.
///
/// Both joins run on every refs read — which is every poll tick — asking
/// "is there a remote for this branch", "is this remote already spoken
/// for" and "does this repository hold this tag". Answering those by
/// walking the whole listing again makes one read cost seconds on
/// `JetBrains/kotlin` (53,614 refs, 45,782 of them tags); through these
/// it is milliseconds.
pub(super) struct RefJoins<'a> {
    remotes: refs::RemoteBranches<'a>,
    /// Remote branches whose chip a local branch already carries.
    folded: std::collections::HashSet<&'a str>,
    /// Where each tag this repository holds points, by short name. Answers
    /// both "is this name here" and "is it on the same commit as there".
    tag_commit: HashMap<&'a str, Oid>,
    /// Where each local branch points, by refname — for the one upstream
    /// the remote index cannot answer, a branch here (`remote = .`).
    local_commit: HashMap<&'a str, Oid>,
    /// The other direction of the upstream setting: the local branch
    /// configured against each ref, by that ref's refname. **Built here
    /// with the rest**, because the question is asked of a remote row
    /// and answering it by walking the listing would be a scan per row
    /// (CLAUDE.md §性能予算 — ref 同士の突き合わせは索引を 1 本作ってから回す).
    tracked_by: HashMap<&'a str, &'a RefEntry>,
    /// The branches other working copies have checked out. A third join
    /// on the same listing, and the reason it is here: **the sidebar row
    /// and the graph chip ask the same question**, and answering it per
    /// row per copy is the shape the refs budget rules out
    /// (CLAUDE.md §性能予算). The set is a handful of names, so it is
    /// handed in.
    held: &'a WorktreeHolders,
}

/// What the working copies other than this session's are standing on, as
/// of the last worktree read. Kept behind a name because it travels from
/// that read to the ref joins and to the walk, which are three different
/// reads.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WorktreeHolders {
    /// The branches they have out, by short name, each with whether the
    /// copy holding it is locked. What marks a row and a chip as
    /// somewhere a move cannot go, and what the chip's padlock is read
    /// from. **One lookup answers both** — the two are one question
    /// about one copy, and a set beside a set would let a name be held
    /// by nobody and locked all the same.
    pub branches: std::collections::HashMap<String, bool>,
    /// The copies standing on no branch at all. **These carry their
    /// commit**, because nothing else in the repository names it: a
    /// branch is found again through the refs listing, and a detached
    /// checkout is reachable from the worktree listing alone — which is
    /// also why the walk has to be told about them (`walk_command`).
    pub detached: Vec<DetachedCheckout>,
}

/// One working copy standing on no branch: the commit it is on, the
/// name the WORKTREES row shows for it (the last segment of its path),
/// and whether `git worktree lock` is on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetachedCheckout {
    pub oid: Oid,
    pub name: crate::Name,
    pub locked: bool,
}

impl<'a> RefJoins<'a> {
    pub(super) fn new(refs: &'a [RefEntry], held: &'a WorktreeHolders) -> Self {
        let remotes = refs::RemoteBranches::index(refs);
        let folded = remotes.folded_into_local(refs);
        let tag_commit = refs
            .iter()
            .filter(|r| r.kind == RefKind::Tag)
            .map(|r| (r.short.as_str(), r.commit_oid()))
            .collect();
        let local_commit = refs
            .iter()
            .filter(|r| r.kind == RefKind::LocalBranch)
            .map(|r| (r.name.as_str(), r.commit_oid()))
            .collect();
        // First one wins: git lets two branches name one reading, and a
        // row can only carry one name. The listing arrives in refname
        // order, so the one carried is the same from read to read.
        let mut tracked_by: HashMap<&'a str, &'a RefEntry> = HashMap::new();
        for local in refs.iter().filter(|r| r.kind == RefKind::LocalBranch) {
            if let Some(up) = local.upstream.as_deref() {
                tracked_by.entry(up).or_insert(local);
            }
        }
        Self {
            remotes,
            folded,
            tag_commit,
            local_commit,
            tracked_by,
            held,
        }
    }

    /// The commit a local branch's configured upstream stands on, or
    /// `None` where it has none or the ref it names is not in the listing
    /// — git's own two halves of `branch --delete`'s reference point
    /// (`BranchItem::upstream_oid`). The remote index answers first; a
    /// branch here is the one upstream it does not hold.
    fn upstream_commit(&self, r: &RefEntry) -> Option<Oid> {
        if let Some(remote) = self.remotes.spoken_for(r) {
            return Some(remote.commit_oid());
        }
        let up = r.upstream.as_deref()?;
        self.local_commit.get(up).copied()
    }

    /// The upstream this branch is configured for while **nothing in the
    /// listing answers to that name** — git's own `[gone]`, shortened the
    /// way the row says it. Empty where the ref is there, and where the
    /// branch has no upstream at all.
    ///
    /// Both indexes are asked, because both can hold the answer: an
    /// ordinary upstream is a remote-tracking ref, and one configured
    /// `remote = .` is a branch here (the same pair
    /// [`Self::upstream_commit`] reads).
    fn upstream_gone(&self, r: &RefEntry) -> crate::Name {
        let Some(up) = r.upstream.as_deref() else {
            return crate::Name::default();
        };
        if self.remotes.spoken_for(r).is_some() || self.local_commit.contains_key(up) {
            return crate::Name::default();
        }
        let short = up
            .strip_prefix("refs/remotes/")
            .or_else(|| up.strip_prefix("refs/heads/"))
            .unwrap_or(up);
        crate::Name::from(short)
    }

    /// The local branch configured against this remote-tracking ref —
    /// the far side of [`refs::RemoteBranches::spoken_for`], and the one
    /// whose counts the reading is measured by. `None` where nothing here
    /// names it.
    fn tracked_by(&self, r: &RefEntry) -> Option<&'a RefEntry> {
        self.tracked_by.get(r.name.as_str()).copied()
    }

    /// Whether another working copy has this ref out. Only a local branch
    /// can be — a remote-tracking ref is nobody's checkout.
    fn held_elsewhere(&self, r: &RefEntry) -> bool {
        self.holder_of(r).is_some()
    }

    /// And whether that copy is locked, for the padlock the chip wears
    /// beside the name (デザイン規約 §ref の種別). False where nobody is
    /// holding it, so the two read together.
    fn held_locked(&self, r: &RefEntry) -> bool {
        self.holder_of(r) == Some(true)
    }

    /// The one lookup both of those are: `Some(locked)` while another
    /// copy has this branch out, `None` otherwise.
    fn holder_of(&self, r: &RefEntry) -> Option<bool> {
        if r.kind != RefKind::LocalBranch {
            return None;
        }
        self.held.branches.get(r.short.as_str()).copied()
    }

    /// The working copies standing on no branch, for the chips that say
    /// so. Off the same read the marks come from, so a copy cannot be a
    /// mark on one row and missing from another.
    fn detached(&self) -> &[DetachedCheckout] {
        &self.held.detached
    }
}

/// Builds the per-commit label chips from a refs listing and what the
/// remotes carry under `refs/tags/`.
///
/// A branch and the remote it is about get one chip between them: the
/// cloud badge already says the remote is here, so the remote's own label
/// is dropped (see [`refs::RemoteBranches::folded_into_local`]).
///
/// Tags fold on the same terms, but only when both sides point at the same
/// commit. One that points elsewhere over there gets a label of its own on
/// the row it is really on, so the same name stands on two rows — the whole
/// of that signal, since a fetch never resolves the disagreement (measured:
/// `--prune` leaves the local tag silently) and it has to keep showing.
///
/// **Drift takes the cloud off both rows** (デザイン規約 §ref の種別). On
/// the graph the badge is about the row it is standing on: it says the
/// remote's copy of this name is here. Where the two sides disagree the
/// remote's copy is on another row — which the two rows already say — so
/// the badge would be answering the wrong row. The sidebar keeps the wider
/// reading (does this name exist out there at all), because a list of
/// names has no second row to say it with (`build_snapshot`).
pub(super) fn build_label_map(
    refs: &[RefEntry],
    head: &HeadState,
    remote_tags: &RemoteTagIndex,
    joins: &RefJoins<'_>,
) -> LabelIndex {
    let mut pairs: Vec<(Oid, RefLabel)> = Vec::with_capacity(refs.len());
    for r in refs {
        if r.kind == RefKind::RemoteBranch && joins.folded.contains(r.name.as_str()) {
            continue;
        }
        let kind = match r.kind {
            RefKind::LocalBranch => LabelKind::LocalBranch,
            RefKind::RemoteBranch => LabelKind::RemoteBranch,
            RefKind::Tag => LabelKind::Tag,
        };
        let has_remote = match r.kind {
            RefKind::LocalBranch => joins.remotes.folded_counterpart(r).is_some(),
            RefKind::Tag => remote_tags.agrees_at(&r.short, r.commit_oid()),
            RefKind::RemoteBranch => false,
        };
        pairs.push((
            r.commit_oid(),
            RefLabel {
                text: r.short.clone(),
                kind,
                has_remote,
                is_head: r.is_head,
                here: r.kind != RefKind::RemoteBranch,
                remote: String::new(),
                held_elsewhere: joins.held_elsewhere(r),
                locked: joins.held_locked(r),
            },
        ));
    }
    for (name, readings) in remote_tags.names() {
        let local = joins.tag_commit.get(name).copied();
        for reading in readings {
            if local == Some(reading.commit) {
                continue;
            }
            pairs.push((
                reading.commit,
                RefLabel {
                    text: name.into(),
                    kind: LabelKind::Tag,
                    // The badge is the only thing that can say "remote"
                    // about a name this repository does not hold — a tag
                    // has no `origin/` namespace to say it in the name.
                    // Where the name *is* held here, this row is the far
                    // half of a drift, and both halves go bare.
                    has_remote: local.is_none(),
                    is_head: false,
                    here: false,
                    remote: reading
                        .remotes
                        .iter()
                        .map(|c| c.remote.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    // A tag is nobody's checkout, so neither half of
                    // "a working copy is standing here" is about it.
                    held_elsewhere: false,
                    locked: false,
                },
            ));
        }
    }
    if head.detached
        && let Some(oid) = head.oid
    {
        pairs.push((
            oid,
            RefLabel {
                text: crate::Name::const_new("HEAD"),
                kind: LabelKind::Head,
                has_remote: false,
                is_head: true,
                here: true,
                remote: String::new(),
                // The marker for a detached HEAD names no branch, so
                // there is none for another copy to be holding.
                held_elsewhere: false,
                // And the copy it is about is this window's own, which
                // the holders never carry: both of these are "somebody
                // else is standing here", and nobody else is.
                locked: false,
            },
        ));
    }
    // The other working copies that are on no branch. A copy with one is
    // already said by that branch's chip (`held_elsewhere`) — saying it
    // twice on one row would be two answers to where that copy is — and
    // this session's own is the marker above.
    for copy in joins.detached() {
        pairs.push((
            copy.oid,
            RefLabel {
                text: copy.name.clone(),
                kind: LabelKind::Worktree,
                has_remote: false,
                is_head: false,
                // The copy is this repository's, and the name on the chip
                // is the one its WORKTREES row shows.
                here: true,
                remote: String::new(),
                // The flag is about a branch being out somewhere else,
                // and this chip names no branch. What it draws instead is
                // its own kind — the same green frame, off the kind
                // letter rather than off this.
                held_elsewhere: false,
                // The padlock is the copy's either way, so this half is
                // the marker's own (デザイン規約 §ref の種別).
                locked: copy.locked,
            },
        ));
    }
    // The order the chips are drawn in is settled here, once, by the sort
    // that groups them (see `LabelIndex::from_pairs`).
    LabelIndex::from_pairs(pairs)
}

/// Fingerprint of what status reported: which paths, in which state.
///
/// The paths and their states alone — ahead/behind move when a fetch
/// lands and say nothing about anyone's line endings.
pub(super) fn status_key(status: &WorkTreeStatus) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for item in &status.items {
        item.hash(&mut hasher);
    }
    hasher.finish()
}

/// The two fingerprints one listing answers for, so two reads can be
/// compared without keeping the listing around.
///
/// **They are not the same question, and a listing can move one without
/// the other.** `git for-each-ref` lists in refname order, so equal
/// listings hash equal either way.
pub(super) struct RefsKeys {
    /// Where every ref points, and where HEAD is — **what the walk asks
    /// about**. Only what moves the walk counts: a renamed upstream or a
    /// changed sort date redraws chips through the label diff, and
    /// walking again for those would repaint the graph over nothing.
    pub walk: u64,
    /// That, and the rest of what each entry carries — **what the joins
    /// are built out of** (`join_key`). A branch that changed what it
    /// reads moves no ref at all: the walk stays put and the rows still
    /// have to be built again, or the badge and the counts beside a name
    /// go on saying what they said before the write (`Set upstream…`,
    /// observed).
    pub listing: u64,
}

pub(super) fn refs_keys(refs: &[RefEntry], head: &HeadState) -> RefsKeys {
    use std::hash::{Hash, Hasher};
    let mut walk = std::collections::hash_map::DefaultHasher::new();
    // The fields the joins read past the three above, in one pass beside
    // them: the badge a row wears and the name under it (`upstream`), the
    // pair at its right edge (`ahead` / `behind`), the mark on the branch
    // HEAD is on, and the date the lists are ordered by. `short` and
    // `kind` are read off `name`, so hashing the name says them too.
    let mut rest = std::collections::hash_map::DefaultHasher::new();
    for entry in refs {
        entry.name.hash(&mut walk);
        entry.target.hash(&mut walk);
        entry.peeled.hash(&mut walk);
        entry.upstream.hash(&mut rest);
        entry.is_head.hash(&mut rest);
        entry.created_unix.hash(&mut rest);
        entry.ahead.hash(&mut rest);
        entry.behind.hash(&mut rest);
    }
    head.branch.hash(&mut walk);
    head.oid.hash(&mut walk);
    head.detached.hash(&mut walk);
    let walk = walk.finish();
    let mut listing = std::collections::hash_map::DefaultHasher::new();
    walk.hash(&mut listing);
    rest.finish().hash(&mut listing);
    RefsKeys {
        walk,
        listing: listing.finish(),
    }
}

/// Everything the two joins read, [`RefsKeys::listing`] included.
///
/// **A second key.** The two answer different questions and only one of
/// them reaches the walk:
///
/// - [`RefsKeys::walk`] moving means commits the graph has never seen, so
///   the history is walked again.
/// - this moving means the snapshot and the chip map have to be rebuilt —
///   which what the remotes carry does on its own, because it decides the
///   cloud badges and adds the tags only they have. **The rows stand**,
///   so the chip diff delivers it and the walk stays put: a walk that
///   ends in "the same picture" pays for the whole walk to find that
///   out.
///
/// **So this one takes the listing key, not the walk's.** The listing is
/// what the joins are built out of, and the parts of it no ref move shows
/// up in — what a branch reads, how far it stands from it — are on the
/// rows this decides to rebuild.
///
/// Takes a counter the session bumps when the index became
/// different readings, so this stays O(1) on top of the key it
/// wraps.
pub(super) fn join_key(
    listing: u64,
    remote_tags_gen: u64,
    worktrees_gen: u64,
    remotes: &remote::Remotes,
) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    listing.hash(&mut hasher);
    remote_tags_gen.hash(&mut hasher);
    // A working copy taken or given back moves no ref, so nothing else
    // here would notice it — and the mark it decides is on rows the
    // joins build.
    worktrees_gen.hash(&mut hasher);
    // Which remote is origin is the same kind of thing: it moves no ref,
    // it rides in the snapshot, and the sidebar reads it from there. Left
    // out, moving it republishes the held snapshot and the mark stays on
    // the row it was on.
    remotes.push_default.hash(&mut hasher);
    remotes.checkout_default.hash(&mut hasher);
    for r in &remotes.list {
        r.name.hash(&mut hasher);
        // The URL rides in the snapshot too (the settings screen reads it
        // from there), and `set-url` moves no ref.
        r.fetch_url.hash(&mut hasher);
    }
    hasher.finish()
}

/// Builds the sorted sidebar snapshot.
///
/// Tags a remote has and this repository does not are listed too: no local
/// ref puts them on a graph row, so the sidebar is the only place they can
/// be read at all. Where a name exists on both sides it is listed once —
/// the sidebar is a list of names to act on, and which commits the two
/// sides point at is what the graph rows are for.
pub(super) fn build_snapshot(
    refs: &[RefEntry],
    head: &HeadState,
    remote_tags: &std::sync::Arc<RemoteTagIndex>,
    joins: &RefJoins<'_>,
) -> RefsSnapshot {
    let mut snapshot = RefsSnapshot {
        head: Some(head.clone()),
        // The same index this join reads, carried on for the rows to be
        // asked of later: which remotes have a tag is what its row opens
        // on (`RefsSnapshot::remote_tags`).
        remote_tags: std::sync::Arc::clone(remote_tags),
        ..Default::default()
    };
    for r in refs {
        match r.kind {
            RefKind::LocalBranch => {
                // The one lookup answers both halves: which reading this
                // branch speaks for, and whether it is standing where the
                // branch is — the same co-location the chips fold on.
                let spoken = joins.remotes.spoken_for(r);
                snapshot.locals.push(BranchItem {
                    short: r.short.clone(),
                    full: r.name.clone(),
                    oid: r.commit_oid(),
                    has_remote: spoken.is_some(),
                    is_head: r.is_head,
                    upstream: spoken.map(|u| u.short.clone()).unwrap_or_default(),
                    upstream_gone: joins.upstream_gone(r),
                    upstream_oid: joins.upstream_commit(r),
                    upstream_drifted: spoken.is_some_and(|u| u.commit_oid() != r.commit_oid()),
                    held_elsewhere: joins.held_elsewhere(r),
                    // The setting reads one way on this side: a local
                    // branch names its reading, and nothing names it.
                    tracked_by: crate::Name::default(),
                    ahead: r.ahead,
                    behind: r.behind,
                });
            }
            // The branch this reading is measured by, if any: the same
            // setting read from the other end, and the counts with it —
            // the row's opened lines name that branch and draw them
            // beside it (デザイン規約 §左メニューの所作).
            RefKind::RemoteBranch => snapshot.remotes.push(BranchItem {
                short: r.short.clone(),
                full: r.name.clone(),
                oid: r.commit_oid(),
                has_remote: true,
                is_head: false,
                upstream: crate::Name::default(),
                // The setting is the local branch's, so this side has
                // none to be missing either.
                upstream_gone: crate::Name::default(),
                upstream_oid: None,
                // The setting is the local branch's, so this side has
                // none to have drifted from.
                upstream_drifted: false,
                // A remote-tracking ref is nobody's checkout. Whether the
                // local branch a `switch` here would land on is held is
                // the menu's question, asked of the worktree list by the
                // name it would take (`RefRowMenu`).
                held_elsewhere: false,
                tracked_by: joins
                    .tracked_by(r)
                    .map(|local| local.short.clone())
                    .unwrap_or_default(),
                // The measurement belongs to whichever local branch names
                // this one as its upstream. The row's own line does not
                // draw it — the line that names that branch does.
                ahead: joins.tracked_by(r).map_or(0, |local| local.ahead),
                behind: joins.tracked_by(r).map_or(0, |local| local.behind),
            }),
            RefKind::Tag => snapshot.tags.push(TagItem {
                short: r.short.clone(),
                oid: r.commit_oid(),
                annotated: r.peeled.is_some(),
                created_unix: r.created_unix,
                has_remote: remote_tags.carries(&r.short),
                here: true,
            }),
        }
    }
    for (name, readings) in remote_tags.names() {
        if let Some(here) = joins.tag_commit.get(name).copied() {
            // Held on both sides. The row is already listed; what is
            // collected here is every remote that has the name somewhere
            // else, which is what turns the menu's push row into a leased
            // overwrite (`TagDrift`).
            for reading in readings.iter().filter(|r| r.commit != here) {
                snapshot
                    .tag_drifts
                    .extend(reading.remotes.iter().map(|carrier| TagDrift {
                        name: name.into(),
                        remote: carrier.remote.clone(),
                        commit: reading.commit,
                    }));
            }
            continue;
        }
        // Remotes that disagree about a name still name one tag, and the
        // sidebar answers "does this name exist".
        let Some(reading) = readings.first() else {
            continue;
        };
        snapshot.tags.push(TagItem {
            short: name.into(),
            oid: reading.commit,
            annotated: reading.annotated(),
            // An advertisement carries no date; these sort last, after
            // every tag whose creation this repository can see.
            created_unix: 0,
            has_remote: true,
            here: false,
        });
    }
    snapshot.locals.sort_by(|a, b| a.short.cmp(&b.short));
    snapshot.remotes.sort_by(|a, b| a.short.cmp(&b.short));
    // Tags newest-first (product decision), name as the tie-breaker.
    snapshot.tags.sort_by(|a, b| {
        b.created_unix
            .cmp(&a.created_unix)
            .then(a.short.cmp(&b.short))
    });
    // Built by pushing, so each list is holding up to twice the room it
    // needs, and this one is kept for as long as the repository is open —
    // on tens of thousands of tags the overshoot alone is over a megabyte
    // (ci/baseline/code-costs-windows-x64.md §メモリの形).
    snapshot.locals.shrink_to_fit();
    snapshot.remotes.shrink_to_fit();
    snapshot.tags.shrink_to_fit();
    // The lookup's order over the tags, taken once they are in the eye's.
    snapshot.index_tags();
    // Looked up by name and remote when a menu opens over a tag, so it is
    // sorted the once here.
    snapshot
        .tag_drifts
        .sort_by(|a, b| a.name.cmp(&b.name).then(a.remote.cmp(&b.remote)));
    snapshot.tag_drifts.shrink_to_fit();
    snapshot
}

/// What feeds the join above: the branches other working copies have
/// out, filed away by the worktree read so the next refs read can join
/// against them.
impl RepoSession {
    /// Files away which branches the *other* working copies have out, and
    /// says so to the ref joins by bumping their generation.
    ///
    /// **The other copies only.** Moving onto the branch this window
    /// already has out is a no-op, and marking it would put the mark on
    /// the row every reader is standing on.
    ///
    /// Answers which reads have to be asked again for it — the caller's
    /// cue. **The re-read is asked for at once**: until the join runs
    /// again the rows offer a move git will refuse, and a session's
    /// first worktree read lands *after* its first refs read, so the
    /// window is exactly the moment somebody is looking at a repository
    /// they have just opened (measured, a run photographed a second in
    /// had no marks on it). The remote-tag index asks for the same
    /// re-read on the same terms.
    pub(super) fn note_worktree_holders(
        &self,
        worktrees: &[crate::worktrees::WorktreeEntry],
        workdir: &Path,
    ) -> WorktreeNews {
        let here = same_path_key(&workdir.to_string_lossy());
        self.note_copy_heads(worktrees, workdir);
        let mine = worktrees
            .iter()
            .filter(|w| !w.bare && same_path_key(&w.path) != here);
        let mut fresh = WorktreeHolders::default();
        for copy in mine {
            match &copy.branch {
                Some(branch) => {
                    fresh.branches.insert(branch.clone(), copy.locked);
                }
                // Nothing here names the commit but the entry itself, so
                // the oid is carried along with the name the row shows.
                // An entry git listed without a `HEAD` line is bare, and
                // bare entries never reach this loop.
                None => {
                    if let Some(oid) = copy
                        .head_hex
                        .as_deref()
                        .and_then(|hex| Oid::from_hex_str(hex.trim()).ok())
                    {
                        fresh.detached.push(DetachedCheckout {
                            oid,
                            name: shown_name(&copy.path),
                            locked: copy.locked,
                        });
                    }
                }
            }
        }
        // **In an order of their own.** What decides whether the graph
        // is walked again is whether this list came out different
        // (below), and the listing's order is the order of
        // `.git/worktrees/` — so a set that merely came back shuffled
        // would spend a whole `git log` over the history saying nothing
        // (CLAUDE.md §性能予算). It settles the walk's arguments and the
        // chips' order along with it.
        fresh
            .detached
            .sort_by(|a, b| a.oid.cmp(&b.oid).then_with(|| a.name.cmp(&b.name)));
        // A poisoned lock is taken, the way the tag index's is: what
        // is behind it is a snapshot, and dropping it would take
        // every mark off the rows for the rest of the
        // session.
        let mut held = relock(&self.worktree_holders);
        if **held == fresh {
            return WorktreeNews::default();
        }
        // **The walk is only re-asked for the copies it names itself.**
        // A branch taken or given back moves no row — the commit is in
        // the walk through the branch — so the chips are the whole of
        // what changed, and re-walking for that repaints the graph over
        // nothing (CLAUDE.md §性能予算).
        let news = WorktreeNews {
            joins: true,
            walk: held.detached != fresh.detached,
        };
        *held = Arc::new(fresh);
        self.worktree_gen.fetch_add(1, Ordering::SeqCst);
        news
    }

    /// The set as the last worktree read left it, for the join that marks
    /// the rows with it.
    pub(super) fn worktree_holders(&self) -> Arc<WorktreeHolders> {
        Arc::clone(&relock(&self.worktree_holders))
    }
}

/// What a worktree read changed, and so which reads have to run again.
///
/// Two answers, because they cost differently: the joins are a pass
/// over a listing already in hand, and the walk is a git process over
/// the whole history.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct WorktreeNews {
    /// The marks and the chips: something a working copy holds moved.
    pub(super) joins: bool,
    /// The rows themselves: a copy standing on no branch moved, and those
    /// commits are in the graph only because the walk is told to name
    /// them (`walk_command`).
    pub(super) walk: bool,
}

/// What a working copy is called: the last segment of its path, which is
/// what its WORKTREES row shows (`models::nav`). git prints these paths
/// with its own separator, so both are cut on.
pub(super) fn shown_name(path: &str) -> crate::Name {
    path.rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .unwrap_or(path)
        .into()
}

/// How two spellings of one folder are compared. git prints worktree
/// paths its own way and the session holds the platform's, which differ
/// in separator on Windows and in case on both Windows and macOS.
pub fn same_path_key(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}
