//! What this session already has open, and why it stays this session's.
//!
//! A chip is worked by another session on another seat, from a main
//! without this branch — so a chip over a file this branch is changing is
//! two branches editing one file, and a conflict and a rebase come back
//! for it.

use std::collections::BTreeSet;

use crate::subprocess::git_query;

/// Why a chip over `claimed` is this session's own work to finish, or
/// `None` when nothing it names is open here (the live set judges it then).
pub(super) fn objection(cwd: &str, claimed: &BTreeSet<String>) -> Option<String> {
    if claimed.is_empty() {
        return None;
    }
    let shared = super::guard::shared(claimed, &open_here(cwd));
    (!shared.is_empty()).then(|| reason(&shared))
}

/// The files this seat has open: its uncommitted changes and what its
/// branch carries that main has not taken — a file committed here is still
/// missing from the main a chip's session starts on.
fn open_here(cwd: &str) -> BTreeSet<String> {
    let mut open = uncommitted_paths(cwd);
    open.extend(carried(cwd));
    open
}

fn uncommitted_paths(cwd: &str) -> BTreeSet<String> {
    touched(
        &git_query(
            cwd,
            &[
                "-c",
                "core.quotepath=false",
                "status",
                "--porcelain",
                "--untracked-files=all",
            ],
        )
        .unwrap_or_default(),
    )
}

/// The files the branch's own commits change against main; empty in the
/// primary checkout.
fn carried(cwd: &str) -> BTreeSet<String> {
    let Some(base) = git_query(cwd, &["merge-base", "main", "HEAD"]) else {
        return BTreeSet::new();
    };
    git_query(
        cwd,
        &[
            "-c",
            "core.quotepath=false",
            "diff",
            "--no-renames",
            "--name-only",
            &base,
            "HEAD",
        ],
    )
    .unwrap_or_default()
    .lines()
    .filter(|line| !line.is_empty())
    .map(str::to_string)
    .collect()
}

/// The paths in a `git status --porcelain` listing, both names of a
/// rename (a chip may name either).
///
/// The letters are split off at the first blank, not by column:
/// `git_query` trims its answer, so a first line of ` M` arrives one
/// column short.
fn touched(status: &str) -> BTreeSet<String> {
    status
        .lines()
        .filter_map(|line| line.trim_start().split_once(' '))
        .flat_map(|(_letters, path)| path.trim_start().split(" -> "))
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect()
}

/// What the refusal says; its ways out are in order of preference.
fn reason(shared: &[String]) -> String {
    format!(
        "This chip claims {}, which this session already has open — the seat's \
         worktree, or the commits its branch is carrying. A chip is worked \
         later, by another session on another seat, starting from a main without \
         this branch in it: over a file this branch is changing, that session and \
         this one edit it from two sides, and the user gets a conflict and a \
         rebase for it. The verification of a change, the cleanup beside it and \
         the fix it asks for next are this session's own work \
         (CLAUDE.md ビルド・テスト). Do it in this turn, \
         in this seat, on this branch. If it must have a session of its own — it \
         needs another machine, a real window, or a decision this one cannot get \
         — land this branch first and stack the chip once the files are on main. \
         If it is neither, it is a leftover to write down: \
         internal-docs/P3-確認事項.md (P5 for what waits on distribution) is the \
         list the user reads back, and it costs no seat.",
        shared.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::{reason, touched};

    #[test]
    fn reads_the_paths_out_of_a_status_listing() {
        // Trimmed, as `git_query` answers it.
        let status = "M crates/xtask/src/hook/chips/guard.rs\n\
                      \x20M crates/xtask/src/hook/chips/ledger.rs\n\
                       ?? crates/xtask/src/hook/chips/at_hand.rs\n\
                       R  internal-docs/古い.md -> internal-docs/新しい.md";
        let found = touched(status);
        assert!(found.contains("crates/xtask/src/hook/chips/guard.rs"));
        assert!(found.contains("crates/xtask/src/hook/chips/ledger.rs"));
        assert!(found.contains("crates/xtask/src/hook/chips/at_hand.rs"));
        assert!(found.contains("internal-docs/古い.md"));
        assert!(found.contains("internal-docs/新しい.md"));
        assert_eq!(found.len(), 5);
        assert!(touched("").is_empty());
    }

    #[test]
    fn the_refusal_names_the_file_and_where_a_leftover_goes_instead() {
        let said = reason(&["crates/xtask/src/hook/chips/guard.rs".to_string()]);
        assert!(said.contains("crates/xtask/src/hook/chips/guard.rs"));
        assert!(said.contains("確認事項"));
    }
}
