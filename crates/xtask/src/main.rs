//! Development task runner (`cargo xtask <command>`).
//!
//! Cross-platform by construction (CLAUDE.md: no Windows-only dev tooling):
//! plain Rust + std, with OS differences expressed as code, not as parallel
//! script files.

mod app_env;
mod check;
mod demo;
mod gui;
mod hook;
mod keepsakes;
mod land;
mod linux;
mod perf;
mod qt;
mod seats;
mod shots;
mod structure;
mod usage;
mod verify;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    if let Some(code) = verify::git_shim() {
        return code;
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("check") => check::run(&args[1..]),
        Some("structure") => structure::run(&args[1..]),
        Some("demo-repo") => demo::run(&args[1..]).map(|path| {
            // The path is the output: scripts consume `(cargo xtask ...)`.
            println!("{}", path.display());
        }),
        Some("verify-ui") => verify::run(&args[1..]),
        Some("perf") => perf::run(&args[1..]),
        Some("linux") => linux::run(&args[1..]),
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

/// The workspace root, resolved at compile time from this crate's location.
/// A worktree builds its own task runner, so this is that worktree's root.
pub(crate) fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap_or(Path::new("."))
        .to_path_buf()
}

/// Builds the app in release unless `build` says not to, and answers
/// where its binary sits either way — `--no-build` still needs the path.
///
/// `extra` follows `build --release`: the package and features a caller
/// needs, and the `--features` value names itself in the building line.
/// `path` is the PATH the *build* runs with, which is not always the one
/// the run itself gets — verify-ui stages a git shim onto its child's
/// PATH, and building against that would build against the shim.
pub(crate) fn app_exe(
    root: &Path,
    path: &std::ffi::OsStr,
    build: bool,
    extra: &[&str],
) -> Result<PathBuf, String> {
    if build {
        let features = extra
            .windows(2)
            .find(|pair| pair[0] == "--features")
            .map(|pair| format!(", {}", pair[1]))
            .unwrap_or_default();
        println!("building (release{features})…");
        let status = std::process::Command::new("cargo")
            .args(["build", "--release"])
            .args(extra)
            .current_dir(root)
            .env("PATH", path)
            .status()
            .map_err(|e| format!("failed to run cargo: {e}"))?;
        if !status.success() {
            return Err("cargo build --release failed".into());
        }
    }
    let exe = root.join("target").join("release").join(if cfg!(windows) {
        "platitude-gg.exe"
    } else {
        "platitude-gg"
    });
    if !exe.is_file() {
        return Err(format!(
            "{} not found — build first (or drop --no-build)",
            exe.display()
        ));
    }
    Ok(exe)
}

/// Runs a command to completion, capturing output; errors carry context.
pub(crate) fn run_captured(
    cmd: &mut std::process::Command,
) -> Result<std::process::Output, String> {
    let display = format!("{cmd:?}");
    cmd.output()
        .map_err(|e| format!("failed to spawn {display}: {e}"))
}

/// Runs git in `dir`, answering its trimmed stdout with backslashes
/// forward, and None when it fails at all — callers treat a repository
/// that cannot answer as one they are not judging.
pub(crate) fn git_query(dir: &str, arguments: &[&str]) -> Option<String> {
    if dir.is_empty() {
        return None;
    }
    let mut command = std::process::Command::new("git");
    command.arg("-C").arg(dir).args(arguments);
    let output = run_captured(&mut command).ok()?;
    output.status.success().then(|| {
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .replace('\\', "/")
    })
}
