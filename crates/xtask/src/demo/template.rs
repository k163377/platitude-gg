//! The built repository a run copies. A preset is dozens of git
//! subprocesses and the gate builds one per verb, yet the result is the
//! same repository every time, so the first run to ask builds it and the
//! rest copy it.
//!
//! A built repository names where it was built: `origin`'s URL is a
//! `file://` of an absolute path, a linked worktree's `.git` names one,
//! and `.git/worktrees/*/gitdir` names the way back. A copy that kept them
//! would push into the template and take the next run's worktrees with
//! it, so a copy is rebound as it is made.
//!
//! That path is spelled two ways on Windows: `std::env::temp_dir()` gives
//! the profile's short name (`WRONGW~1`), while git writes a worktree's
//! paths with the long one. So the template stands beside the runs and
//! only the leaf they differ in is rewritten — ASCII, which neither git
//! nor Windows respells.
//!
//! A preset that names its own path anywhere but git's metadata is
//! refused — rewriting a tracked file would leave the copy dirty where
//! the template was clean — and is built a run at a time.

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The file a finished template carries, and the one a preset that
/// cannot be copied carries instead. Both sit at the template's top
/// level and are not copied into a run.
const READY: &str = ".pgg-template-ready";
const REFUSED: &str = ".pgg-template-refused";

/// Copies the template for `preset` into `root`, building it first if
/// missing. Falls back to building in `root` when there is no template to
/// be had (refused, copy failed, swept away). Answers the worktree, as
/// [`super::presets::build`] does.
pub(super) fn build_or_copy(root: &Path, preset: &str, name: &str) -> Result<PathBuf, String> {
    let leaf = leaf(preset, name);
    let template = super::presets::base().join(&leaf);
    if standing(&template, preset, name)? {
        match copy_in(&template, &leaf, root) {
            Ok(()) => return Ok(root.join(name)),
            // The sweep can take yesterday's template mid-copy; nothing is
            // wrong with this run, so it builds its own.
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

/// Builds the template under a staging name of this run's own and renames
/// it into place.
///
/// The rename settles the race: any number of runs can find the template
/// missing at once, each builds its own, and the first rename wins, since
/// renaming onto an existing directory fails on both systems; the losers
/// drop theirs and read the winner's. No lock: it would be held across a
/// build, and the duplicated work is only what each run would do without
/// a template.
fn make(template: &Path, preset: &str, name: &str) -> Result<bool, String> {
    let base = super::presets::base();
    let leaf = template
        .file_name()
        .ok_or("a template needs a name")?
        .to_string_lossy()
        .to_string();
    // Yesterday's templates go here: one is built only once a
    // fingerprint, and inside a container the gate's own sweep never runs
    // (`linux`).
    crate::verify::sweep_yesterdays_runs();
    // The staging name carries the template's, so the rebind has one
    // string to look for and the scan one string to prove gone.
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
        // Another run renamed first; the fingerprint makes theirs equal.
        Err(_) => {
            let _ = std::fs::remove_dir_all(&staging);
            Ok(template.join(READY).is_file())
        }
    }
}

/// The name a preset's template stands under, beside the runs. Shorter
/// than a run's name in the common case, because the deepest preset is
/// built at this name and its paths must fit Windows' limit.
fn leaf(preset: &str, name: &str) -> String {
    let fingerprint = fingerprint();
    if name == "repo" {
        format!("{preset}-{fingerprint}")
    } else {
        format!("{preset}-{name}-{fingerprint}")
    }
}

/// What a template is keyed on: the sources that type its git commands,
/// the git that ran them, and the UTC day.
///
/// Not the executable's bytes: the linker stamps every link (PE time, PDB
/// GUID), so every relink and every seat would get its own set of
/// templates, each built mid-gate; the sources are the same on every
/// seat.
///
/// The git version because the repository is that git's output
/// (`--old-git` runs still build with the real one). The day because a
/// preset dates its commits from its build (`DemoRepo::init`); one day is
/// also the age the sweep allows a run's litter.
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

/// Every source a preset is built by: this module's own (tests aside) and
/// the PNG writer its pictures come out of; its git runs under its own
/// spawner (`repo::output_of`). A test holds the list to the code
/// (`the_fingerprint_reads_every_source_a_preset_is_built_by`).
const BUILT_BY: &[(&str, &[u8])] = &[
    ("demo/authorship.rs", include_bytes!("authorship.rs")),
    ("demo/basic.rs", include_bytes!("basic.rs")),
    ("demo/conflict.rs", include_bytes!("conflict.rs")),
    ("demo/deep.rs", include_bytes!("deep.rs")),
    ("demo/discards.rs", include_bytes!("discards.rs")),
    ("demo/lfs.rs", include_bytes!("lfs.rs")),
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

/// The UTC day since the epoch; a clock before 1970 reads day zero.
fn day() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() / 86_400)
        .unwrap_or_default()
}

/// The version of the git on PATH, which `DemoRepo` spawns. Unreadable
/// answers empty; the build that follows says why.
fn git_version() -> String {
    let mut command = std::process::Command::new("git");
    command.arg("--version");
    super::repo::output_of(&mut command)
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Rewrites `from` to `to` wherever a file under `dir` names it. Refuses
/// when such a file is content rather than git's own metadata (see the
/// module doc).
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

/// Copies a template's contents into `root`, rebound to it on the way,
/// leaving the markers behind.
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

/// Whether `path` is one of git's own files: under a worktree's `.git`,
/// or under a bare repository (a preset's `origin.git`). A bare
/// repository is recognised by its contents, not its name: a `foo.git`
/// directory without them could be committed content.
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

/// Whether `haystack` holds `needle`, as bytes: pack files go through
/// here, and xtask is std-only.
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
