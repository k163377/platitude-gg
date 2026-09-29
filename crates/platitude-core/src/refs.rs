//! Refs listing (`for-each-ref`) and HEAD state.
//!
//! Records are newline-separated (refnames cannot contain newlines), fields
//! NUL-separated via `%00`.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::oid::Oid;
use crate::process::{GitCommand, GitExecutor};

/// `--format=` for `for-each-ref`; keep in sync with [`parse_refs`].
pub const REFS_FORMAT_ARG: &str = "--format=%(refname)%00%(objecttype)%00%(objectname)%00%(*objectname)%00%(upstream)%00%(HEAD)%00%(creatordate:unix)%00%(upstream:track)";

const REFS_FIELDS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefKind {
    LocalBranch,
    RemoteBranch,
    Tag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefEntry {
    /// Full refname (`refs/heads/main`).
    pub name: crate::Name,
    /// Display name (`main`, `origin/main`, `v1.0`).
    pub short: crate::Name,
    pub kind: RefKind,
    /// Direct target of the ref (a tag object for annotated tags).
    pub target: Oid,
    /// Peeled commit for annotated tags.
    pub peeled: Option<Oid>,
    /// Configured upstream refname (`refs/remotes/origin/main`), if any.
    /// May point at a deleted remote branch; see
    /// [`RemoteBranches::has_counterpart`].
    pub upstream: Option<crate::Name>,
    /// True for the branch HEAD is on (never true when detached).
    pub is_head: bool,
    /// Creator date (unix seconds); tag date for annotated tags, commit
    /// date otherwise.
    pub created_unix: i64,
    /// Commits ahead of / behind the upstream as of the last fetch. Both
    /// zero when level, with no upstream, or with the remote ref gone —
    /// tell those apart by `upstream` and
    /// [`RemoteBranches::has_counterpart`].
    pub ahead: u32,
    pub behind: u32,
}

impl RefEntry {
    /// The commit this ref designates (annotated tags peeled).
    pub fn commit_oid(&self) -> Oid {
        self.peeled.unwrap_or(self.target)
    }
}

/// Non-fatal: a malformed entry is skipped with a warning, the rest kept.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("malformed for-each-ref line: {0}")]
pub struct RefsParseError(pub String);

/// Parses `for-each-ref` output produced with [`REFS_FORMAT_ARG`].
///
/// Skips `refs/remotes/<remote>/HEAD` symrefs (UI noise) and refs outside
/// the three queried namespaces.
pub fn parse_refs(bytes: &[u8]) -> Vec<RefEntry> {
    let mut out = Vec::new();
    for line in bytes.split(|b| *b == b'\n') {
        if line.is_empty() {
            continue;
        }
        match parse_line(line) {
            Ok(Some(entry)) => out.push(entry),
            Ok(None) => {}
            Err(e) => tracing::warn!(error = %e, "skipping unparsable ref"),
        }
    }
    out
}

fn parse_line(line: &[u8]) -> Result<Option<RefEntry>, RefsParseError> {
    let lossy = || String::from_utf8_lossy(line).into_owned();
    let fields: Vec<&[u8]> = line.split(|b| *b == 0).collect();
    if fields.len() != REFS_FIELDS {
        return Err(RefsParseError(lossy()));
    }
    let name: crate::Name = String::from_utf8_lossy(fields[0]).into_owned().into();

    let (kind, short) = if let Some(rest) = name.strip_prefix("refs/heads/") {
        (RefKind::LocalBranch, crate::Name::from(rest))
    } else if let Some(rest) = name.strip_prefix("refs/remotes/") {
        if rest.ends_with("/HEAD") {
            return Ok(None);
        }
        (RefKind::RemoteBranch, crate::Name::from(rest))
    } else if let Some(rest) = name.strip_prefix("refs/tags/") {
        (RefKind::Tag, crate::Name::from(rest))
    } else {
        return Ok(None);
    };

    let target = Oid::from_hex(fields[2]).map_err(|_| RefsParseError(lossy()))?;
    let peeled = if fields[3].is_empty() {
        None
    } else {
        Some(Oid::from_hex(fields[3]).map_err(|_| RefsParseError(lossy()))?)
    };
    let upstream = if fields[4].is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(fields[4]).into_owned().into())
    };
    let is_head = fields[5] == b"*";
    let created_unix = std::str::from_utf8(fields[6])
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    let (ahead, behind) = parse_track(fields[7]);

    Ok(Some(RefEntry {
        name,
        short,
        kind,
        target,
        peeled,
        upstream,
        is_head,
        created_unix,
        ahead,
        behind,
    }))
}

/// Reads `%(upstream:track)` (and `%(push:track)`, the same words); git
/// omits a zero leg. Parsing the words is safe only because
/// `for-each-ref` is plumbing and never localizes them
/// (rules-refs/core.md「ahead / behind は listing に同乗する」).
pub(crate) fn parse_track(field: &[u8]) -> (u32, u32) {
    let Some(inside) = field
        .strip_prefix(b"[")
        .and_then(|rest| rest.strip_suffix(b"]"))
    else {
        return (0, 0);
    };
    let mut ahead = 0;
    let mut behind = 0;
    for leg in inside.split(|b| *b == b',') {
        let leg = std::str::from_utf8(leg).unwrap_or_default().trim();
        if let Some(count) = leg.strip_prefix("ahead ") {
            ahead = count.parse().unwrap_or(0);
        } else if let Some(count) = leg.strip_prefix("behind ") {
            behind = count.parse().unwrap_or(0);
        }
    }
    (ahead, behind)
}

/// The remote branches, by the refname an upstream names.
///
/// Built once per listing: the question is asked once per local branch,
/// and scanning the listing instead makes the join a product of the two.
pub struct RemoteBranches<'a> {
    by_refname: HashMap<&'a str, &'a RefEntry>,
}

impl<'a> RemoteBranches<'a> {
    pub fn index(refs: &'a [RefEntry]) -> Self {
        let by_refname = refs
            .iter()
            .filter(|r| r.kind == RefKind::RemoteBranch)
            .map(|r| (r.name.as_str(), r))
            .collect();
        Self { by_refname }
    }

    /// The remote branch a local one speaks for, wherever the two stand:
    /// its configured upstream only. A same-named remote branch is a
    /// different branch, as git reads it (デザイン規約 §グラフ行のダブルクリック).
    pub fn spoken_for(&self, local: &RefEntry) -> Option<&'a RefEntry> {
        let up = local.upstream.as_deref()?;
        self.by_refname.get(up).copied()
    }

    /// Whether the configured upstream still exists; anything else, a
    /// pruned upstream included, is "local only".
    pub fn has_counterpart(&self, local: &RefEntry) -> bool {
        self.spoken_for(local).is_some()
    }

    /// The remote this branch speaks for when both stand on the same
    /// commit (デザイン規約 §グラフ行のダブルクリック). One answer for the
    /// chip's fold and the graph's cloud, so the two cannot disagree.
    pub fn folded_counterpart(&self, local: &RefEntry) -> Option<&'a RefEntry> {
        self.spoken_for(local)
            .filter(|remote| remote.commit_oid() == local.commit_oid())
    }

    /// Remote branches folded into a local branch's chip
    /// ([`Self::folded_counterpart`]).
    pub fn folded_into_local(&self, refs: &'a [RefEntry]) -> HashSet<&'a str> {
        refs.iter()
            .filter(|r| r.kind == RefKind::LocalBranch)
            .filter_map(|local| self.folded_counterpart(local))
            .map(|remote| remote.name.as_str())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadState {
    /// Current branch short name; `None` when detached or unborn.
    pub branch: Option<String>,
    /// Commit HEAD resolves to; `None` for an unborn branch (empty repo).
    pub oid: Option<Oid>,
    pub detached: bool,
}

impl HeadState {
    /// From a read's branch and commit; detached is a commit with no
    /// branch. The single constructor for the reads compared in one record
    /// (`session::standing`).
    pub fn of(branch: Option<String>, oid: Option<Oid>) -> Self {
        Self {
            detached: branch.is_none() && oid.is_some(),
            branch,
            oid,
        }
    }
}

/// Reads HEAD out of a listing, sparing [`head_state`]'s two processes.
/// `None` (detached or unborn: no ref is marked) means ask git.
pub fn head_in(refs: &[RefEntry]) -> Option<HeadState> {
    let on = refs.iter().find(|r| r.is_head)?;
    Some(HeadState {
        branch: Some(on.short.to_string()),
        oid: Some(on.commit_oid()),
        detached: false,
    })
}

pub async fn load(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Vec<RefEntry>, GitError> {
    let cmd = GitCommand::new().cwd(workdir).args([
        "for-each-ref",
        REFS_FORMAT_ARG,
        "refs/heads",
        "refs/remotes",
        "refs/tags",
    ]);
    let out = executor.run(cmd, cancel).await?;
    Ok(parse_refs(&out.stdout))
}

/// Just the commits the remote-tracking branches stand on.
///
/// A listing of its own because the graph pass asks it before the refs
/// read has landed, and the narrow listing stays cheap where the whole
/// one is not.
pub async fn remote_tips(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Vec<Oid>, GitError> {
    let cmd = GitCommand::new().cwd(workdir).args([
        "for-each-ref",
        "--format=%(objectname)",
        "refs/remotes",
    ]);
    let out = executor.run(cmd, cancel).await?;
    Ok(out
        .stdout_utf8()
        .lines()
        .filter_map(|line| Oid::from_hex_str(line.trim()).ok())
        .collect())
}

/// Just the commit HEAD stands on: [`head_state`] without the branch, in
/// one process for the walk before any refs read has landed
/// (`session::walk`) — on Windows the launch is most of a read's cost
/// (ci/baseline/code-costs-windows-x64.md §git のプロセス代).
///
/// `None` is an unborn HEAD.
pub async fn head_tip(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Option<Oid>, GitError> {
    let head = executor
        .run_unchecked(
            GitCommand::new()
                .cwd(workdir)
                // Exit 1: HEAD is unborn.
                .answers_by_code(1)
                .args(["rev-parse", "--verify", "-q", "HEAD"]),
            cancel,
        )
        .await?;
    if head.code != 0 {
        return Ok(None);
    }
    Oid::from_hex(head.stdout_utf8().trim().as_bytes())
        .map(Some)
        .map_err(|_| GitError::UnexpectedOutput {
            command: "git rev-parse --verify -q HEAD".to_string(),
            message: head.stdout_utf8().trim().to_string(),
        })
}

/// Where HEAD is: the branch it is on, if any, and its commit. The two
/// reads run concurrently; neither feeds the other.
pub async fn head_state(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<HeadState, GitError> {
    // Full name, cut here: with a same-named tag `--short` answers
    // `heads/x` while status says `x`, and the record would see two HEADs.
    let symbolic = executor.run_unchecked(
        GitCommand::new()
            .cwd(workdir)
            // Exit 1: HEAD is detached.
            .answers_by_code(1)
            .args(["symbolic-ref", "-q", "HEAD"]),
        cancel,
    );
    let (sym, oid) = tokio::join!(symbolic, head_tip(executor, workdir, cancel));
    let sym = sym?;
    let branch = (sym.code == 0).then(|| {
        let full = sym.stdout_utf8().trim().to_string();
        full.strip_prefix("refs/heads/")
            .unwrap_or(&full)
            .to_string()
    });

    Ok(HeadState::of(branch, oid?))
}

/// Splits a remote-tracking display name (`origin/main`) into the remote
/// and the branch on it, against the configured remote names. A remote's
/// own name may contain `/`, so the cut is the longest configured name
/// the ref starts with.
pub fn split_remote_ref<'a>(
    full: &'a str,
    remotes: impl IntoIterator<Item = &'a str>,
) -> Option<(&'a str, &'a str)> {
    let mut best: Option<&'a str> = None;
    for name in remotes {
        if name.is_empty() || name.len() <= best.map_or(0, str::len) {
            continue;
        }
        let cut = full.strip_prefix(name);
        if cut.is_some_and(|rest| rest.starts_with('/')) {
            best = Some(name);
        }
    }
    best.map(|name| (name, &full[name.len() + 1..]))
}

/// [`split_remote_ref`], falling back to the first slash when no
/// configured remote owns the ref (list still loading, or refs left
/// without a remote). Acting on the fallback lets git answer loudly;
/// refusing to split would make the press a silent no-op.
pub fn split_remote_ref_or_first_slash<'a>(
    full: &'a str,
    remotes: impl IntoIterator<Item = &'a str>,
) -> Option<(&'a str, &'a str)> {
    split_remote_ref(full, remotes).or_else(|| full.split_once('/'))
}

#[cfg(test)]
mod tests;
