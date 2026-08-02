//! Working-tree status: `git status --porcelain=v2 -z --branch`.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::oid::Oid;
use crate::process::{GitCommand, GitExecutor};

/// One entry of porcelain-v2 output. Change codes are git's raw letters
/// (`M`, `T`, `A`, `D`, `R`, `C`, `U`; `.` = unchanged).
#[derive(Debug, Clone, PartialEq, Eq)]
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
    Ignored {
        path: String,
    },
}

impl StatusItem {
    pub fn path(&self) -> &str {
        match self {
            StatusItem::Tracked { path, .. }
            | StatusItem::Unmerged { path, .. }
            | StatusItem::Untracked { path }
            | StatusItem::Ignored { path } => path,
        }
    }
}

/// Parsed status snapshot (entries + branch headers).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkTreeStatus {
    /// HEAD commit; `None` on an unborn branch.
    pub branch_oid: Option<Oid>,
    /// Current branch name; `None` when detached.
    pub branch_head: Option<String>,
    pub upstream: Option<String>,
    pub ahead: i32,
    pub behind: i32,
    pub items: Vec<StatusItem>,
}

impl WorkTreeStatus {
    /// Entries with something in the index.
    pub fn staged(&self) -> impl Iterator<Item = &StatusItem> {
        self.items
            .iter()
            .filter(|i| matches!(i, StatusItem::Tracked { staged, .. } if *staged != '.'))
    }

    /// Entries with unstaged working-tree modifications.
    pub fn unstaged(&self) -> impl Iterator<Item = &StatusItem> {
        self.items
            .iter()
            .filter(|i| matches!(i, StatusItem::Tracked { unstaged, .. } if *unstaged != '.'))
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

    /// True when anything is staged, modified, untracked or conflicted.
    pub fn is_dirty(&self) -> bool {
        !self.items.is_empty()
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
            "!" => status.items.push(StatusItem::Ignored {
                path: rest.to_string(),
            }),
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

/// Runs `git status` and parses the snapshot.
pub async fn load(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<WorkTreeStatus, GitError> {
    // `-unormal` pins untracked listing against user config.
    let cmd = GitCommand::new().cwd(workdir).args([
        "status",
        "--porcelain=v2",
        "-z",
        "--branch",
        "-unormal",
    ]);
    let out = executor.run(cmd, cancel).await?;
    parse_status(&out.stdout).map_err(|e| GitError::UnexpectedOutput {
        command: "git status --porcelain=v2 -z --branch".to_string(),
        message: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn z(tokens: &[&str]) -> Vec<u8> {
        let mut v = Vec::new();
        for t in tokens {
            v.extend_from_slice(t.as_bytes());
            v.push(0);
        }
        v
    }

    const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn parses_headers_and_ordinary_entries() {
        let bytes = z(&[
            &format!("# branch.oid {SHA}"),
            "# branch.head main",
            "# branch.upstream origin/main",
            "# branch.ab +2 -1",
            "1 M. N... 100644 100644 100644 1111111111111111111111111111111111111111 2222222222222222222222222222222222222222 staged.txt",
            "1 .M N... 100644 100644 100644 1111111111111111111111111111111111111111 1111111111111111111111111111111111111111 un staged with spaces.txt",
            "? new file.txt",
        ]);
        let s = parse_status(&bytes).unwrap();
        assert_eq!(s.branch_oid.unwrap().to_hex(), SHA);
        assert_eq!(s.branch_head.as_deref(), Some("main"));
        assert_eq!(s.upstream.as_deref(), Some("origin/main"));
        assert_eq!((s.ahead, s.behind), (2, 1));
        assert_eq!(s.items.len(), 3);
        assert_eq!(s.staged().count(), 1);
        assert_eq!(s.unstaged().count(), 1);
        assert_eq!(s.untracked().count(), 1);
        assert_eq!(
            s.unstaged().next().unwrap().path(),
            "un staged with spaces.txt"
        );
        assert!(!s.has_conflicts());
    }

    #[test]
    fn parses_rename_with_following_orig_path() {
        let bytes = z(&[
            "2 R. N... 100644 100644 100644 1111111111111111111111111111111111111111 1111111111111111111111111111111111111111 R100 new name.txt",
            "old name.txt",
            "? other.txt",
        ]);
        let s = parse_status(&bytes).unwrap();
        assert_eq!(s.items.len(), 2);
        match &s.items[0] {
            StatusItem::Tracked {
                staged,
                path,
                orig_path,
                ..
            } => {
                assert_eq!(*staged, 'R');
                assert_eq!(path, "new name.txt");
                assert_eq!(orig_path.as_deref(), Some("old name.txt"));
            }
            other => panic!("expected rename entry, got {other:?}"),
        }
    }

    #[test]
    fn parses_unmerged_and_detached() {
        let bytes = z(&[
            &format!("# branch.oid {SHA}"),
            "# branch.head (detached)",
            "u UU N... 100644 100644 100644 100644 1111111111111111111111111111111111111111 2222222222222222222222222222222222222222 3333333333333333333333333333333333333333 conflicted.txt",
        ]);
        let s = parse_status(&bytes).unwrap();
        assert_eq!(s.branch_head, None, "(detached) maps to None");
        assert!(s.has_conflicts());
        match &s.items[0] {
            StatusItem::Unmerged { ours, theirs, path } => {
                assert_eq!((*ours, *theirs), ('U', 'U'));
                assert_eq!(path, "conflicted.txt");
            }
            other => panic!("expected unmerged entry, got {other:?}"),
        }
    }

    #[test]
    fn unborn_branch_has_no_oid() {
        let bytes = z(&["# branch.oid (initial)", "# branch.head main", "? x.txt"]);
        let s = parse_status(&bytes).unwrap();
        assert_eq!(s.branch_oid, None);
        assert_eq!(s.branch_head.as_deref(), Some("main"));
    }

    #[test]
    fn empty_output_is_a_clean_tree() {
        let s = parse_status(b"").unwrap();
        assert!(s.items.is_empty());
        assert!(!s.has_conflicts());
    }

    #[test]
    fn garbage_entry_is_fatal() {
        let bytes = z(&["Z whatever"]);
        assert!(parse_status(&bytes).is_err());
    }
}
