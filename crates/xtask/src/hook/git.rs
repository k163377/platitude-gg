//! The git a shell line may not run unasked: landing a branch on main,
//! and rewriting the branch under the session.

use super::commit::{common_git_dir, shared_rules_denied};
use super::payload::string_field;
use super::{MAIN_ESCAPE, REBASE_ESCAPE};
use crate::git_query;

/// PreToolUse(Bash|PowerShell): the git this repository holds until the
/// user asks for it in so many words (CLAUDE.md Git 運用) — landing a
/// branch on main, rewriting a branch under the session, and committing
/// the shared session rules from the primary checkout. Answers whether it
/// refused, so the guard after it stays quiet when it did.
pub(super) fn pre_git(input: &str) -> Result<bool, String> {
    let Some(command) = string_field(input, "command") else {
        return Ok(false);
    };
    let cwd = string_field(input, "cwd").unwrap_or_default();
    // Each rule keeps its own escape, so asking for one is not asking for
    // the others: a merge the user called for still may not rebase.
    Ok(guarded_git_denied(&command, &cwd)
        || (!command.contains(MAIN_ESCAPE) && shared_rules_denied(&command, &cwd)))
}

/// Putting a branch onto main and rewriting the branch under the session
/// are both the user's call (CLAUDE.md Git 運用). Prints the refusal and
/// says so.
fn guarded_git_denied(command: &str, cwd: &str) -> bool {
    let Some(reflection) = reflection(command) else {
        return false;
    };
    if command.contains(reflection.offence.escape()) {
        return false;
    }
    let dir = reflection.dir.unwrap_or(cwd);
    // Any git that cannot answer is git we are not guarding: a throwaway
    // repository (CLAUDE.md Rust 規約: measure git in one) is on main as
    // often as not and rebases freely, and the command would fail here
    // anyway if the path is not a repository at all.
    let (Some(session_repo), Some(target_repo)) = (common_git_dir(cwd), common_git_dir(dir)) else {
        return false;
    };
    if !session_repo.eq_ignore_ascii_case(&target_repo) {
        return false;
    }
    if reflection.offence.only_from_main()
        && git_query(dir, &["rev-parse", "--abbrev-ref", "HEAD"]).as_deref() != Some("main")
    {
        return false;
    }
    println!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
         \"permissionDecision\":\"deny\",\"permissionDecisionReason\":\"{}\"}}}}",
        reflection.offence.reason(reflection.what)
    );
    true
}

/// A git command in a shell line that a rule holds back.
struct Reflection<'a> {
    /// The repository it acts on: `git -C <dir>`, else a `cd` that preceded
    /// it, else wherever the session sits.
    dir: Option<&'a str>,
    offence: Offence,
    what: &'static str,
}

/// Which rule the command runs into.
#[derive(Debug, PartialEq)]
enum Offence {
    /// Writes refs/heads/main. `only_from_main` is whether it reaches main
    /// only while main is the checked-out branch — a refspec or a forced
    /// update names main from anywhere.
    LandsOnMain { only_from_main: bool },
    /// Rewrites the branch it runs on, whichever branch that is.
    Rebase,
}

impl Offence {
    /// What a command carries to say this rule's exception was asked for.
    fn escape(&self) -> &'static str {
        match self {
            Offence::LandsOnMain { .. } => MAIN_ESCAPE,
            Offence::Rebase => REBASE_ESCAPE,
        }
    }

    fn only_from_main(&self) -> bool {
        matches!(
            self,
            Offence::LandsOnMain {
                only_from_main: true
            }
        )
    }

    fn reason(&self, what: &str) -> String {
        match self {
            Offence::LandsOnMain { .. } => format!(
                "{what} would put commits on main, and main moves only when the \
                 user asks for it in so many words (CLAUDE.md Git 運用). Leave \
                 the work on its branch and report it as ready to merge instead. \
                 If the user did ask for this one, run \
                 `{MAIN_ESCAPE}=1 cargo xtask land <branch>` — it works from any \
                 session, worktree ones included, and reads where main actually \
                 is before it moves anything; a hand-typed merge inherits \
                 whatever HEAD the primary checkout happens to be on. Should \
                 the permission layer refuse the env-prefixed form, the \
                 PowerShell spelling `$env:{MAIN_ESCAPE}='1'; cargo xtask land \
                 <branch>` says the same thing."
            ),
            Offence::Rebase => format!(
                "{what} rewrites the branch under the session, and a rebase runs \
                 only when the user asks for it in so many words (CLAUDE.md Git \
                 運用). A branch behind main is a seat's normal resting state, \
                 not something to fix — leave it and report what is on the \
                 branch. A merged seat starts over with `git reset --hard main`, \
                 which is not a rebase and needs nothing. If the user did ask \
                 for this one, run the same command again with \
                 {REBASE_ESCAPE}=1 in front of it."
            ),
        }
    }
}

/// The first guarded git invocation in `command`, if any. Read-only git and
/// git that names main as a source (`git log main`, `git switch main`) are
/// not it — the verbs that write refs/heads/main, and rebase, which
/// rewrites whichever branch it runs on.
fn reflection(command: &str) -> Option<Reflection<'_>> {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    // The sanctioned landing verb is still a landing: the escape in front
    // is what says the user asked for this one.
    if xtask_verb(&tokens, "land") {
        return Some(Reflection {
            dir: None,
            offence: Offence::LandsOnMain {
                only_from_main: false,
            },
            what: "`cargo xtask land`",
        });
    }
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
        let mut dir = None;
        index += 1;
        while let Some(option) = tokens.get(index).filter(|token| token.starts_with('-')) {
            if *option == "-C" {
                dir = tokens.get(index + 1).map(|dir| unquote(dir));
            }
            index += if matches!(*option, "-C" | "-c") { 2 } else { 1 };
        }
        let Some(subcommand) = tokens.get(index) else {
            break;
        };
        let arguments: Vec<&str> = tokens[index + 1..]
            .iter()
            .take_while(|token| !matches!(**token, "&&" | "||" | ";" | "|" | "git"))
            .copied()
            .collect();
        let lands = |only_from_main| Offence::LandsOnMain { only_from_main };
        let guarded = match *subcommand {
            // --abort and --quit walk a merge back; they never move the branch on.
            "merge" if !arguments.iter().any(|a| matches!(*a, "--abort" | "--quit")) => {
                Some(("`git merge`", lands(true)))
            }
            // The same two exits walk a rebase back. Every other form moves
            // the rewrite on, --continue and --skip included: a rebase that
            // stopped is one nobody asked to start.
            "rebase" if !arguments.iter().any(|a| matches!(*a, "--abort" | "--quit")) => {
                Some(("`git rebase`", Offence::Rebase))
            }
            "push" | "fetch" | "pull" if arguments.iter().any(|a| writes_main(a)) => {
                Some(("A refspec writing main", lands(false)))
            }
            "pull" if arguments.iter().any(|a| matches!(*a, "--rebase" | "-r")) => {
                Some(("`git pull --rebase`", Offence::Rebase))
            }
            "branch"
                if arguments
                    .iter()
                    .any(|a| matches!(*a, "-f" | "--force" | "-M"))
                    && arguments.iter().any(|a| is_main_ref(a)) =>
            {
                Some(("Forcing the main branch", lands(false)))
            }
            "update-ref" if arguments.iter().any(|a| is_main_ref(a)) => {
                Some(("Updating refs/heads/main", lands(false)))
            }
            _ => None,
        };
        if let Some((what, offence)) = guarded {
            return Some(Reflection {
                dir: dir.or(cd_dir),
                offence,
                what,
            });
        }
    }
    None
}

/// Whether a refspec's destination — the half after the colon — is main.
/// The colon is what separates a write from a read: `git push origin main`
/// sends main somewhere, `git push . x:main` rewrites it here.
fn writes_main(token: &str) -> bool {
    token
        .split_once(':')
        .is_some_and(|(_, destination)| is_main_ref(destination))
}

fn is_main_ref(token: &str) -> bool {
    matches!(token, "main" | "heads/main" | "refs/heads/main")
}

pub(super) fn unquote(token: &str) -> &str {
    token.trim_matches(['"', '\''])
}

/// Whether the line invokes `cargo xtask <verb>` (or the unaliased
/// `cargo run -p xtask -- <verb>`): a bare `xtask` token whose next
/// positional token is the verb. A quoted mention keeps its quote
/// character on the token and does not match.
pub(super) fn xtask_verb(tokens: &[&str], verb: &str) -> bool {
    let mut after = tokens.iter().skip_while(|token| **token != "xtask");
    after.next().is_some()
        && after
            .find(|token| !token.starts_with('-'))
            .is_some_and(|token| *token == verb)
}

#[cfg(test)]
mod tests {
    use super::{Offence, reflection};

    #[test]
    fn flags_every_verb_that_writes_main() {
        for command in [
            "git merge --ff-only worktree-labels",
            "git push . worktree-labels:main",
            "git fetch . worktree-labels:refs/heads/main",
            "git branch -f main worktree-labels",
            "git update-ref refs/heads/main worktree-labels",
        ] {
            assert!(reflection(command).is_some(), "{command}");
        }
    }

    #[test]
    fn leaves_git_that_only_reads_main_alone() {
        for command in [
            "git merge --abort",
            "git merge-base main HEAD",
            "git log main",
            "git switch main",
            "git show HEAD:main",
            "git push origin worktree-labels",
            "git branch main-ish",
            // How a merged seat starts over (CLAUDE.md ビルド・テスト): it
            // writes the seat's own branch, and rewrites no history.
            "git reset --hard main",
        ] {
            assert!(reflection(command).is_none(), "{command}");
        }
    }

    #[test]
    fn flags_a_rebase_however_it_starts_and_lets_its_exits_alone() {
        for command in [
            "git rebase main",
            "git rebase -i HEAD~3",
            "git rebase --onto main HEAD~2",
            "git rebase --continue",
            "git rebase --skip",
            "git pull --rebase",
            "git pull -r origin main",
        ] {
            assert_eq!(
                reflection(command).map(|r| r.offence),
                Some(Offence::Rebase),
                "{command}"
            );
        }
        for command in ["git rebase --abort", "git rebase --quit"] {
            assert!(reflection(command).is_none(), "{command}");
        }
        // The demo repositories rebase on purpose, through the task runner —
        // no `git` token, so nothing here sees them.
        for command in [
            "cargo xtask demo-repo rebase-conflict",
            "cargo xtask verify-ui rebase-stop --preset rebase-conflict",
        ] {
            assert!(reflection(command).is_none(), "{command}");
        }
    }

    #[test]
    fn reads_the_directory_the_merge_would_run_in() {
        let from_option = reflection("git -c core.pager=cat -C ../.. merge worktree-labels");
        assert_eq!(from_option.map(|r| r.dir), Some(Some("../..")));
        let from_cd = reflection("cd \"C:/IdeaProjects/platitude-gg\" && git merge --ff-only x");
        assert_eq!(
            from_cd.map(|r| r.dir),
            Some(Some("C:/IdeaProjects/platitude-gg"))
        );
        let refspec = reflection("git push . HEAD:main");
        assert_eq!(
            refspec.map(|r| r.offence),
            Some(Offence::LandsOnMain {
                only_from_main: false
            })
        );
    }

    #[test]
    fn flags_the_landing_verb_and_reads_its_raw_form_too() {
        for command in [
            "cargo xtask land",
            "cargo xtask land worktree-a",
            "cargo run --quiet -p xtask -- land worktree-a",
        ] {
            let landing = reflection(command);
            assert!(
                landing.as_ref().is_some_and(|r| r.what.contains("land")),
                "{command}"
            );
        }
        for command in [
            "cargo xtask seats",
            "cargo xtask launch",
            "git commit -m \"xtask land notes\"",
        ] {
            assert!(reflection(command).is_none(), "{command}");
        }
    }
}
