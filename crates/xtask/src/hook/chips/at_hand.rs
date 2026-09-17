//! What this session already has open, and why it stays this session's.
//!
//! A chip is work handed to another session, on another seat, starting
//! from a main that does not have this branch — so a chip over a file
//! this branch is changing is two branches editing one file, and a
//! conflict and a rebase are what comes back for it.
//!
//! The work that kept arriving as a chip is the work the session was
//! finishing: the verification of what it just changed, the cleanup it
//! passed on the way, the fix the change asked for next. All of it names
//! the files the seat already has open, which is exactly what makes this
//! the session that can answer it — the seat is entered, the files are
//! read, and doing it here costs a turn.

use std::collections::BTreeSet;

use crate::subprocess::git_query;

/// Why a chip over `claimed` is this session's own work to finish, if it
/// is. `None` when nothing it names is open here — the chip is then for
/// ground this session is not standing on, and only the live set has
/// anything left to say about it.
pub(super) fn objection(cwd: &str, claimed: &BTreeSet<String>) -> Option<String> {
    if claimed.is_empty() {
        return None;
    }
    let shared = super::guard::shared(claimed, &open_here(cwd));
    (!shared.is_empty()).then(|| reason(&shared))
}

/// The files this session's seat has open: what its working tree has
/// changed, and what its branch is carrying that main has not taken yet.
/// Both, because a chip is worked after this branch lands at the
/// earliest — a file already committed here is still a file that
/// session would start without.
fn open_here(cwd: &str) -> BTreeSet<String> {
    let mut open = working_tree(cwd);
    open.extend(carried(cwd));
    open
}

fn working_tree(cwd: &str) -> BTreeSet<String> {
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

/// The branch's own commits against the main they would land on. Empty
/// in the primary checkout, which is sitting on main and carrying
/// nothing.
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

/// The paths in a `git status --porcelain` listing. Two status letters
/// lead every line, and a rename carries both of its names — a chip
/// naming either of them is over the same work.
///
/// The letters are split off at the first blank: `git_query` trims
/// what it answers, so a first line whose first letter is a blank (` M`,
/// the shape of an unstaged edit) arrives one column short of every
/// other line. Pure so the tests can ask.
fn touched(status: &str) -> BTreeSet<String> {
    status
        .lines()
        .filter_map(|line| line.trim_start().split_once(' '))
        .flat_map(|(_letters, path)| path.trim_start().split(" -> "))
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect()
}

/// What the refusal says. The three ways out are ranked: the first is
/// the answer, the second is where a leftover goes when it is not, and
/// the third is what a chip is for at all.
fn reason(shared: &[String]) -> String {
    format!(
        "This chip claims {}, which this session already has open — the seat's \
         working tree, or the commits its branch is carrying. A chip is worked \
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
        // As `git_query` answers it: trimmed, so the leading blank of a
        // first ` M` line is gone and the rest still have theirs.
        let status = "M crates/xtask/src/hook/chips/guard.rs\n\
                      \x20M crates/xtask/src/hook/chips/ledger.rs\n\
                       ?? crates/xtask/src/hook/chips/at_hand.rs\n\
                       R  internal-docs/古い.md -> internal-docs/新しい.md";
        let found = touched(status);
        assert!(found.contains("crates/xtask/src/hook/chips/guard.rs"));
        assert!(found.contains("crates/xtask/src/hook/chips/ledger.rs"));
        assert!(found.contains("crates/xtask/src/hook/chips/at_hand.rs"));
        // A rename is over both of its names: a chip may know the file
        // by the one this branch moved it off.
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
