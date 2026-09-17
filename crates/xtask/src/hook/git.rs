//! The git a shell line may not run unasked: writing main, and rewriting
//! the branch under the session.

use super::commit::primary_commit_denied;
use super::payload::{deny, string_field};
use super::{MAIN_APPROVAL_FLAG, REBASE_APPROVAL_FLAG, permit};
use crate::subprocess::common_git_dir;
use crate::subprocess::git_query;

/// PreToolUse(Bash|PowerShell): the git this repository holds until the
/// user asks for it in so many words (CLAUDE.md Git 運用) — putting a
/// branch on main, rewriting a branch under the session, and committing
/// anything at all from the primary checkout. Answers whether it refused,
/// so the guard after it stays quiet when it did.
pub(super) fn pre_git(input: &str) -> Result<bool, String> {
    let Some(command) = string_field(input, "command") else {
        return Ok(false);
    };
    let cwd = string_field(input, "cwd").unwrap_or_default();
    if gate_control_change_denied(&command) {
        return Ok(true);
    }
    if guarded_git_denied(input, &command, &cwd) {
        return Ok(true);
    }
    // The commit guard's one exception rides the command: the user asked,
    // in so many words, for a commit in the primary checkout.
    if !command.contains(MAIN_APPROVAL_FLAG) && primary_commit_denied(&command, &cwd) {
        return Ok(true);
    }
    Ok(false)
}

/// The two names that decide whether refs/heads/main answers to the gate
/// at all: its manual skip flag, and the mark that tells a session's git from the
/// user's. A session that spells either is stepping around the pre-merge
/// tests, which is the one thing the gate exists to make impossible
/// (internal-docs/反映前テストの機械化.md).
fn gate_control_change_denied(command: &str) -> bool {
    let names = [crate::gate::SKIP, crate::gate::SESSION];
    let Some(name) = names.into_iter().find(|name| command.contains(name)) else {
        return false;
    };
    deny(&format!(
        "{name} decides whether refs/heads/main answers to the gate ({} is the user's own \
         manual test-skip control, {} is how the gate knows a session's git from the user's), so \
         either name is the user's. Run `cargo xtask gate` (or `land`, which gates on the \
         way) and require a passing stamp for main. If validation appears incorrect, stop \
         and report the validation error to the user.",
        crate::gate::SKIP,
        crate::gate::SESSION
    ));
    true
}

/// Putting a branch on main and rewriting the branch under the session
/// are both the user's call (CLAUDE.md Git 運用). The landing verb alone
/// goes through, on the user's permit — whatever a session puts in
/// front of a git line of its own. A rebase carries its own approval
/// flag.
fn guarded_git_denied(input: &str, command: &str, cwd: &str) -> bool {
    let Some(guarded) = guarded_call(command) else {
        return false;
    };
    if guarded.offence == Offence::Landing {
        return permit::landing_denied(input, guarded.what);
    }
    if guarded.offence == Offence::Rebase && command.contains(REBASE_APPROVAL_FLAG) {
        return false;
    }
    let dir = guarded.dir.unwrap_or(cwd);
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
    if guarded.offence.only_from_main()
        && git_query(dir, &["rev-parse", "--abbrev-ref", "HEAD"]).as_deref() != Some("main")
    {
        return false;
    }
    deny(&guarded.offence.reason(guarded.what));
    true
}

/// A git command in a shell line that a rule holds back.
struct GuardedGit<'a> {
    /// The repository it acts on: `git -C <dir>`, else a `cd` that preceded
    /// it, else wherever the session sits.
    dir: Option<&'a str>,
    offence: Offence,
    what: &'static str,
}

/// Which rule the command runs into.
#[derive(Debug, PartialEq)]
enum Offence {
    /// `cargo xtask land`: the one way a branch reaches main, which the
    /// permit answers for.
    Landing,
    /// Writes refs/heads/main with git of the session's own.
    /// `only_from_main` is whether it reaches main only while main is the
    /// checked-out branch — a refspec or a forced update names main from
    /// anywhere.
    WritesMain { only_from_main: bool },
    /// Rewrites the branch it runs on, whichever branch that is.
    Rebase,
}

impl Offence {
    fn only_from_main(&self) -> bool {
        matches!(
            self,
            Offence::WritesMain {
                only_from_main: true
            }
        )
    }

    /// The refusal for what a session may not run at all. The landing
    /// verb's answer is the permit's, and the one here is what a
    /// hand-written main would get in its place.
    fn reason(&self, what: &str) -> String {
        match self {
            Offence::WritesMain { .. } | Offence::Landing => format!(
                "{what} would write refs/heads/main by hand, \
                 whatever stands in front of the line. {} is the one way a branch reaches \
                 main (CLAUDE.md Git 運用): it rebases the branch in its seat, gates it, \
                 fast-forwards main where main actually is (a hand-typed merge inherits \
                 whatever HEAD the primary checkout happens to be on), and runs only on \
                 the permit the user's own message opened by asking for it (反映). If the \
                 user asked for this landing, run `{}`; otherwise leave the work on its \
                 branch and report it as ready to merge.",
                crate::land::LAND.instruction(),
                crate::land::LAND.line()
            ),
            Offence::Rebase => format!(
                "{what} rewrites the branch under the session, and a rebase runs \
                 only when the user asks for it in so many words (CLAUDE.md Git \
                 運用). A branch behind main is a seat's normal resting state \
                 — leave it and report what is on the \
                 branch. A merged seat starts over with `git reset --hard main`, \
                 which is not a rebase and needs nothing. If the user did ask \
                 for this one, run the same command again with \
                 {REBASE_APPROVAL_FLAG}=1 in front of it."
            ),
        }
    }
}

/// The first guarded git invocation in `command`, if any: the verbs that
/// write refs/heads/main, and rebase, which rewrites whichever branch it
/// runs on. Git that names main as a source (`git log main`, `git switch
/// main`) only reads it.
fn guarded_call(command: &str) -> Option<GuardedGit<'_>> {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    // The sanctioned landing verb is a landing: the permit is what says
    // the user asked for this one.
    if xtask_verb(&tokens, "land") {
        return Some(GuardedGit {
            dir: None,
            offence: Offence::Landing,
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
        let writes = |only_from_main| Offence::WritesMain { only_from_main };
        let guarded = match *subcommand {
            // --abort and --quit walk a merge back; they never move the branch on.
            "merge" if !arguments.iter().any(|a| matches!(*a, "--abort" | "--quit")) => {
                Some(("`git merge`", writes(true)))
            }
            // The same two exits walk a rebase back. Every other form moves
            // the rewrite on, --continue and --skip included: a rebase that
            // stopped is one nobody asked to start.
            "rebase" if !arguments.iter().any(|a| matches!(*a, "--abort" | "--quit")) => {
                Some(("`git rebase`", Offence::Rebase))
            }
            "push" | "fetch" | "pull" if arguments.iter().any(|a| writes_main(a)) => {
                Some(("A refspec writing main", writes(false)))
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
                Some(("Forcing the main branch", writes(false)))
            }
            "update-ref" if arguments.iter().any(|a| is_main_ref(a)) => {
                Some(("Updating refs/heads/main", writes(false)))
            }
            _ => None,
        };
        if let Some((what, offence)) = guarded {
            return Some(GuardedGit {
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
/// `cargo run -p xtask -- <verb>`), anywhere in it: the verb is the first
/// positional token after `xtask` when `cargo` stands right before it,
/// or the token after the `--` that ends cargo's own options when `run`
/// does. `cargo test -p xtask land` names a test filter.
/// A quoted mention keeps its quote character on the token and does not
/// match.
pub(super) fn xtask_verb(tokens: &[&str], verb: &str) -> bool {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| **token == "xtask")
        .any(|(at, _)| {
            let before = &tokens[..at];
            let mut after = tokens[at + 1..].iter();
            let first = if before.last() == Some(&"cargo") {
                after.find(|token| !token.starts_with('-'))
            } else if before.contains(&"run") {
                after
                    .by_ref()
                    .find(|token| **token == "--")
                    .and_then(|_| after.next())
            } else {
                None
            };
            first.is_some_and(|token| *token == verb)
        })
}

#[cfg(test)]
mod tests {
    use super::{Offence, guarded_call};

    #[test]
    fn flags_every_verb_that_writes_main() {
        for command in [
            "git merge --ff-only worktree-labels",
            "git push . worktree-labels:main",
            "git fetch . worktree-labels:refs/heads/main",
            "git branch -f main worktree-labels",
            "git update-ref refs/heads/main worktree-labels",
        ] {
            assert!(
                guarded_call(command)
                    .is_some_and(|r| matches!(r.offence, Offence::WritesMain { .. })),
                "{command}"
            );
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
            assert!(guarded_call(command).is_none(), "{command}");
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
                guarded_call(command).map(|r| r.offence),
                Some(Offence::Rebase),
                "{command}"
            );
        }
        for command in ["git rebase --abort", "git rebase --quit"] {
            assert!(guarded_call(command).is_none(), "{command}");
        }
        // The demo repositories rebase on purpose, through the task runner —
        // no `git` token, so nothing here sees them.
        for command in [
            "cargo xtask demo-repo rebase-conflict",
            "cargo xtask verify-ui rebase-stop --preset rebase-conflict",
        ] {
            assert!(guarded_call(command).is_none(), "{command}");
        }
    }

    #[test]
    fn reads_the_directory_the_merge_would_run_in() {
        let from_option = guarded_call("git -c core.pager=cat -C ../.. merge worktree-labels");
        assert_eq!(from_option.map(|r| r.dir), Some(Some("../..")));
        let from_cd = guarded_call("cd \"C:/IdeaProjects/platitude-gg\" && git merge --ff-only x");
        assert_eq!(
            from_cd.map(|r| r.dir),
            Some(Some("C:/IdeaProjects/platitude-gg"))
        );
        let refspec = guarded_call("git push . HEAD:main");
        assert_eq!(
            refspec.map(|r| r.offence),
            Some(Offence::WritesMain {
                only_from_main: false
            })
        );
    }

    #[test]
    fn flags_the_landing_verb_wherever_the_line_reaches_it() {
        for command in [
            "cargo xtask land",
            "cargo xtask land worktree-a",
            "PGG_ALLOW_MAIN=1 cargo xtask land worktree-a",
            "cargo run --quiet -p xtask -- land worktree-a",
            "cargo build -p xtask && cargo xtask land worktree-a",
        ] {
            assert_eq!(
                guarded_call(command).map(|r| r.offence),
                Some(Offence::Landing),
                "{command}"
            );
        }
        for command in [
            "cargo xtask seats",
            "cargo xtask launch",
            "cargo test -p xtask land",
            "cargo test -p xtask -- land",
            "git commit -m \"xtask land notes\"",
        ] {
            assert!(guarded_call(command).is_none(), "{command}");
        }
    }
}
