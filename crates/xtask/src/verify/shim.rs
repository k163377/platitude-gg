//! What a run is staged with before the app starts: a git that answers
//! `--version` old, and the identity the screen begins from.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// Set on the app when `--old-git` asks for one: the version a copy of this
/// binary, standing on PATH under git's name, answers `--version` with.
pub(super) const SHIM_VERSION: &str = "PGG_SHIM_GIT_VERSION";
/// The git that copy passes everything else to.
pub(super) const SHIM_REAL: &str = "PGG_SHIM_REAL_GIT";
/// Set on the app when `--other-git` asks for one: where a second git
/// stands, staged beside the pictures and **off** PATH.
pub(super) const OTHER_GIT: &str = "PGG_OTHER_GIT";

/// Stands in for git when this binary was copied onto a run's PATH under
/// git's name (`--old-git`), and returns `None` in every other process —
/// including the xtask that set it up, which never has these two set.
///
/// Only `--version` is answered here; the rest is handed to the real git,
/// so what the app sees is an installation that works and is old.
///
/// This binary: CLAUDE.md rules out `.bat`/`.ps1` dev tooling, and a
/// second binary would have to be built before it could be copied. This
/// one is already built — it is the one running.
pub fn git_shim() -> Option<ExitCode> {
    let version = std::env::var(SHIM_VERSION).ok()?;
    let real = std::env::var_os(SHIM_REAL)?;
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    if args.iter().any(|a| a == "--version") {
        println!("git version {version}");
        return Some(ExitCode::SUCCESS);
    }
    // Straight through, stdio and all: the app reads this child's output as
    // if it were git's, because it is.
    let status = Command::new(&real).args(&args).status();
    let code = match status {
        // 128 is git's own "fatal", which is what a git that could not be
        // reached at all amounts to here.
        Err(e) => {
            eprintln!("fatal: shim could not run {}: {e}", real.to_string_lossy());
            128
        }
        Ok(s) => s.code().unwrap_or(1),
    };
    Some(ExitCode::from(u8::try_from(code).unwrap_or(1)))
}

/// The git the shim hands everything else to, found the way the app finds
/// it — the first one on the PATH the run was going to use.
pub(super) fn real_git(path: &std::ffi::OsStr) -> Result<PathBuf, String> {
    let name = if cfg!(windows) { "git.exe" } else { "git" };
    std::env::split_paths(path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| format!("--old-git needs a real git on PATH; no {name} found on it"))
}

/// Stands a second git beside the pictures and leaves PATH alone,
/// returning where it is.
///
/// **The one thing a run cannot find for itself.** The settings screen's
/// git chapter grows its button only for a git that answers and is not the
/// one running, and no path names a second installation on both a desk and
/// a container. This one is the same copy `--old-git` puts on PATH, put
/// somewhere nothing resolves to — so a run can point the box at
/// it and the app spawns it exactly as it would any other git.
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
pub(super) fn stage_old_git(
    shot_dir: &std::path::Path,
    path: &std::ffi::OsStr,
) -> Result<OsString, String> {
    let dir = shot_dir.join("gitshim");
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not make {}: {e}", dir.display()))?;
    let me = std::env::current_exe().map_err(|e| format!("could not find this binary: {e}"))?;
    let shim = dir.join(if cfg!(windows) { "git.exe" } else { "git" });
    // Copies the permission bits with it, which is what makes the Unix side
    // executable without a chmod of its own.
    std::fs::copy(&me, &shim).map_err(|e| format!("could not write {}: {e}", shim.display()))?;
    let mut parts = vec![dir];
    parts.extend(std::env::split_paths(path));
    std::env::join_paths(parts).map_err(|e| format!("rebuilding PATH failed: {e}"))
}

/// What the identity verbs type in when nothing else is asked for. Both
/// halves differ from anything the seed below holds, so a mark means
/// the write landed.
const IDENTITY_ASKED: &str = "Ada Lovelace|ada@example.com";

/// The identity handed to every run that is not about the identity screen.
///
/// The picture is the same whatever machine it was taken on, and a git
/// with no `user.*` would put this one in it: the window opens a modal
/// asking for one, and that modal and its dimmer are two popups counted
/// by verbs with nothing to do with either (`verbs::graph`,
/// `commit-menu`). Deliberately unlike what the demo repositories commit
/// as (`demo::repo`), so a settings screen holding a global value beside
/// a repository's own still holds two different things.
const MACHINE_IDENTITY: &str =
    "[user]\n\tname = Verify Fixture\n\temail = verify@example.invalid\n";

/// The whole global git configuration a run starts from: the seed the
/// identity verbs are about, or the fixture identity for everyone else.
pub(super) fn global_seed(verb: &str) -> &'static str {
    identity_seed(verb).unwrap_or(MACHINE_IDENTITY)
}

/// The git configuration an identity run starts from, or `None` for every
/// verb that has nothing to do with one.
///
/// `identity-half` is the whole point of the pair. A `user.email` with two
/// values in the file refuses a plain set (measured: exit 5, `cannot
/// overwrite multiple values`) while the `user.name` written just before
/// it goes in — the same half-landed write a configuration lock lost
/// between the two calls leaves behind, and the only version of it that
/// can be produced on demand. Neither seed names a name, so the screen
/// asks for an identity on its own.
pub(super) fn identity_seed(verb: &str) -> Option<&'static str> {
    match verb {
        // The `badges` pair wants the mark: an empty seed is the one
        // state that raises it without a save having to fail first, and
        // the repository keeps its own `user.*` so everything else on
        // the page goes on working.
        // `quit-save-held` needs the screen up for the save it holds:
        // the dialog's own submit is the save the close lands on.
        "identity"
        | "badges"
        | "badges-hover"
        | "badges-hover-early"
        | super::child::HELD_SAVE_VERB => Some(""),
        // `identity-tip` walks the same half-landed save and then closes
        // the dialog on it: the badge the tooltip belongs to only stands
        // while the identity is half of what was asked for.
        "identity-half" | "identity-tip" => {
            Some("[user]\n\temail = personal@example.com\n\temail = second@example.com\n")
        }
        _ => None,
    }
}

/// What the dialog on top of that seed is told to do. The identity verbs
/// are about the write, so they type an identity in; `badges` is about
/// what stands behind the dialog once it has been waved away, and its
/// own argument names the shape of the window.
pub(super) fn identity_answer<'a>(verb: &str, arg: &'a str) -> &'a str {
    match verb {
        "badges" | "badges-hover" | "badges-hover-early" => "skip",
        _ if arg.is_empty() => IDENTITY_ASKED,
        _ => arg,
    }
}

/// Whether the identity the held save wrote is in `config`, read once
/// the app has ended — `None` for every verb with no such witness
/// (`super::child::HELD_SAVE_VERB`). What is looked for is the name and
/// the address the dialog was told to type, so a save that half landed
/// answers false as well. **The one witness outside the app**: a
/// process that ended before its save had written could still have
/// printed every line the run is judged on.
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

    /// Every run starts from a seed. A git with no `user.*` opens a
    /// modal, the modal and its dimmer are popups, and the verbs that
    /// count popups are about menus — so a run left on the machine's own
    /// configuration fails or passes by who is sitting at it.
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
