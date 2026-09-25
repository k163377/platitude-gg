//! What the files on disk look like, as git reports them.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor, literal_pathspec};

use super::Eol;

/// Files above this size are not sampled: `ls-files --eol` reads the whole
/// worktree file to fill its `w/` column, so what one costs is its own
/// bytes (ci/baseline/code-costs-windows-x64.md).
const SAMPLE_MAX_BYTES: u64 = 1 << 20;

/// Every untracked, non-ignored file and what its bytes look like, in one
/// spawn instead of a patch per file. For a new file the shape is the whole
/// answer, except [`Shape::Mixed`], whose line count only the patch gives.
pub async fn untracked_shapes(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Vec<(String, Shape)>, GitError> {
    let cmd = GitCommand::new().cwd(workdir).args([
        "ls-files",
        "--eol",
        "-z",
        "--others",
        "--exclude-standard",
    ]);
    let out = executor.run(cmd, cancel).await?;
    Ok(worktree_shapes(&out.stdout))
}

/// What a file's bytes on disk look like, as `ls-files --eol` reports them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// Every line ends the same way.
    Uniform(Eol),
    /// Both endings are in there.
    Mixed,
    /// No endings at all, not text, or not checked out — nothing to say and
    /// nothing to vote with.
    Nothing,
}

/// Reads `ls-files --eol -z` output into one shape per path, in the order
/// git listed them.
///
/// Reads the `w/` column — the bytes on disk, the space a new file's patch
/// is read in. `i/` differs only where git converts, and there none of this
/// is asked.
pub fn worktree_shapes(stdout: &[u8]) -> Vec<(String, Shape)> {
    let mut out = Vec::new();
    for record in stdout.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let text = String::from_utf8_lossy(record);
        // `i/lf    w/crlf  attr/                 \t<path>`
        let Some((columns, path)) = text.split_once('\t') else {
            continue;
        };
        let worktree = columns
            .split_whitespace()
            .find_map(|c| c.strip_prefix("w/"))
            .unwrap_or_default();
        let shape = match worktree {
            "lf" => Shape::Uniform(Eol::Lf),
            "crlf" => Shape::Uniform(Eol::Crlf),
            "mixed" => Shape::Mixed,
            _ => Shape::Nothing,
        };
        out.push((path.to_string(), shape));
    }
    out
}

/// The worktree ending of each path git can name one for, with the index it
/// came in at. Unusable samples are simply absent.
pub(super) async fn worktree_endings(
    executor: &GitExecutor,
    workdir: &Path,
    paths: &[String],
    cancel: &CancellationToken,
) -> Result<Vec<(usize, Eol)>, GitError> {
    let mut cmd = GitCommand::new()
        .cwd(workdir)
        .args(["ls-files", "--eol", "-z", "--"]);
    for p in paths {
        cmd = cmd.arg(literal_pathspec(p));
    }
    let out = executor.run(cmd, cancel).await?;

    let mut found: Vec<(usize, Eol)> = Vec::new();
    for (path, shape) in worktree_shapes(&out.stdout) {
        let Shape::Uniform(eol) = shape else { continue };
        if let Some(index) = paths.iter().position(|p| *p == path) {
            found.push((index, eol));
        }
    }
    // git answers in its own order; the caller's ranking decides who votes.
    found.sort_by_key(|(index, _)| *index);
    Ok(found)
}

pub(super) fn readable(workdir: &Path, path: &str) -> bool {
    let Ok(meta) = std::fs::metadata(workdir.join(path)) else {
        return false;
    };
    meta.is_file() && meta.len() <= SAMPLE_MAX_BYTES
}
