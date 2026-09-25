//! The working copies beside the corpus (`--copies`): a fixture of the
//! slots record, which is taken while other working trees are being read
//! (ci/baseline/git-slots-windows-x64.md).

use std::path::{Path, PathBuf};

use super::{git, token};

/// Where the `nth` working copy of the corpus at `at` stands
/// (`stand_copies`).
pub(super) fn copy_path(at: &Path, nth: usize) -> PathBuf {
    let name = at
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    at.with_file_name(format!("{name}-copy-{nth}"))
}

/// `--copies n`: exactly `n` working copies stand beside the corpus when
/// this returns. Each is a full checkout and a ref that moves the corpus
/// token, so the measurement stands them at its entrance and takes them
/// down with `--copies 0` at its exit
/// (ci/baseline/git-slots-windows-x64.md §再実行).
pub(super) fn set_copies(at: &Path, count: usize) -> Result<(), String> {
    fold_copies(at, count)?;
    stand_copies(at, count)
}

/// `count` linked working trees of the corpus (`<corpus>-copy-<n>`), each
/// holding one untracked file so the read has a row to find.
///
/// Each on a branch of its own (`pgg-copy-<n>`): a detached copy is a row
/// only the walk can draw (`session::joins::WorktreeNews`), so the
/// opening's listing would ask for a rebuild that takes over the opening
/// stream. The branches move the corpus token by one per copy, and runs
/// compare only under the same token.
///
/// A copy already standing is kept and only put on its branch where it
/// is detached (a `checkout -b` at the same commit moves no file).
pub(super) fn stand_copies(at: &Path, count: usize) -> Result<(), String> {
    for nth in 1..=count {
        let copy = copy_path(at, nth);
        let copy_text = copy.to_string_lossy().replace('\\', "/");
        let branch = format!("pgg-copy-{nth}");
        if !copy.join(".git").exists() {
            println!("standing copy {nth}: {copy_text}");
            git(at, &["worktree", "add", "-b", &branch, &copy_text, "HEAD"])?;
            std::fs::write(
                copy.join(format!("carried-by-copy-{nth}.txt")),
                format!("copy {nth}\n"),
            )
            .map_err(|e| format!("could not dirty {copy_text}: {e}"))?;
            continue;
        }
        let standing = git(&copy, &["symbolic-ref", "-q", "--short", "HEAD"]).unwrap_or_default();
        if standing.trim().is_empty() {
            println!("putting copy {nth} on {branch}");
            git(&copy, &["checkout", "-b", &branch])?;
        }
    }
    Ok(())
}

/// Takes down every copy standing above `keep`, tree and branch. The
/// branch is read from the listing, since a detached copy has none;
/// `--force` because every copy carries its untracked file. The token is
/// printed before and after: it is what a later run is compared under
/// (ci/baseline/perf-windows-x64.md §計測条件).
fn fold_copies(at: &Path, keep: usize) -> Result<(), String> {
    let name = at
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let listing = git(at, &["worktree", "list", "--porcelain"])?;
    let folding: Vec<Standing> = copies_standing(&name, &listing)
        .into_iter()
        .filter(|copy| copy.nth > keep)
        .collect();
    if folding.is_empty() {
        return Ok(());
    }
    let before = token(at, &git(at, &["show-ref"])?)?;
    for copy in &folding {
        println!("folding copy {}: {}", copy.nth, copy.path);
        git(at, &["worktree", "remove", "--force", &copy.path])?;
        if let Some(branch) = &copy.branch {
            git(at, &["branch", "-D", branch])?;
        }
    }
    // A tree removed by hand leaves its `.git/worktrees` entry, and the
    // next `worktree add` of that name refuses.
    git(at, &["worktree", "prune"])?;
    println!("token {before} -> {}", token(at, &git(at, &["show-ref"])?)?);
    Ok(())
}

/// A copy standing beside the corpus, as the corpus's own worktree
/// listing has it.
#[derive(Debug, PartialEq, Eq)]
struct Standing {
    nth: usize,
    /// As the listing spells it, which `worktree remove` is sure to take.
    path: String,
    /// `None` for a detached copy.
    branch: Option<String>,
}

/// The copies of a corpus directory called `name`, read out of
/// `git worktree list --porcelain`. A `branch` line belongs to the record
/// it sits in, so one under another tree (the corpus's own, a partial
/// build) names no copy's branch.
fn copies_standing(name: &str, listing: &str) -> Vec<Standing> {
    let prefix = format!("{name}-copy-");
    let mut found: Vec<Standing> = Vec::new();
    let mut current = None;
    for line in listing.lines() {
        let line = line.trim_end();
        if let Some(path) = line.strip_prefix("worktree ") {
            current = copy_number(&prefix, path).map(|nth| {
                found.push(Standing {
                    nth,
                    path: path.to_string(),
                    branch: None,
                });
                found.len() - 1
            });
        } else if let Some(at) = current
            && let Some(branch) = line.strip_prefix("branch refs/heads/")
        {
            found[at].branch = Some(branch.to_string());
        }
    }
    found
}

/// The number a path is the `<corpus>-copy-<n>` of, if it is one.
fn copy_number(prefix: &str, path: &str) -> Option<usize> {
    path.rsplit(['/', '\\'])
        .next()?
        .strip_prefix(prefix)?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::{Standing, copies_standing};

    /// A detached copy has no branch to delete, and a `branch` line under
    /// a tree that is not a copy belongs to that tree.
    #[test]
    fn the_copies_are_read_out_of_the_worktree_listing() {
        let listing = r"worktree C:/x/.pgg-perf-corpus
HEAD 1111111111111111111111111111111111111111
branch refs/heads/main

worktree C:/x/.pgg-perf-corpus-copy-1
HEAD 1111111111111111111111111111111111111111
branch refs/heads/pgg-copy-1

worktree C:/x/.pgg-perf-corpus-copy-2
HEAD 1111111111111111111111111111111111111111
detached

worktree C:\x\.pgg-perf-corpus-copy-10
HEAD 1111111111111111111111111111111111111111
branch refs/heads/pgg-copy-10

worktree C:/x/.pgg-perf-corpus.partial-42
HEAD 1111111111111111111111111111111111111111
branch refs/heads/pgg-copy-9
";
        let standing = copies_standing(".pgg-perf-corpus", listing);
        assert_eq!(
            standing.iter().map(|copy| copy.nth).collect::<Vec<_>>(),
            vec![1, 2, 10]
        );
        assert_eq!(
            standing
                .iter()
                .map(|copy| copy.branch.clone())
                .collect::<Vec<_>>(),
            vec![
                Some("pgg-copy-1".to_string()),
                None,
                Some("pgg-copy-10".to_string())
            ]
        );
        assert_eq!(
            standing[0],
            Standing {
                nth: 1,
                path: "C:/x/.pgg-perf-corpus-copy-1".to_string(),
                branch: Some("pgg-copy-1".to_string()),
            }
        );
        assert!(copies_standing(".pgg-perf-corpus", "").is_empty());
    }
}
