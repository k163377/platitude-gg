//! Tests of [`crate::conflict`]'s parsers, in a file of their own
//! (structure.md §分割).

use crate::conflict::tool::parse_tool_help;
use crate::conflict::*;
use crate::status::{StatusItem, WorkTreeStatus};

#[test]
fn classifies_stage_letters() {
    assert_eq!(
        ConflictKind::from_stages('U', 'U'),
        ConflictKind::BothModified
    );
    assert_eq!(
        ConflictKind::from_stages('D', 'U'),
        ConflictKind::DeletedByUs
    );
    assert_eq!(ConflictKind::from_stages('X', 'Y'), ConflictKind::Other);
}

/// Real `git mergetool --tool-help` output (2.51.0.windows.1), cut down.
/// The not-available group is indented like the installed one, so its
/// heading is what has to stop the read.
const TOOL_HELP: &str = "\
'git mergetool --tool=<tool>' may be set to one of the following:
\t\tvimdiff          Use Vim with a custom layout (see `git help mergetool`'s `BACKEND SPECIFIC HINTS` section)
\t\tvimdiff1         Use Vim with a 2 panes layout (LOCAL and REMOTE)
\t\tvscode           Use Visual Studio Code (requires a graphical session)

\tuser-defined:
\t\tmytool.cmd true

The following tools are valid, but not currently available:
\t\twinmerge         Use WinMerge (requires a graphical session)

Some of the tools listed above only work in a windowed
environment. If run in a terminal-only session, they will fail.
";

#[test]
fn tool_help_lists_only_what_is_installed_and_windowed() {
    // vimdiff is installed but draws in a terminal, winmerge is
    // windowed but not installed, mytool is read from config instead.
    assert_eq!(parse_tool_help(TOOL_HELP), vec!["vscode".to_string()]);
}

#[test]
fn tool_help_without_a_user_defined_block_still_stops_at_the_next_group() {
    let text = TOOL_HELP.replace("\tuser-defined:\n\t\tmytool.cmd true\n", "");
    assert_eq!(parse_tool_help(&text), vec!["vscode".to_string()]);
}

#[test]
fn tool_help_that_found_nothing_offers_nothing() {
    assert!(parse_tool_help("No suitable tool for 'git mergetool' found.").is_empty());
    assert!(parse_tool_help("").is_empty());
}

#[test]
fn only_content_conflicts_are_worth_a_merge_tool() {
    assert!(ConflictKind::BothModified.is_content_conflict());
    assert!(!ConflictKind::BothDeleted.is_content_conflict());
    assert!(!ConflictKind::DeletedByThem.is_content_conflict());
}

#[test]
fn extracts_conflicted_entries_only() {
    let status = WorkTreeStatus {
        items: vec![
            StatusItem::Unmerged {
                ours: 'U',
                theirs: 'U',
                path: "both.txt".into(),
            },
            StatusItem::Untracked {
                path: "new.txt".into(),
            },
        ],
        ..Default::default()
    };
    let files = conflicted(&status);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "both.txt");
    assert_eq!(files[0].kind, ConflictKind::BothModified);
}
