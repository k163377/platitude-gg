//! Cloud sessions — Claude Code on the web — change code and run nothing
//! (internal-docs/クラウドセッション.md). Their container has none of what
//! the checks stand on (Qt, the Linux container, a display), and every
//! session there would build from nothing, so the checks and the landing
//! stay on the desk. A cloud session's hooks answer from [`run`]: what
//! holds a desk session to the seats, the gate and the landing permit is
//! left out, and what keeps the session from running the project is added.

use super::approval::RUN_APPROVAL_FLAG;
use super::payload::{deny, string_field};
use super::shell::{is_cargo, pipe_pieces, program_at};
use super::still::HARMLESS;

/// What Claude Code on the web sets to `true` in the environment of
/// everything a cloud session runs, and never on the desk.
const MARK: &str = "CLAUDE_CODE_REMOTE";

/// What a cloud session is told as it starts (plain stdout is context).
const BRIEF: &str = "This is a cloud session (Claude Code on the web): it changes code and runs \
     nothing. internal-docs/クラウドセッション.md is the rule here — it takes the place of \
     CLAUDE.md ビルド・テスト, the landing half of Git 運用, and every instruction elsewhere to \
     run something to check a change. Work in this checkout, on the branch the cloud gave this \
     session: no seat, no worktree. Run no build, test or verb of the task runner (gate, \
     verify-ui, linux, launch, land, seat) — the pre-shell hook refuses a cargo that compiles, \
     and `cargo fmt` goes through. Everything else stands: the absolute constraints, the code \
     rules, the document caps, and how git is written (Conventional Commits in English, \
     additive pushes, a rebase only when the user asks for one). The work is done when the \
     requested change is committed and pushed to this branch; the report gives the branch and \
     SHA and what the desk still owes — `cargo xtask gate`, the verify-ui verbs a UI change \
     touched, and the generated files left as they stood.";

/// Why a cloud session enters no worktree.
const NO_WORKTREE: &str = "A cloud session works in the checkout it started in, on the branch \
     the cloud gave it (internal-docs/クラウドセッション.md). The seats are the desk's, where \
     sessions share one machine; a worktree here only parts this session's work from the branch \
     it pushes.";

/// What a message asking for main hears in a cloud session, where no
/// landing permit opens.
const NO_LANDING: &str = "The user's message asks for main (反映), and a cloud session does not \
     land: `cargo xtask land` runs on the desk, where it gates the branch first \
     (internal-docs/クラウドセッション.md). Finish the requested work, commit it and push it to \
     this branch, and report the branch and SHA as waiting for the desk — its gate, then the \
     landing from a seat there.";

/// Whether this hook runs for a cloud session.
pub(super) fn session() -> bool {
    std::env::var(MARK).is_ok_and(|value| value == "true")
}

/// The cloud's answer to each event. What the desk answers and the cloud
/// does not — the seats, the task chips, the measurement, launch and kill
/// guards, the gate's git hook, the landing permit — has nothing to hold
/// here.
pub(super) fn run(event: &str, input: &str) -> Result<(), String> {
    match event {
        "session-start" => {
            println!("{BRIEF}");
            Ok(())
        }
        "pre-write" => {
            pre_write(input);
            Ok(())
        }
        "post-write" => super::write::post_write(input),
        "pre-shell" => pre_shell(input),
        "pre-worktree" => {
            deny(NO_WORKTREE);
            Ok(())
        }
        "prompt-submit" => {
            prompt_submit(input);
            Ok(())
        }
        "session-end" => super::session_end(input),
        "post-worktree" | "pre-chip" | "post-chip" | "stop" | "stop-failure" | "pre-tool"
        | "post-tool" | "permission-request" | "permission-denied" | "elicitation"
        | "elicitation-result" | "subagent-start" | "subagent-stop" => Ok(()),
        other => Err(format!("unknown hook event: {other:?}")),
    }
}

/// PreToolUse(Write|Edit): the cloud writes in the checkout it started
/// in, so no seat holds the write; what it writes still is.
fn pre_write(input: &str) {
    let Some(path) = string_field(input, "file_path") else {
        return;
    };
    if let Some(reason) = super::write::second_test_binary(&path.replace('\\', "/")) {
        deny(&reason);
    }
}

/// PreToolUse(Bash|PowerShell). One decision per call, so the first
/// refusal is the answer, as on the desk.
fn pre_shell(input: &str) -> Result<(), String> {
    let _refused = runs_the_project(input)
        || super::git::pre_git_in_the_cloud(input)?
        || super::attribution::pre_comment(input)?
        || super::dump::pre_shell(input)?
        || super::repeat::pre_shell(input)?;
    Ok(())
}

/// UserPromptSubmit: a message asking for main hears where the landing
/// happens instead of opening a permit; the review note answers as on
/// the desk.
fn prompt_submit(input: &str) {
    let Some(prompt) = string_field(input, "prompt") else {
        return;
    };
    if super::permit::is_the_users_own(&prompt) && super::permit::asks_for_main(&prompt) {
        println!("{NO_LANDING}");
    }
    super::review::note_if_asked(input, &prompt);
}

/// The run a cloud session leaves to the desk, refused. Answers whether
/// it refused.
fn runs_the_project(input: &str) -> bool {
    let Some(command) = string_field(input, "command") else {
        return false;
    };
    if !held(&command) {
        return false;
    }
    deny(&format!(
        "This is a cloud session (Claude Code on the web): it changes code and runs nothing — no \
         build, no test, no verb of the task runner (internal-docs/クラウドセッション.md). The \
         checks stand on the desk's machine (Qt, the Linux container, a display), and a build \
         here is paid again by every session. Leave this run to the desk and name it in the \
         report as owed. `cargo fmt` compiles nothing and goes through. If the user asked for \
         this run in so many words, run the same command again with {RUN_APPROVAL_FLAG}=1 in \
         front of it."
    ));
    true
}

/// Whether a line runs cargo for anything that compiles — a build, a
/// test, a run, a verb of the task runner — without the escape. Pure, for
/// the tests.
fn held(command: &str) -> bool {
    !command.contains(RUN_APPROVAL_FLAG) && pipe_pieces(command).into_iter().any(compiles)
}

/// Whether one piece runs cargo for a subcommand past the harmless ones.
/// A global option's value read as the subcommand holds the line: a false
/// hold costs a retyped line, a false pass a build the session was told
/// to leave to the desk.
fn compiles(piece: &str) -> bool {
    let Some((tokens, at)) = program_at(piece) else {
        return false;
    };
    is_cargo(tokens[at])
        && tokens[at + 1..]
            .iter()
            .find(|token| !token.starts_with('+') && !token.starts_with('-'))
            .is_some_and(|subcommand| !HARMLESS.contains(subcommand))
}

#[cfg(test)]
mod tests {
    use super::held;

    #[test]
    fn a_cargo_that_builds_tests_or_runs_a_verb_is_held() {
        for line in [
            "cargo build",
            "cargo test -p platitude-core",
            "cargo check --workspace",
            "cargo clippy --workspace -- -D warnings",
            "cargo xtask gate",
            "cargo xtask docs",
            "cargo run -p xtask -- structure",
            "cd crates && cargo test",
            "cargo +1.99.0 build",
            "env RUSTFLAGS=-Dwarnings cargo test",
            "bash -c \"cargo build\"",
            "cargo fmt --all && cargo test",
        ] {
            assert!(held(line), "{line}");
        }
    }

    #[test]
    fn what_compiles_nothing_and_the_users_escape_go_through() {
        for line in [
            "cargo fmt --all",
            "cargo fmt --all -- --check",
            "cargo +1.99.0 fmt",
            "cargo --version",
            "cargo tree -p xtask",
            "cargo",
            "git status --short",
            "git commit -m \"fix: cargo build on arm\"",
            "grep -rn 'cargo test' internal-docs",
            "PGG_ALLOW_RUN=1 cargo test -p platitude-core",
        ] {
            assert!(!held(line), "{line}");
        }
    }
}
