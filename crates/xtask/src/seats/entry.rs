//! Which seat a session has already entered, so the seat report stops
//! telling it to enter the tree it is in (EnterWorktree refuses that).
//! The report cannot tell from its working directory — `cd <checkout>
//! && cargo xtask seat` runs it in the checkout while the session works
//! in the seat — so the entry is recorded when it happens
//! (`hook::seat::post_worktree`).

use std::path::{Path, PathBuf};

/// One file per session beside the primary checkout, holding the letter
/// it entered (the `.chips` / `.permits` layout), so every seat's
/// sessions write to one directory.
fn mark_path(root: &Path, session: &str) -> Option<PathBuf> {
    let session: String = session
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    (!session.is_empty()).then(|| root.join(".entered").join(session))
}

pub(crate) fn entered(root: &Path, session: &str, seat: &str) -> bool {
    mark_path(root, session)
        .and_then(|path| std::fs::read_to_string(path).ok())
        .is_some_and(|marked| marked.trim() == seat)
}

/// Advisory: a mark that cannot be written only leaves the report
/// saying how to enter the seat.
pub(crate) fn mark(root: &Path, session: &str, seat: &str) {
    let Some(path) = mark_path(root, session) else {
        return;
    };
    let Some(dir) = path.parent() else {
        return;
    };
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    if let Err(_unheard) = std::fs::write(&path, seat) {
        // Nobody to tell from inside a hook.
    }
}

/// SessionEnd drops the mark only — the seat's claim outlives the
/// conversation (`seats::how_claims_move`).
pub(crate) fn forget(root: &Path, session: &str) {
    let Some(path) = mark_path(root, session) else {
        return;
    };
    if let Err(_unheard) = std::fs::remove_file(&path) {
        // Never written, or already gone.
    }
}

#[cfg(test)]
mod tests {
    use super::{entered, forget, mark, mark_path};

    #[test]
    fn a_seat_is_entered_by_one_session_for_one_letter() {
        let root = crate::verify::claim_dir(&std::env::temp_dir().join("pgg-hook"), "entered")
            .expect("a directory of its own");
        assert!(!entered(&root, "s1", "a"));
        mark(&root, "s1", "a");
        assert!(entered(&root, "s1", "a"));
        assert!(!entered(&root, "s2", "a"));
        assert!(!entered(&root, "s1", "b"));
        // A session that moves to another seat carries no stale mark.
        mark(&root, "s1", "b");
        assert!(!entered(&root, "s1", "a"));
        assert!(entered(&root, "s1", "b"));
        forget(&root, "s1");
        assert!(!entered(&root, "s1", "b"));
        // A session with no identity writes nowhere.
        assert_eq!(mark_path(&root, "  "), None);

        std::fs::remove_dir_all(&root).ok();
    }
}
