//! The working copies that stand beside the corpus (`--copies`).
//!
//! They are a measurement's fixture rather than part of the corpus: the
//! slots record is taken against a machine that is reading other
//! working trees while the window is clicked
//! (ci/baseline/git-slots-windows-x64.md), and these are those trees.

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

/// `--copies n`: exactly `n` working copies stand beside the corpus
/// when this returns, whichever direction that is from where it
/// started.
///
/// **One flag stands them and takes them down**, because they are a
/// fixture the measurement holds and not part of the corpus: each is a
/// hundred and ten thousand checked-out files and one more ref, so a
/// set left standing keeps half a gigabyte apiece and holds the corpus
/// token away from the one the judgement record is taken under
/// (`stand_copies`). The measurement stands them at its entrance and
/// `--copies 0` takes them down at its exit
/// (ci/baseline/git-slots-windows-x64.md §再実行).
pub(super) fn set_copies(at: &Path, count: usize) -> Result<(), String> {
    fold_copies(at, count)?;
    stand_copies(at, count)
}

/// The other working copies the slots measurement reads beside the
/// corpus (ci/baseline/git-slots-windows-x64.md): `count` linked
/// working trees of it, each holding one untracked file so the pass
/// that reads them has a row to find, and named for their number beside
/// the corpus (`<corpus>-copy-<n>`, ignored like the corpus itself).
///
/// **Each on a branch of its own (`pgg-copy-<n>`).** A
/// copy standing on no branch is a row only the walk can draw
/// (`session::joins::WorktreeNews`), so the opening's listing asks for a
/// rebuild that takes the opening stream over before its first chunk —
/// and a run the harness cannot see the walk of is no reading at all.
/// The branches are refs, so **the corpus token moves by one for every
/// copy**: a run against a corpus with copies is compared with runs
/// against the same, and the record says which token it was taken
/// under. `--copies 0` puts the token back (`fold_copies`).
///
/// A copy already standing is kept and only put on its branch where it
/// is detached — a `checkout -b` at the same commit moves no file. One
/// asked for beyond what stands is added; the ones past what was asked
/// for are `fold_copies`'s, which `set_copies` has already run.
pub(super) fn stand_copies(at: &Path, count: usize) -> Result<(), String> {
    for nth in 1..=count {
        let copy = copy_path(at, nth);
        let copy_text = copy.to_string_lossy().replace('\\', "/");
        let branch = format!("pgg-copy-{nth}");
        if !copy.join(".git").exists() {
            println!("standing copy {nth}: {copy_text}");
            git(at, &["worktree", "add", "-b", &branch, &copy_text, "HEAD"])?;
            // One untracked file: the cheapest dirt there is, and enough
            // for the read to count the copy as carrying something.
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

/// Takes down every copy standing above `keep`.
///
/// **A copy is a tree and the branch it stands on**, so both go — and
/// the branch is read out of the listing rather than spelled from the
/// number, because a copy standing detached has none (`stand_copies`).
/// `worktree remove` is given `--force`: every copy carries the
/// untracked file it was stood with.
///
/// **The token is said on both sides of it.** The token is the
/// fingerprint of the refs and the copies' branches are refs, so which
/// record a later run may be compared against is exactly what this
/// moves (ci/baseline/perf-windows-x64.md §計測条件).
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
    // The administrative directory under `.git/worktrees` is the other
    // half of a copy, and a tree taken away by hand leaves that half
    // behind — where the next `worktree add` under the same name
    // refuses.
    git(at, &["worktree", "prune"])?;
    println!("token {before} -> {}", token(at, &git(at, &["show-ref"])?)?);
    Ok(())
}

/// A copy standing beside the corpus, as the corpus's own worktree
/// listing has it.
#[derive(Debug, PartialEq, Eq)]
struct Standing {
    nth: usize,
    /// Spelled the way the listing spells it, which is the spelling
    /// `worktree remove` is sure to take.
    path: String,
    /// `None` where the copy stands detached and so has no branch to go
    /// with it.
    branch: Option<String>,
}

/// The copies standing beside a corpus directory called `name`, read
/// out of `git worktree list --porcelain`.
///
/// Every other tree in the listing is somebody else's — the corpus's
/// own, a build left behind (`corpus::build::partial_path`) — and a `branch`
/// line belongs to the tree whose record it sits in, so one under a
/// tree that is not a copy names no copy's branch.
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

    /// What `--copies 0` takes down is what the listing calls a copy,
    /// and a copy is a tree plus the branch of its own record: a
    /// detached one has no branch to delete, and a `branch` line under
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
