//! What a run is staged with before the app starts: a git that answers
//! `--version` old or a second git off PATH, and the identity the screen
//! begins from.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub use super::git_shim::git_shim;
pub(super) use super::git_shim::{
    OTHER_GIT, SHIM_NO_LFS, SHIM_REAL, SHIM_SLOW, SHIM_VERSION, Slow,
};

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
