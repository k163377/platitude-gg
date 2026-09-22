//! The measurement guard: a cargo a session types while a measurement
//! holds the machine still is refused, because it runs outside any verb
//! that could wait for the hold (`crate::still`).
//!
//! What is let through is the cargo that waits by itself — `cargo xtask
//! <verb>` and its unquieted spelling `cargo run -p xtask -- <verb>` —
//! and the cargo that compiles nothing. Everything else cargo does is
//! held, whatever it is called and whatever it hides behind: a false
//! hold costs one shell line typed again once the hold lifts, a false
//! pass costs a spoiled measurement.

use super::payload::{deny, string_field};
use super::shell::{is_cargo, pipe_pieces, program_at};

/// The cargo subcommands that compile nothing, and so may run beside a
/// measurement.
const HARMLESS: [&str; 8] = [
    "fmt",
    "tree",
    "metadata",
    "help",
    "search",
    "version",
    "locate-project",
    "pkgid",
];

/// PreToolUse(Bash|PowerShell). Answers whether it refused, so the guards
/// after it stay quiet when it did.
pub(super) fn pre_shell(input: &str) -> Result<bool, String> {
    let Some(command) = string_field(input, "command") else {
        return Ok(false);
    };
    if !bare_cargo(&command) {
        return Ok(false);
    }
    let cwd = string_field(input, "cwd").unwrap_or_default();
    let Some(hold) = crate::still::standing(&cwd) else {
        return Ok(false);
    };
    deny(&format!(
        "A measurement is holding the machine still ({hold}): a build beside it is counted as \
         load that was not the application, and the run is refused and taken again — so this \
         cargo line is held the way every `cargo xtask` verb holds itself. Use the verb \
         (`cargo xtask gate` / `check` / `verify-ui` wait for the hold by themselves), or come \
         back once `cargo xtask still` says the hold is gone."
    ));
    Ok(true)
}

/// Whether `command` runs cargo in a way that would compile beside a
/// measurement: cargo in command position of any segment — a pipe's
/// halves included — with a subcommand that is neither the task runner
/// nor one of the harmless few. Judged per segment, so `cargo xtask
/// structure && cargo build` is still a bare build, and only in command
/// position, so a commit message that mentions `cargo test` is not.
fn bare_cargo(command: &str) -> bool {
    pipe_pieces(command).into_iter().any(segment_is_bare_cargo)
}

fn segment_is_bare_cargo(segment: &str) -> bool {
    let Some((tokens, at)) = program_at(segment) else {
        return false;
    };
    if !is_cargo(tokens[at]) {
        return false;
    }
    // Past the toolchain and the global flags: `cargo +stable -q build`.
    let Some(subcommand) = tokens[at + 1..]
        .iter()
        .find(|token| !token.starts_with('+') && !token.starts_with('-'))
    else {
        return false;
    };
    if *subcommand == "xtask" || HARMLESS.contains(subcommand) {
        return false;
    }
    if *subcommand == "run" && runs_the_task_runner(&tokens) {
        return false;
    }
    true
}

/// `cargo run -p xtask -- <verb>`: the task runner, unquieted.
fn runs_the_task_runner(tokens: &[&str]) -> bool {
    tokens
        .windows(2)
        .any(|pair| matches!(pair[0], "-p" | "--package") && pair[1] == "xtask")
        || tokens
            .iter()
            .any(|token| *token == "-pxtask" || *token == "--package=xtask")
}

#[cfg(test)]
mod tests {
    use super::bare_cargo;

    /// The cargo that compiles, in command position, is held whatever
    /// stands between `cargo` and its subcommand, and whatever it runs
    /// behind.
    #[test]
    fn a_cargo_that_compiles_is_held_however_it_is_spelled() {
        for line in [
            "cargo build --release",
            "cd x && cargo test -p platitude-core",
            "PGG_X=1 cargo clippy --workspace -- -D warnings",
            "cargo xtask structure && cargo build",
            "cargo run -p platitude-app",
            "cargo +stable build",
            "cargo -q --locked test",
            "cargo.exe build",
            "C:/Users/x/.cargo/bin/cargo.exe check",
            "cargo nextest run",
            "cargo test -p xtask",
            "cargo build -p xtask --release",
            "cargo xtask gate --host-only\ncargo build",
            "echo x | cargo build --release",
            "env RUSTFLAGS=-Dwarnings cargo test",
            "time cargo build",
            "& cargo build",
            "(cargo build)",
            "bash -c \"cargo build\"",
            "cmd /c cargo build",
        ] {
            assert!(bare_cargo(line), "{line}");
        }
    }

    /// The task runner waits by itself, the harmless subcommands compile
    /// nothing, and a mention of cargo is not a run of it.
    #[test]
    fn the_task_runner_and_the_harmless_are_let_through() {
        for line in [
            "cargo xtask gate --host-only",
            "PGG_ALLOW_GUI=1 cargo xtask perf --at main --repo C:/r",
            "cargo run -p xtask -- structure",
            "cargo run --package xtask -- waits",
            "cargo run --quiet -p xtask --profile hooks -- hook pre-shell",
            "cargo xtask gate 2>&1 | tail -3",
            "cargo fmt --all -- --check",
            "cargo tree -p xtask",
            "cargo --version",
            "cargo",
            "git commit -m \"test(xtask): cargo test flakes under load\"",
            "echo cargo build",
            "echo cargo | grep cargo",
            "grep -rn 'cargo build' docs/",
            "git status",
        ] {
            assert!(!bare_cargo(line), "{line}");
        }
    }
}
