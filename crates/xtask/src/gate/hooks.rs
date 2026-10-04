//! The git side of the gate: the hook that holds `refs/heads/main` to a
//! stamp, and the verdict it asks for (反映前テストの機械化.md §git hook).
//!
//! `reference-transaction` is the one hook every ref update passes
//! through, and a non-zero exit in its `prepared` state aborts the update,
//! so one checked-in script (`.githooks/`) guards main against every road a
//! session can take and none of the user's ([`SESSION`]); the verdict stays
//! here, where it can be tested. Passing through every update, it is also
//! where whose a branch is gets written down ([`creator`]), for that
//! session's landing to clear (`land::leftovers`).
//!
//! The installed copy lives beside the repository's own `.git`
//! (`pgg-gate/hooks/`), where no seat's edit, reset or removal reaches it.
//! Beside it, `tree` names the checkout whose task runner answers, so the
//! verdict never depends on the tree the git command ran in (a seat
//! mid-edit, say).

use std::path::{Path, PathBuf};

use super::stamp::Store;
use crate::subprocess::git_query;

/// The user's own way past the gate, by name. The pre-shell hook
/// refuses a session's command that spells it.
pub(crate) const SKIP: &str = "PGG_GATE_SKIP";

/// The mark Claude Code leaves in the environment of everything it runs,
/// and so in every git a session starts: it tells a session's ref update
/// from the user's. The pre-shell hook refuses a session's command that
/// spells it, as it does one spelling [`SKIP`].
pub(crate) const SESSION: &str = "CLAUDECODE";

const HOOKS_DIR: &str = ".githooks";
const HOOK: &str = "reference-transaction";

/// Where the hook writes down whose a branch is ([`creator`]): beside the
/// installed copy, which the script spells `$(dirname "$0")/../made`.
const MADE: &str = "made";

/// The session whose git created `reference` (`refs/heads/…`), as the hook
/// wrote it down: None for a branch no session's git created — the user's
/// own, the app's — or one created before the hook kept the record.
pub(crate) fn creator(common: &Path, reference: &str) -> Option<String> {
    let path = common.join("pgg-gate").join(MADE).join(reference);
    let id = std::fs::read_to_string(path).ok()?.trim().to_string();
    (!id.is_empty()).then_some(id)
}

/// Installs the hook beside `.git` and points `core.hooksPath` at it;
/// idempotent, and says what it did. `dir` is any tree of the repository.
/// The script, and the tree the verdict runs in, are the primary
/// checkout's when it carries one and `dir`'s own otherwise (the first
/// landing of the gate).
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
    let hooks = PathBuf::from(&common).join("pgg-gate").join("hooks");
    std::fs::create_dir_all(&hooks).map_err(|e| format!("{}: {e}", hooks.display()))?;
    write_executable(&hooks.join(HOOK), &script)?;
    std::fs::write(hooks.join("tree"), format!("{source_tree}\n"))
        .map_err(|e| format!("{}: {e}", hooks.display()))?;
    let hooks_path = hooks.display().to_string().replace('\\', "/");
    let current = git_query(&here, &["config", "--get", "core.hooksPath"]).unwrap_or_default();
    if current == hooks_path {
        return Ok(format!("gate hook: installed (verdict from {source_tree})"));
    }
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

/// Makes the file executable where the bit exists — git silently ignores
/// a hook it cannot execute.
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
/// on main. The refusal reason is what git shows the person whose command
/// was stopped.
pub(crate) fn verdict(dir: &Path, old: &str, new: &str) -> Result<(), String> {
    // A git without `SESSION` is the user's own — their terminal, their
    // IDE, a window they are clicking in — and their main is theirs to
    // move, rewinds included. The script asks this before it needs cargo;
    // here it is asked again for the hand-run `gate verdict` and the tests.
    if std::env::var_os(SESSION).is_none_or(|mark| mark.is_empty()) {
        return Ok(());
    }
    if std::env::var(SKIP).is_ok_and(|v| v == "1") {
        eprintln!("gate: {SKIP}=1 — main moves without a gate (the user's own call)");
        return Ok(());
    }
    // git runs the transaction for a ref written back to its own value
    // too (`git reset --hard HEAD` on main); that moves nothing.
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

#[cfg(test)]
mod tests {
    /// The record the script writes is the one `creator` reads.
    #[test]
    fn the_hook_script_writes_creators_where_they_are_read() {
        let script = include_str!("../../../../.githooks/reference-transaction");
        let spelled = format!("made=\"$(dirname \"$0\")/../{}\"", super::MADE);
        assert!(script.contains(&spelled), "{spelled}");
        // The installed copy stands in `pgg-gate/hooks`, so `..` is the
        // `pgg-gate` that `creator` joins.
        let common = crate::yard::Yard::new("hooks-creator");
        let made = common.join("pgg-gate").join(super::MADE);
        std::fs::create_dir_all(made.join("refs/heads/keep")).expect("the record's directory");
        std::fs::write(made.join("refs/heads/keep/copy"), "session-1\n").expect("a creator");
        assert_eq!(
            super::creator(&common, "refs/heads/keep/copy").as_deref(),
            Some("session-1")
        );
        assert_eq!(super::creator(&common, "refs/heads/nobodys"), None);
    }

    /// The script reads the mark itself, spelled as `SESSION` does, ahead of
    /// its cargo guard.
    #[test]
    fn the_hook_script_reads_the_same_session_mark() {
        let script = include_str!("../../../../.githooks/reference-transaction");
        let mark = script
            .find(super::SESSION)
            .unwrap_or_else(|| panic!("the hook script has to read {} itself", super::SESSION));
        let cargo = script.find("command -v cargo").expect("the cargo guard");
        assert!(
            mark < cargo,
            "a git without {} has to be past the hook before cargo is asked for — an IDE's \
             has no cargo on PATH, and the guard below fails closed",
            super::SESSION
        );
    }
}
