//! The comment guard: a comment carries the constraint or the trap that
//! holds now, so a note saying who asked for the code and when is held at
//! the commit that would add it (CLAUDE.md Rust 規約).

use super::commit::{commit_dir, common_git_dir};
use super::launch::resolve;
use super::payload::string_field;
use crate::git_query;

/// The words a note reaches for to say who asked, rather than what the
/// code must hold to. Japanese in a .rs or .qml file is already against
/// the language rule, so a bare match is evidence enough — nothing here
/// has to tell a comment from code to be sure of what it found.
const ATTRIBUTIONS: [&str; 6] = [
    "ユーザー判断",
    "ユーザー決定",
    "ユーザー指示",
    "ユーザー報告",
    "ユーザー選択",
    "ユーザー指定",
];

/// The other half of the same habit: a proposal stamped with the day it
/// was made. The year stays out of the needle — a date is a date in every
/// one of them, and a needle carrying this year rots into silence at the
/// turn of it.
const PROPOSAL: &str = "提案";

/// The files the rule covers.
const GUARDED: [&str; 2] = ["*.rs", "*.qml"];

/// How many offending lines the refusal names before it counts the rest.
const LISTED: usize = 5;

/// PreToolUse(Bash|PowerShell): a commit that would add a dated
/// attribution note is held. Only the lines the tree adds are read, so
/// the notes already sitting in the tree are not this gate's business —
/// they are removed by the work that owns them, and until then no commit
/// is blocked by them. Answers whether it refused, so the guard after it
/// stays quiet when it did.
pub(super) fn pre_comment(input: &str) -> Result<bool, String> {
    let Some(command) = string_field(input, "command") else {
        return Ok(false);
    };
    let cwd = string_field(input, "cwd").unwrap_or_default();
    let Some(dir) = commit_dir(&command, &cwd) else {
        return Ok(false);
    };
    // The same bounds as the guards beside it: a throwaway repository
    // measuring git behaviour is nobody's style to police.
    let (Some(session_repo), Some(target_repo)) = (common_git_dir(&cwd), common_git_dir(dir))
    else {
        return Ok(false);
    };
    if !session_repo.eq_ignore_ascii_case(&target_repo) {
        return Ok(false);
    }
    let found = pending(&resolve(&cwd, dir));
    if found.is_empty() {
        return Ok(false);
    }
    let listed = found
        .iter()
        .take(LISTED)
        .cloned()
        .collect::<Vec<_>>()
        .join("; ");
    let rest = found.len().saturating_sub(LISTED);
    let rest = if rest > 0 {
        format!(" and {rest} more line(s)")
    } else {
        String::new()
    };
    println!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
         \"permissionDecision\":\"deny\",\"permissionDecisionReason\":\
         \"This commit would add a dated attribution note: {listed}{rest}. \
         A comment carries the present-tense constraint or the trap, and \
         nothing else: who asked, when they asked and what was decided \
         instead already live in the transcript and in git log, and the \
         note in the source goes stale the moment the code moves \
         (CLAUDE.md Rust 規約). Delete the attribution, keep whatever rule \
         it was hung on, and commit again. Only the lines this tree adds \
         are read here — notes already in the tree are not what stopped \
         this.\"}}}}"
    );
    Ok(true)
}

/// Every line the tree at `root` would add to a guarded file, with the
/// note found in it. Staging on the commit's own line has not run when
/// this hook answers, so the index is not the truth yet: the tree against
/// HEAD is, and the untracked files beside it are new down to their last
/// line.
fn pending(root: &str) -> Vec<String> {
    // The prefixes are spelled out because the reading below keys on them,
    // and diff.noprefix in somebody's config would otherwise leave every
    // hit pathless — including the ones this module is exempt from.
    let mut arguments = vec![
        "diff",
        "--no-ext-diff",
        "--src-prefix=a/",
        "--dst-prefix=b/",
        "-U0",
        "HEAD",
        "--",
    ];
    arguments.extend(GUARDED);
    let mut found = git_query(root, &arguments)
        .map(|diff| added_lines(&diff))
        .unwrap_or_default();
    for path in untracked(root) {
        let Ok(content) = std::fs::read_to_string(format!("{root}/{path}")) else {
            continue;
        };
        found.extend(content.lines().enumerate().filter_map(|(at, line)| {
            attribution_in(line).map(|note| format!("{path}:{} ({note})", at + 1))
        }));
    }
    found
}

/// The untracked guarded files, listed one by one: an untracked directory
/// is one line of `git status` and any number of files. This module drops
/// out here as it does out of the diff — a gate that has not been
/// committed yet is still not evidence against itself.
fn untracked(root: &str) -> Vec<String> {
    let mut arguments = vec!["status", "--porcelain", "--untracked-files=all", "--"];
    arguments.extend(GUARDED);
    git_query(root, &arguments)
        .map(|status| {
            status
                .lines()
                .filter_map(|line| line.strip_prefix("?? ").map(str::to_string))
                .filter(|path| !hunts_itself(path))
                .collect()
        })
        .unwrap_or_default()
}

/// The notes in the added lines of a unified diff, read with the line
/// numbers the hunk headers give them. Pure so the tests can hand it
/// diffs git never produced.
fn added_lines(diff: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut path = String::new();
    let mut number = 0;
    for line in diff.lines() {
        if let Some(name) = line.strip_prefix("+++ b/") {
            path = name.to_string();
            continue;
        }
        if let Some(header) = line.strip_prefix("@@ ") {
            // `@@ -12,0 +13,2 @@`: the half after the plus is where the
            // added lines land in the file the commit would leave behind.
            number = header
                .split_whitespace()
                .find_map(|span| span.strip_prefix('+'))
                .and_then(|span| span.split(',').next())
                .and_then(|start| start.parse::<usize>().ok())
                .unwrap_or(0);
            continue;
        }
        let Some(added) = line.strip_prefix('+') else {
            continue;
        };
        if let Some(note) = attribution_in(added)
            && !hunts_itself(&path)
        {
            found.push(format!("{path}:{number} ({note})"));
        }
        number += 1;
    }
    found
}

/// Whether `path` is this module. It names every word it hunts, so its own
/// lines are never evidence — the gate would otherwise refuse the commit
/// that adds it. `file!()` keeps the exemption on the module however the
/// module is moved or renamed.
fn hunts_itself(path: &str) -> bool {
    path.replace('\\', "/")
        .ends_with(&file!().replace('\\', "/"))
}

/// The note in one line, if it holds one.
fn attribution_in(line: &str) -> Option<&'static str> {
    if let Some(word) = ATTRIBUTIONS.into_iter().find(|word| line.contains(word)) {
        return Some(word);
    }
    line.split(PROPOSAL)
        .skip(1)
        .any(starts_with_date)
        .then_some(PROPOSAL)
}

/// Whether the text opens with a date: four digits and the month behind a
/// dash. What follows the proposal is what says it was stamped with a day
/// rather than written as a sentence.
fn starts_with_date(text: &str) -> bool {
    let mut characters = text.trim_start().chars();
    (0..4).all(|_| characters.next().is_some_and(|c| c.is_ascii_digit()))
        && characters.next() == Some('-')
}

#[cfg(test)]
mod tests {
    use super::{added_lines, attribution_in, hunts_itself};

    #[test]
    fn reads_the_note_out_of_a_line_and_leaves_the_rest_alone() {
        assert_eq!(
            attribution_in("    /// the base's branch name first (ユーザー判断 2026-08-30)"),
            Some("ユーザー判断")
        );
        assert_eq!(
            attribution_in("// the band says what the face cannot (提案 2026-08-30)"),
            Some("提案")
        );
        assert_eq!(
            attribution_in("// **fontMd, not a step down** (2026-08-30 ユーザー指示)."),
            Some("ユーザー指示")
        );
        // A proposal that is a sentence, not a stamp, and code that only
        // looks like a date.
        assert_eq!(attribution_in("// 提案 is a word, not a note"), None);
        assert_eq!(attribution_in("    let stamp = \"2026-08-30\";"), None);
        assert_eq!(
            attribution_in("    // the plan pane keeps the draft while the wait runs"),
            None
        );
    }

    #[test]
    fn numbers_the_added_lines_the_way_the_hunk_headers_do() {
        let diff = "diff --git a/crates/platitude-app/src/ui/RepoPage.qml \
                    b/crates/platitude-app/src/ui/RepoPage.qml\n\
                    --- a/crates/platitude-app/src/ui/RepoPage.qml\n\
                    +++ b/crates/platitude-app/src/ui/RepoPage.qml\n\
                    @@ -90,0 +91,2 @@\n\
                    +    /// **From the press** (ユーザー判断 2026-08-30): the mode's.\n\
                    +    property bool armed: false\n\
                    @@ -500,1 +502,1 @@\n\
                    -    // old\n\
                    +    // the seat is taken at the press (ユーザー指定 2026-08-30)\n";
        assert_eq!(
            added_lines(diff),
            vec![
                "crates/platitude-app/src/ui/RepoPage.qml:91 (ユーザー判断)".to_string(),
                "crates/platitude-app/src/ui/RepoPage.qml:502 (ユーザー指定)".to_string(),
            ]
        );
    }

    #[test]
    fn a_removal_is_not_an_addition() {
        let diff = "--- a/crates/platitude-core/src/rebase_plan.rs\n\
                    +++ b/crates/platitude-core/src/rebase_plan.rs\n\
                    @@ -58,1 +58,1 @@\n\
                    -    /// (ユーザー判断 2026-08-30: onto はブランチ名優先). Empty when.\n\
                    +    /// The branch name comes first, and the id is the fallback.\n\
                    --- a/crates/platitude-app/src/ui/Gone.qml\n\
                    +++ /dev/null\n\
                    @@ -1,2 +0,0 @@\n\
                    -// (ユーザー報告 2026-08-30): the whole file goes.\n\
                    -Item {}\n";
        assert!(added_lines(diff).is_empty(), "{:?}", added_lines(diff));
    }

    #[test]
    fn does_not_stop_the_commit_that_adds_this_gate() {
        assert!(hunts_itself("crates/xtask/src/hook/attribution.rs"));
        assert!(hunts_itself(
            "C:/x/platitude-gg/crates/xtask/src/hook/attribution.rs"
        ));
        assert!(!hunts_itself("crates/platitude-app/src/ui/RepoPage.qml"));
        let diff = "+++ b/crates/xtask/src/hook/attribution.rs\n\
                    @@ -0,0 +1,2 @@\n\
                    +const ATTRIBUTIONS: [&str; 6] = [\"ユーザー判断\", \"ユーザー指示\"];\n\
                    +const PROPOSAL: &str = \"提案\";\n";
        assert!(added_lines(diff).is_empty(), "{:?}", added_lines(diff));
    }
}
