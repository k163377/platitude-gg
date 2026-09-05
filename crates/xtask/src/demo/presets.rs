//! The preset table and the command entry that drives it.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::authorship::{authorship, co_authors};
use super::basic::{
    basic, detached, dirty, eol, noremote, one_commit, plan, rewrite_merge, shallow, stashes,
};
use super::conflict::{
    cherry_pick_conflict, cherry_pick_quit, clashing, conflict, conflict_kinds, conflict_ours,
    conflict_staged, conflict_typed, drop_collides, drop_stops, rebase_clashes, rebase_conflict,
    rebase_empty, rebase_staged, revert_clashes,
};
use super::deep::{deep, deep_detached, replay};
use super::pictures::{bigpicture, pictures};
use super::remote::{behind, diverged, forkmark, hooked, outrun, protected, slowhook, unpublished};
use super::repo::DemoRepo;
use super::scale::{edges, long, longpaths, manyhunks, widechars, widelines};
use super::signing::{errsig, signed};
use super::stack::{stack, stack_max};
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

/// Where the repository gets built: what `--at` named, resolved against
/// the directory the command was typed in, or a fresh one under the
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
        None => {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos();
            Ok(std::env::temp_dir()
                .join("pg-demo")
                .join(format!("{preset}-{nanos}")))
        }
    }
}

/// Builds `preset` with the work tree called `name` rather than `repo` —
/// a tab is titled after its work-tree folder (`models::tab_name`).
pub fn create_named(preset: &str, at: Option<PathBuf>, name: &str) -> Result<PathBuf, String> {
    let root = root_for(preset, at)?;
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
        "plan" => plan(&mut repo)?,
        "signed" => signed(&mut repo)?,
        "errsig" => errsig(&mut repo)?,
        "co-authors" => co_authors(&mut repo)?,
        "authorship" => authorship(&mut repo)?,
        "tags" => tags(&mut repo)?,
        "manytags" => manytags(&mut repo)?,
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
        "replay" => replay(&mut repo)?,
        "edges" => edges(&mut repo)?,
        "long" => long(&mut repo)?,
        "longpaths" => longpaths(&mut repo)?,
        "manyhunks" => manyhunks(&mut repo)?,
        "widechars" => widechars(&mut repo)?,
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
