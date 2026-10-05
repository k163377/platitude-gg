//! What destructive operations took away (破棄記録仕様.md §3), read off
//! git's own reflogs and Platitude GG's own record, and how each is brought
//! back (§4).
//!
//! - **The reflogs**: every local branch's, and every working copy's HEAD —
//!   a reset, an amend, a rebase, a branch set by hand, a detached HEAD
//!   left behind, and the deletes another tool wrote into HEAD's reflog
//!   (`moves`). HEAD's reflog keeps what a branch's own lost with the
//!   branch: a branch deleted and made again starts a fresh reflog.
//! - **"Took away"** is whether the old tip is an ancestor of the new, and
//!   only a move that left commits no ref reaches is listed: both are
//!   answered off one walk of the commits the reflogs name that HEAD, the
//!   branches, the remote-tracking branches, the tags, the stash and the
//!   other copies' detached HEADs do not reach (`lost`).
//! - **The record** ([`RECORD_REF`]): what git keeps no line for — copies
//!   of uncommitted work thrown away, stashes dropped, names deleted,
//!   working copies removed, remote tips overwritten (`record` writes it,
//!   `records` reads it).
//!
//! A handful of git processes whatever the repository holds: the listing,
//! the names taken, one walk of every reflog, the walk of what they reach,
//! and the record.

mod copy;
mod entries;
mod lost;
mod moves;
mod read;
mod record;
mod records;
mod restore;

pub use copy::{Copied, CopyOf, CopyOperation, copy_work, record_copy, recorded_whole};
pub use read::read_repo;
pub use record::{
    BranchBefore, RemoteTip, StashBefore, TagBefore, branch_before, commit_of,
    record_branch_delete, record_force_push, record_moves, record_remote_delete, record_restored,
    record_stash, record_tag_delete, record_tag_force_push, record_worktree_remove, stash_before,
    tag_before,
};
pub use restore::{Restored, restore};

use crate::oid::Oid;

/// What took something away, read off the reflog's message or the
/// record's `Operation:`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscardKind {
    /// `reset` — and, from the record, the work a `reset --hard` threw away
    /// with the commits, or alone where it took none.
    Reset,
    /// `commit --amend`.
    Amend,
    /// A rebase, interactive or not, and the rebase a pull runs — one entry
    /// for the branches a rebase moved together (`--update-refs`).
    Rebase,
    /// The branch set somewhere by hand (`branch --force`,
    /// `switch --force-create` — Platitude GG's `Move here`).
    Moved,
    /// A working copy left a detached HEAD.
    LeftDetached,
    /// A branch deleted: by Platitude GG (the record, its remote side too
    /// where both went), or by another tool that wrote it into HEAD's reflog
    /// (`delete_branch: <name> <upstream> [<tip>]`).
    Deleted,
    /// Uncommitted work thrown away: whole files, hunks or lines.
    Discarded,
    /// Untracked files deleted.
    DeletedUntracked,
    /// A stash dropped.
    DroppedStash,
    /// A stash popped: its entry went from the list as it was applied.
    PoppedStash,
    /// A tag deleted, here or on a remote too.
    DeletedTag,
    /// A working copy removed.
    RemovedWorktree,
    /// A branch deleted on a remote only.
    DeletedRemoteBranch,
    /// A tag deleted on a remote only.
    DeletedRemoteTag,
    /// A remote branch's tip pushed over.
    ForcePushed,
    /// A remote's tag pushed over.
    ForcePushedTag,
}

/// The ref whose reflog holds Platitude GG's own records (§2: outside
/// `refs/heads`, one for the repository). Each line's value is the record —
/// for thrown-away work the stash-shaped copy itself, for the rest a commit
/// on what was taken carrying what to put back in its trailers (`record`).
pub const RECORD_REF: &str = "refs/pgg/discards";

/// What the record's `Worktree:` names the main working copy by (§2.1): no
/// name git gives a linked one under `$GIT_DIR/worktrees/` — those are a
/// folder's name, and a linked copy may well sit in a folder `main`.
const MAIN_COPY: &str = ".";

/// One thing an entry brings back, and as what (§4). Names are the ones the
/// restore would take now: the old one where it is free, else with
/// `-pgg-restored` (and `-1`, `-2` …) after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Restore {
    /// A branch `name` at `tip`, measured against `upstream` (remote,
    /// branch) where the record kept one.
    Branch {
        name: String,
        tip: Oid,
        upstream: Option<(String, String)>,
    },
    /// A tag `name` on `object` (an annotated tag's object comes back as
    /// the same tag).
    Tag { name: String, object: Oid },
    /// A working copy at `path`, on `branch` or detached at `head`.
    Worktree {
        path: String,
        branch: Option<String>,
        head: Oid,
    },
    /// The stash entry `commit` put back in the list as `message`.
    Stash { commit: Oid, message: String },
    /// The copy `copy` of thrown-away work put back into the working copy at
    /// `path`: as it was staged where that goes, unstaged where only that
    /// goes, else as a stash entry (§4). `git_dir` is that copy's git
    /// directory as a session opened there names it — the write order the
    /// restore waits its turn in — empty where the copy is gone.
    Changes {
        copy: Oid,
        path: String,
        git_dir: String,
    },
}

/// How a part's tip draws on the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    /// A commit, ringed dashed.
    Commit,
    /// A copy of thrown-away work, as the uncommitted row it was.
    Uncommitted,
    /// A dropped stash, as the stash it was.
    Stash,
    /// Nothing: no commit this repository has (a remote's tag never
    /// fetched, a commit gc took). No walk draws it, and its restore is
    /// git's to refuse (§4).
    Absent,
}

/// Where a stash-shaped commit draws when its own base is a commit
/// Platitude GG made for it (§2.1 `Stands-on:`): the commit `made` — what a
/// discard left, which draws no row — and the commit `on` it was made on,
/// which the stash's row draws on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Stands {
    pub made: Oid,
    pub on: Oid,
}

impl Stands {
    /// Read off a stash-shaped commit's parents and its `Stands-on:`
    /// trailer; none where it names no commit or its base is HEAD itself.
    pub fn of(parents: &[Oid], stands_on: &str) -> Option<Self> {
        let on = Oid::from_hex_str(stands_on.trim()).ok()?;
        let made = *parents.first()?;
        (made != on).then_some(Self { made, on })
    }
}

/// One part of an entry: what one restore brings back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub restore: Restore,
    /// Where it stands on the graph: a branch's old tip, the copy, the
    /// stash, the commit a name or a working copy stood on.
    pub tip: Oid,
    pub look: Look,
    /// The commits only `tip` reaches, in `rev-list` order; empty where it
    /// is still reached (a merged branch, a tag's commit). A copy and a
    /// stash are their own one.
    pub lost: Vec<Oid>,
    /// How many paths a copy or a stash holds; 0 otherwise.
    pub files: u32,
    /// Paths thrown away without a copy (§2.1 `Not-copied:`), which the
    /// restore does not bring back.
    pub not_copied: Vec<String>,
    /// The remote it was taken from — a remote's branch or tag, deleted or
    /// pushed over — which the restore stands here (§4); empty otherwise.
    pub remote: String,
    /// The record line it was read from, and its number there — what the
    /// note a restore leaves names (`record::record_restored`). `None` for
    /// a part git's own reflog gives: the branch a restore puts at the old
    /// tip reaches it, and the move is no longer listed.
    pub record: Option<(Oid, usize)>,
    /// For a copy whose base is a commit made for it: where its row draws.
    pub stands: Option<Stands>,
}

/// One operation that took something away.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discard {
    pub kind: DiscardKind,
    /// What it happened to: the branch, the tag, the working copy's folder
    /// (and a detached HEAD's), the stash's message; for thrown-away work
    /// the branch it was on, empty while detached.
    pub name: String,
    /// The remote it reached, for what went from a remote; empty otherwise.
    pub remote: String,
    /// The working copy it happened in when that is not the one this
    /// session reads from; empty otherwise (a branch is every copy's).
    pub copy: String,
    /// When it happened, epoch seconds: the line git or the record wrote.
    pub at: i64,
    /// What it brings back, one part each; never empty. More than one for
    /// a `reset --hard` that threw work away with the commits, a delete
    /// that reached a remote holding another tip, and the branches one
    /// rebase moved.
    pub parts: Vec<Part>,
}

impl Discard {
    /// The commits every part would bring back — what the graph draws for
    /// the entry.
    pub fn lost(&self) -> Vec<Oid> {
        let mut seen = std::collections::HashSet::new();
        self.parts
            .iter()
            .flat_map(|part| part.lost.iter().copied())
            .filter(|oid| seen.insert(*oid))
            .collect()
    }

    /// Whether a part can be brought back alone (§4). Not the branches one
    /// rebase moved: they went together and come back together, and a
    /// commit of theirs alone is a cherry-pick off the graph.
    pub fn restores_by_part(&self) -> bool {
        self.parts.len() > 1 && self.kind != DiscardKind::Rebase
    }
}

/// The name a restore takes: `name` while no `taken` one has it, else the
/// first free one of [`suffixed`] (§4).
pub fn free_name(name: &str, taken: &impl Fn(&str) -> bool) -> String {
    if taken(name) {
        suffixed(name, taken)
    } else {
        name.to_string()
    }
}

/// `<name>-pgg-restored`, then `-1`, `-2` … after it, the first no `taken`
/// one has — where a thing had no name of its own, the start (§4: a
/// detached HEAD's commits as `detached-pgg-restored`).
pub fn suffixed(name: &str, taken: &impl Fn(&str) -> bool) -> String {
    let base = format!("{name}-pgg-restored");
    if !taken(&base) {
        return base;
    }
    (1..)
        .map(|n| format!("{base}-{n}"))
        .find(|candidate| !taken(candidate))
        .unwrap_or(base)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_free_name_is_kept_and_a_taken_one_gets_the_suffix_then_a_count() {
        let taken = |set: &'static [&'static str]| move |name: &str| set.contains(&name);
        assert_eq!(free_name("spike", &taken(&[])), "spike");
        assert_eq!(free_name("main", &taken(&["main"])), "main-pgg-restored");
        assert_eq!(
            free_name(
                "main",
                &taken(&["main", "main-pgg-restored", "main-pgg-restored-1"])
            ),
            "main-pgg-restored-2"
        );
        assert_eq!(suffixed("detached", &taken(&[])), "detached-pgg-restored");
    }

    fn entry(kind: DiscardKind, branches: &[&str]) -> Discard {
        let tip = Oid::zero_unsized();
        let parts = branches
            .iter()
            .map(|name| Part {
                restore: Restore::Branch {
                    name: (*name).to_string(),
                    tip,
                    upstream: None,
                },
                tip,
                look: Look::Commit,
                lost: Vec::new(),
                files: 0,
                not_copied: Vec::new(),
                remote: String::new(),
                record: None,
                stands: None,
            })
            .collect();
        Discard {
            kind,
            name: branches.join(", "),
            remote: String::new(),
            copy: String::new(),
            at: 0,
            parts,
        }
    }

    #[test]
    fn the_branches_one_rebase_moved_come_back_together_and_other_parts_alone() {
        assert!(!entry(DiscardKind::Rebase, &["stack/base", "stack/top"]).restores_by_part());
        assert!(entry(DiscardKind::Deleted, &["topic", "origin-topic"]).restores_by_part());
        assert!(!entry(DiscardKind::Deleted, &["topic"]).restores_by_part());
    }
}
