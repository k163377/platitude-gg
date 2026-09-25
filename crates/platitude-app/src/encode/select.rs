//! What the panes point at: the hunks and lines a selection covers, and
//! the diff target a working-tree row names.

use platitude_core::details::DiffTarget;
use platitude_core::patch::HunkSelect;

/// One hunk or one line of it, as the diff pane addresses them. A
/// negative line means the whole hunk.
pub fn hunk_selection(hunk: i32, line: i32) -> Vec<HunkSelect> {
    let Ok(hunk) = usize::try_from(hunk) else {
        return Vec::new();
    };
    match usize::try_from(line) {
        Ok(line) => vec![HunkSelect::lines(hunk, [line])],
        Err(_) => vec![HunkSelect::whole(hunk)],
    }
}

/// Rebuilds the diff target a working-tree selection refers to.
/// `kind` is the prefix [`diff_key`] uses (`staged` / `unstaged` /
/// `untracked`); a committed diff is not stageable and yields `None`.
pub fn worktree_target(kind: &str, path: &str, orig_path: &str) -> Option<DiffTarget> {
    let orig = (!orig_path.is_empty()).then(|| orig_path.to_string());
    match kind {
        "staged" => Some(DiffTarget::Staged {
            path: path.to_string(),
            orig_path: orig,
        }),
        "unstaged" => Some(DiffTarget::Unstaged {
            path: path.to_string(),
        }),
        "untracked" => Some(DiffTarget::Untracked {
            path: path.to_string(),
        }),
        _ => None,
    }
}

pub fn diff_key(target: &DiffTarget) -> String {
    match target {
        DiffTarget::Commit { oid, path, .. } => format!("commit:{}:{path}", oid.to_hex()),
        // Both ends, so the key changes when either does — the pane
        // re-reads a file whose comparison moved under it.
        DiffTarget::Range { from, to, path, .. } => {
            format!("range:{}:{}:{path}", from.to_hex(), to.to_hex())
        }
        // Every commit in it, for the same reason.
        DiffTarget::Choice { oids, path, .. } => {
            let held: Vec<String> = oids.iter().map(platitude_core::Oid::to_hex).collect();
            format!("choice:{}:{path}", held.join(","))
        }
        DiffTarget::Staged { path, .. } => format!("staged:{path}"),
        DiffTarget::Unstaged { path } => format!("unstaged:{path}"),
        DiffTarget::Untracked { path } => format!("untracked:{path}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_negative_line_selects_the_whole_hunk() {
        assert_eq!(hunk_selection(3, -1), vec![HunkSelect::whole(3)]);
        assert_eq!(hunk_selection(3, 4), vec![HunkSelect::lines(3, [4])]);
    }

    #[test]
    fn a_row_with_no_hunk_selects_nothing() {
        // Rows outside any hunk (the binary-file note) carry -1.
        assert!(hunk_selection(-1, -1).is_empty());
        assert!(hunk_selection(-1, 2).is_empty());
    }

    #[test]
    fn worktree_targets_exclude_committed_diffs() {
        assert_eq!(
            worktree_target("unstaged", "f.txt", ""),
            Some(DiffTarget::Unstaged {
                path: "f.txt".into()
            })
        );
        assert_eq!(
            worktree_target("staged", "new.txt", "old.txt"),
            Some(DiffTarget::Staged {
                path: "new.txt".into(),
                orig_path: Some("old.txt".into())
            })
        );
        assert_eq!(worktree_target("commit", "f.txt", ""), None);
    }
}
