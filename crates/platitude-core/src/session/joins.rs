//! The ref joins behind the sidebar snapshot and the row chips, and the
//! per-listing lookups they share.
//!
//! Split out of `build` when that file crossed its ceiling: the two
//! halves answer different questions (one moves a working tree around a
//! stash, this one reads a refs listing) and share nothing but the
//! module they sat in.

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
}

impl<'a> RefJoins<'a> {
    pub(super) fn new(refs: &'a [RefEntry]) -> Self {
        let remotes = refs::RemoteBranches::index(refs);
        let folded = remotes.folded_into_local(refs);
        let tag_commit = refs
            .iter()
            .filter(|r| r.kind == RefKind::Tag)
            .map(|r| (r.short.as_str(), r.commit_oid()))
            .collect();
        Self {
            remotes,
            folded,
            tag_commit,
        }
    }
}

/// Builds the per-commit label chips from a refs listing and what the
/// remotes carry under `refs/tags/`.
///
/// A branch and the remote it is about get one chip between them: the
/// cloud badge already says the remote is here, so the remote's own label
/// is dropped (see [`refs::remotes_folded_into_local`]).
///
/// Tags fold on the same terms, but only when both sides point at the same
/// commit. One that points elsewhere over there gets a label of its own on
/// the row it is really on, so the same name stands on two rows — the whole
/// of that signal, since a fetch never resolves the disagreement (measured:
/// `--prune` leaves the local tag silently) and it has to keep showing.
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
            RefKind::LocalBranch => joins.remotes.has_counterpart(r),
            RefKind::Tag => remote_tags.carries(&r.short),
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
                    has_remote: true,
                    is_head: false,
                    here: false,
                    remote: reading
                        .remotes
                        .iter()
                        .map(|c| c.remote.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
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
            },
        ));
    }
    // The order the chips are drawn in is settled here, once, by the sort
    // that groups them (see `LabelIndex::from_pairs`).
    LabelIndex::from_pairs(pairs)
}

/// Fingerprint of what status reported: which paths, in which state.
///
/// Deliberately not the branch headers — ahead/behind move when a fetch
/// lands and say nothing about anyone's line endings.
pub(super) fn status_key(status: &WorkTreeStatus) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for item in &status.items {
        item.hash(&mut hasher);
    }
    hasher.finish()
}

/// Fingerprints where every ref points, so two reads can be compared
/// without keeping the listing around.
///
/// Only what moves the walk counts: a renamed upstream or a changed sort
/// date redraws chips through the label diff, and rebuilding for those
/// would repaint the graph over nothing. `git for-each-ref` lists in
/// refname order, so equal layouts hash equal.
pub(super) fn refs_key(refs: &[RefEntry], head: &HeadState) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for entry in refs {
        entry.name.hash(&mut hasher);
        entry.target.hash(&mut hasher);
        entry.peeled.hash(&mut hasher);
    }
    head.branch.hash(&mut hasher);
    head.oid.hash(&mut hasher);
    head.detached.hash(&mut hasher);
    hasher.finish()
}

/// Everything the two joins read, [`refs_key`] included.
///
/// **A second key, and deliberately not the first one widened.** The two
/// answer different questions and only one of them may reach the walk:
///
/// - `refs_key` moving means commits the graph has never seen, so the
///   history is walked again.
/// - this moving means the snapshot and the chip map have to be rebuilt —
///   which what the remotes carry does on its own, because it decides the
///   cloud badges and adds the tags only they have. **The rows do not
///   change**, so the chip diff delivers it and the walk must not run: a
///   walk that ends in "the same picture" pays for the whole walk to find
///   that out.
///
/// Takes a counter the session bumps when the index became different
/// readings rather than the index's 45,909 entries, so this stays O(1) on
/// top of the key it wraps.
pub(super) fn join_key(refs: u64, remote_tags_gen: u64, remotes: &[remote::Remote]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    refs.hash(&mut hasher);
    remote_tags_gen.hash(&mut hasher);
    for r in remotes {
        r.name.hash(&mut hasher);
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
    remote_tags: &RemoteTagIndex,
    joins: &RefJoins<'_>,
) -> RefsSnapshot {
    let mut snapshot = RefsSnapshot {
        head: Some(head.clone()),
        ..Default::default()
    };
    for r in refs {
        match r.kind {
            RefKind::LocalBranch => snapshot.locals.push(BranchItem {
                short: r.short.clone(),
                full: r.name.clone(),
                oid: r.commit_oid(),
                has_remote: joins.remotes.has_counterpart(r),
                is_head: r.is_head,
                upstream: joins
                    .remotes
                    .spoken_for(r)
                    .map(|u| u.short.clone())
                    .unwrap_or_default(),
            }),
            RefKind::RemoteBranch => snapshot.remotes.push(BranchItem {
                short: r.short.clone(),
                full: r.name.clone(),
                oid: r.commit_oid(),
                has_remote: true,
                is_head: false,
                upstream: crate::Name::default(),
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
        if joins.tag_commit.contains_key(name) {
            continue;
        }
        // Remotes that disagree about a name still name one tag, and the
        // sidebar answers "does this name exist" rather than "where".
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
    // needs, and this one is kept for as long as the repository is open
    // (45,901 tags overshoot by 1.2MB on their own).
    snapshot.locals.shrink_to_fit();
    snapshot.remotes.shrink_to_fit();
    snapshot.tags.shrink_to_fit();
    snapshot
}
