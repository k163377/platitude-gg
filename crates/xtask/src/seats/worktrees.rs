//! Worktree listings, roster paths and what one tree measures (on disk, dirty, commits ahead).
//! Nothing here moves a claim.

/// The reusable worktree seats: a topical worktree pays a cold target/
/// build and keeps the gigabytes, so sessions rotate through these six.
pub(crate) const SEATS: [&str; 6] = ["a", "b", "c", "d", "e", "f"];

pub(super) const WORKTREES: &str = "/.claude/worktrees/";

/// One roster seat as `git worktree list --porcelain` shows it: the
/// listing's own block, and the roster letter its path names.
#[derive(Debug, PartialEq)]
pub(crate) struct SeatEntry {
    pub seat: &'static str,
    pub tree: WorktreeBlock,
}

/// One tree of `git worktree list --porcelain`, as the listing gives it.
///
/// Nothing is filtered and the order is the listing's, because the first
/// entry is the primary checkout and `land` turns on that.
#[derive(Debug, PartialEq)]
pub(crate) struct WorktreeBlock {
    /// The worktree's path, slashes forward, for `git -C`.
    pub path: String,
    /// Branch name, empty when HEAD is detached.
    pub branch: String,
    pub locked: bool,
    /// The lock's reason, empty when unlocked or given none.
    pub reason: String,
}

/// Every tree the listing shows, in listing order.
pub(crate) fn worktree_blocks(listing: &str) -> Vec<WorktreeBlock> {
    let mut blocks = Vec::new();
    for block in listing.split("\n\n") {
        let mut path = None;
        let mut branch = String::new();
        let mut locked = false;
        let mut reason = String::new();
        for line in block.lines() {
            if let Some(rest) = line.strip_prefix("worktree ") {
                path = Some(rest.replace('\\', "/"));
            } else if let Some(rest) = line.strip_prefix("branch refs/heads/") {
                branch = rest.to_string();
            } else if let Some(rest) = line.strip_prefix("locked") {
                locked = true;
                reason = rest.trim().to_string();
            }
        }
        if let Some(path) = path {
            blocks.push(WorktreeBlock {
                path,
                branch,
                locked,
                reason,
            });
        }
    }
    blocks
}

/// Every roster seat the listing shows, in listing order.
pub(crate) fn seat_entries(listing: &str) -> Vec<SeatEntry> {
    seat_entries_of(worktree_blocks(listing))
}

pub(super) fn seat_entries_of(trees: Vec<WorktreeBlock>) -> Vec<SeatEntry> {
    trees
        .into_iter()
        .filter_map(|block| {
            let seat = roster_letter(&block.path)?;
            Some(SeatEntry { seat, tree: block })
        })
        .collect()
}

/// The roster letter `path` points into, if it is a seat's tree at all —
/// the letters being the directory names under the worktree directory,
/// and a path inside a seat being that seat's as much as its root is.
pub(crate) fn roster_letter(path: &str) -> Option<&'static str> {
    let root = worktree_root(path)?;
    let name = root.rsplit('/').next()?;
    SEATS.iter().find(|seat| **seat == name).copied()
}

/// Where a repository keeps its seats. A repository with no such
/// directory has no roster, whatever its branches are called.
pub(crate) fn roster_dir(primary: &str) -> String {
    format!("{}{WORKTREES}", primary.trim_end_matches('/'))
}

/// `roster_letter`, with the repository whose roster that letter belongs
/// to — for a reader that has to tell this checkout's seats from the same
/// layout somewhere else (hook/git.rs).
pub(crate) fn seat_in_repository(path: &str) -> Option<(String, &'static str)> {
    let letter = roster_letter(path)?;
    let root = worktree_root(path)?;
    let repository = root.strip_suffix(&format!("{WORKTREES}{letter}"))?;
    Some((repository.to_string(), letter))
}

/// The worktree `cwd` sits in: the path down to the directory named under
/// .claude/worktrees/, and None for the primary checkout.
pub(crate) fn worktree_root(cwd: &str) -> Option<String> {
    let cwd = cwd.replace('\\', "/");
    let at = cwd.find(WORKTREES)? + WORKTREES.len();
    if at >= cwd.len() {
        return None;
    }
    let end = cwd[at..].find('/').map_or(cwd.len(), |slash| at + slash);
    Some(cwd[..end].to_string())
}

/// The measurement rig: the one tree under the roster's directory that is
/// no seat (`perf::rig`; internal-docs/性能.md「計測は計測台(rig)で撃つ」).
/// The entry and write hooks refuse it (hook/seat.rs).
pub(crate) const RIG: &str = "rig";

/// Where the rig stands, written against the primary checkout the way
/// every seat path is.
pub(crate) fn rig_path(primary: &str) -> String {
    format!("{}{WORKTREES}{RIG}", primary.trim_end_matches('/'))
}

/// Whether `path` is the rig's tree or something inside it.
pub(crate) fn in_rig(path: &str) -> bool {
    worktree_root(path).is_some_and(|root| root.rsplit('/').next() == Some(RIG))
}

/// Whether two paths name one tree, asked of the filesystem: git writes
/// forward slashes and the long name, while a process holds the spelling
/// it was started with — an 8.3 short name, a junction, either case. An
/// unresolvable path falls back to its text, separators levelled and, on
/// Windows, case.
pub(crate) fn same_tree(left: &str, right: &str) -> bool {
    if let (Ok(left), Ok(right)) = (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
        return left == right;
    }
    let trim = |path: &str| path.replace('\\', "/").trim_end_matches('/').to_string();
    let (left, right) = (trim(left), trim(right));
    if cfg!(windows) {
        left.eq_ignore_ascii_case(&right)
    } else {
        left == right
    }
}

/// A path as the listing spells it: slashes forward, for `git -C` and
/// for comparing against what git said.
pub(crate) fn slashed(path: &std::path::Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// The primary checkout — the listing's first entry, where seats are
/// created and the tree every seat path is written against — and the
/// listing it came from.
pub(crate) fn primary_checkout(cwd: &str) -> Result<(String, Vec<WorktreeBlock>), String> {
    let listing = crate::subprocess::git_query(cwd, &["worktree", "list", "--porcelain"])
        .ok_or("git worktree list failed — is git on PATH and this a repository?")?;
    let trees = worktree_blocks(&listing);
    let primary = trees
        .first()
        .ok_or("git worktree list named no tree at all")?
        .path
        .clone();
    Ok((primary, trees))
}

/// Whether a listed tree is on disk. git still lists (as prunable) a tree
/// whose directory is gone, and such a letter is treeless for every
/// purpose here.
pub(crate) fn has_a_directory(path: &str) -> bool {
    std::path::Path::new(path).is_dir()
}

/// Uncommitted changes in `dir`, untracked files included. --no-optional-
/// locks because a plain status opportunistically rewrites the index it
/// refreshed, and the survey reports index mtimes.
pub(crate) fn dirty_lines(dir: &str) -> Option<usize> {
    crate::subprocess::git_query(dir, &["--no-optional-locks", "status", "--porcelain"])
        .map(|status| status.lines().filter(|line| !line.is_empty()).count())
}

/// How many commits `git rev-list --count` sees in `range`, run in `dir`.
pub(crate) fn commits_in(dir: &str, range: &str) -> Option<u32> {
    crate::subprocess::git_query(dir, &["rev-list", "--count", range])
        .and_then(|count| count.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::{SeatEntry, WorktreeBlock, roster_letter, seat_entries};

    #[test]
    fn knows_a_seat_path_from_the_rest() {
        assert_eq!(
            roster_letter("C:\\x\\platitude-gg\\.claude\\worktrees\\a"),
            Some("a")
        );
        assert_eq!(
            roster_letter("C:/x/platitude-gg/.claude/worktrees/b/crates"),
            Some("b")
        );
        assert_eq!(
            roster_letter("C:/x/platitude-gg/.claude/worktrees/tooltip"),
            None
        );
        // The rig sits beside the seats and is none of them.
        assert_eq!(
            roster_letter("C:/x/platitude-gg/.claude/worktrees/rig"),
            None
        );
        assert_eq!(roster_letter("C:/x/platitude-gg"), None);
        // A relative spelling reads as no seat, which is why a tool's path
        // goes through `named_tree` first.
        assert_eq!(roster_letter(".claude/worktrees/e"), None);
    }

    /// A name that merely starts with the rig's is not it.
    #[test]
    fn knows_the_rig_from_the_seats() {
        assert!(super::in_rig("C:/x/platitude-gg/.claude/worktrees/rig"));
        assert!(super::in_rig(
            "C:\\x\\platitude-gg\\.claude\\worktrees\\rig\\crates\\xtask"
        ));
        assert!(!super::in_rig(
            "C:/x/platitude-gg/.claude/worktrees/rigging"
        ));
        assert!(!super::in_rig("C:/x/platitude-gg/.claude/worktrees/a"));
        assert!(!super::in_rig("C:/x/platitude-gg"));
        assert_eq!(
            super::rig_path("C:/x/platitude-gg/"),
            "C:/x/platitude-gg/.claude/worktrees/rig"
        );
        assert!(!super::SEATS.contains(&super::RIG));
        let listing = format!(
            "worktree C:/x/platitude-gg\nHEAD 1111\nbranch refs/heads/main\n\n\
             worktree {}\nHEAD 2222\ndetached\n",
            super::rig_path("C:/x/platitude-gg")
        );
        assert!(seat_entries(&listing).is_empty());
    }

    #[test]
    fn one_tree_spelled_two_ways_is_one_tree() {
        assert!(super::same_tree("C:/x/rig/", "C:\\x\\rig"));
        assert_eq!(super::same_tree("C:/x/RIG", "C:/x/rig"), cfg!(windows));
        assert!(!super::same_tree("C:/x/rig", "C:/x/rigging"));
        assert_eq!(super::slashed(std::path::Path::new("C:\\x\\y")), "C:/x/y");
    }

    #[test]
    fn reads_seats_out_of_a_worktree_listing() {
        let listing = "worktree C:/x/platitude-gg\nHEAD 1111\nbranch refs/heads/main\n\n\
                       worktree C:/x/platitude-gg/.claude/worktrees/a\nHEAD 2222\nbranch refs/heads/worktree-a\n\n\
                       worktree C:/x/platitude-gg/.claude/worktrees/tooltip\nHEAD 3333\nbranch refs/heads/worktree-tooltip\n\n\
                       worktree C:/x/platitude-gg/.claude/worktrees/b\nHEAD 4444\nbranch refs/heads/worktree-b\nlocked claude session b (pid 1)\n";
        let seats = seat_entries(listing);
        assert_eq!(
            seats,
            vec![
                SeatEntry {
                    seat: "a",
                    tree: WorktreeBlock {
                        path: "C:/x/platitude-gg/.claude/worktrees/a".to_string(),
                        branch: "worktree-a".to_string(),
                        locked: false,
                        reason: String::new(),
                    },
                },
                SeatEntry {
                    seat: "b",
                    tree: WorktreeBlock {
                        path: "C:/x/platitude-gg/.claude/worktrees/b".to_string(),
                        branch: "worktree-b".to_string(),
                        locked: true,
                        reason: "claude session b (pid 1)".to_string(),
                    },
                },
            ]
        );
    }
}
