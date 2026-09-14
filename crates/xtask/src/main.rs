//! Development task runner (`cargo xtask <command>`).
//!
//! Cross-platform by construction (CLAUDE.md: no Windows-only dev tooling):
//! plain Rust + std, with OS differences expressed as code, not as parallel
//! script files.

mod app_env;
mod app_out;
mod budget;
mod check;
mod command;
mod commands;
mod corpus;
mod demo;
mod deny;
mod digest;
mod docs;
mod gate;
mod gui;
mod hook;
mod keepsakes;
mod land;
mod lanes;
mod linux;
mod locks;
mod note;
mod perf;
mod png;
mod qmltest;
mod qt;
mod reap;
mod seats;
mod shipped;
mod shots;
mod still;
mod structure;
mod subprocess;
mod tree;
mod usage;
mod verify;
mod wait;
mod waits;

use std::process::ExitCode;

fn main() -> ExitCode {
    if let Some(code) = verify::git_shim() {
        return code;
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("check") => check::run(&args[1..]),
        Some("gate") => gate::run(&args[1..]),
        Some("structure") => structure::run(&args[1..]),
        Some("docs") => docs::run(&args[1..]),
        Some("waits") => waits::run(&args[1..]),
        Some("qmltest") => qmltest::run(&args[1..]),
        Some("deny") => deny::run(&args[1..]),
        Some("demo-repo") => demo::run(&args[1..]).map(|path| {
            // The path is the output: scripts consume `(cargo xtask ...)`.
            println!("{}", path.display());
        }),
        Some("verify-ui") => verify::run(&args[1..]),
        Some("verbs") => verify::verbs(&args[1..]),
        Some("wedge-check") => verify::wedge_check(&args[1..]),
        Some("corpus") => corpus::run(&args[1..]),
        Some("perf") => perf::run(&args[1..]),
        Some("shipped") => shipped::run(&args[1..]),
        Some("linux") => linux::run(&args[1..]),
        Some("seat") => seats::take(&args[1..]),
        Some("seats") => seats::run(&args[1..]),
        Some("shots") => shots::run(&args[1..]),
        Some("still") => still::run(&args[1..]),
        Some("budget") => budget::run(&args[1..]),
        Some("land") => land::run(&args[1..]),
        Some("kill") => gui::kill(&args[1..]),
        Some("launch") => gui::launch(&args[1..]),
        Some("hook") => hook::run(&args[1..]),
        _ => {
            usage::print();
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

// No helpers here on purpose: the crate root dispatches to every module,
// and a helper on it would tie every module to every other in the gate's
// dependency graph (`tree`, `subprocess`).
