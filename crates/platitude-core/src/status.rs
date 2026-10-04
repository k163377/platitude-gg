//! Working-tree status: `git status --porcelain=v2 -z --branch -uall`.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::oid::Oid;
use crate::process::{GitCommand, GitExecutor};

/// One entry of porcelain-v2 output. Change codes are git's raw letters
/// (`M`, `T`, `A`, `D`, `R`, `C`, `U`; `.` = unchanged).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StatusItem {
    /// Ordinary or renamed/copied tracked entry.
    Tracked {
        staged: char,
        unstaged: char,
        path: String,
        /// Original path for renames/copies.
        orig_path: Option<String>,
    },
    /// Conflict entry (`u`); `ours`/`theirs` are the two stage letters.
    Unmerged {
        ours: char,
        theirs: char,
        path: String,
    },
    Untracked {
        path: String,
    },
}

impl StatusItem {
    pub fn path(&self) -> &str {
        match self {
            StatusItem::Tracked { path, .. }
            | StatusItem::Unmerged { path, .. }
            | StatusItem::Untracked { path } => path,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkTreeStatus {
    /// HEAD commit; `None` on an unborn branch.
    pub branch_oid: Option<Oid>,
    /// Current branch name; `None` when detached.
    pub branch_head: Option<String>,
    pub upstream: Option<String>,
    /// Whether git could compare against the upstream. False when it has no
    /// remote-tracking ref (never fetched, or gone): `ahead` / `behind` then
    /// say nothing, though the upstream is still configured.
    pub upstream_tracked: bool,
    pub ahead: i32,
    pub behind: i32,
    pub items: Vec<StatusItem>,
}

impl WorkTreeStatus {
    /// Where this read saw HEAD, for the session's one record of it
    /// (`session::standing`).
    pub fn head(&self) -> crate::refs::HeadState {
        crate::refs::HeadState::of(self.branch_head.clone(), self.branch_oid)
    }
}

impl WorkTreeStatus {
    pub fn staged(&self) -> impl Iterator<Item = &StatusItem> {
        self.items
            .iter()
            .filter(|i| matches!(i, StatusItem::Tracked { staged, .. } if *staged != '.'))
    }

    pub fn unstaged(&self) -> impl Iterator<Item = &StatusItem> {
        self.items
            .iter()
            .filter(|i| matches!(i, StatusItem::Tracked { unstaged, .. } if *unstaged != '.'))
    }

    /// Entries whose change is split across the index and the working
    /// tree (`MM` and friends).
    ///
    /// `git stash push --staged` cannot take these apart: it writes the
    /// entry, then fails to remove the staged half, leaving the entry
    /// behind. The UI refuses first on these.
    pub fn partially_staged(&self) -> impl Iterator<Item = &StatusItem> {
        self.items.iter().filter(|i| {
            matches!(i, StatusItem::Tracked { staged, unstaged, .. }
                     if *staged != '.' && *unstaged != '.')
        })
    }

    pub fn untracked(&self) -> impl Iterator<Item = &StatusItem> {
        self.items
            .iter()
            .filter(|i| matches!(i, StatusItem::Untracked { .. }))
    }

    pub fn conflicted(&self) -> impl Iterator<Item = &StatusItem> {
        self.items
            .iter()
            .filter(|i| matches!(i, StatusItem::Unmerged { .. }))
    }

    pub fn has_conflicts(&self) -> bool {
        self.conflicted().next().is_some()
    }

    pub fn is_dirty(&self) -> bool {
        !self.items.is_empty()
    }
}

/// How many entries fall into each bucket, counted in one pass: the
/// headline shows all five, and `-uall` makes the list as long as the
/// tree is dirty.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub staged: usize,
    pub unstaged: usize,
    pub untracked: usize,
    pub conflicted: usize,
    /// Split across index and working tree; also counted in both `staged`
    /// and `unstaged`.
    pub partially_staged: usize,
}

impl Counts {
    pub fn of(status: &WorkTreeStatus) -> Self {
        let mut counts = Self::default();
        for item in &status.items {
            match item {
                StatusItem::Tracked {
                    staged, unstaged, ..
                } => {
                    counts.staged += usize::from(*staged != '.');
                    counts.unstaged += usize::from(*unstaged != '.');
                    counts.partially_staged += usize::from(*staged != '.' && *unstaged != '.');
                }
                StatusItem::Unmerged { .. } => counts.conflicted += 1,
                StatusItem::Untracked { .. } => counts.untracked += 1,
            }
        }
        counts
    }

    /// How many files a `reset --hard` takes with it: each changed tracked
    /// path once, whichever sides changed, plus the unmerged ones.
    /// Untracked files survive; a staged addition does not (it is in the
    /// index, so the reset removes it).
    pub fn hard_reset_takes(&self) -> usize {
        (self.staged + self.unstaged).saturating_sub(self.partially_staged) + self.conflicted
    }
}

/// How many rows of each change kind a working-tree list holds, counted
/// the way that list is built so the tally and the list agree:
/// [`Kinds::of`] for this window's pane, which lists a path once per side
/// it changed on, and [`Kinds::folded`] for another copy's pane, which
/// cannot move that index and lists one row per path. The kinds are the
/// file rows' letters (`models::nav` builds a row per side; `ChangeIcon`
/// reads the letter).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Kinds {
    pub added: usize,
    pub modified: usize,
    pub deleted: usize,
    pub renamed: usize,
    pub copied: usize,
    pub conflicted: usize,
}

impl Kinds {
    pub fn of(status: &WorkTreeStatus) -> Self {
        let mut kinds = Self::default();
        for item in &status.items {
            match item {
                // One row whatever the stage letters say: the pane lists
                // conflicts in their own bucket.
                StatusItem::Unmerged { .. } => kinds.conflicted += 1,
                StatusItem::Untracked { .. } => kinds.added += 1,
                StatusItem::Tracked {
                    staged, unstaged, ..
                } => {
                    kinds.take(*staged);
                    kinds.take(*unstaged);
                }
            }
        }
        kinds
    }

    /// The same six counted once per path, for another copy's pane
    /// ([`Kinds`]).
    pub fn folded(status: &WorkTreeStatus) -> Self {
        let mut kinds = Self::default();
        for item in &status.items {
            match item {
                StatusItem::Unmerged { .. } => kinds.conflicted += 1,
                StatusItem::Untracked { .. } => kinds.added += 1,
                // The index's letter wins: added then edited is still an
                // addition against HEAD.
                StatusItem::Tracked {
                    staged, unstaged, ..
                } => kinds.take(match *staged == '.' {
                    true => *unstaged,
                    false => *staged,
                }),
            }
        }
        kinds
    }

    /// One side's letter. `.` has no row; `T` is an edit (the same path
    /// holding something else).
    fn take(&mut self, code: char) {
        match code {
            '.' => {}
            'A' => self.added += 1,
            'D' => self.deleted += 1,
            'R' => self.renamed += 1,
            'C' => self.copied += 1,
            _ => self.modified += 1,
        }
    }

    #[cfg(test)]
    pub fn total(&self) -> usize {
        self.added + self.modified + self.deleted + self.renamed + self.copied + self.conflicted
    }
}

/// Fatal parse error (the stream shape is fixed; a mismatch means the
/// snapshot cannot be trusted).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("malformed status --porcelain=v2 entry: {0}")]
pub struct StatusParseError(pub String);

/// Parses `git status --porcelain=v2 -z --branch` output.
pub fn parse_status(bytes: &[u8]) -> Result<WorkTreeStatus, StatusParseError> {
    let mut status = WorkTreeStatus::default();
    let mut tokens = bytes
        .split(|b| *b == 0)
        .filter(|t| !t.is_empty())
        .map(|t| String::from_utf8_lossy(t).into_owned());

    while let Some(token) = tokens.next() {
        if let Some(header) = token.strip_prefix("# ") {
            parse_header(header, &mut status);
            continue;
        }
        let mut kind_split = token.splitn(2, ' ');
        let kind = kind_split.next().unwrap_or_default();
        let rest = kind_split.next().unwrap_or_default();
        match kind {
            // `1 XY sub mH mI mW hH hI path`: 6 fields between XY and path.
            "1" => {
                let (xy, path) = split_fields(rest, 6, &token)?;
                status.items.push(StatusItem::Tracked {
                    staged: xy.0,
                    unstaged: xy.1,
                    path,
                    orig_path: None,
                });
            }
            // `2 XY sub mH mI mW hH hI Xscore path` + NUL + origPath.
            "2" => {
                let (xy, path) = split_fields(rest, 7, &token)?;
                let orig = tokens
                    .next()
                    .ok_or_else(|| StatusParseError(token.clone()))?;
                status.items.push(StatusItem::Tracked {
                    staged: xy.0,
                    unstaged: xy.1,
                    path,
                    orig_path: Some(orig),
                });
            }
            // `u XY sub m1 m2 m3 mW h1 h2 h3 path`: 8 between XY and path.
            "u" => {
                let (xy, path) = split_fields(rest, 8, &token)?;
                status.items.push(StatusItem::Unmerged {
                    ours: xy.0,
                    theirs: xy.1,
                    path,
                });
            }
            "?" => status.items.push(StatusItem::Untracked {
                path: rest.to_string(),
            }),
            // Only `--ignored` emits `!`; skipped so a caller passing it
            // still parses.
            "!" => {}
            _ => return Err(StatusParseError(token.clone())),
        }
    }
    Ok(status)
}

/// Splits `<XY> <field...>{n} <path>`: XY, then `n` skipped fields, then
/// the path (which may itself contain spaces).
fn split_fields(
    rest: &str,
    skipped: usize,
    context: &str,
) -> Result<((char, char), String), StatusParseError> {
    let mut parts = rest.splitn(skipped + 2, ' ');
    let xy = parts
        .next()
        .ok_or_else(|| StatusParseError(context.into()))?;
    let mut chars = xy.chars();
    let (Some(x), Some(y), None) = (chars.next(), chars.next(), chars.next()) else {
        return Err(StatusParseError(context.into()));
    };
    for _ in 0..skipped {
        parts
            .next()
            .ok_or_else(|| StatusParseError(context.into()))?;
    }
    let path = parts
        .next()
        .ok_or_else(|| StatusParseError(context.into()))?;
    Ok(((x, y), path.to_string()))
}

fn parse_header(header: &str, status: &mut WorkTreeStatus) {
    let Some((key, value)) = header.split_once(' ') else {
        return;
    };
    match key {
        "branch.oid" => {
            status.branch_oid = Oid::from_hex(value.trim().as_bytes()).ok();
        }
        "branch.head" => {
            let v = value.trim();
            status.branch_head = (v != "(detached)").then(|| v.to_string());
        }
        "branch.upstream" => status.upstream = Some(value.trim().to_string()),
        "branch.ab" => {
            status.upstream_tracked = true;
            for part in value.split_whitespace() {
                if let Some(n) = part.strip_prefix('+') {
                    status.ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix('-') {
                    status.behind = n.parse().unwrap_or(0);
                }
            }
        }
        _ => {}
    }
}

pub async fn load(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<WorkTreeStatus, GitError> {
    // `-uall` pins the untracked listing against user config and lists a
    // new directory's files; `-unormal` would give one `dir/` entry with no
    // per-file diff to stage from. A nested repository still stays one
    // `dir/` entry (rules-refs/core.md「`status` の答えの中で末尾が `/` の path」).
    load_listing(executor, workdir, "-uall", cancel).await
}

/// [`load`] without the untracked files, for a write that asks only after
/// tracked paths: the walk for untracked files is a big tree's larger
/// part of a status.
pub async fn load_tracked(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<WorkTreeStatus, GitError> {
    load_listing(executor, workdir, "-uno", cancel).await
}

async fn load_listing(
    executor: &GitExecutor,
    workdir: &Path,
    untracked: &str,
    cancel: &CancellationToken,
) -> Result<WorkTreeStatus, GitError> {
    let cmd = GitCommand::new().cwd(workdir).args([
        "status",
        "--porcelain=v2",
        "-z",
        "--branch",
        untracked,
    ]);
    let out = executor.run(cmd, cancel).await?;
    parse_status(&out.stdout).map_err(|e| GitError::UnexpectedOutput {
        command: "git status --porcelain=v2 -z --branch".to_string(),
        message: e.to_string(),
    })
}
