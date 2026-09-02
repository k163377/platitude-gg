//! Development task runner (`cargo xtask <command>`).
//!
//! Cross-platform by construction (CLAUDE.md: no Windows-only dev tooling):
//! plain Rust + std, with OS differences expressed as code, not as parallel
//! script files.

mod app_env;
mod check;
mod demo;
mod gate;
mod gui;
mod hook;
mod keepsakes;
mod land;
mod linux;
mod perf;
mod qt;
mod seats;
mod shipped;
mod shots;
mod structure;
mod subprocess;
mod tree;
mod usage;
mod verify;
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
        Some("waits") => waits::run(&args[1..]),
        Some("demo-repo") => demo::run(&args[1..]).map(|path| {
            // The path is the output: scripts consume `(cargo xtask ...)`.
            println!("{}", path.display());
        }),
        Some("verify-ui") => verify::run(&args[1..]),
        Some("perf") => perf::run(&args[1..]),
        Some("shipped") => shipped::run(&args[1..]),
        Some("linux") => linux::run(&args[1..]),
        Some("seat") => seats::take(&args[1..]),
        Some("seats") => seats::run(&args[1..]),
        Some("shots") => shots::run(&args[1..]),
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
