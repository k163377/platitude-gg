//! The ref joins behind the sidebar snapshot and the row chips, and the
//! per-listing lookups they share.

use super::*;

/// The per-listing lookups the two ref joins share, built once so neither
/// of them scans the listing from inside a loop (CLAUDE.md §性能予算;
/// the cost otherwise is in ci/baseline/refs-join-windows-x64.md).
pub(super) struct RefJoins<'a> {
    remotes: refs::RemoteBranches<'a>,
    /// Remote branches whose chip a local branch already carries.
    folded: std::collections::HashSet<&'a str>,
    /// Where each tag this repository holds points, by short name.
    tag_commit: HashMap<&'a str, Oid>,
    /// Where each local branch points, by refname — for the one upstream
    /// the remote index cannot answer, a branch here (`remote = .`).
    local_commit: HashMap<&'a str, Oid>,
    /// The other direction of the upstream setting: the local branch
    /// configured against each ref, by that ref's refname.
    tracked_by: HashMap<&'a str, &'a RefEntry>,
    held: &'a WorktreeHolders,
}

/// What the working copies other than this session's are standing on, as
/// of the last worktree read.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WorktreeHolders {
    /// The branches they have out, by short name, each with whether the
    /// copy holding it is locked. One map rather than two sets, so a name
    /// cannot be locked without being held.
    pub branches: std::collections::HashMap<String, bool>,
    /// The copies standing on no branch. These carry their commit, since
    /// only the worktree listing names it — which is also why the walk is
    /// told about them (`walk_command`).
    pub detached: Vec<DetachedCheckout>,
}

/// One working copy standing on no branch. `name` is what its WORKTREES
/// row shows (`shown_name`).
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
        // First one wins: two branches can name one reading and a row
        // carries one name. Refname order keeps the pick stable.
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
    /// — the reference point of `branch --delete` (`BranchItem::upstream_oid`).
    fn upstream_commit(&self, r: &RefEntry) -> Option<Oid> {
        if let Some(remote) = self.remotes.spoken_for(r) {
            return Some(remote.commit_oid());
        }
        let up = r.upstream.as_deref()?;
        self.local_commit.get(up).copied()
    }

    /// The upstream this branch is configured for while nothing in the
    /// listing answers to that name — git's `[gone]`, shortened the way
    /// the row says it; empty otherwise.
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

    /// The local branch configured against this remote-tracking ref (the
    /// far side of [`refs::RemoteBranches::spoken_for`]), whose counts the
    /// reading is measured by.
    fn tracked_by(&self, r: &RefEntry) -> Option<&'a RefEntry> {
        self.tracked_by.get(r.name.as_str()).copied()
    }

    fn held_elsewhere(&self, r: &RefEntry) -> bool {
        self.holder_of(r).is_some()
    }

    /// Whether the copy holding this branch is locked — the chip's padlock
    /// (デザイン規約 §ref の種別).
    fn held_locked(&self, r: &RefEntry) -> bool {
        self.holder_of(r) == Some(true)
    }

    /// `Some(locked)` while another copy has this branch out.
    fn holder_of(&self, r: &RefEntry) -> Option<bool> {
        if r.kind != RefKind::LocalBranch {
            return None;
        }
        self.held.branches.get(r.short.as_str()).copied()
    }

    fn detached(&self) -> &[DetachedCheckout] {
        &self.held.detached
    }
}

/// Builds the per-commit label chips from a refs listing and what the
/// remotes carry under `refs/tags/`.
///
/// A branch and its remote get one chip, the cloud badge saying the remote
/// is here ([`refs::RemoteBranches::folded_into_local`]). Tags fold only
/// when both sides point at the same commit: a drifted tag stands on both
/// rows with no cloud on either, while the sidebar keeps the wider reading
/// (`build_snapshot`; デザイン規約「雲が答えている問いは、グラフとサイドバーで違う」).
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
                    // No cloud where the name is held here: this row is
                    // then the far half of a drift.
                    has_remote: local.is_none(),
                    is_head: false,
                    here: false,
                    remote: reading
                        .remotes
                        .iter()
                        .map(|c| c.remote.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
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
                // This window's own copy, which the holders never carry.
                held_elsewhere: false,
                locked: false,
            },
        ));
    }
    // Copies on no branch only: one with a branch is said by that
    // branch's chip (`held_elsewhere`), and this session's own is the
    // marker above.
    for copy in joins.detached() {
        pairs.push((
            copy.oid,
            RefLabel {
                text: copy.name.clone(),
                kind: LabelKind::Worktree,
                has_remote: false,
                is_head: false,
                here: true,
                remote: String::new(),
                // This chip names no branch; its green frame comes from
                // its kind.
                held_elsewhere: false,
                locked: copy.locked,
            },
        ));
    }
    LabelIndex::from_pairs(pairs)
}

/// Fingerprint of which paths status reported, in which state — not
/// ahead/behind, which a fetch moves and which say nothing about line
/// endings.
pub(super) fn status_key(status: &WorkingTreeStatus) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for item in &status.items {
        item.hash(&mut hasher);
    }
    hasher.finish()
}

/// The two fingerprints of one listing
/// (rules-refs/core.md「refs のキーは 2 本」). `git for-each-ref` lists in
/// refname order, so equal listings hash equal.
pub(super) struct RefsKeys {
    /// Where every ref points and where HEAD is — whether the walk runs
    /// again. Nothing else goes in: re-walking for a chip-only change
    /// repaints the graph over nothing.
    pub walk: u64,
    /// That and the rest of each entry — whether the joins are rebuilt
    /// (`join_key`).
    pub listing: u64,
}

pub(super) fn refs_keys(refs: &[RefEntry], head: &HeadState) -> RefsKeys {
    use std::hash::{Hash, Hasher};
    let mut walk = std::collections::hash_map::DefaultHasher::new();
    // The rest the joins read. `short` and `kind` derive from `name`, so
    // hashing the name covers them.
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

/// Everything the two joins read: the [`RefsKeys::listing`] key, not the
/// walk's, and the inputs that move no ref. The remote-tag index and the
/// worktree holders come in as generation counters, so this stays O(1).
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
    // A worktree taken or given back moves no ref.
    worktrees_gen.hash(&mut hasher);
    // The remote defaults and URLs ride in the snapshot and move no ref
    // (rules-refs/core.md「remotes の名前・URL と印の全部」).
    remotes.push_default.hash(&mut hasher);
    remotes.checkout_default.hash(&mut hasher);
    for r in &remotes.list {
        r.name.hash(&mut hasher);
        r.fetch_url.hash(&mut hasher);
    }
    hasher.finish()
}

/// Builds the sorted sidebar snapshot. Tags only a remote has are listed
/// too, and a name on both sides is listed once — which commits the sides
/// point at is the graph's to say.
pub(super) fn build_snapshot(
    refs: &[RefEntry],
    head: &HeadState,
    remote_tags: &std::sync::Arc<RemoteTagIndex>,
    joins: &RefJoins<'_>,
) -> RefsSnapshot {
    let mut snapshot = RefsSnapshot {
        head: Some(head.clone()),
        // Carried on for the tag rows, which open on which remotes have
        // them (`RefsSnapshot::remote_tags`).
        remote_tags: std::sync::Arc::clone(remote_tags),
        ..Default::default()
    };
    for r in refs {
        match r.kind {
            RefKind::LocalBranch => {
                // `upstream_drifted` is the same co-location the chips
                // fold on.
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
                    // Only a remote row is tracked by a branch.
                    tracked_by: crate::Name::default(),
                    ahead: r.ahead,
                    behind: r.behind,
                });
            }
            // The branch measured against this reading, and its counts —
            // the opened row draws them beside it (デザイン規約 §左メニューの所作).
            RefKind::RemoteBranch => snapshot.remotes.push(BranchItem {
                short: r.short.clone(),
                full: r.name.clone(),
                oid: r.commit_oid(),
                has_remote: true,
                is_head: false,
                upstream: crate::Name::default(),
                upstream_gone: crate::Name::default(),
                upstream_oid: None,
                upstream_drifted: false,
                // Nobody's checkout. Whether the branch a `switch` here
                // would create is held is `RefRowMenu`'s question.
                held_elsewhere: false,
                tracked_by: joins
                    .tracked_by(r)
                    .map(|local| local.short.clone())
                    .unwrap_or_default(),
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
            // Held on both sides and already listed: collect the remotes
            // holding it elsewhere, which turn the menu's push into a
            // leased overwrite (`TagDrift`).
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
        // Remotes that disagree still name one tag: the sidebar answers
        // "does this name exist".
        let Some(reading) = readings.first() else {
            continue;
        };
        snapshot.tags.push(TagItem {
            short: name.into(),
            oid: reading.commit,
            annotated: reading.annotated(),
            // An advertisement carries no date, so these sort last.
            created_unix: 0,
            has_remote: true,
            here: false,
        });
    }
    snapshot.locals.sort_by(|a, b| a.short.cmp(&b.short));
    snapshot.remotes.sort_by(|a, b| a.short.cmp(&b.short));
    // Tags newest-first (product decision).
    snapshot.tags.sort_by(|a, b| {
        b.created_unix
            .cmp(&a.created_unix)
            .then(a.short.cmp(&b.short))
    });
    // Pushed lists hold up to twice their room, and this snapshot lives as
    // long as the repository is open (ci/baseline/code-costs-windows-x64.md
    // §メモリの形).
    snapshot.locals.shrink_to_fit();
    snapshot.remotes.shrink_to_fit();
    snapshot.tags.shrink_to_fit();
    // After the sort: the index holds positions.
    snapshot.index_tags();
    // Sorted for the menu's lookup by name and remote.
    snapshot
        .tag_drifts
        .sort_by(|a, b| a.name.cmp(&b.name).then(a.remote.cmp(&b.remote)));
    snapshot.tag_drifts.shrink_to_fit();
    snapshot
}

/// What feeds the joins.
impl RepoSession {
    /// Files away which branches the *other* working copies have out, and
    /// says so to the ref joins by bumping their generation. This window's
    /// own is left out: marking it would mark the row every reader stands on.
    ///
    /// Answers which reads have to run again; the caller asks for them at
    /// once, since a session's first worktree read lands after its first
    /// refs read (rules-refs/app-ui.md「他所が持っている」).
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
                // Only a bare entry lacks a `HEAD` line, and bare ones are
                // filtered out above.
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
        // Sorted: the listing comes in `.git/worktrees/` order, and a list
        // that merely came back reshuffled must not compare different and
        // re-walk the history (below).
        fresh
            .detached
            .sort_by(|a, b| a.oid.cmp(&b.oid).then_with(|| a.name.cmp(&b.name)));
        let mut held = relock(&self.worktree_holders);
        if **held == fresh {
            return WorktreeNews::default();
        }
        // Re-walk only for the detached copies, which the walk names
        // itself; a branch taken or given back changes chips alone.
        let news = WorktreeNews {
            joins: true,
            walk: held.detached != fresh.detached,
        };
        *held = Arc::new(fresh);
        self.worktree_gen.fetch_add(1, Ordering::SeqCst);
        news
    }

    /// The set as the last worktree read left it.
    pub(super) fn worktree_holders(&self) -> Arc<WorktreeHolders> {
        Arc::clone(&relock(&self.worktree_holders))
    }
}

/// What a worktree read changed, and so which reads have to run again —
/// two answers, because a join is a pass over a listing in hand and the
/// walk is a git process over the whole history.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct WorktreeNews {
    /// The marks and the chips: something a working copy holds moved.
    pub(super) joins: bool,
    /// A copy standing on no branch moved; those commits are in the graph
    /// only because the walk names them (`walk_command`).
    pub(super) walk: bool,
}

/// The last segment of a working copy's path — what its WORKTREES row
/// shows (`models::nav`). Cut on both separators: git prints its own.
pub(super) fn shown_name(path: &str) -> crate::Name {
    path.rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .unwrap_or(path)
        .into()
}

/// How two spellings of one folder are compared: git prints worktree
/// paths its own way and the session holds the platform's, which differ
/// in separator (Windows) and in case (Windows, macOS).
pub fn same_path_key(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}
