//! Development task runner (`cargo xtask <command>`).
//!
//! Cross-platform by construction (CLAUDE.md: no Windows-only dev tooling):
//! plain Rust + std, with OS differences expressed as code, not as parallel
//! script files.

mod demo;
mod hook;
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
        rebase-conflict  a rebase stopped on conflicts, 1 of 2 steps
        rebase-staged    the same rebase, conflict resolved and staged
        rebase-empty     a rebase stopped on a commit that came out empty
        cherry-pick-conflict  a cherry-pick stopped on a conflict
        conflict-kinds   four kinds of conflict in one stopped merge
        stashes   three stashes, one with untracked files
        detached  HEAD detached at a tag
        behind    remote has commits fetch would bring in
        diverged  the same, fetched, with a commit of our own on top
        unpublished  a branch never sent anywhere, two remotes, one taken name
        noremote  commits and no remote at all: the first push writes one down
        signed    ssh-signed commits: verified, unjudgeable, unsigned
        co-authors  one, three and no co-authors, signed and not
        tags      a tag in every state a remote can put it in (fetch first)
        manytags  basic, with more tags than any pane can show at once
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
        --config-dir <d>  settings.toml / state.toml directory. Fresh per
                          run by default; name one to carry what a run
                          wrote into the next one.
        --restore         open the tabs the config directory remembers
                          instead of a named repository

  hook <event>
      Claude Code hook handler (wired from .claude/settings.json; reads
      the hook payload from stdin). Events: pre-write, post-write,
      pre-git, session-start.
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("demo-repo") => demo::run(&args[1..]).map(|path| {
            // The path is the output: scripts consume `(cargo xtask ...)`.
            println!("{}", path.display());
        }),
        Some("verify-ui") => verify::run(&args[1..]),
        Some("hook") => hook::run(&args[1..]),
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
