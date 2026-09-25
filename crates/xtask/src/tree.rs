//! The tree this runner serves. Not in main.rs: .claude/rules/structure.md §分割「クレート root」.

use std::path::{Path, PathBuf};

/// The workspace root, fixed at compile time: a worktree builds its own
/// task runner, so this is that worktree's root.
pub(crate) fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap_or(Path::new("."))
        .to_path_buf()
}

/// The primary checkout the tree at `cwd` belongs to — read off the
/// filesystem, not asked of git, because the readers are hooks that run
/// on every line a session types.
///
/// A checkout and its seats must give one answer, or what a session wrote
/// before entering a seat is left behind (`hook::repeat`, `seats::entry`).
pub(crate) fn primary_root(cwd: &str) -> Option<PathBuf> {
    let mut at: &Path = Path::new(cwd);
    loop {
        let dot_git = at.join(".git");
        if dot_git.is_dir() {
            return Some(at.to_path_buf());
        }
        if dot_git.is_file() {
            return linked_primary(&dot_git);
        }
        at = at.parent()?;
    }
}

/// The primary checkout a linked worktree's `.git` file names; a gitdir
/// not under `<primary>/.git/worktrees/` (`--separate-git-dir`) has none.
fn linked_primary(dot_git: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(dot_git).ok()?;
    let named = text
        .lines()
        .find_map(|line| line.trim().strip_prefix("gitdir:"))?;
    let root = Path::new(named.trim()).ancestors().nth(3)?.to_path_buf();
    root.join(".git").is_dir().then_some(root)
}

#[cfg(test)]
mod tests {
    use super::primary_root;

    #[test]
    fn a_seat_and_its_checkout_name_one_root() {
        let base = crate::verify::claim_dir(&std::env::temp_dir().join("pgg-hook"), "roots")
            .expect("a directory of its own");
        let primary = base.join("platitude-gg");
        let seat = primary.join(".claude/worktrees/a");
        std::fs::create_dir_all(primary.join(".git/worktrees/a")).expect("a primary checkout");
        std::fs::create_dir_all(seat.join("crates/xtask")).expect("a seat");
        std::fs::write(
            seat.join(".git"),
            format!("gitdir: {}/.git/worktrees/a\n", primary.display()),
        )
        .expect("a seat's .git file");

        let named = |at: &std::path::Path| primary_root(&at.to_string_lossy());
        assert_eq!(named(&primary).as_deref(), Some(primary.as_path()));
        assert_eq!(named(&seat).as_deref(), Some(primary.as_path()));
        // From a subdirectory that carries no .git of its own.
        assert_eq!(
            named(&seat.join("crates/xtask")).as_deref(),
            Some(primary.as_path())
        );
        // A .git naming a layout this reader does not know.
        std::fs::write(seat.join(".git"), "gitdir: /elsewhere\n").expect("a foreign .git file");
        assert_eq!(named(&seat), None);

        std::fs::remove_dir_all(&base).ok();
    }
}
