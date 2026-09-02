//! The commit guard: the primary checkout is where this repository is
//! read, and every commit of it rides a worktree branch.

use super::MAIN_ESCAPE;
use super::git::unquote;
use super::launch::resolve;
use crate::seats::worktree_root;
use crate::subprocess::git_query;

/// The primary checkout commits nothing of its own — implementation,
/// documents, settings and the shared session rules alike ride worktree
/// branches, because parallel sessions keep reaching for the same files
/// and direct commits to main collide with theirs (CLAUDE.md Git 運用).
/// What the commit carries no longer narrows this: where it would land is
/// the whole question.
pub(super) fn primary_commit_denied(command: &str, cwd: &str) -> bool {
    let Some(dir) = commit_dir(command, cwd) else {
        return false;
    };
    // The same bounds as the merge guard: only this repository answers,
    // and a worktree branch is exactly where the work belongs.
    let (Some(session_repo), Some(target_repo)) = (common_git_dir(cwd), common_git_dir(dir)) else {
        return false;
    };
    if !session_repo.eq_ignore_ascii_case(&target_repo) {
        return false;
    }
    if worktree_root(&resolve(cwd, dir)).is_some() {
        return false;
    }
    println!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
         \"permissionDecision\":\"deny\",\"permissionDecisionReason\":\
         \"This would commit in the primary checkout, and the primary \
         checkout is only ever read: implementation, documents, settings \
         and the shared session rules all ride worktree branches, because \
         parallel sessions keep reaching for the same files and direct \
         commits to main collide with theirs (CLAUDE.md Git 運用). Take a \
         seat — `claude --worktree <letter>`, or EnterWorktree by path — \
         redo the edit there, and report the branch as ready to merge. If \
         the user asked for this direct commit in so many words, run the \
         same command again with {}=1 in front of it.\"}}}}",
        MAIN_ESCAPE
    );
    true
}

/// The repository the first `git commit` in `command` would run in:
/// `git -C <dir>`, else a `cd` that preceded it, else wherever the session
/// sits. None when the line commits nothing — `git add X && git commit`
/// stages X only after this hook has answered, so the line is the only
/// place a coming commit shows in time.
pub(super) fn commit_dir<'a>(command: &'a str, cwd: &'a str) -> Option<&'a str> {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    let mut cd_dir = None;
    let mut index = 0;
    while index < tokens.len() {
        if tokens[index] == "cd" {
            cd_dir = tokens.get(index + 1).map(|dir| unquote(dir));
            index += 2;
            continue;
        }
        if tokens[index] != "git" {
            index += 1;
            continue;
        }
        // git's own options come before the subcommand; -C and -c take a
        // separate value, so stepping one token at a time would read that
        // value as the subcommand.
        let mut git_dir = None;
        index += 1;
        while let Some(option) = tokens.get(index).filter(|token| token.starts_with('-')) {
            if *option == "-C" {
                git_dir = tokens.get(index + 1).map(|dir| unquote(dir));
            }
            index += if matches!(*option, "-C" | "-c") { 2 } else { 1 };
        }
        match tokens.get(index) {
            None => break,
            // Everything past the verb belongs to the commit, message
            // included: reading no further is what keeps a `cd` quoted
            // inside a message from being read as a directory change.
            Some(&"commit") => return Some(git_dir.or(cd_dir).unwrap_or(cwd)),
            Some(_) => index += 1,
        }
    }
    None
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
    use super::commit_dir;

    const CWD: &str = "C:/x/platitude-gg";

    #[test]
    fn finds_the_commit_however_the_line_reaches_it() {
        for command in [
            "git commit -m \"x\"",
            "git commit -am \"x\"",
            "git add . && git commit -m \"x\"",
            "git add \".claude/skills/a b/SKILL.md\" && git commit -m \"x\"",
            "git rm -r .claude/rules && git commit -m \"x\"",
            "git add it's-notes.md && git commit -m x",
        ] {
            assert_eq!(commit_dir(command, CWD), Some(CWD), "{command}");
        }
        assert_eq!(
            commit_dir("git add .claude/rules/core.md", CWD),
            None,
            "a line with no commit commits nothing"
        );
    }

    #[test]
    fn holds_the_files_the_narrow_guard_used_to_wave_through() {
        // Settings and plain documents rode past the old shared-rules
        // reading; the primary checkout writes none of them now.
        for command in [
            "git commit .claude/settings.json -m \"x\"",
            "git add CLAUDE.md && git commit -m \"x\"",
            "git commit internal-docs/notes.md -m \"x\"",
        ] {
            assert_eq!(commit_dir(command, CWD), Some(CWD), "{command}");
        }
    }

    #[test]
    fn reads_the_directory_the_commit_would_run_in() {
        assert_eq!(
            commit_dir("git -C ../.. commit -m \"x\"", CWD),
            Some("../..")
        );
        assert_eq!(
            commit_dir("cd C:/x/platitude-gg && git commit -am \"x\"", "C:/other"),
            Some("C:/x/platitude-gg")
        );
        // A -C on a staging verb is that verb's, not the commit's.
        assert_eq!(
            commit_dir("git -C ../.. add . && git commit -m \"x\"", CWD),
            Some(CWD)
        );
    }

    #[test]
    fn reads_a_message_as_text_and_not_as_machinery() {
        for command in [
            "git commit -m \"docs: cd into the seat before committing\"",
            "git commit --message=\"see .claude/skills for verbs\" CLAUDE.md",
            "git commit -m \"$(cat <<'EOF'\ndocs: cd C:/elsewhere notes\nEOF\n)\"",
        ] {
            assert_eq!(commit_dir(command, CWD), Some(CWD), "{command}");
        }
    }
}
