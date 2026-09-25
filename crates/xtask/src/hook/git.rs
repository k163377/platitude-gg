//! The git a shell line may not run unasked: writing main, and rewriting
//! the branch under the session.

use super::commit::primary_commit_denied;
use super::payload::{deny, string_field};
use super::{MAIN_APPROVAL_FLAG, REBASE_APPROVAL_FLAG, permit};
use crate::subprocess::common_git_dir;
use crate::subprocess::git_query;

/// PreToolUse(Bash|PowerShell): the git held until the user asks for it in
/// so many words (CLAUDE.md Git 運用), and any commit from the primary
/// checkout. Answers whether it refused, so the guard after it stays quiet
/// when it did.
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
    // The user's own ask for a commit in the primary checkout rides the command.
    if !command.contains(MAIN_APPROVAL_FLAG) && primary_commit_denied(&command, &cwd) {
        return Ok(true);
    }
    Ok(false)
}

/// The gate's skip flag and session mark decide whether refs/heads/main
/// answers to the gate at all; a session spelling either is stepping
/// around the pre-merge tests.
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

/// The landing verb goes through on the user's permit alone, whatever a
/// session puts in front of it; a rebase carries its own approval flag.
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
    if guarded.offence == Offence::TouchesASeat {
        if !names_a_seat(cwd, guarded.dir, guarded.target.as_ref()) {
            return false;
        }
        deny(&guarded.offence.reason(guarded.what));
        return true;
    }
    let dir = guarded.dir.unwrap_or(cwd);
    // Only this repository: a throwaway one (.claude/rules/code.md:
    // measure git in one) is on main as often as not and rebases freely.
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
    // A repository with no roster has no letters: a throwaway may carry a
    // `worktree-c` of its own.
    if guarded.offence == Offence::DeletesASeatsBranch && !keeps_seats(dir) {
        return false;
    }
    deny(&guarded.offence.reason(guarded.what));
    true
}

/// Whether the repository `dir` sits in keeps a seat roster at all.
fn keeps_seats(dir: &str) -> bool {
    crate::seats::primary_checkout(dir).is_ok_and(|(primary, _)| {
        std::path::Path::new(&crate::seats::roster_dir(&primary)).is_dir()
    })
}

/// Whether the tree a worktree verb names is a seat of *this* roster;
/// corpus copies, topical trees and throwaways go through even when laid
/// out the same way.
///
/// The base (`-C <dir>` / a `cd`) is resolved too: left relative, it
/// leaves the target relative, which names no seat and lets the line
/// through.
fn names_a_seat(cwd: &str, dir: Option<&str>, target: Option<&String>) -> bool {
    let Some(target) = target else {
        return false;
    };
    let resolve = super::launch::resolve;
    let base = dir.map_or_else(|| cwd.to_string(), |dir| resolve(cwd, unquote(dir)));
    let Some((repository, _)) = crate::seats::seat_in_repository(&resolve(&base, unquote(target)))
    else {
        return false;
    };
    crate::seats::primary_checkout(cwd)
        .is_ok_and(|(primary, _)| crate::seats::same_tree(&primary, &repository))
}

/// The path a worktree verb names: its first argument that is not an
/// option, stepping over the values of `-b`, `-B` and `--reason`
/// (`--orphan` takes none). A value or a path the shell split on the
/// spaces inside its quotes is put back together.
fn worktree_path(arguments: &[&str]) -> Option<String> {
    let mut rest = arguments.iter().skip(1).copied();
    while let Some(argument) = rest.next() {
        if matches!(argument, "-b" | "-B" | "--reason") {
            if let Some(value) = rest.next() {
                quoted(value, &mut rest);
            }
        } else if !argument.starts_with('-') {
            return Some(quoted(argument, &mut rest));
        }
    }
    None
}

/// One argument, with the tokens the shell split out of its quotes put
/// back: everything up to the token that closes the quote this one
/// opened.
fn quoted<'a>(first: &'a str, rest: &mut impl Iterator<Item = &'a str>) -> String {
    let mut whole = first.to_string();
    let Some(quote) = first.chars().next().filter(|c| matches!(c, '"' | '\'')) else {
        return whole;
    };
    if first.len() > 1 && first.ends_with(quote) {
        return whole;
    }
    for token in rest.by_ref() {
        whole.push(' ');
        whole.push_str(token);
        if token.ends_with(quote) {
            break;
        }
    }
    whole
}

/// Whether a `git branch` deletion names a roster letter's branch.
fn deletes_a_seat_branch(arguments: &[&str]) -> bool {
    arguments
        .iter()
        .any(|a| matches!(*a, "-d" | "-D" | "--delete"))
        && arguments.iter().any(|a| {
            a.strip_prefix("worktree-")
                .is_some_and(|letter| crate::seats::SEATS.contains(&letter))
        })
}

/// A git command in a shell line that a rule holds back.
struct GuardedGit<'a> {
    /// The repository it acts on: `git -C <dir>`, else a `cd` that preceded
    /// it, else wherever the session sits.
    dir: Option<&'a str>,
    offence: Offence,
    what: &'static str,
    /// The tree the command names, for the offence that is about a tree
    /// rather than a branch.
    target: Option<String>,
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
    /// Acts on a roster seat's tree by hand, which only the seat verbs do.
    TouchesASeat,
    /// Deletes a roster letter's branch.
    DeletesASeatsBranch,
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

    /// The refusal. The landing verb's real answer is the permit's; the
    /// one here is a hand-written main's.
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
            Offence::TouchesASeat => format!(
                "{what} would act on a roster seat by hand. A seat's tree and its claim are \
                 the roster's to make, lock and lift: `{}` hands a letter out, `{}` hands it \
                 back, `{}` frees it when the branch reaches main, and `{}` moves it on the \
                 user's word — a letter whose tree went away included, which it grows back \
                 on its own branch. A tree taken away by hand strands its branch (that is how \
                 two letters sat unreachable for days), and a claim written or lifted by hand \
                 moves a letter without the user's word (CLAUDE.md ビルド・テスト).",
                crate::seats::commands::TAKE.line(),
                crate::seats::commands::RELEASE.line(),
                crate::land::LAND.line(),
                crate::seats::commands::TAKEOVER.line()
            ),
            Offence::DeletesASeatsBranch => format!(
                "{what} would delete a roster letter's branch. Its commits are what the roster \
                 refuses to hand the letter out over, and what a takeover grows the tree back \
                 on. To drop the work, take the letter over (`{}`) and put the branch at \
                 main's tip in its own tree (`git switch -C worktree-<letter> main`) — the \
                 commits stay in the reflog, and the letter is at main for the next session \
                 (CLAUDE.md ビルド・テスト).",
                crate::seats::commands::TAKEOVER.line()
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

/// The first guarded git invocation in `command`, if any. Git that names
/// main as a source (`git log main`, `git switch main`) only reads it.
fn guarded_call(command: &str) -> Option<GuardedGit<'_>> {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    if xtask_verb(&tokens, "land") {
        return Some(GuardedGit {
            dir: None,
            offence: Offence::Landing,
            what: "`cargo xtask land`",
            target: None,
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
        // -C and -c take a separate value, which one-token steps would read
        // as the subcommand.
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
            "branch" if deletes_a_seat_branch(&arguments) => {
                Some(("`git branch -D`", Offence::DeletesASeatsBranch))
            }
            "update-ref" if arguments.iter().any(|a| is_main_ref(a)) => {
                Some(("Updating refs/heads/main", writes(false)))
            }
            "worktree" => match arguments.first().copied() {
                Some("remove") => Some(("`git worktree remove`", Offence::TouchesASeat)),
                Some("add") => Some(("`git worktree add`", Offence::TouchesASeat)),
                Some("move") => Some(("`git worktree move`", Offence::TouchesASeat)),
                Some("lock") => Some(("`git worktree lock`", Offence::TouchesASeat)),
                Some("unlock") => Some(("`git worktree unlock`", Offence::TouchesASeat)),
                _ => None,
            },
            _ => None,
        };
        if let Some((what, offence)) = guarded {
            return Some(GuardedGit {
                dir: dir.or(cd_dir),
                offence,
                what,
                target: worktree_path(&arguments),
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
/// `cargo run -p xtask -- <verb>`), anywhere in it. `cargo test -p xtask
/// land` names a test filter, and a quoted mention keeps its quote on the
/// token; neither matches.
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

    /// Only the path is read here (`names_a_seat` judges it). An option's
    /// value or a split quoted reason read as the path lets the line
    /// through quietly.
    #[test]
    fn reads_the_tree_a_worktree_verb_names_whichever_side_the_options_are_on() {
        for command in [
            "git worktree remove .claude/worktrees/c",
            "git worktree remove --force .claude/worktrees/c",
            "git worktree remove .claude/worktrees/c --force",
            "git -C .. worktree remove .claude/worktrees/c",
            "git worktree add .claude/worktrees/c worktree-c",
            "git worktree add -b worktree-c .claude/worktrees/c main",
            "git worktree add --lock --reason \"claude-seat x\" -B worktree-c .claude/worktrees/c main",
            // --orphan takes no value of its own.
            "git worktree add --orphan .claude/worktrees/c",
            "git worktree lock --reason \"claude-seat x pid 1\" .claude/worktrees/c",
            "git worktree unlock .claude/worktrees/c",
            "git worktree move .claude/worktrees/c C:/elsewhere",
        ] {
            let guarded = guarded_call(command).expect(command);
            assert_eq!(guarded.offence, Offence::TouchesASeat, "{command}");
            assert_eq!(
                guarded.target.as_deref(),
                Some(".claude/worktrees/c"),
                "{command}"
            );
        }
        // A path the shell split on the spaces inside its quotes comes
        // back whole.
        let guarded = guarded_call("git worktree remove \"C:/x y/.claude/worktrees/c\"")
            .expect("a quoted path");
        assert_eq!(
            guarded.target.as_deref(),
            Some("\"C:/x y/.claude/worktrees/c\"")
        );
        // Reading and pruning touch no tree a session is in.
        for command in ["git worktree list --porcelain", "git worktree prune"] {
            assert!(guarded_call(command).is_none(), "{command}");
        }
    }

    /// A letter's branch is deleted by nobody; every other branch is the
    /// session's business.
    #[test]
    fn flags_the_deletion_of_a_roster_letters_branch() {
        for command in [
            "git branch -D worktree-b",
            "git branch -d worktree-f",
            "git branch --delete worktree-c",
        ] {
            let guarded = guarded_call(command).expect(command);
            assert_eq!(guarded.offence, Offence::DeletesASeatsBranch, "{command}");
        }
        for command in [
            "git branch -D panel-wip",
            "git branch -D worktree-tooltip",
            "git branch worktree-b",
            "git branch -f worktree-b HEAD~1",
        ] {
            assert!(guarded_call(command).is_none(), "{command}");
        }
    }

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
            // How a merged seat starts over: it writes the seat's own
            // branch, and rewrites no history.
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
        // The demo repositories rebase through the task runner — no `git` token.
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
