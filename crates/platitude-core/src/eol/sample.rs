//! Picking neighbouring files and taking a majority ending from them.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor, literal_pathspec};

use super::attrs::{Ruling, ruling};
use super::scan::Tally;
use super::worktree::{readable, worktree_endings};
use super::{Baseline, Scope};

/// How many usable samples a baseline needs.
const SAMPLES: usize = 3;
/// How many files may be read looking for them. Neighbours that turn out to
/// be unusable are replaced, but not forever.
const READS: usize = 9;

/// What the files around `path` look like, or `None` for "unknown" — in
/// which case nothing is shown.
///
/// Unknown deliberately covers several cases: git already deciding the
/// endings, too few readable neighbours, and a sample with no majority all
/// come back the same way, because in each the app has no basis for naming
/// a house style.
pub async fn baseline(
    executor: &GitExecutor,
    workdir: &Path,
    path: &str,
    cancel: &CancellationToken,
) -> Result<Option<Baseline>, GitError> {
    match ruling(executor, workdir, path, cancel).await? {
        Ruling::NotText | Ruling::Normalised => Ok(None),
        Ruling::Open => sample(executor, workdir, path, cancel).await,
    }
}

/// A majority ending sampled from neighbouring files, or `None`.
async fn sample(
    executor: &GitExecutor,
    workdir: &Path,
    path: &str,
    cancel: &CancellationToken,
) -> Result<Option<Baseline>, GitError> {
    let (dir, ext) = split_dir_ext(path);
    let mut picked: Vec<(String, Group)> = Vec::new();
    let held = |picked: &[(String, Group)], p: &String| picked.iter().any(|(q, _)| q == p);

    // Same extension in the same directory first: a repository with a house
    // style usually has it per directory, and this listing is the cheapest.
    if !ext.is_empty() {
        let here = list(executor, workdir, Some(dir), cancel).await?;
        let neighbours: Vec<String> = here
            .into_iter()
            .filter(|p| p != path && parent_of(p) == dir && extension_of(p) == ext)
            .collect();
        take_spread(&neighbours, READS, Group::Here, &mut picked);
    }

    if picked.len() < READS {
        let all = list(executor, workdir, None, cancel).await?;
        if !ext.is_empty() {
            let by_ext: Vec<String> = all
                .iter()
                .filter(|p| *p != path && !held(&picked, p) && extension_of(p) == ext)
                .cloned()
                .collect();
            take_spread(&by_ext, READS - picked.len(), Group::Ext, &mut picked);
        }
        if picked.len() < READS {
            let rest: Vec<String> = all
                .into_iter()
                .filter(|p| p != path && !held(&picked, p))
                .collect();
            take_spread(&rest, READS - picked.len(), Group::Any, &mut picked);
        }
    }

    // Reading a file is the expensive part, so the ones that cannot be read
    // usefully are dropped before git is asked, not after. The group travels
    // with the path so dropping one cannot shift what the rest claim.
    picked.retain(|(p, _)| readable(workdir, p));
    if picked.len() < SAMPLES {
        return Ok(None);
    }

    let paths: Vec<String> = picked.iter().map(|(p, _)| p.clone()).collect();
    let mut votes = Tally::default();
    let mut counted = 0usize;
    // The notice may only claim the range every voter actually came from.
    let mut widest = Group::Here;
    for (index, eol) in worktree_endings(executor, workdir, &paths, cancel).await? {
        if counted == SAMPLES {
            break;
        }
        votes.add(eol);
        counted += 1;
        if let Some((_, group)) = picked.get(index) {
            widest = widest.max(*group);
        }
    }
    if counted < SAMPLES {
        return Ok(None);
    }
    let Some(eol) = votes.majority() else {
        return Ok(None);
    };
    let scope = match widest {
        Group::Here => Scope::Here(ext.to_string()),
        Group::Ext => Scope::Ext(ext.to_string()),
        Group::Any => Scope::Repo,
    };
    Ok(Some(Baseline { eol, scope }))
}

/// How far from the file a sample had to be drawn. Ordered widest-last: the
/// scope a notice may claim is the widest any of its voters came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Group {
    Here,
    Ext,
    Any,
}

/// Index paths, optionally under one directory. The index only — asking for
/// endings here would read every worktree file in the repository (24.7s on
/// the 106k-file reference repository, measured, against 42ms for a handful
/// of settled paths).
async fn list(
    executor: &GitExecutor,
    workdir: &Path,
    dir: Option<&str>,
    cancel: &CancellationToken,
) -> Result<Vec<String>, GitError> {
    let mut cmd = GitCommand::new().cwd(workdir).args(["ls-files", "-z"]);
    // A literal directory pathspec matches its whole subtree, which is
    // narrower than the index and needs no glob escaping.
    if let Some(dir) = dir.filter(|d| !d.is_empty()) {
        cmd = cmd.arg("--").arg(literal_pathspec(dir));
    }
    let out = executor.run(cmd, cancel).await?;
    Ok(out
        .stdout
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty())
        .map(|p| String::from_utf8_lossy(p).into_owned())
        .collect())
}

/// Takes up to `want` entries spread across the list rather than the first
/// `want`: index order is alphabetical, so the head is all one directory.
/// The stride keeps the pick deterministic.
fn take_spread(from: &[String], want: usize, group: Group, into: &mut Vec<(String, Group)>) {
    if want == 0 || from.is_empty() {
        return;
    }
    if from.len() <= want {
        into.extend(from.iter().map(|p| (p.clone(), group)));
        return;
    }
    let stride = from.len() / want;
    for step in 0..want {
        if let Some(p) = from.get(step * stride) {
            into.push((p.clone(), group));
        }
    }
}

/// What a baseline may be reused for: everything of the same extension in
/// the same directory has the same neighbours and the same answer.
pub fn cache_key(path: &str) -> (String, String) {
    let (dir, ext) = split_dir_ext(path);
    (dir.to_string(), ext.to_string())
}

/// The directory a path sits in and the extension of its file name, both
/// empty when it has none.
fn split_dir_ext(path: &str) -> (&str, &str) {
    (parent_of(path), extension_of(path))
}

fn parent_of(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..i],
        None => "",
    }
}

/// The extension of the file name, without the dot. A leading dot is a
/// name, not an extension: `.gitignore` has none.
fn extension_of(path: &str) -> &str {
    let name = match path.rfind('/') {
        Some(i) => &path[i + 1..],
        None => path,
    };
    match name.rfind('.') {
        Some(i) if i > 0 => &name[i + 1..],
        _ => "",
    }
}
