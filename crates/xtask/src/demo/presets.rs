//! The preset table and the command entry that drives it.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::authorship::{authorship, co_authors};
use super::basic::{basic, detached, dirty, eol, noremote, stashes};
use super::conflict::{
    cherry_pick_conflict, cherry_pick_quit, clashing, conflict, conflict_kinds, conflict_ours,
    conflict_staged, conflict_typed, drop_collides, drop_stops, rebase_clashes, rebase_conflict,
    rebase_empty, rebase_staged, revert_clashes,
};
use super::deep::{deep, deep_detached};
use super::remote::{behind, diverged, forkmark, protected, unpublished};
use super::repo::DemoRepo;
use super::scale::{edges, long, longpaths, manyhunks, widechars, widelines};
use super::signing::{errsig, signed};
use super::tags::{manytags, tags};
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
    create(preset, at)
}

pub fn create(preset: &str, at: Option<PathBuf>) -> Result<PathBuf, String> {
    create_named(preset, at, "repo")
}

/// Builds `preset` with the work tree called `name` rather than `repo` —
/// a tab is titled after its work-tree folder (`models::tab_name`).
pub fn create_named(preset: &str, at: Option<PathBuf>, name: &str) -> Result<PathBuf, String> {
    let root = match at {
        Some(dir) => dir,
        None => {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos();
            std::env::temp_dir()
                .join("pg-demo")
                .join(format!("{preset}-{nanos}"))
        }
    };
    let mut repo = DemoRepo::init(&root, name)?;
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
        "behind" => behind(&mut repo)?,
        "diverged" => diverged(&mut repo)?,
        "unpublished" => unpublished(&mut repo)?,
        "forkmark" => forkmark(&mut repo)?,
        "protected" => protected(&mut repo)?,
        "noremote" => noremote(&mut repo)?,
        "signed" => signed(&mut repo)?,
        "errsig" => errsig(&mut repo)?,
        "co-authors" => co_authors(&mut repo)?,
        "authorship" => authorship(&mut repo)?,
        "tags" => tags(&mut repo)?,
        "manytags" => manytags(&mut repo)?,
        "deep" => deep(&mut repo)?,
        "deep-detached" => deep_detached(&mut repo)?,
        "edges" => edges(&mut repo)?,
        "long" => long(&mut repo)?,
        "longpaths" => longpaths(&mut repo)?,
        "manyhunks" => manyhunks(&mut repo)?,
        "widechars" => widechars(&mut repo)?,
        "widelines" => widelines(&mut repo)?,
        "worktrees" => worktrees(&mut repo)?,
        "empty" => {}
        other => return Err(format!("unknown preset: {other}")),
    }
    Ok(repo.work)
}
