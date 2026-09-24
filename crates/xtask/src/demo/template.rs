//! The built repository a run copies.
//!
//! A preset is a run of git subprocesses — twenty-nine for `basic`,
//! thirty-three for `stack` — and a side of the gate builds one per verb,
//! a hundred and thirty-five times over. What comes out is the same
//! repository every time, so the first run to ask builds it and the rest
//! copy it.
//!
//! **A built repository names where it was built.** `origin`'s URL is a
//! `file://` of an absolute path, a linked worktree's `.git` is a line
//! naming one, and `.git/worktrees/*/gitdir` names the way back. A copy
//! that kept them would push into the template and take the next run's
//! worktrees with it, so a copy is rebound as it is made.
//!
//! **That path is spelled two ways.** What this process hands git
//! comes out of `std::env::temp_dir()`, which on Windows is the profile's
//! short name (`C:/Users/WRONGW~1/…`); git resolves a worktree's own
//! paths and writes the long one (`C:/Users/wrongwrong/…`). One
//! directory, two names, two files of one repository (measured). So the
//! template stands beside the runs, and what gets rewritten is the
//! single path segment they differ in: a
//! leaf that is ASCII, that neither git nor Windows respells, and that is
//! the whole of the difference between the template and the copy.
//!
//! A preset that names its own path anywhere but git's metadata is
//! refused — rewriting a tracked file would leave the copy dirty where
//! the template was clean — and refused presets go on being built a run
//! at a time.

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The file a finished template carries, and the one a preset that
/// cannot be copied carries instead. Both sit at the template's top
/// level, and neither is copied into a run.
const READY: &str = ".pgg-template-ready";
const REFUSED: &str = ".pgg-template-refused";

/// Copies the template for `preset` into `root`, building the template
/// first if this run is the one that finds it missing.
///
/// Falls back to building in `root` whenever there is no template to be
/// had: a preset that was refused, a copy that failed, a template swept
/// out from under this run. Answers the work tree, as [`super::presets::build`] does.
pub(super) fn build_or_copy(root: &Path, preset: &str, name: &str) -> Result<PathBuf, String> {
    let leaf = leaf(preset, name);
    let template = super::presets::base().join(&leaf);
    if standing(&template, preset, name)? {
        match copy_in(&template, &leaf, root) {
            Ok(()) => return Ok(root.join(name)),
            // A template can go while it is being read — the sweep takes
            // yesterday's, and yesterday's is still a template until it
            // does. Nothing about this run is wrong, so it builds its own.
            Err(e) => {
                println!("demo template ({preset}): not copied, building instead — {e}");
                clear(root)?;
            }
        }
    }
    super::presets::build(preset, root, name)
}

/// Whether a template for this preset is standing and copyable, building
/// it if this is the first run to ask.
fn standing(template: &Path, preset: &str, name: &str) -> Result<bool, String> {
    if template.join(REFUSED).is_file() {
        return Ok(false);
    }
    if template.join(READY).is_file() {
        return Ok(true);
    }
    make(template, preset, name)
}

/// Builds the template, at a name of this run's own, and moves it under
/// the one every run reads.
///
/// **The move is what settles a race.** Runs start together and any
/// number of them can find the template missing at once; each builds its
/// own and the first to rename wins, because a rename onto a directory
/// that is already there fails on both operating systems. The losers drop
/// what they built and read the winner's. Nothing is locked: a lock held
/// across a build is a lock held for seconds, and the work being
/// duplicated is the work every run used to do.
fn make(template: &Path, preset: &str, name: &str) -> Result<bool, String> {
    let base = super::presets::base();
    let leaf = template
        .file_name()
        .ok_or("a template needs a name")?
        .to_string_lossy()
        .to_string();
    // The one moment a template is made is the one worth taking
    // yesterday's away in: templates and runs share a directory, a
    // template is built once a fingerprint, and inside a container the
    // gate's own sweep never runs (`linux`).
    crate::verify::sweep_yesterdays_runs();
    // The staging name carries the template's, so the rebind below has one
    // string to look for and the scan has one string to prove gone.
    let staging = crate::verify::claim_dir(&base, &format!("{leaf}-staging"))?;
    let staging_leaf = staging
        .file_name()
        .ok_or("a staged template needs a name")?
        .to_string_lossy()
        .to_string();
    super::presets::build(preset, &staging, name)?;

    let refusal = rebind(&staging, &staging_leaf, &leaf).err();
    let marker = staging.join(if refusal.is_some() { REFUSED } else { READY });
    let note = refusal.clone().unwrap_or_else(|| "copyable".to_string());
    std::fs::write(&marker, format!("{note}\n")).map_err(|e| e.to_string())?;

    match std::fs::rename(&staging, template) {
        Ok(()) => {
            match &refusal {
                Some(reason) => println!("demo template ({preset}): built per run — {reason}"),
                None => println!("demo template ({preset}): built once, copied from here on"),
            }
            Ok(refusal.is_none())
        }
        // Somebody else got there first. Theirs is this one's equal — the
        // fingerprint says so — so this one goes and theirs is read.
        Err(_) => {
            let _ = std::fs::remove_dir_all(&staging);
            Ok(template.join(READY).is_file())
        }
    }
}

/// The name a preset's template stands under, beside the runs.
///
/// Shorter than a run's own name for the common case, which is what the
/// deepest preset needs it to be: a template is built at this name and
/// the paths inside it are the ones Windows measures against its limit.
fn leaf(preset: &str, name: &str) -> String {
    let fingerprint = fingerprint();
    if name == "repo" {
        format!("{preset}-{fingerprint}")
    } else {
        format!("{preset}-{name}-{fingerprint}")
    }
}

/// What a template is good for: the sources that type its git commands,
/// the git that ran them, and the day it was built on.
///
/// **Not the executable's bytes.** The linker stamps every link (the PE
/// header's time, the PDB's GUID), so a key on the exe names a new set of
/// templates after every relink — a rebase is one — and every seat's exe
/// is its own. Each new set is built by the runs that find it missing,
/// all of them at once, in the middle of a gate. The sources are the same
/// bytes on every seat that stands on the same presets, so one set serves
/// all of them.
///
/// The git version is in it because the repository is that git's output:
/// the runs that stand a copy in for git (`--old-git`) build their
/// repositories with the real one, so what changes this is an upgrade.
/// The day is in it because a preset dates its commits from the moment it
/// is built (`DemoRepo::init`): a template stands for one UTC day, the age
/// the sweep already allows a run's litter. The day turns at 09:00 in
/// Japan, and the runs going at that moment build the new set between
/// them, as after any change to a preset's source.
fn fingerprint() -> &'static str {
    static FINGERPRINT: OnceLock<String> = OnceLock::new();
    FINGERPRINT.get_or_init(|| {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for (name, bytes) in BUILT_BY {
            name.hash(&mut hasher);
            bytes.hash(&mut hasher);
        }
        git_version().hash(&mut hasher);
        day().hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    })
}

/// Every source a preset is built by: this module's own (its tests aside)
/// and the PNG writer its pictures come out of. Its git runs under this
/// module's own spawner (`repo::output_of`), so nothing else of the crate
/// shapes a template. A test holds the list to the code
/// (`the_fingerprint_reads_every_source_a_preset_is_built_by`).
const BUILT_BY: &[(&str, &[u8])] = &[
    ("demo/authorship.rs", include_bytes!("authorship.rs")),
    ("demo/basic.rs", include_bytes!("basic.rs")),
    ("demo/conflict.rs", include_bytes!("conflict.rs")),
    ("demo/deep.rs", include_bytes!("deep.rs")),
    ("demo/mod.rs", include_bytes!("mod.rs")),
    ("demo/pictures.rs", include_bytes!("pictures.rs")),
    ("demo/presets.rs", include_bytes!("presets.rs")),
    ("demo/remote.rs", include_bytes!("remote.rs")),
    ("demo/repo.rs", include_bytes!("repo.rs")),
    ("demo/scale.rs", include_bytes!("scale.rs")),
    ("demo/signing.rs", include_bytes!("signing.rs")),
    ("demo/stack.rs", include_bytes!("stack.rs")),
    ("demo/tags.rs", include_bytes!("tags.rs")),
    ("demo/template.rs", include_bytes!("template.rs")),
    ("demo/worktrees.rs", include_bytes!("worktrees.rs")),
    ("png/checksum.rs", include_bytes!("../png/checksum.rs")),
    ("png/mod.rs", include_bytes!("../png/mod.rs")),
    ("png/write.rs", include_bytes!("../png/write.rs")),
];

/// The UTC day, counted from the epoch. A clock before 1970 reads day
/// zero, which is still one day.
fn day() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() / 86_400)
        .unwrap_or_default()
}

/// The git every preset is built by: the one on PATH, which is the one
/// `DemoRepo` spawns. Unreadable answers empty, and the build that comes
/// next says why.
fn git_version() -> String {
    let mut command = std::process::Command::new("git");
    command.arg("--version");
    super::repo::output_of(&mut command)
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Rewrites `from` to `to` wherever a file under `dir` names it.
///
/// Refuses a tree that names it anywhere but git's own metadata. git
/// writes absolute paths into its own files and reads them back; a file
/// git *tracks* is content, and rewriting content would leave the copy
/// dirty where the template was clean — so a preset that commits its own
/// path is one this cannot copy, and says
/// so.
fn rebind(dir: &Path, from: &str, to: &str) -> Result<(), String> {
    walk(dir, &mut |path, kind| {
        match kind {
            Entry::Dir => return Ok(()),
            Entry::File => {}
            // Refused: a copy is made with `walk` as well, and what it
            // cannot copy the template cannot promise.
            Entry::Other => {
                return Err(format!(
                    "{} is neither a file nor a directory",
                    path.display()
                ));
            }
        }
        let bytes = std::fs::read(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
        if !holds(&bytes, from.as_bytes()) {
            return Ok(());
        }
        if !under_git_metadata(dir, path) {
            return Err(format!(
                "{} names the directory it was built in, and git tracks it",
                path.display()
            ));
        }
        let text = String::from_utf8(bytes).map_err(|_| {
            format!(
                "{} names the build directory but is not text",
                path.display()
            )
        })?;
        write_keeping_mode(path, text.replace(from, to).as_bytes())
    })
}

/// Copies a template's contents into `root`, rebound to it on the way.
///
/// The markers stay behind: they say what the template is, and a run is
/// handed a repository.
fn copy_in(template: &Path, leaf: &str, root: &Path) -> Result<(), String> {
    let into = root
        .file_name()
        .ok_or("a run's root needs a name")?
        .to_string_lossy()
        .to_string();
    walk(template, &mut |path, kind| {
        let rest = path
            .strip_prefix(template)
            .map_err(|e| format!("{} is not under the template: {e}", path.display()))?;
        if rest.parent() == Some(Path::new("")) && is_marker(rest) {
            return Ok(());
        }
        let target = root.join(rest);
        match kind {
            Entry::Dir => {
                std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
                Ok(())
            }
            Entry::File => {
                let bytes =
                    std::fs::read(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
                if !holds(&bytes, leaf.as_bytes()) {
                    std::fs::copy(path, &target)
                        .map_err(|e| format!("copying {}: {e}", path.display()))?;
                    return Ok(());
                }
                let text = String::from_utf8(bytes).map_err(|_| {
                    format!("{} names the template and is not text", path.display())
                })?;
                std::fs::write(&target, text.replace(leaf, &into))
                    .map_err(|e| format!("writing {}: {e}", target.display()))?;
                copy_mode(path, &target)
            }
            Entry::Other => Err(format!(
                "{} is neither a file nor a directory",
                path.display()
            )),
        }
    })
}

/// What an entry under a template is allowed to be.
#[derive(PartialEq, Eq, Clone, Copy)]
enum Entry {
    Dir,
    File,
    Other,
}

/// Hands every entry under `dir` to `visit`, directories before what is
/// in them. Depth-first and iterative: a repository of linked worktrees
/// is deep enough to keep off the stack.
fn walk(
    dir: &Path,
    visit: &mut impl FnMut(&Path, Entry) -> Result<(), String>,
) -> Result<(), String> {
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        let entries =
            std::fs::read_dir(&next).map_err(|e| format!("reading {}: {e}", next.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("reading {}: {e}", next.display()))?;
            let path = entry.path();
            let kind = entry
                .file_type()
                .map_err(|e| format!("reading {}: {e}", path.display()))?;
            let kind = if kind.is_dir() {
                Entry::Dir
            } else if kind.is_file() {
                Entry::File
            } else {
                Entry::Other
            };
            visit(&path, kind)?;
            if kind == Entry::Dir {
                pending.push(path);
            }
        }
    }
    Ok(())
}

fn is_marker(rest: &Path) -> bool {
    rest == Path::new(READY) || rest == Path::new(REFUSED)
}

/// Whether `path` is one of git's own files: under the `.git` a work
/// tree stands on, or under a bare repository — a preset's
/// `origin.git`, and the clone that seeds it, name their paths in the
/// files a work tree names them in, and git tracks nothing in
/// either.
///
/// A bare repository is recognised by what is in it: a directory
/// called `foo.git` that git does not keep is a directory a preset
/// could commit, and rewriting what it holds would leave the copy
/// dirty where the template was clean.
fn under_git_metadata(root: &Path, path: &Path) -> bool {
    let Ok(rest) = path.strip_prefix(root) else {
        return false;
    };
    let mut at = root.to_path_buf();
    for part in rest.components() {
        if part.as_os_str() == ".git" {
            return true;
        }
        at.push(part);
        if at.join("HEAD").is_file() && at.join("objects").is_dir() {
            return true;
        }
    }
    false
}

/// Whether `haystack` holds `needle`. Written out because xtask
/// depends on std alone, and a repository's pack files go through
/// here.
fn holds(haystack: &[u8], needle: &[u8]) -> bool {
    let Some((first, rest)) = needle.split_first() else {
        return false;
    };
    if haystack.len() < needle.len() {
        return false;
    }
    haystack[..=haystack.len() - needle.len()]
        .iter()
        .enumerate()
        .filter(|(_, byte)| *byte == first)
        .any(|(at, _)| &haystack[at + 1..at + needle.len()] == rest)
}

/// Replaces a file's contents and leaves its mode as it was: a hook is a
/// hook because it is executable, and rewriting one leaves it runnable.
fn write_keeping_mode(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mode = std::fs::metadata(path).map(|meta| meta.permissions()).ok();
    std::fs::write(path, bytes).map_err(|e| format!("writing {}: {e}", path.display()))?;
    if let Some(mode) = mode {
        let _unchanged_mode = std::fs::set_permissions(path, mode);
    }
    Ok(())
}

fn copy_mode(from: &Path, to: &Path) -> Result<(), String> {
    if let Ok(meta) = std::fs::metadata(from) {
        let _unchanged_mode = std::fs::set_permissions(to, meta.permissions());
    }
    Ok(())
}

/// Empties a run's root after a copy stopped partway, so the build that
/// follows starts on the bare directory it was handed.
fn clear(root: &Path) -> Result<(), String> {
    if root.exists() {
        std::fs::remove_dir_all(root).map_err(|e| format!("clearing {}: {e}", root.display()))?;
    }
    std::fs::create_dir_all(root).map_err(|e| format!("making {}: {e}", root.display()))
}

#[cfg(test)]
mod tests;
