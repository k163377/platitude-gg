//! What the panes point at: the hunks and lines a selection covers, and
//! the diff target a working-tree row names.

use std::collections::{BTreeMap, BTreeSet};

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

/// Groups hand-picked `(hunk, line)` pairs into one selection per hunk.
///
/// Sorted and de-duplicated on both keys: the patch builder addresses
/// lines by position, so a repeated index would emit the same line twice,
/// and hunks handed over out of order would build a patch git refuses.
/// A pair with a negative index is dropped rather than taken for "the
/// whole hunk" — this way in, unlike [`hunk_selection`], only ever means
/// lines that were pointed at.
pub fn line_selection(pairs: &[(i32, i32)]) -> Vec<HunkSelect> {
    let mut by_hunk: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for &(hunk, line) in pairs {
        let (Ok(hunk), Ok(line)) = (usize::try_from(hunk), usize::try_from(line)) else {
            continue;
        };
        by_hunk.entry(hunk).or_default().insert(line);
    }
    by_hunk
        .into_iter()
        .map(|(hunk, lines)| HunkSelect::lines(hunk, lines))
        .collect()
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
        // Rows outside any hunk (the binary-file note) carry -1, and
        // staging one of those must not fall back to hunk zero.
        assert!(hunk_selection(-1, -1).is_empty());
        assert!(hunk_selection(-1, 2).is_empty());
    }

    #[test]
    fn picked_lines_gather_into_one_selection_per_hunk() {
        let picked = [(1, 5), (0, 3), (1, 2), (0, 1), (1, 5)];
        assert_eq!(
            line_selection(&picked),
            vec![HunkSelect::lines(0, [1, 3]), HunkSelect::lines(1, [2, 5])]
        );
    }

    #[test]
    fn picked_lines_never_stand_for_a_whole_hunk() {
        // `hunk_selection` reads a negative line as "all of it"; this way
        // in must not, or a row that carries -1 would quietly widen a
        // choice made line by line.
        assert!(line_selection(&[(0, -1)]).is_empty());
        assert!(line_selection(&[(-1, 4)]).is_empty());
        assert!(line_selection(&[]).is_empty());
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
