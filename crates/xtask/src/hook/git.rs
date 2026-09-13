//! The git a shell line may not run unasked: landing a branch on main,
//! and rewriting the branch under the session.

use super::commit::primary_commit_denied;
use super::payload::string_field;
use super::{MAIN_APPROVAL_FLAG, REBASE_APPROVAL_FLAG, permit};
use crate::subprocess::common_git_dir;
use crate::subprocess::git_query;

/// What the git guard made of a shell line.
#[derive(Debug, PartialEq)]
pub(super) enum Verdict {
    /// Printed a refusal; the guards after it stay quiet.
    Refused,
    /// A command that writes main, let through on the user's permit —
    /// which the caller spends once every other guard has let it pass.
    Landing,
    /// Nothing here to hold back.
    Clear,
}

/// PreToolUse(Bash|PowerShell): the git this repository holds until the
/// user asks for it in so many words (CLAUDE.md Git 運用) — landing a
/// branch on main, rewriting a branch under the session, and committing
/// anything at all from the primary checkout. Answers whether it refused,
/// so the guard after it stays quiet when it did, and whether it let a
/// landing through.
pub(super) fn pre_git(input: &str) -> Result<Verdict, String> {
    let Some(command) = string_field(input, "command") else {
        return Ok(Verdict::Clear);
    };
    let cwd = string_field(input, "cwd").unwrap_or_default();
    if gate_control_change_denied(&command) {
        return Ok(Verdict::Refused);
    }
    // Each operation has its own approval flag, so asking for one is not asking for
    // the others: a merge the user called for still may not rebase.
    match guarded_git(input, &command, &cwd) {
        Verdict::Clear => {}
        verdict => return Ok(verdict),
    }
    if !command.contains(MAIN_APPROVAL_FLAG) && primary_commit_denied(&command, &cwd) {
        return Ok(Verdict::Refused);
    }
    Ok(Verdict::Clear)
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
    println!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
         \"permissionDecision\":\"deny\",\"permissionDecisionReason\":\
         \"{name} decides whether refs/heads/main answers to the gate ({} is the user's own \
         manual test-skip control, {} is how the gate knows a session's git from the user's), so a \
         session may spell neither. Run `cargo xtask gate` (or `land`, which gates on the \
         way) and require a passing stamp for main. If validation appears incorrect, stop \
         and report the validation error to the user.\"}}}}",
        crate::gate::SKIP,
        crate::gate::SESSION
    );
    true
}

/// Putting a branch onto main and rewriting the branch under the session
/// are both the user's call (CLAUDE.md Git 運用). Prints the refusal and
/// says so. A landing with an approval flag is checked against the permit:
/// the flag declares that the user asked; the permit independently checks
/// whether the user did, in the user's own message.
fn guarded_git(input: &str, command: &str, cwd: &str) -> Verdict {
    let Some(guarded) = guarded_call(command) else {
        return Verdict::Clear;
    };
    let approval_declared = command.contains(guarded.offence.approval_flag());
    if approval_declared && guarded.offence == Offence::Rebase {
        return Verdict::Clear;
    }
    let dir = guarded.dir.unwrap_or(cwd);
    // Any git that cannot answer is git we are not guarding: a throwaway
    // repository (CLAUDE.md Rust 規約: measure git in one) is on main as
    // often as not and rebases freely, and the command would fail here
    // anyway if the path is not a repository at all.
    let (Some(session_repo), Some(target_repo)) = (common_git_dir(cwd), common_git_dir(dir)) else {
        return Verdict::Clear;
    };
    if !session_repo.eq_ignore_ascii_case(&target_repo) {
        return Verdict::Clear;
    }
    if guarded.offence.only_from_main()
        && git_query(dir, &["rev-parse", "--abbrev-ref", "HEAD"]).as_deref() != Some("main")
    {
        return Verdict::Clear;
    }
    if approval_declared {
        return if permit::landing_denied(input, guarded.what) {
            Verdict::Refused
        } else {
            Verdict::Landing
        };
    }
    println!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
         \"permissionDecision\":\"deny\",\"permissionDecisionReason\":\"{}\"}}}}",
        guarded.offence.reason(guarded.what)
    );
    Verdict::Refused
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
    /// Writes refs/heads/main. `only_from_main` is whether it reaches main
    /// only while main is the checked-out branch — a refspec or a forced
    /// update names main from anywhere.
    LandsOnMain { only_from_main: bool },
    /// Rewrites the branch it runs on, whichever branch that is.
    Rebase,
}

impl Offence {
    /// What a command carries to say this rule's exception was asked for.
    fn approval_flag(&self) -> &'static str {
        match self {
            Offence::LandsOnMain { .. } => MAIN_APPROVAL_FLAG,
            Offence::Rebase => REBASE_APPROVAL_FLAG,
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
                 If the user did ask for this one — in their latest message, \
                 which is where the hook reads the ask (反映) — run \
                 `{MAIN_APPROVAL_FLAG}=1 cargo xtask land <branch>` — it works from any \
                 session, worktree ones included, and reads where main actually \
                 is before it moves anything; a hand-typed merge inherits \
                 whatever HEAD the primary checkout happens to be on. For \
                 PowerShell, the equivalent environment-variable syntax is \
                 `$env:{MAIN_APPROVAL_FLAG}='1'; cargo xtask land \
                 <branch>`. This syntax does not grant approval or change execution permissions."
            ),
            Offence::Rebase => format!(
                "{what} rewrites the branch under the session, and a rebase runs \
                 only when the user asks for it in so many words (CLAUDE.md Git \
                 運用). A branch behind main is a seat's normal resting state, \
                 not something to fix — leave it and report what is on the \
                 branch. A merged seat starts over with `git reset --hard main`, \
                 which is not a rebase and needs nothing. If the user did ask \
                 for this one, run the same command again with \
                 {REBASE_APPROVAL_FLAG}=1 in front of it."
            ),
        }
    }
}

/// The first guarded git invocation in `command`, if any. Read-only git and
/// git that names main as a source (`git log main`, `git switch main`) are
/// not it — the verbs that write refs/heads/main, and rebase, which
/// rewrites whichever branch it runs on.
fn guarded_call(command: &str) -> Option<GuardedGit<'_>> {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    // The sanctioned landing verb is still a landing: the approval flag
    // is what says the user asked for this one.
    if xtask_verb(&tokens, "land") {
        return Some(GuardedGit {
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
            assert!(guarded_call(command).is_some(), "{command}");
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
            let landing = guarded_call(command);
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
            assert!(guarded_call(command).is_none(), "{command}");
        }
    }
}
