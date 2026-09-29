//! What a run is staged with before the app starts: a git that answers
//! `--version` old or a second git off PATH, and the identity the screen
//! begins from.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// Set on the app when `--old-git` asks for one: the version a copy of this
/// binary, standing on PATH under git's name, answers `--version` with.
pub(super) const SHIM_VERSION: &str = "PGG_SHIM_GIT_VERSION";
/// The git that copy passes everything else to.
pub(super) const SHIM_REAL: &str = "PGG_SHIM_REAL_GIT";
/// Set on the app when `--no-lfs` asks for it: the copy answers `git lfs`
/// as a git without Git LFS does, whatever this machine has.
pub(super) const SHIM_NO_LFS: &str = "PGG_SHIM_NO_LFS";
/// Set on the app when `--other-git` asks for one: where a second git
/// stands, staged beside the pictures and **off** PATH.
pub(super) const OTHER_GIT: &str = "PGG_OTHER_GIT";

/// Stands in for git when this binary was copied onto a run's PATH under
/// git's name (`--old-git` / `--no-lfs`); `None` in every other process,
/// the xtask that set it up included. Only `--version` (the old version)
/// and `lfs` (not there) are answered here — the rest goes to the real
/// git, so the app sees a working install that is old or has no Git LFS.
///
/// This binary because it is already built, and script work belongs in
/// xtask (CLAUDE.md §技術スタック).
pub fn git_shim() -> Option<ExitCode> {
    let version = std::env::var(SHIM_VERSION).ok();
    let no_lfs = std::env::var_os(SHIM_NO_LFS).is_some();
    if version.is_none() && !no_lfs {
        return None;
    }
    let real = std::env::var_os(SHIM_REAL)?;
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    if let Some(version) = version
        && args.iter().any(|a| a == "--version")
    {
        println!("git version {version}");
        return Some(ExitCode::SUCCESS);
    }
    // git's own words and code for a subcommand it cannot find.
    if no_lfs && subcommand(&args).is_some_and(|sub| sub == "lfs") {
        eprintln!("git: 'lfs' is not a git command. See 'git --help'.");
        return Some(ExitCode::from(1));
    }
    // Stdio inherited: the app reads this output as git's.
    let status = Command::new(&real).args(&args).status();
    let code = match status {
        // 128 is git's own "fatal".
        Err(e) => {
            eprintln!("fatal: shim could not run {}: {e}", real.to_string_lossy());
            128
        }
        Ok(s) => s.code().unwrap_or(1),
    };
    Some(ExitCode::from(u8::try_from(code).unwrap_or(1)))
}

/// The subcommand git was asked to run: the first word after the global
/// options, two of which take the next word as their value (the app
/// leads every command with `-c <key>=<value>` pairs).
fn subcommand(args: &[OsString]) -> Option<&OsString> {
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        if arg == "-c" || arg == "-C" {
            it.next();
        } else if !arg.to_string_lossy().starts_with('-') {
            return Some(arg);
        }
    }
    None
}

/// The git the shim hands everything else to, found the way the app finds
/// it — the first one on the PATH the run was going to use.
pub(super) fn real_git(path: &std::ffi::OsStr) -> Result<PathBuf, String> {
    let name = if cfg!(windows) { "git.exe" } else { "git" };
    std::env::split_paths(path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| format!("the git shim needs a real git on PATH; no {name} found on it"))
}

/// Stands a second git (the copy `--old-git` uses) beside the pictures,
/// off PATH, and returns where it is. The settings' git chapter grows its
/// button only for a git that answers and is not the one running, and no
/// path names a second install on both a desk and a container.
pub(super) fn stage_other_git(shot_dir: &std::path::Path) -> Result<PathBuf, String> {
    let dir = shot_dir.join("gitother");
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not make {}: {e}", dir.display()))?;
    let me = std::env::current_exe().map_err(|e| format!("could not find this binary: {e}"))?;
    let other = dir.join(if cfg!(windows) { "git.exe" } else { "git" });
    std::fs::copy(&me, &other).map_err(|e| format!("could not write {}: {e}", other.display()))?;
    Ok(other)
}

/// Puts a copy of this binary on the front of `path` under git's name, and
/// returns the PATH the app should run with.
pub(super) fn stage_shim(
    shot_dir: &std::path::Path,
    path: &std::ffi::OsStr,
) -> Result<OsString, String> {
    let dir = shot_dir.join("gitshim");
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not make {}: {e}", dir.display()))?;
    let me = std::env::current_exe().map_err(|e| format!("could not find this binary: {e}"))?;
    let shim = dir.join(if cfg!(windows) { "git.exe" } else { "git" });
    // Keeps the permission bits, so Unix needs no chmod.
    std::fs::copy(&me, &shim).map_err(|e| format!("could not write {}: {e}", shim.display()))?;
    let mut parts = vec![dir];
    parts.extend(std::env::split_paths(path));
    std::env::join_paths(parts).map_err(|e| format!("rebuilding PATH failed: {e}"))
}

/// What the identity verbs type in when nothing else is asked for. Both
/// halves differ from anything the seed below holds, so a mark means
/// the write landed.
const IDENTITY_ASKED: &str = "Ada Lovelace|ada@example.com";

/// The identity handed to every run not about the identity screen: with
/// no `user.*` (every container) the window opens a modal whose two
/// popups are counted by unrelated verbs (`verbs::graph`, `commit-menu`).
/// Unlike what the demo repositories commit as (`demo::repo`), so the
/// settings screen's global and repository values differ.
const MACHINE_IDENTITY: &str =
    "[user]\n\tname = Verify Fixture\n\temail = verify@example.invalid\n";

/// The whole global git configuration a run starts from: the seed the
/// identity verbs are about, or the fixture identity for everyone else.
pub(super) fn global_seed(verb: &str) -> &'static str {
    identity_seed(verb).unwrap_or(MACHINE_IDENTITY)
}

/// The git configuration an identity run starts from, or `None` for every
/// other verb. Neither seed names a name, so the screen asks on its own.
///
/// `identity-half`: a `user.email` with two values refuses a plain set
/// (exit 5, `cannot overwrite multiple values`) after `user.name` went in
/// — the half-landed write a lost config lock leaves, on demand.
pub(super) fn identity_seed(verb: &str) -> Option<&'static str> {
    match verb {
        // `badges` wants the mark, which an empty seed raises without a
        // failed save (the repository keeps its own `user.*`).
        // `quit-save-held` needs the screen up: its submit is the save the
        // close lands on.
        "identity"
        | "badges"
        | "badges-hover"
        | "badges-hover-early"
        | "badges-all"
        | "badges-all-hover"
        | super::child::HELD_SAVE_VERB => Some(""),
        // `identity-tip` closes the dialog on the half-landed save: its
        // badge stands only while the identity is half there.
        "identity-half" | "identity-tip" => {
            Some("[user]\n\temail = personal@example.com\n\temail = second@example.com\n")
        }
        _ => None,
    }
}

/// What the dialog on that seed is told to do: the identity verbs type an
/// identity in; `badges` waves it away, its argument being the window's
/// shape.
pub(super) fn identity_answer<'a>(verb: &str, arg: &'a str) -> &'a str {
    match verb {
        "badges" | "badges-hover" | "badges-hover-early" | "badges-all" | "badges-all-hover" => {
            "skip"
        }
        _ if arg.is_empty() => IDENTITY_ASKED,
        _ => arg,
    }
}

/// Whether the identity the held save wrote is in `config`, read once
/// the app has ended; `None` for every other verb
/// (`super::child::HELD_SAVE_VERB`). Both name and address are looked
/// for, so a half-landed save is false. The one witness outside the app:
/// a process that ended before its save wrote could still have printed
/// every line the run is judged on.
pub(super) fn held_save_landed(verb: &str, config: &Path) -> Option<bool> {
    if verb != super::child::HELD_SAVE_VERB {
        return None;
    }
    let (name, email) = IDENTITY_ASKED.split_once('|')?;
    let text = std::fs::read_to_string(config).unwrap_or_default();
    Some(text.contains(&format!("name = {name}")) && text.contains(&format!("email = {email}")))
}

#[cfg(test)]
mod tests {
    use super::{global_seed, identity_seed};

    /// Every run starts from a seed: one left on the machine's own
    /// configuration would pass or fail by who is sitting at it.
    #[test]
    fn only_the_identity_verbs_start_without_an_identity() {
        for verb in ["commit-menu", "reset-menu", "perf", "wip", "old-git", ""] {
            assert!(identity_seed(verb).is_none(), "{verb} is not one of them");
            let seed = global_seed(verb);
            assert!(seed.contains("name = "), "{verb} starts with no name");
            assert!(seed.contains("email = "), "{verb} starts with no address");
        }
        // The screen those two photograph only stands over a configuration
        // that names nobody, or half of somebody.
        assert_eq!(global_seed("identity"), "");
        assert!(!global_seed("identity-half").contains("name = "));
    }
}
