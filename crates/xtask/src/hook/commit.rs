//! The commit guard: the shared session rules ride worktree branches, so a
//! direct commit of them from the primary checkout is held.

use super::MAIN_ESCAPE;
use super::git::unquote;
use super::launch::resolve;
use crate::git_query;
use crate::seats::worktree_root;

/// The primary checkout may commit documents directly, except the files
/// every session loads: .claude/skills and .claude/rules ride worktree
/// branches (CLAUDE.md Git 運用). A commit is held only when it
/// demonstrably carries them — named on the line, already staged, or
/// swept in by broad staging while they sit changed.
pub(super) fn shared_rules_denied(command: &str, cwd: &str) -> bool {
    let Some(commit) = commit(command) else {
        return false;
    };
    let dir = commit.dir.unwrap_or(cwd);
    // The same bounds as the merge guard: only this repository answers,
    // and a worktree branch is exactly where these edits belong.
    let (Some(session_repo), Some(target_repo)) = (common_git_dir(cwd), common_git_dir(dir)) else {
        return false;
    };
    if !session_repo.eq_ignore_ascii_case(&target_repo) {
        return false;
    }
    if worktree_root(&resolve(cwd, dir)).is_some() {
        return false;
    }
    let carries =
        commit.named || staged_shared_rules(dir) || (commit.broad && changed_shared_rules(dir));
    if !carries {
        return false;
    }
    println!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
         \"permissionDecision\":\"deny\",\"permissionDecisionReason\":\
         \"This commit would carry .claude/skills, .claude/rules or \
         .claude/rules-refs onto main from the primary checkout, and those \
         files ride worktree branches: parallel sessions keep reaching for \
         them, and direct commits to main collide (CLAUDE.md Git 運用). \
         Make the edit on a worktree branch and report the branch as ready \
         to merge. If the user asked for this direct commit in so many \
         words, run the same command again with {}=1 in front of it.\"}}}}",
        MAIN_ESCAPE
    );
    true
}

/// A `git commit` found in a shell line, with what the whole line stages
/// around it.
struct Commit<'a> {
    /// The repository it acts on, read the way Reflection reads it.
    dir: Option<&'a str>,
    /// Whether a pathspec of a staging or commit verb names the shared
    /// rules.
    named: bool,
    /// Whether staging is broad (`-a`, `add -A`, `add .`), sweeping in
    /// whatever sits changed without naming it.
    broad: bool,
}

/// The first `git commit` in `command`, if any, folding in every staging
/// verb on the line: `git add X && git commit` stages X only after this
/// hook has answered, so the line is the only place X shows in time.
fn commit(command: &str) -> Option<Commit<'_>> {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    let mut cd_dir = None;
    let mut dir = None;
    let mut seen = false;
    let mut named = false;
    let mut broad = false;
    let mut index = 0;
    while index < tokens.len() {
        if tokens[index] == "cd" {
            cd_dir = tokens.get(index + 1).map(|d| unquote(d));
            index += 2;
            continue;
        }
        if tokens[index] != "git" {
            index += 1;
            continue;
        }
        let mut git_dir = None;
        index += 1;
        while let Some(option) = tokens.get(index).filter(|token| token.starts_with('-')) {
            if *option == "-C" {
                git_dir = tokens.get(index + 1).map(|d| unquote(d));
            }
            index += if matches!(*option, "-C" | "-c") { 2 } else { 1 };
        }
        let Some(subcommand) = tokens.get(index).copied() else {
            break;
        };
        index += 1;
        if !matches!(subcommand, "commit" | "add" | "stage" | "mv" | "rm") {
            continue;
        }
        while let Some(token) = tokens.get(index).copied() {
            if token == "git" {
                break;
            }
            if matches!(token, "&&" | "||" | ";" | "|") {
                index += 1;
                break;
            }
            let span = quoted_span(&tokens, index);
            if token.starts_with('-') {
                broad |= stages_broadly(subcommand, token);
                index = if span == index + 1 && takes_value(subcommand, token) {
                    quoted_span(&tokens, index + 1)
                } else {
                    span
                };
                continue;
            }
            let path = unquote(token).replace('\\', "/");
            named |= names_shared_rules(&path);
            let path = path.trim_end_matches('/');
            broad |= matches!(path, "." | ":/" | ".claude") || path.ends_with("/.claude");
            index = span;
        }
        if subcommand == "commit" && !seen {
            seen = true;
            dir = git_dir.or(cd_dir);
        }
    }
    seen.then_some(Commit { dir, named, broad })
}

/// Index past the token at `at`, extended to the closing quote when the
/// token opens one it does not close: a quoted value is one word to the
/// shell however many words this whitespace split made of it.
fn quoted_span(tokens: &[&str], at: usize) -> usize {
    let Some(first) = tokens.get(at) else {
        return at;
    };
    let Some(open) = first.find(['"', '\'']) else {
        return at + 1;
    };
    let quote = char::from(first.as_bytes()[open]);
    if first[open + 1..].contains(quote) {
        return at + 1;
    }
    let mut end = at + 1;
    while let Some(token) = tokens.get(end) {
        end += 1;
        if token.contains(quote) {
            break;
        }
    }
    end
}

/// Whether `option` makes `subcommand` stage broadly — sweeping in
/// whatever sits changed without naming it.
fn stages_broadly(subcommand: &str, option: &str) -> bool {
    let staging = matches!(subcommand, "add" | "stage");
    match option {
        "--all" => staging || subcommand == "commit",
        "--update" => staging,
        _ => option.strip_prefix('-').is_some_and(|cluster| {
            !cluster.starts_with('-')
                && ((subcommand == "commit" && cluster.contains('a'))
                    || (staging && cluster.contains(['A', 'u'])))
        }),
    }
}

/// Whether a commit option takes the next token as its value. Only commit
/// is read this closely — its message is where pathspec-looking words
/// live; the staging verbs take no values worth skipping.
fn takes_value(subcommand: &str, option: &str) -> bool {
    subcommand == "commit"
        && (matches!(
            option,
            "--message"
                | "--file"
                | "--author"
                | "--date"
                | "--template"
                | "--cleanup"
                | "--fixup"
                | "--squash"
                | "--trailer"
                | "--reuse-message"
                | "--reedit-message"
                | "--pathspec-from-file"
        ) || option.strip_prefix('-').is_some_and(|cluster| {
            !cluster.starts_with('-') && cluster.ends_with(['m', 'F', 'C', 'c', 't'])
        }))
}

/// Whether `text` — a pathspec or a line of git status output — names
/// .claude/skills, .claude/rules or .claude/rules-refs as a path segment.
/// .claude/settings.json stays directly committable; only the files every
/// session loads or greps as rules ride worktree branches.
fn names_shared_rules(text: &str) -> bool {
    let text = text.replace('\\', "/");
    [".claude/skills", ".claude/rules-refs", ".claude/rules"]
        .iter()
        .any(|shared| {
            text.match_indices(*shared).any(|(at, _)| {
                let before = text[..at].chars().next_back();
                let after = text[at + shared.len()..].chars().next();
                before.is_none_or(|c| matches!(c, '/' | '"' | '\'' | ' '))
                    && after.is_none_or(|c| matches!(c, '/' | '"' | '\'' | ' '))
            })
        })
}

/// Whether the index already carries the shared rules — staged by an
/// earlier tool call, with nothing left on this line to name them.
fn staged_shared_rules(dir: &str) -> bool {
    git_query(dir, &["diff", "--cached", "--name-only"])
        .is_some_and(|paths| paths.lines().any(names_shared_rules))
}

/// Whether they sit changed at all — what broad staging would sweep into
/// the commit, untracked files included.
fn changed_shared_rules(dir: &str) -> bool {
    git_query(dir, &["status", "--porcelain"])
        .is_some_and(|status| status.lines().any(names_shared_rules))
}

/// The repository `dir` belongs to, shared by all of its worktrees, so that
/// a merge run from a worktree is recognised as the same repository as the
/// session that runs it.
pub(super) fn common_git_dir(dir: &str) -> Option<String> {
    git_query(
        dir,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
}

#[cfg(test)]
mod tests {
    use super::{commit, names_shared_rules};

    #[test]
    fn holds_a_commit_that_names_the_shared_rules() {
        let staged = commit("git add .claude/rules/core.md && git commit -m \"x\"").unwrap();
        assert!(staged.named);
        let direct = commit("git commit .claude/skills/verify-ui/SKILL.md -m \"x\"").unwrap();
        assert!(direct.named);
        let quoted =
            commit("git add \".claude/skills/a b/SKILL.md\" && git commit -m \"x\"").unwrap();
        assert!(quoted.named);
        let removed = commit("git rm -r .claude/rules && git commit -m \"x\"").unwrap();
        assert!(removed.named);
        assert!(
            commit("git add .claude/rules/core.md").is_none(),
            "a line with no commit stages nothing to hold"
        );
    }

    #[test]
    fn reads_the_message_as_one_value_not_as_pathspecs() {
        let mention = commit("git commit -m \"docs: note .claude/rules/core.md moved\"").unwrap();
        assert!(!mention.named);
        let glued =
            commit("git commit --message=\"see .claude/skills for verbs\" CLAUDE.md").unwrap();
        assert!(!glued.named);
        let heredoc =
            commit("git commit -m \"$(cat <<'EOF'\ndocs: .claude/rules/core.md notes\nEOF\n)\"")
                .unwrap();
        assert!(!heredoc.named);
    }

    #[test]
    fn sees_broad_staging_for_what_it_would_sweep() {
        assert!(commit("git commit -am \"x\"").unwrap().broad);
        assert!(commit("git add -A && git commit -m \"x\"").unwrap().broad);
        assert!(commit("git add . && git commit -m \"x\"").unwrap().broad);
        assert!(
            commit("git add .claude && git commit -m \"x\"")
                .unwrap()
                .broad
        );
        let narrow = commit("git add CLAUDE.md && git commit -m \"x\"").unwrap();
        assert!(!narrow.broad && !narrow.named);
    }

    #[test]
    fn reads_the_directory_the_commit_would_run_in() {
        let by_option = commit("git -C ../.. commit -m \"x\"").unwrap();
        assert_eq!(by_option.dir, Some("../.."));
        let by_cd = commit("cd C:/x/platitude-gg && git commit -am \"x\"").unwrap();
        assert_eq!(by_cd.dir, Some("C:/x/platitude-gg"));
    }

    #[test]
    fn knows_the_shared_directories_by_their_segments() {
        assert!(names_shared_rules(".claude/rules/core.md"));
        assert!(names_shared_rules(
            "C:/x/platitude-gg/.claude/skills/verify-ui/SKILL.md"
        ));
        assert!(names_shared_rules(".claude\\rules\\core.md"));
        assert!(names_shared_rules(".claude/skills"));
        assert!(names_shared_rules(".claude/rules-refs/app-ui.md"));
        assert!(!names_shared_rules(".claude/settings.json"));
        assert!(!names_shared_rules("docs/.claude/rules-of-thumb.md"));
        assert!(!names_shared_rules("internal-docs/skills.md"));
    }
}
