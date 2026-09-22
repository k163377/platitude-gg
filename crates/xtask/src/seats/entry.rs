//! Which seat a session has already entered, as a fact both sides can
//! read.
//!
//! EnterWorktree refuses the tree it is run from, so a session working
//! in its seat that is told to enter it spends a call to be told so.
//! What sends it there is the seat report, and the report cannot see
//! where the session's tools stand: it is printed by a process the
//! session starts, and `cd <checkout> && cargo xtask seat` leaves that
//! process in the checkout while the session works in the seat. So the
//! entry is written down when it happens (`hook::seat::post_worktree`)
//! rather than guessed at from a working directory.

use std::path::{Path, PathBuf};

/// One file per session beside the primary checkout, holding the letter
/// that session entered — the layout `.chips` and `.permits` use, so
/// every seat's sessions write to one directory.
fn mark_path(root: &Path, session: &str) -> Option<PathBuf> {
    let session: String = session
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    (!session.is_empty()).then(|| root.join(".entered").join(session))
}

/// Whether this session has entered this seat.
pub(crate) fn entered(root: &Path, session: &str, seat: &str) -> bool {
    mark_path(root, session)
        .and_then(|path| std::fs::read_to_string(path).ok())
        .is_some_and(|marked| marked.trim() == seat)
}

/// Records that it has. Advisory: a mark that cannot be written leaves
/// the report saying how to enter the seat, which is what it said
/// before this existed.
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

/// SessionEnd: the mark goes with the session that wrote it. The seat
/// does not — a claim outlives the conversation and is handed back by
/// landing, releasing or a takeover (`seats::how_claims_move`).
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
        // Another session's entry is not this one's, and neither is
        // another letter.
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
