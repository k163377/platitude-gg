//! The working copies that stand beside the corpus (`--copies`).
//!
//! They are a measurement's fixture rather than part of the corpus: the
//! slots record is taken against a machine that is reading other
//! working trees while the window is clicked
//! (ci/baseline/git-slots-windows-x64.md), and these are those trees.

use std::path::{Path, PathBuf};

use super::git;

/// Where the `nth` working copy of the corpus at `at` stands
/// (`stand_copies`).
pub(super) fn copy_path(at: &Path, nth: usize) -> PathBuf {
    let name = at
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    at.with_file_name(format!("{name}-copy-{nth}"))
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
/// The branches are refs, so **the corpus token moves by eight**: a run
/// against a corpus with copies is compared with runs against the same,
/// and the record says which token it was taken under. Taking the
/// copies down puts the token back (`git worktree remove` each, then
/// `git branch -D pgg-copy-<n>`).
///
/// A copy already standing is kept and only put on its branch where it
/// is detached — a `checkout -b` at the same commit moves no file. One
/// asked for beyond what stands is added; none is ever taken away here.
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
