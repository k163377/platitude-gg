//! The preset table and the command entry that drives it.

use std::path::{Path, PathBuf};

use super::authorship::{authorship, co_authors};
use super::basic::{
    basic, detached, dirty, eol, noremote, one_commit, plan, rewrite_merge, shallow, stashes,
};
use super::conflict::{
    cherry_pick_conflict, cherry_pick_quit, clashing, conflict, conflict_kinds, conflict_ours,
    conflict_staged, conflict_typed, drop_collides, drop_stops, rebase_clashes, rebase_conflict,
    rebase_empty, rebase_staged, revert_clashes,
};
use super::deep::{deep, deep_detached, deep_parked, replay};
use super::pictures::{bigpicture, pictures};
use super::remote::{
    behind, diverged, forkmark, hooked, outrun, protected, slowhook, unpublished, unreachable,
};
use super::repo::DemoRepo;
use super::scale::{
    coloured, edges, long, longpaths, manyhunks, spread, widechars, widelate, widelines,
};
use super::signing::{errsig, signed};
use super::stack::{stack, stack_max};
use super::tags::{manytags, tagonly, tags};
use super::worktrees::worktrees;

pub fn run(args: &[String]) -> Result<PathBuf, String> {
    let mut preset: Option<&str> = None;
    let mut at: Option<PathBuf> = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--at" => {
                let dir = it.next().ok_or("--at needs a directory")?;
                at = Some(PathBuf::from(dir));
            }
            other if preset.is_none() => preset = Some(other),
            other => return Err(format!("unexpected argument: {other}")),
        }
    }
    let preset = preset.ok_or("demo-repo needs a preset (see `cargo xtask`)")?;
    let named = at.is_some();
    let root = root_for(preset, at)?;
    let work = build_or_copy_in(preset, &root, "repo", named)?;
    // This door is the one a person types at, and what comes out of it is
    // a tree to look at rather than a run's leavings — so it is marked,
    // and the sweep that takes the runs of a day ago leaves it standing
    // (`verify::keep`).
    crate::verify::keep(&root)?;
    Ok(work)
}

pub fn create(preset: &str, at: Option<PathBuf>) -> Result<PathBuf, String> {
    create_named(preset, at, "repo")
}

/// Where the repository gets built: what `--at` named, resolved against
/// the directory the command was typed in, or one claimed under the
/// system temp.
///
/// **A relative `--at` is written into the repository and read back by a
/// git process of git's own choosing.** `origin`'s URL comes out of this
/// path (`repo::file_url`), so `--at target/probe` makes
/// `file:///target/probe/origin.git` — a POSIX absolute path, which
/// git.exe rewrites into its own install directory
/// (`C:/Program Files/Git/target/probe/origin.git`) and the first push
/// fails against (measured). The default path is absolute already; this
/// makes a given one the same.
fn root_for(preset: &str, at: Option<PathBuf>) -> Result<PathBuf, String> {
    match at {
        Some(dir) => std::path::absolute(&dir)
            .map_err(|e| format!("could not resolve {}: {e}", dir.display())),
        None => claim_root(preset),
    }
}

/// A directory of this run's own to build a demo repository in.
///
/// **`pg-demo` is one directory for the whole machine**, so the name
/// under it is the whole of what keeps two runs apart, and a name read
/// off a clock is not enough — the seats build their repositories
/// concurrently, and two that share a root `git init` into each other.
/// `claim_dir` makes `create_dir` say which run owns the answer.
pub(crate) fn claim_root(stem: &str) -> Result<PathBuf, String> {
    crate::verify::claim_dir(&base(), stem)
}

/// The one directory the runs and the templates they copy share.
///
/// Sharing it is what makes a copy a rewrite of one path segment rather
/// than of a whole path (`template`), and it puts the templates where the
/// sweep that takes yesterday's runs already looks.
pub(super) fn base() -> PathBuf {
    std::env::temp_dir().join("pg-demo")
}

/// Builds `preset` with the work tree called `name` rather than `repo` —
/// a tab is titled after its work-tree folder (`models::tab_name`).
///
/// A root of this run's own is a copy of the preset's template
/// (`template`); a root somebody named is built in directly, because what
/// a template holds are paths under the root the runs share and `--at`
/// points anywhere on the machine.
pub fn create_named(preset: &str, at: Option<PathBuf>, name: &str) -> Result<PathBuf, String> {
    let named = at.is_some();
    let root = root_for(preset, at)?;
    build_or_copy_in(preset, &root, name, named)
}

/// Builds `preset` in a root already resolved, so the command entry can
/// hold that root and mark it. Unmarked is what a run's root stays: the
/// verify-ui runs come through here too, and theirs is the litter the
/// sweep exists for.
fn build_or_copy_in(preset: &str, root: &Path, name: &str, named: bool) -> Result<PathBuf, String> {
    if named {
        return build(preset, root, name);
    }
    super::template::build_or_copy(root, preset, name)
}

/// The preset itself: git, as many times as the state takes.
pub(super) fn build(preset: &str, root: &Path, name: &str) -> Result<PathBuf, String> {
    let mut repo = DemoRepo::init(root, name)?;
    match preset {
        "basic" => basic(&mut repo)?,
        "dirty" => dirty(&mut repo)?,
        "eol" => eol(&mut repo)?,
        "clashing" => clashing(&mut repo)?,
        "revert-clashes" => revert_clashes(&mut repo)?,
        "conflict" => conflict(&mut repo)?,
        "conflict-typed" => conflict_typed(&mut repo)?,
        "conflict-staged" => conflict_staged(&mut repo)?,
        "conflict-ours" => conflict_ours(&mut repo)?,
        "rebase-clashes" => rebase_clashes(&mut repo)?,
        "rebase-conflict" => rebase_conflict(&mut repo)?,
        "rebase-staged" => rebase_staged(&mut repo)?,
        "rebase-empty" => rebase_empty(&mut repo)?,
        "cherry-pick-conflict" => cherry_pick_conflict(&mut repo)?,
        "cherry-pick-quit" => cherry_pick_quit(&mut repo)?,
        "conflict-kinds" => conflict_kinds(&mut repo)?,
        "drop-collides" => drop_collides(&mut repo)?,
        "drop-stops" => drop_stops(&mut repo)?,
        "stashes" => stashes(&mut repo)?,
        "detached" => detached(&mut repo)?,
        "rewrite-merge" => rewrite_merge(&mut repo)?,
        "one-commit" => one_commit(&mut repo)?,
        "shallow" => shallow(&mut repo)?,
        "behind" => behind(&mut repo)?,
        "diverged" => diverged(&mut repo)?,
        "unpublished" => unpublished(&mut repo)?,
        "forkmark" => forkmark(&mut repo)?,
        "protected" => protected(&mut repo)?,
        "outrun" => outrun(&mut repo)?,
        "hooked" => hooked(&mut repo)?,
        "slowhook" => slowhook(&mut repo)?,
        "noremote" => noremote(&mut repo)?,
        "unreachable" => unreachable(&mut repo)?,
        "plan" => plan(&mut repo)?,
        "signed" => signed(&mut repo)?,
        "errsig" => errsig(&mut repo)?,
        "co-authors" => co_authors(&mut repo)?,
        "authorship" => authorship(&mut repo)?,
        "tags" => tags(&mut repo)?,
        "manytags" => manytags(&mut repo)?,
        "tagonly" => tagonly(&mut repo)?,
        "stack" => stack(&mut repo)?,
        "stack-max" => stack_max(&mut repo)?,
        "deep" => deep(&mut repo)?,
        "perf" => {
            deep(&mut repo)?;
            repo.commit(
                "f.txt",
                "A changed file for the visible diff.\n",
                "test: performance scenario",
            )?;
        }
        "deep-detached" => deep_detached(&mut repo)?,
        "deep-parked" => deep_parked(&mut repo)?,
        "replay" => replay(&mut repo)?,
        "edges" => edges(&mut repo)?,
        "long" => long(&mut repo)?,
        "spread" => spread(&mut repo)?,
        "longpaths" => longpaths(&mut repo)?,
        "coloured" => coloured(&mut repo)?,
        "manyhunks" => manyhunks(&mut repo)?,
        "widechars" => widechars(&mut repo)?,
        "widelate" => widelate(&mut repo)?,
        "widelines" => widelines(&mut repo)?,
        "worktrees" => worktrees(&mut repo)?,
        "pictures" => pictures(&mut repo)?,
        "bigpicture" => bigpicture(&mut repo)?,
        "empty" => {}
        other => return Err(format!("unknown preset: {other}")),
    }
    Ok(repo.work)
}

#[cfg(test)]
mod tests {
    use super::root_for;

    /// `pg-demo` is one directory for every seat on the machine, so the
    /// root a preset is built in is the whole of what keeps two runs
    /// apart — the repositories, the `origin.git` they push to and the
    /// configuration they are isolated by all sit in it. The gate starts
    /// a side's verbs together (`gate::verbs`), and each of them builds
    /// its own repository, which is where a clock that two of them read
    /// inside one tick would have handed them one root to `git init` in.
    #[test]
    fn preset_runs_started_together_are_handed_a_root_each() {
        let start = std::sync::Arc::new(std::sync::Barrier::new(16));
        let threads: Vec<_> = (0..16)
            .map(|_| {
                let start = start.clone();
                std::thread::spawn(move || {
                    start.wait();
                    root_for("basic", None).expect("a root for this run")
                })
            })
            .collect();
        let made: Vec<_> = threads
            .into_iter()
            .map(|thread| thread.join().expect("a claiming thread"))
            .collect();

        let unique: std::collections::BTreeSet<_> = made.iter().collect();
        assert_eq!(unique.len(), made.len(), "two runs were handed one root");
        for root in made {
            // Empty, and so nobody else's: the claim made it rather than
            // finding it, which is what `create_dir_all` could not say.
            std::fs::remove_dir(&root).expect("an empty directory this call created");
        }
    }

    /// A relative `--at` reaches `origin`'s URL, which git resolves
    /// against a directory of its own choosing — so it has to be made
    /// absolute before anything is written from it.
    #[test]
    fn a_given_at_is_absolute_before_any_url_is_written_from_it() {
        let root = root_for("basic", Some(std::path::PathBuf::from("target/probe")))
            .expect("the working directory resolves");
        assert!(root.is_absolute(), "{}", root.display());
        assert!(root.ends_with("target/probe"), "{}", root.display());
        let absolute = std::env::current_dir().expect("a working directory");
        assert_eq!(
            root_for("basic", Some(absolute.clone())).expect("an absolute path resolves"),
            absolute
        );
    }
}
