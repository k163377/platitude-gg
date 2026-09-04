//! Refs listing (`for-each-ref`) and HEAD state.
//!
//! Record separator is newline (refnames cannot contain newlines), field
//! separator is NUL via `%00`.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::oid::Oid;
use crate::process::{GitCommand, GitExecutor};

/// `--format=` for `for-each-ref`; keep in sync with [`parse_refs`].
/// Fields: refname, objecttype, objectname, peeled objectname, upstream,
/// HEAD marker, creator date (unix).
pub const REFS_FORMAT_ARG: &str = "--format=%(refname)%00%(objecttype)%00%(objectname)%00%(*objectname)%00%(upstream)%00%(HEAD)%00%(creatordate:unix)";

const REFS_FIELDS: usize = 7;

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
    /// May point at a deleted remote branch; see [`branches_with_remote`].
    pub upstream: Option<crate::Name>,
    /// True for the branch HEAD is on (never true when detached).
    pub is_head: bool,
    /// Creator date (unix seconds); tag date for annotated tags, commit
    /// date otherwise.
    pub created_unix: i64,
}

impl RefEntry {
    /// The commit this ref designates (annotated tags peeled).
    pub fn commit_oid(&self) -> Oid {
        self.peeled.unwrap_or(self.target)
    }
}

/// Non-fatal parse problem: malformed entries are skipped with a warning
/// (a single broken ref must not blank the whole sidebar).
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

    Ok(Some(RefEntry {
        name,
        short,
        kind,
        target,
        peeled,
        upstream,
        is_head,
        created_unix,
    }))
}

/// The remote branches, by the refname an upstream names.
///
/// Built once per listing because the question is asked once per local
/// branch, and answering it by scanning the listing makes the two a
/// product — on a repository carrying thousands of remote branches
/// (`JetBrains/kotlin`: 7,831) that is the whole cost of the join.
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
    /// **its configured upstream, and nothing else**.
    ///
    /// A remote branch that happens to carry the same name is a different
    /// branch (デザイン規約 §ref の種別). git answers this question the
    /// same way and declines to guess when it has no answer — `status`,
    /// `branch -vv` and `pull` all read `branch.<name>.merge` and report
    /// nothing from a matching name — so inferring one here would be this
    /// application saying something about a repository that git does not
    /// (CLAUDE.md 絶対制約). Setting the upstream is a row on the branch's
    /// own menu, which is where a reader who wants the two joined says so.
    pub fn spoken_for(&self, local: &RefEntry) -> Option<&'a RefEntry> {
        let up = local.upstream.as_deref()?;
        self.by_refname.get(up).copied()
    }

    /// Whether this local branch verifiably has a remote counterpart
    /// **right now**: the configured upstream still exists. Everything
    /// else is "local only" — a branch whose upstream has been pruned
    /// away included.
    pub fn has_counterpart(&self, local: &RefEntry) -> bool {
        self.spoken_for(local).is_some()
    }

    /// The remote this branch speaks for **when it is standing on the same
    /// commit**: the one the row folds into its chip, and the one the
    /// graph's cloud is about.
    ///
    /// One answer for the two, so a chip cannot fold a remote away without
    /// saying that it did, and cannot claim a remote is on the row when
    /// the graph is drawing it on another (デザイン規約 §ref の種別).
    /// Drifted, the remote keeps a row of its own and neither row wears
    /// the badge — folding never hides a divergence.
    pub fn folded_counterpart(&self, local: &RefEntry) -> Option<&'a RefEntry> {
        self.spoken_for(local)
            .filter(|remote| remote.commit_oid() == local.commit_oid())
    }

    /// Remote branches a local branch on the **same commit** already
    /// speaks for. Listing one again in the row's chip says twice what the
    /// badge says once, so the row folds it away (デザイン規約 §グラフ行のダブルクリック).
    pub fn folded_into_local(&self, refs: &'a [RefEntry]) -> HashSet<&'a str> {
        refs.iter()
            .filter(|r| r.kind == RefKind::LocalBranch)
            .filter_map(|local| self.folded_counterpart(local))
            .map(|remote| remote.name.as_str())
            .collect()
    }
}

/// Local branches that verifiably have a remote counterpart right now
/// ([`RemoteBranches::has_counterpart`]), by full refname.
pub fn branches_with_remote(refs: &[RefEntry]) -> HashSet<crate::Name> {
    let remotes = RemoteBranches::index(refs);
    refs.iter()
        .filter(|r| r.kind == RefKind::LocalBranch)
        .filter(|r| remotes.has_counterpart(r))
        .map(|r| r.name.clone())
        .collect()
}

/// Remote branches folded into a local branch's chip
/// ([`RemoteBranches::folded_into_local`]), by full refname.
pub fn remotes_folded_into_local(refs: &[RefEntry]) -> HashSet<String> {
    RemoteBranches::index(refs)
        .folded_into_local(refs)
        .into_iter()
        .map(str::to_string)
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadState {
    /// Current branch short name; `None` when detached or unborn.
    pub branch: Option<String>,
    /// Commit HEAD resolves to; `None` for an unborn branch (empty repo).
    pub oid: Option<Oid>,
    pub detached: bool,
}

/// Reads HEAD out of a listing that already has it, sparing
/// [`head_state`]'s two processes. `None` means the listing cannot say —
/// HEAD is detached, or on a branch with no commits yet, and neither has
/// a marked ref to be found — so ask git.
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
/// A listing of its own rather than a read of [`load`]'s: this is asked by
/// the graph pass, which runs before the refs read has landed and must not
/// wait for one — and it is the narrow half, so it stays cheap where the
/// whole listing is not (`JetBrains/kotlin`: 7,823 remote branches out of
/// 54,286 refs).
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

pub async fn head_state(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<HeadState, GitError> {
    let sym = executor
        .run_unchecked(
            GitCommand::new()
                .cwd(workdir)
                // Exit 1 is the answer "HEAD is detached".
                .answers_by_code(1)
                .args(["symbolic-ref", "-q", "--short", "HEAD"]),
            cancel,
        )
        .await?;
    let branch = (sym.code == 0).then(|| sym.stdout_utf8().trim().to_string());

    let head = executor
        .run_unchecked(
            GitCommand::new()
                .cwd(workdir)
                // Exit 1 is the answer "HEAD is unborn".
                .answers_by_code(1)
                .args(["rev-parse", "--verify", "-q", "HEAD"]),
            cancel,
        )
        .await?;
    let oid = if head.code == 0 {
        Some(
            Oid::from_hex(head.stdout_utf8().trim().as_bytes()).map_err(|_| {
                GitError::UnexpectedOutput {
                    command: "git rev-parse --verify -q HEAD".to_string(),
                    message: head.stdout_utf8().trim().to_string(),
                }
            })?,
        )
    } else {
        None
    };

    Ok(HeadState {
        detached: branch.is_none() && oid.is_some(),
        branch,
        oid,
    })
}

/// Splits a remote-tracking display name (`origin/main`) into the remote
/// and the branch on it, against the configured remote names. A remote's
/// own name may contain `/`, so the cut is the longest configured name
/// the ref starts with — never the first slash.
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
/// configured remote owns the ref: the list may still be loading, or the
/// remote may be gone from configuration while its refs remain (git-svn
/// trees, hand-written `refs/remotes/*`). A gesture that acts on the
/// fallback lets git answer loudly, where refusing to split would turn
/// the press into a silent no-op.
pub fn split_remote_ref_or_first_slash<'a>(
    full: &'a str,
    remotes: impl IntoIterator<Item = &'a str>,
) -> Option<(&'a str, &'a str)> {
    split_remote_ref(full, remotes).or_else(|| full.split_once('/'))
}

#[cfg(test)]
mod tests;
