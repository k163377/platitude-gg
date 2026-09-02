//! The git side of the gate: the hook that holds `refs/heads/main` to a
//! stamp, and the verdict it asks for.
//!
//! `reference-transaction` is the one hook every ref update passes
//! through — merge, fetch with a refspec, branch -f, update-ref, a commit
//! on main, a reset on main — and in its `prepared` state a non-zero exit
//! aborts the update. So one checked-in shell script (`.githooks/`)
//! guards main against every road, and the verdict itself stays here
//! where it can be tested. The script is POSIX sh: git for Windows runs
//! hooks in its own sh, and Linux and macOS have one, so a single file
//! serves all three (CLAUDE.md ビルド・テスト: no .ps1 / .bat).
//!
//! The installed copy lives beside the repository's own `.git`
//! (`pg-gate/hooks/`), where no worktree edits it and no seat's reset or
//! removal can pull it out from under main; `core.hooksPath` is written
//! absolute, to that copy. Beside it, `tree` names the checkout whose
//! task runner answers the verdict: the script `cd`s there before it
//! runs cargo, so the answer never depends on which tree the git command
//! happened to run in — a primary checkout on a main that predates the
//! gate, or a seat mid-edit, would otherwise be the one asked.

use std::path::{Path, PathBuf};

use super::stamp::Store;
use crate::subprocess::git_query;

/// The user's own way past the gate, by name. Sessions may not spell it:
/// the pre-shell hook refuses a command that does.
pub(crate) const SKIP: &str = "PG_GATE_SKIP";

/// Where the hook is checked in, from a checkout's root.
const HOOKS_DIR: &str = ".githooks";
const HOOK: &str = "reference-transaction";

/// Installs the hook: copies the checked-in script beside `.git`, writes
/// the tree the verdict runs in, and points `core.hooksPath` at the copy.
/// Idempotent, and says what it did. `dir` is any tree of the repository;
/// the script is taken from the primary checkout when it carries one and
/// from `dir`'s own tree otherwise (the first landing of the gate), and
/// the tree written is the one the script came from.
pub(crate) fn install(dir: &Path) -> Result<String, String> {
    let here = dir.display().to_string();
    let common = git_query(
        &here,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
    .ok_or_else(|| format!("{here} is not a git repository"))?;
    let listing =
        git_query(&here, &["worktree", "list", "--porcelain"]).ok_or("git worktree list failed")?;
    let primary = crate::seats::worktree_blocks(&listing)
        .into_iter()
        .next()
        .ok_or("git worktree list answered with no trees")?;
    let top = git_query(&here, &["rev-parse", "--show-toplevel"]).unwrap_or_default();
    let source_tree = [primary.path.as_str(), top.as_str()]
        .into_iter()
        .filter(|tree| !tree.is_empty())
        .map(|tree| tree.trim_end_matches('/').to_string())
        .find(|tree| Path::new(tree).join(HOOKS_DIR).join(HOOK).is_file())
        .ok_or_else(|| {
            format!(
                "{HOOKS_DIR}/{HOOK} is in neither the primary checkout nor this tree — the hook \
                 is checked in, and main has to carry it"
            )
        })?;
    let script = std::fs::read_to_string(Path::new(&source_tree).join(HOOKS_DIR).join(HOOK))
        .map_err(|e| format!("{source_tree}/{HOOKS_DIR}/{HOOK}: {e}"))?;
    let hooks = PathBuf::from(&common).join("pg-gate").join("hooks");
    std::fs::create_dir_all(&hooks).map_err(|e| format!("{}: {e}", hooks.display()))?;
    write_executable(&hooks.join(HOOK), &script)?;
    std::fs::write(hooks.join("tree"), format!("{source_tree}\n"))
        .map_err(|e| format!("{}: {e}", hooks.display()))?;
    let hooks_path = hooks.display().to_string().replace('\\', "/");
    let current = git_query(&here, &["config", "--get", "core.hooksPath"]).unwrap_or_default();
    if current == hooks_path {
        return Ok(format!("gate hook: installed (verdict from {source_tree})"));
    }
    // Somebody's own hooks directory is not ours to replace: the two
    // would have to be merged by hand.
    if !current.is_empty() {
        return Err(format!(
            "core.hooksPath is already {current}, which is not the gate's — the gate's hook \
             ({}) has to be reachable from there, or that setting has to go",
            hooks.join(HOOK).display()
        ));
    }
    git_query(&here, &["config", "core.hooksPath", &hooks_path])
        .ok_or("git config core.hooksPath failed")?;
    Ok(format!(
        "gate hook: core.hooksPath = {hooks_path} (verdict from {source_tree})"
    ))
}

/// Writes `text` to `path` and, where the bit exists, makes it
/// executable — git ignores a hook it cannot execute, silently.
fn write_executable(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

/// The hook's question: may refs/heads/main move from `old` to `new`?
/// Yes only for a fast-forward onto a commit gated in full while it sat
/// on main. The refusal reason goes to stderr, which is what git shows
/// the person whose command was stopped.
pub(crate) fn verdict(dir: &Path, old: &str, new: &str) -> Result<(), String> {
    if std::env::var(SKIP).is_ok_and(|v| v == "1") {
        eprintln!("gate: {SKIP}=1 — main moves without a gate (the user's own call)");
        return Ok(());
    }
    // A ref written back to its own value (`git reset --hard HEAD` on
    // main, a checkout) moves nothing and owes nothing — measured: git
    // runs the transaction for it all the same.
    if old == new {
        return Ok(());
    }
    let short: String = new.chars().take(10).collect();
    if new.chars().all(|c| c == '0') {
        return Err("deleting refs/heads/main is not a landing".into());
    }
    let store = Store::open(dir)?;
    let Some(found) = store.commit(new) else {
        return Err(format!(
            "refs/heads/main may not move onto {short}: no gate stamp for it. `cargo xtask gate` \
             on that commit (or `cargo xtask land` for its branch) stamps it once every step the \
             change owes is green on both sides."
        ));
    };
    if !found.full {
        return Err(format!(
            "refs/heads/main may not move onto {short}: its stamp is host-only (a --host-only \
             run). Run the full gate — `cargo xtask gate`, or `cargo xtask land`."
        ));
    }
    if !found.onto_main {
        return Err(format!(
            "refs/heads/main may not move onto {short}: it was gated while its branch sat off main \
             (base {} != main {}). `cargo xtask land` rebases and gates again — the steps whose \
             inputs the rebase does not touch stay cached.",
            found.base.chars().take(10).collect::<String>(),
            found.main.chars().take(10).collect::<String>()
        ));
    }
    let here = dir.display().to_string();
    if !old.chars().all(|c| c == '0')
        && git_query(&here, &["merge-base", "--is-ancestor", old, new]).is_none()
    {
        return Err(format!(
            "refs/heads/main may not move onto {short}: not a fast-forward from {} — main moved \
             since this commit was gated, or this is a rewind. `cargo xtask land` rebases and gates \
             again; a rewind is the user's own ({SKIP}=1).",
            old.chars().take(10).collect::<String>()
        ));
    }
    eprintln!("gate: {short} carries a full, on-main stamp — main may move onto it");
    Ok(())
}
