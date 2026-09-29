//! The preset table and the command entry that drives it.

use std::path::{Path, PathBuf};

use super::authorship::{authorship, co_authors};
use super::basic::{
    basic, detached, dirty, embedded, eol, mergetools, nested, noremote, one_commit, plan,
    rewrite_merge, shallow, stashes,
};
use super::conflict::{
    cherry_pick_conflict, cherry_pick_quit, clashing, conflict, conflict_kinds, conflict_ours,
    conflict_staged, conflict_typed, drop_collides, drop_stops, rebase_clashes, rebase_conflict,
    rebase_empty, rebase_staged, revert_clashes,
};
use super::deep::{deep, deep_detached, deep_parked, perf, perf_sequence, replay};
use super::lfs::{lfs, lfs_conflict};
use super::pictures::{bigpicture, pictures};
use super::remote::{
    behind, diverged, forkdiverged, forkmark, gone, hooked, outrun, protected, slowhook,
    unpublished, unreachable,
};
use super::repo::DemoRepo;
use super::scale::{
    coloured, edges, long, longpaths, manyhunks, spread, widechars, widelate, widelines,
};
use super::signing::{errsig, signed};
use super::stack::{stack, stack_max};
use super::tags::{manytags, tagonly, tagremotes, tags};
use super::worktrees::{
    carried, carried_clashing, carried_many, long_names, nested_copy, panel, tracked_elsewhere,
    worktree_detached, worktrees,
};

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
    // A tree a person asked for: marked so the sweep of old runs leaves
    // it (`verify::keep`).
    crate::verify::keep(&root)?;
    Ok(work)
}

pub fn create(preset: &str, at: Option<PathBuf>) -> Result<PathBuf, String> {
    create_named(preset, at, "repo")
}

/// Where the repository gets built: `--at` made absolute, or one claimed
/// under the system temp.
///
/// A relative `--at` would reach `origin`'s URL (`repo::file_url`) as
/// `file:///target/probe/origin.git`, which git.exe resolves under its own
/// install directory, and the first push fails.
fn root_for(preset: &str, at: Option<PathBuf>) -> Result<PathBuf, String> {
    match at {
        Some(dir) => std::path::absolute(&dir)
            .map_err(|e| format!("could not resolve {}: {e}", dir.display())),
        None => claim_root(preset),
    }
}

/// The roots this process claimed, in order.
///
/// Only what [`claim_root`] made is in here — never an `--at` tree or a
/// template (`super::template`) — which is what makes the list safe to
/// delete from.
static CLAIMED: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

/// A directory of this run's own to build a demo repository in.
///
/// `pgg-demo` is shared by every seat on the machine, so a clock-read
/// name can hand two concurrent runs one root; `claim_dir` makes
/// `create_dir` decide which run owns it.
pub(crate) fn claim_root(stem: &str) -> Result<PathBuf, String> {
    let root = crate::verify::claim_dir(&base(), stem)?;
    if let Ok(mut claimed) = CLAIMED.lock() {
        claimed.push(root.clone());
    }
    Ok(root)
}

/// What this process claimed under `pgg-demo`, for the one caller that
/// takes its own away again (`verify::ownership::give_back_claimed`).
pub(crate) fn claimed_roots() -> Vec<PathBuf> {
    CLAIMED.lock().map(|held| held.clone()).unwrap_or_default()
}

/// The one directory the runs and their templates share, so a copy is a
/// rewrite of one path segment (`template`).
pub(super) fn base() -> PathBuf {
    std::env::temp_dir().join("pgg-demo")
}

/// Builds `preset` with the work tree called `name` — a tab is titled
/// after its work-tree folder (`models::tab_name`).
///
/// A claimed root is a copy of the preset's template (`template`); an
/// `--at` root is built in directly, since a template's paths are under
/// the shared root.
pub fn create_named(preset: &str, at: Option<PathBuf>, name: &str) -> Result<PathBuf, String> {
    let named = at.is_some();
    let root = root_for(preset, at)?;
    build_or_copy_in(preset, &root, name, named)
}

/// Builds `preset` in a root already resolved, so the command entry can
/// mark it; the verify-ui runs come through here too and stay unmarked
/// for the sweep.
fn build_or_copy_in(preset: &str, root: &Path, name: &str, named: bool) -> Result<PathBuf, String> {
    if named {
        return build(preset, root, name);
    }
    super::template::build_or_copy(root, preset, name)
}

pub(super) fn build(preset: &str, root: &Path, name: &str) -> Result<PathBuf, String> {
    let mut repo = DemoRepo::init(root, name)?;
    match preset {
        "basic" => basic(&mut repo)?,
        "nested" => nested(&mut repo)?,
        "dirty" => dirty(&mut repo)?,
        "embedded" => embedded(&mut repo)?,
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
        "lfs" => lfs(&mut repo)?,
        "lfs-conflict" => lfs_conflict(&mut repo)?,
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
        "gone" => gone(&mut repo)?,
        "forkmark" => forkmark(&mut repo)?,
        "forkdiverged" => forkdiverged(&mut repo)?,
        "protected" => protected(&mut repo)?,
        "outrun" => outrun(&mut repo)?,
        "hooked" => hooked(&mut repo)?,
        "slowhook" => slowhook(&mut repo)?,
        "noremote" => noremote(&mut repo)?,
        "unreachable" => unreachable(&mut repo)?,
        "plan" => plan(&mut repo)?,
        "mergetools" => mergetools(&mut repo)?,
        "signed" => signed(&mut repo)?,
        "errsig" => errsig(&mut repo)?,
        "co-authors" => co_authors(&mut repo)?,
        "authorship" => authorship(&mut repo)?,
        "tags" => tags(&mut repo)?,
        "tagremotes" => tagremotes(&mut repo)?,
        "manytags" => manytags(&mut repo)?,
        "tagonly" => tagonly(&mut repo)?,
        "stack" => stack(&mut repo)?,
        "stack-max" => stack_max(&mut repo)?,
        "deep" => deep(&mut repo)?,
        "perf" => perf(&mut repo)?,
        "perf-sequence" => perf_sequence(&mut repo)?,
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
        "longnames" => long_names(&mut repo)?,
        "panel" => panel(&mut repo)?,
        "carried" => carried(&mut repo)?,
        "carried-many" => carried_many(&mut repo)?,
        "carried-clashing" => carried_clashing(&mut repo)?,
        "worktree-detached" => worktree_detached(&mut repo)?,
        "nested-copy" => nested_copy(&mut repo)?,
        "tracked-elsewhere" => tracked_elsewhere(&mut repo)?,
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

    /// The gate starts a side's verbs together (`gate::sides::verbs`),
    /// each building its own repository (`claim_root` says why a clock is
    /// not enough).
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
            // Empty, so nobody else's: the claim created it.
            std::fs::remove_dir(&root).expect("an empty directory this call created");
        }
    }

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
