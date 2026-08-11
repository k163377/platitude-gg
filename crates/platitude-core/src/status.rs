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

    /// True when anything is staged, modified, untracked or conflicted.
    pub fn is_dirty(&self) -> bool {
        !self.items.is_empty()
    }
}

/// How many entries fall into each bucket, counted in one pass.
///
/// The headline shows all five at once, and `-uall` (which hunk and line
/// staging need) lists every untracked file individually — so the list is
/// as long as the working tree is dirty, and walking it once per number
/// is five walks for one answer.
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
                StatusItem::Ignored { .. } => {}
            }
        }
        counts
    }
}

/// How many rows of each change kind the working-tree list holds.
///
/// **Rows, not files.** One file changed on both sides is listed twice —
/// once under the index and once under the working tree — and these count
/// what the pane lists, so the row's tally and the list agree by
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
                StatusItem::Ignored { .. } => {}
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

/// Runs `git status` and parses the snapshot.
pub async fn load(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<WorkTreeStatus, GitError> {
    // `-uall` pins untracked listing against user config and expands a new
    // directory into its files; `-unormal` would collapse it to one `dir/`
    // entry, which has no per-file diff to stage hunks or lines from.
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
    const H1: &str = "1111111111111111111111111111111111111111";
    const H2: &str = "2222222222222222222222222222222222222222";

    /// The graph row's tally counts **rows**, and a file changed on both
    /// sides is two of them — one under the index and one under the
    /// working tree, which is exactly what the pane lists. Counting files
    /// instead would put a number on the row that the list below it
    /// contradicts.
    #[test]
    fn a_file_changed_on_both_sides_is_two_rows() {
        let bytes = z(&[
            &format!("# branch.oid {SHA}"),
            &format!("1 MM N... 100644 100644 100644 {H1} {H2} both.txt"),
        ]);
        let status = parse_status(&bytes).unwrap();
        assert_eq!(status.staged().count(), 1);
        assert_eq!(status.unstaged().count(), 1);
        let kinds = Kinds::of(&status);
        assert_eq!(kinds.modified, 2);
        assert_eq!(kinds.total(), 2);
    }

    #[test]
    fn every_kind_lands_where_its_letter_says() {
        let bytes = z(&[
            &format!("# branch.oid {SHA}"),
            &format!("1 A. N... 000000 100644 100644 {H1} {H2} added.txt"),
            &format!("1 .D N... 100644 100644 000000 {H1} {H1} gone.txt"),
            // A type change is still the same path holding something else.
            &format!("1 .T N... 120000 120000 100644 {H1} {H1} was-a-link.txt"),
            &format!("2 R. N... 100644 100644 100644 {H1} {H1} R100 new-name.txt"),
            "old-name.txt",
            &format!("2 C. N... 100644 100644 100644 {H1} {H1} C75 copy.txt"),
            "source.txt",
            &format!("u UU N... 100644 100644 100644 100644 {H1} {H2} {H2} clash.txt"),
            "? untracked.txt",
        ]);
        let kinds = Kinds::of(&parse_status(&bytes).unwrap());
        assert_eq!(
            kinds,
            Kinds {
                // The staged `A`, and the untracked file — nothing of it is
                // in the index yet, so the whole file is what it adds.
                added: 2,
                modified: 1,
                deleted: 1,
                renamed: 1,
                copied: 1,
                conflicted: 1,
            }
        );
        assert_eq!(kinds.total(), 7);
    }

    /// However the two stage letters read, a conflict is one row: the pane
    /// lists it in its own bucket rather than under either side.
    #[test]
    fn a_conflict_is_one_row_whatever_its_letters_say() {
        let bytes = z(&[
            &format!("# branch.oid {SHA}"),
            &format!("u AA N... 100644 100644 100644 100644 {H1} {H2} {H2} both-added.txt"),
            &format!("u DU N... 100644 100644 100644 100644 {H1} {H2} {H2} we-deleted.txt"),
        ]);
        let kinds = Kinds::of(&parse_status(&bytes).unwrap());
        assert_eq!(kinds.conflicted, 2);
        assert_eq!(kinds.total(), 2);
        assert_eq!(kinds.added, 0);
        assert_eq!(kinds.deleted, 0);
    }

    #[test]
    fn a_clean_tree_has_nothing_to_tally() {
        let bytes = z(&[&format!("# branch.oid {SHA}"), "# branch.head main"]);
        let kinds = Kinds::of(&parse_status(&bytes).unwrap());
        assert_eq!(kinds, Kinds::default());
        assert_eq!(kinds.total(), 0);
    }

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
        assert!(s.upstream_tracked);
    }

    /// A branch can name an upstream that has no remote-tracking ref: git
    /// then leaves out `branch.ab` entirely. Zero ahead and zero behind
    /// would read as "the remote already has this", which is the opposite
    /// of the truth — nothing has ever been sent there.
    #[test]
    fn upstream_without_a_tracking_ref_reports_no_counts() {
        let bytes = z(&[
            &format!("# branch.oid {SHA}"),
            "# branch.head main",
            "# branch.upstream origin/main",
        ]);
        let s = parse_status(&bytes).unwrap();
        assert_eq!(s.upstream.as_deref(), Some("origin/main"));
        assert!(!s.upstream_tracked);
        assert_eq!((s.ahead, s.behind), (0, 0));
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
