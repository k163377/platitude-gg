//! The worktrees beside the corpus (`--worktrees`): a fixture of the
//! slots record, which is taken while other worktrees are being read
//! (ci/baseline/git-slots-windows-x64.md).

use std::path::{Path, PathBuf};

use super::{git, token};

/// Where the `nth` worktree of the corpus at `at` stands
/// (`stand_worktrees`).
pub(super) fn nth_worktree_path(at: &Path, nth: usize) -> PathBuf {
    let name = at
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    at.with_file_name(format!("{name}-worktree-{nth}"))
}

/// `--worktrees n`: exactly `n` worktrees stand beside the corpus when
/// this returns. Each is a full checkout and a ref that moves the corpus
/// token, so the measurement stands them at its entrance and takes them
/// down with `--worktrees 0` at its exit
/// (ci/baseline/git-slots-windows-x64.md §再実行).
pub(super) fn set_worktrees(at: &Path, count: usize) -> Result<(), String> {
    fold_worktrees(at, count)?;
    stand_worktrees(at, count)
}

/// `count` linked worktrees of the corpus (`<corpus>-worktree-<n>`), each
/// holding one untracked file so the read has a row to find.
///
/// Each on a branch of its own (`pgg-worktree-<n>`): a detached worktree is
/// a row only the walk can draw (`session::joins::WorktreeNews`), so the
/// opening's listing would ask for a rebuild that takes over the opening
/// stream. The branches move the corpus token by one per worktree, and runs
/// compare only under the same token.
///
/// A worktree already standing is kept and only put on its branch where it
/// is detached (a `checkout -b` at the same commit moves no file).
pub(super) fn stand_worktrees(at: &Path, count: usize) -> Result<(), String> {
    for nth in 1..=count {
        let worktree = nth_worktree_path(at, nth);
        let worktree_text = worktree.to_string_lossy().replace('\\', "/");
        let branch = format!("pgg-worktree-{nth}");
        if !worktree.join(".git").exists() {
            println!("standing worktree {nth}: {worktree_text}");
            git(
                at,
                &["worktree", "add", "-b", &branch, &worktree_text, "HEAD"],
            )?;
            std::fs::write(
                worktree.join(format!("carried-by-worktree-{nth}.txt")),
                format!("worktree {nth}\n"),
            )
            .map_err(|e| format!("could not dirty {worktree_text}: {e}"))?;
            continue;
        }
        let standing =
            git(&worktree, &["symbolic-ref", "-q", "--short", "HEAD"]).unwrap_or_default();
        if standing.trim().is_empty() {
            println!("putting worktree {nth} on {branch}");
            git(&worktree, &["checkout", "-b", &branch])?;
        }
    }
    Ok(())
}

/// Takes down every worktree standing above `keep`, tree and branch. The
/// branch is read from the listing, since a detached worktree has none;
/// `--force` because every worktree carries its untracked file. The token is
/// printed before and after: it is what a later run is compared under
/// (ci/baseline/perf-windows-x64.md §計測条件).
fn fold_worktrees(at: &Path, keep: usize) -> Result<(), String> {
    let name = at
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let listing = git(at, &["worktree", "list", "--porcelain"])?;
    let folding: Vec<Standing> = worktrees_standing(&name, &listing)
        .into_iter()
        .filter(|worktree| worktree.nth > keep)
        .collect();
    if folding.is_empty() {
        return Ok(());
    }
    let before = token(at, &git(at, &["show-ref"])?)?;
    for worktree in &folding {
        println!("folding worktree {}: {}", worktree.nth, worktree.path);
        git(at, &["worktree", "remove", "--force", &worktree.path])?;
        if let Some(branch) = &worktree.branch {
            git(at, &["branch", "-D", branch])?;
        }
    }
    // A tree removed by hand leaves its `.git/worktrees` entry, and the
    // next `worktree add` of that name refuses.
    git(at, &["worktree", "prune"])?;
    println!("token {before} -> {}", token(at, &git(at, &["show-ref"])?)?);
    Ok(())
}

/// A worktree standing beside the corpus, as the corpus's own worktree
/// listing has it.
#[derive(Debug, PartialEq, Eq)]
struct Standing {
    nth: usize,
    /// As the listing spells it, which `worktree remove` is sure to take.
    path: String,
    /// `None` for a detached worktree.
    branch: Option<String>,
}

/// The worktrees of a corpus directory called `name`, read out of
/// `git worktree list --porcelain`. A `branch` line belongs to the record
/// it sits in, so one under another tree (the corpus's own, a partial
/// build) names no worktree's branch.
fn worktrees_standing(name: &str, listing: &str) -> Vec<Standing> {
    let prefix = format!("{name}-worktree-");
    let mut found: Vec<Standing> = Vec::new();
    let mut current = None;
    for line in listing.lines() {
        let line = line.trim_end();
        if let Some(path) = line.strip_prefix("worktree ") {
            current = worktree_number(&prefix, path).map(|nth| {
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

/// The number a path is the `<corpus>-worktree-<n>` of, if it is one.
fn worktree_number(prefix: &str, path: &str) -> Option<usize> {
    path.rsplit(['/', '\\'])
        .next()?
        .strip_prefix(prefix)?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::{Standing, worktrees_standing};

    /// A detached worktree has no branch to delete, and a `branch` line
    /// under a tree that is not one of the corpus's worktrees belongs to
    /// that tree.
    #[test]
    fn the_worktrees_are_read_out_of_the_porcelain_listing() {
        let listing = r"worktree C:/x/.pgg-perf-corpus
HEAD 1111111111111111111111111111111111111111
branch refs/heads/main

worktree C:/x/.pgg-perf-corpus-worktree-1
HEAD 1111111111111111111111111111111111111111
branch refs/heads/pgg-worktree-1

worktree C:/x/.pgg-perf-corpus-worktree-2
HEAD 1111111111111111111111111111111111111111
detached

worktree C:\x\.pgg-perf-corpus-worktree-10
HEAD 1111111111111111111111111111111111111111
branch refs/heads/pgg-worktree-10

worktree C:/x/.pgg-perf-corpus.partial-42
HEAD 1111111111111111111111111111111111111111
branch refs/heads/pgg-worktree-9
";
        let standing = worktrees_standing(".pgg-perf-corpus", listing);
        assert_eq!(
            standing
                .iter()
                .map(|worktree| worktree.nth)
                .collect::<Vec<_>>(),
            vec![1, 2, 10]
        );
        assert_eq!(
            standing
                .iter()
                .map(|worktree| worktree.branch.clone())
                .collect::<Vec<_>>(),
            vec![
                Some("pgg-worktree-1".to_string()),
                None,
                Some("pgg-worktree-10".to_string())
            ]
        );
        assert_eq!(
            standing[0],
            Standing {
                nth: 1,
                path: "C:/x/.pgg-perf-corpus-worktree-1".to_string(),
                branch: Some("pgg-worktree-1".to_string()),
            }
        );
        assert!(worktrees_standing(".pgg-perf-corpus", "").is_empty());
    }
}
