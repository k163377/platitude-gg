//! Picking neighbouring files and taking a majority ending from them.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor, literal_pathspec};

use super::attrs::{Ruling, ruling};
use super::scan::Tally;
use super::worktree::{readable, worktree_endings};
use super::{Baseline, Eol, Scope};

/// How many usable samples a baseline needs.
const SAMPLES: usize = 3;
/// How many files may be read looking for them. Neighbours that turn out to
/// be unusable are replaced, but not forever.
const READS: usize = 9;

/// What the files around `path` look like, or `None` — nothing is shown —
/// wherever the app has no basis for naming a house style: git decides the
/// endings, too few readable neighbours, or no majority.
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

    // Same extension in the same directory first: a house style is usually
    // per directory, and this listing is the cheapest.
    let mut picked: Vec<(String, Group)> = if ext.is_empty() {
        Vec::new()
    } else {
        nearby(
            path,
            dir,
            ext,
            &list(executor, workdir, Some(dir), cancel).await?,
        )
    };
    if picked.len() < READS {
        widen(
            path,
            ext,
            &list(executor, workdir, None, cancel).await?,
            &mut picked,
        );
    }

    // Unreadable files are dropped before git is asked. The group travels
    // with the path so dropping one cannot shift what the rest claim.
    picked.retain(|(p, _)| readable(workdir, p));
    if picked.len() < SAMPLES {
        return Ok(None);
    }

    let paths: Vec<String> = picked.iter().map(|(p, _)| p.clone()).collect();
    let read = worktree_endings(executor, workdir, &paths, cancel).await?;
    Ok(settle(&picked, &read, ext))
}

/// The neighbours of `path` that sit beside it and share its extension.
fn nearby(path: &str, dir: &str, ext: &str, here: &[String]) -> Vec<(String, Group)> {
    let neighbours: Vec<String> = here
        .iter()
        .filter(|p| *p != path && parent_of(p) == dir && extension_of(p) == ext)
        .cloned()
        .collect();
    let mut picked = Vec::new();
    take_spread(&neighbours, READS, Group::Here, &mut picked);
    picked
}

/// Makes `picked` up to [`READS`] out of the whole index: the same extension
/// anywhere first, then anything at all, each a [`Group`] of its own.
fn widen(path: &str, ext: &str, all: &[String], picked: &mut Vec<(String, Group)>) {
    let held = |picked: &[(String, Group)], p: &String| picked.iter().any(|(q, _)| q == p);
    if !ext.is_empty() {
        let by_ext: Vec<String> = all
            .iter()
            .filter(|p| *p != path && !held(picked, p) && extension_of(p) == ext)
            .cloned()
            .collect();
        take_spread(&by_ext, READS - picked.len(), Group::Ext, picked);
    }
    if picked.len() < READS {
        let rest: Vec<String> = all
            .iter()
            .filter(|p| *p != path && !held(picked, p))
            .cloned()
            .collect();
        take_spread(&rest, READS - picked.len(), Group::Any, picked);
    }
}

/// What the readings come to: a majority of the first [`SAMPLES`] that
/// answered, and the scope every one of those voters came from.
///
/// `read` holds indices into `picked`, only for paths git gave a single
/// ending for — a mixed or ending-less file casts no vote.
fn settle(picked: &[(String, Group)], read: &[(usize, Eol)], ext: &str) -> Option<Baseline> {
    let mut votes = Tally::default();
    let mut counted = 0usize;
    let mut widest = Group::Here;
    for (index, eol) in read {
        if counted == SAMPLES {
            break;
        }
        votes.add(*eol);
        counted += 1;
        if let Some((_, group)) = picked.get(*index) {
            widest = widest.max(*group);
        }
    }
    if counted < SAMPLES {
        return None;
    }
    let eol = votes.majority()?;
    let scope = match widest {
        Group::Here => Scope::Here(ext.to_string()),
        Group::Ext => Scope::Ext(ext.to_string()),
        Group::Any => Scope::Repo,
    };
    Some(Baseline { eol, scope })
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
/// endings here would read every worktree file in the repository, where the
/// picked few cost one process (ci/baseline/code-costs-windows-x64.md).
async fn list(
    executor: &GitExecutor,
    workdir: &Path,
    dir: Option<&str>,
    cancel: &CancellationToken,
) -> Result<Vec<String>, GitError> {
    let mut cmd = GitCommand::new().cwd(workdir).args(["ls-files", "-z"]);
    // A literal directory pathspec matches its subtree with no glob escaping.
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

/// Takes up to `want` entries spread across the list by a fixed stride:
/// index order is alphabetical, so the head is all one directory.
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
/// name: `.gitignore` has none.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| (*n).to_string()).collect()
    }

    fn names(picked: &[(String, Group)]) -> Vec<&str> {
        picked.iter().map(|(p, _)| p.as_str()).collect()
    }

    fn all_read(picked: &[(String, Group)], eol: Eol) -> Vec<(usize, Eol)> {
        (0..picked.len()).map(|i| (i, eol)).collect()
    }

    #[test]
    fn the_neighbours_are_the_files_beside_it_of_its_own_kind() {
        let here = paths(&[
            "src/a.kt",
            "src/b.kt",
            "src/deep/c.kt",
            "src/notes.md",
            "src/new.kt",
        ]);
        let picked = nearby("src/new.kt", "src", "kt", &here);
        assert_eq!(
            names(&picked),
            ["src/a.kt", "src/b.kt"],
            "a subdirectory is not beside it, another extension is not its kind, \
             and the file itself does not vote on itself"
        );
        assert!(picked.iter().all(|(_, g)| *g == Group::Here));
    }

    #[test]
    fn widening_takes_the_extension_first_and_then_anything() {
        let all = paths(&["one/f.kt", "two/f.kt", "docs/a.md", "docs/b.md"]);
        let mut picked = Vec::new();
        widen("fresh/new.kt", "kt", &all, &mut picked);
        assert_eq!(
            picked,
            [
                ("one/f.kt".to_string(), Group::Ext),
                ("two/f.kt".to_string(), Group::Ext),
                ("docs/a.md".to_string(), Group::Any),
                ("docs/b.md".to_string(), Group::Any),
            ],
            "the same extension anywhere comes before anything at all, and each \
             widening is a group of its own"
        );
    }

    #[test]
    fn a_name_with_no_extension_is_measured_against_anything_at_all() {
        let all = paths(&["a.kt", "b.md", "c.txt"]);
        let mut picked = Vec::new();
        widen("Makefile", "", &all, &mut picked);
        assert!(picked.iter().all(|(_, g)| *g == Group::Any));
        assert_eq!(names(&picked), ["a.kt", "b.md", "c.txt"]);
    }

    #[test]
    fn one_already_picked_is_not_picked_again() {
        let all = paths(&["src/a.kt", "src/b.kt", "elsewhere/c.kt"]);
        let mut picked = nearby("src/new.kt", "src", "kt", &all);
        widen("src/new.kt", "kt", &all, &mut picked);
        assert_eq!(names(&picked), ["src/a.kt", "src/b.kt", "elsewhere/c.kt"]);
    }

    #[test]
    fn more_neighbours_than_may_be_read_are_taken_across_the_listing() {
        let many: Vec<String> = (0..27).map(|i| format!("src/f{i:02}.kt")).collect();
        let picked = nearby("src/new.kt", "src", "kt", &many);
        assert_eq!(picked.len(), READS);
        assert_eq!(
            names(&picked),
            [
                "src/f00.kt",
                "src/f03.kt",
                "src/f06.kt",
                "src/f09.kt",
                "src/f12.kt",
                "src/f15.kt",
                "src/f18.kt",
                "src/f21.kt",
                "src/f24.kt"
            ]
        );
    }

    #[test]
    fn the_scope_is_the_widest_any_voter_came_from() {
        let here = vec![("a.kt".to_string(), Group::Here); 3];
        assert_eq!(
            settle(&here, &all_read(&here, Eol::Lf), "kt"),
            Some(Baseline {
                eol: Eol::Lf,
                scope: Scope::Here("kt".to_string())
            })
        );

        let mut mixed = vec![("a.kt".to_string(), Group::Here); 2];
        mixed.push(("far/b.kt".to_string(), Group::Ext));
        assert_eq!(
            settle(&mixed, &all_read(&mixed, Eol::Crlf), "kt"),
            Some(Baseline {
                eol: Eol::Crlf,
                scope: Scope::Ext("kt".to_string())
            }),
            "one voter from further out is what the notice may claim"
        );

        let wide = vec![("Makefile".to_string(), Group::Any); 3];
        assert_eq!(
            settle(&wide, &all_read(&wide, Eol::Lf), ""),
            Some(Baseline {
                eol: Eol::Lf,
                scope: Scope::Repo
            })
        );
    }

    #[test]
    fn a_reading_past_the_count_is_not_weighed() {
        let picked = vec![("a.kt".to_string(), Group::Here); 4];
        let read = vec![(0, Eol::Lf), (1, Eol::Lf), (2, Eol::Lf), (3, Eol::Crlf)];
        assert_eq!(
            settle(&picked, &read, "kt"),
            Some(Baseline {
                eol: Eol::Lf,
                scope: Scope::Here("kt".to_string())
            })
        );
    }

    #[test]
    fn too_few_answers_is_unknown_and_a_majority_of_three_is_not() {
        let picked = vec![("a.kt".to_string(), Group::Here); 3];
        assert_eq!(
            settle(&picked, &[(0, Eol::Lf), (1, Eol::Lf)], "kt"),
            None,
            "two neighbours are not a house style, and answering from them \
             would be the app inventing one"
        );
        assert_eq!(
            settle(&picked, &[(0, Eol::Lf), (1, Eol::Crlf), (2, Eol::Lf)], "kt"),
            Some(Baseline {
                eol: Eol::Lf,
                scope: Scope::Here("kt".to_string())
            })
        );
        let four = vec![("a.kt".to_string(), Group::Here); 4];
        assert_eq!(
            settle(&four, &[(0, Eol::Lf), (1, Eol::Crlf)], "kt"),
            None,
            "the ones that answered are what is counted, not the ones that were asked"
        );
    }
}
