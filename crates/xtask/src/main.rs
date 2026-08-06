//! Development task runner (`cargo xtask <command>`).
//!
//! Cross-platform by construction (CLAUDE.md: no Windows-only dev tooling):
//! plain Rust + std, with OS differences expressed as code, not as parallel
//! script files.

mod demo;
mod qt;
mod verify;

use std::process::ExitCode;

const USAGE: &str = "\
cargo xtask <command>

commands:
  demo-repo <preset> [--at <dir>]
      Build a throwaway repository (isolated from your git config) and
      print its path. Presets:
        basic     branches + remote (ahead) + tags + stash + dirty WIP
        dirty     every WIP bucket: staged, unstaged, untracked, renamed
        conflict  a merge stopped on conflicts (MERGE_HEAD present)
        stashes   three stashes, one with untracked files
        detached  HEAD detached at a tag
        behind    remote has commits fetch would bring in
        empty     `git init` and nothing else

  verify-ui <verb> [arg] [options]
      Build the app (release), run it headless (offscreen QPA) against a
      repository, fire PG_AUTO_ACT=<verb> / PG_AUTO_ACT_ARG=<arg>, and
      judge the run by its 'screenshot saved=true' stderr line.
      options:
        --repo <dir>      run against this repository
        --preset <name>   or against a fresh demo repo (default: basic)
        --no-build        reuse the existing release binary
        --select          also set PG_AUTO_SELECT=1
        --quit-ms <n>     PG_AUTO_QUIT_MS (default 10000)
        --shot-dir <dir>  screenshot directory (default: temp, kept)
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("demo-repo") => demo::run(&args[1..]).map(|path| {
            // The path is the output: scripts consume `(cargo xtask ...)`.
            println!("{}", path.display());
        }),
        Some("verify-ui") => verify::run(&args[1..]),
        _ => {
            print!("{USAGE}");
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

/// Runs a command to completion, capturing output; errors carry context.
pub(crate) fn run_captured(
    cmd: &mut std::process::Command,
) -> Result<std::process::Output, String> {
    let display = format!("{cmd:?}");
    cmd.output()
        .map_err(|e| format!("failed to spawn {display}: {e}"))
}
