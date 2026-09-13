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
    /// Whether git could compare against the upstream at all. False when the
    /// branch names one that has no remote-tracking ref yet (never fetched,
    /// or the remote branch is gone): `ahead` / `behind` then say nothing
    /// rather than zero, and the branch still has somewhere to be published.
    pub upstream_tracked: bool,
    pub ahead: i32,
    pub behind: i32,
    pub items: Vec<StatusItem>,
}

impl WorkTreeStatus {
    /// Where this read saw HEAD — the branch, or detached the commit
    /// alone, or neither on a branch with no commits yet. What the read
    /// offers the session's one record of HEAD (`session::standing`).
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
    /// stash entry and then fails to remove the staged half from the
    /// working tree, leaving the entry behind with nothing else changed.
    /// Knowing they are there is what lets the UI refuse first.
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

/// How many entries fall into each bucket, counted in one pass.
///
/// The headline shows all five at once, and `-uall` lists every untracked
/// file individually — so the list is as long as the working tree is
/// dirty, and walking it once per number is five walks for one answer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub staged: usize,
    pub unstaged: usize,
    pub untracked: usize,
    pub conflicted: usize,
    /// Entries whose change is split across the index and the working
    /// tree — a subset of both `staged` and `unstaged`.
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

    /// How many files a `reset --hard` takes back with it: every tracked
    /// path the index or the working tree has changed, **counted once**
    /// however many sides it changed on, plus the unmerged ones.
    ///
    /// **Untracked files are not in it.** A hard reset writes the paths
    /// the index names, and a file git was never told about is not one of
    /// them (measured). A path *staged* as an addition is — it is in the
    /// index, so the reset removes it — which is why this counts the
    /// index side rather than only what differs from HEAD on disk.
    pub fn hard_reset_takes(&self) -> usize {
        (self.staged + self.unstaged).saturating_sub(self.partially_staged) + self.conflicted
    }
}

/// How many rows of each change kind the working-tree list holds.
///
/// **Counted the way the list that row leads to is built** — which is two
/// different ways, because there are two lists. This window's own pane
/// splits a path into the sides it changed on, so [`Kinds::of`] counts
/// rows: one file changed on both sides is listed twice, once under the
/// index and once under the working tree. Another copy's pane cannot move
/// that index and so does not split it, and [`Kinds::folded`] counts one
/// per path to match. Either way the row's tally and the list agree by
/// construction. The kinds are the same letters the file rows carry, read
/// the same way (`models::nav` builds a row per side; `ChangeIcon` reads
/// the letter).
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
                // A conflict is one row whatever the two stage letters
                // say — the pane lists it in its own bucket.
                StatusItem::Unmerged { .. } => kinds.conflicted += 1,
                // Nothing of it is in the index yet, so the whole file is
                // what it adds.
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

    /// The same six counted **once per path**, for a list that shows one
    /// row per path rather than one per side.
    ///
    /// **A tally counts what its own list shows.** The pane this feeds is
    /// another working copy's, where the split into sides is the index's
    /// and the index is not this window's to move, so its list folds the
    /// two letters into one row ([`Kinds::of`] counts the rows of the
    /// pane that does not). A row whose tally disagreed with the list it
    /// leads to would be the graph saying one thing and the pane another
    /// about the same tree.
    pub fn folded(status: &WorkTreeStatus) -> Self {
        let mut kinds = Self::default();
        for item in &status.items {
            match item {
                StatusItem::Unmerged { .. } => kinds.conflicted += 1,
                StatusItem::Untracked { .. } => kinds.added += 1,
                // The index's letter where it has one: a file added to it
                // and then edited again is an addition, because that is
                // what this copy has that its last commit has not.
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

    /// One side's letter. `.` is "this side did nothing" and has no row;
    /// `M` and `T` are both edits (a type change is still the same path
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
            // Only `--ignored` produces `!` lines and nothing here passes
            // it; skipped rather than fatal so a caller that ever does is
            // not broken by them.
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
    // `-uall` pins untracked listing against user config and expands a new
    // directory into its files; `-unormal` would collapse it to one `dir/`
    // entry, which has no per-file diff to stage hunks or lines from.
    //
    // **A repository of its own inside the working copy stays one `dir/`
    // entry** whatever is asked here: its files are that repository's, and
    // git stops at the boundary (`status_integration`). A reader that cuts
    // paths on `/` meets the trailing one there and nowhere else.
    let cmd = GitCommand::new().cwd(workdir).args([
        "status",
        "--porcelain=v2",
        "-z",
        "--branch",
        "-uall",
    ]);
    let out = executor.run(cmd, cancel).await?;
    parse_status(&out.stdout).map_err(|e| GitError::UnexpectedOutput {
        command: "git status --porcelain=v2 -z --branch".to_string(),
        message: e.to_string(),
    })
}
