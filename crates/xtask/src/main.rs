//! Development task runner (`cargo xtask <command>`).
//!
//! Cross-platform by construction (CLAUDE.md: no Windows-only dev tooling):
//! plain Rust + std, with OS differences expressed as code, not as parallel
//! script files.

mod check;
mod demo;
mod hook;
mod linux;
mod qt;
mod verify;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
cargo xtask <command>

commands:
  check [--verb <v>]...
      Stage-2 verification (CLAUDE.md 確認は 3 段), with the host and the
      container running in parallel: fmt, clippy and the workspace tests
      here, while the container runs test -p platitude-core, verify-ui
      for each --verb, and bare. The two sides write to different build
      trees (target/ vs the docker volume), so the wall clock is
      whichever side finishes last. Each --verb also runs verify-ui on
      the host, so one flag covers the verb on both OSes; its value is
      a whole verify-ui argument line, quoted when the verb needs its
      preset or argument beside it (--verb 'co-authors 4 --preset
      co-authors'). Without --verb the summary says the touched verbs
      still have to run — it never passes for the whole of stage 2 on
      its own.

  demo-repo <preset> [--at <dir>]
      Build a throwaway repository (isolated from your git config) and
      print its path. Presets:
        basic     branches + remote (ahead) + tags + stash + dirty WIP
        dirty     every WIP bucket: staged, unstaged, untracked, renamed
        eol       all four line-ending cases, in a directory of LF files
        conflict  a merge stopped on conflicts (MERGE_HEAD present)
        rebase-conflict  a rebase stopped on conflicts, 1 of 2 steps
        rebase-staged    the same rebase, conflict resolved and staged
        rebase-empty     a rebase stopped on a commit that came out empty
        cherry-pick-conflict  a cherry-pick stopped on a conflict
        conflict-kinds   four kinds of conflict in one stopped merge
        drop-collides    a drop that replays clean and collides restoring
        drop-stops       a drop that stops part-way, over a dirty tree
        stashes   three stashes, one with untracked files
        detached  HEAD detached at a tag
        behind    remote has commits fetch would bring in
        diverged  the same, fetched, with a commit of our own on top
        unpublished  a branch never sent anywhere, two remotes, one taken name
        noremote  commits and no remote at all: the first push writes one down
        signed    ssh-signed commits: verified, unjudgeable, unsigned
        errsig    an embedded OpenPGP signature no key can verify: the E
                  verdict (signature-tip passes only on this preset)
        co-authors  one, three and no co-authors, signed and not
        authorship  every way an author and a committer can be two
        tags      a tag in every state a remote can put it in (fetch first)
        manytags  basic, with more tags than any pane can show at once
        edges     every string at both ends of what git allows, in Japanese
        long      a message, a commit and a work tree that all run past
                  the pane they are shown in (80 files, 60-odd unstaged)
        longpaths one committed-then-changed file under a path wider than
                  any pane: both paths views elide it, and both tree
                  views elide its folder chain (path-tip [..-tree])
        empty     `git init` and nothing else

  verify-ui <verb> [arg] [options]
      Build the app (release), run it headless (offscreen QPA) against a
      repository, fire PG_AUTO_ACT=<verb> / PG_AUTO_ACT_ARG=<arg>, and
      judge the run by its 'screenshot saved=true' stderr line and by
      whether git refused any of the writes it made.
      The verb `solo` is run with this process holding the config
      directory's lock, so the app it starts is a second instance; that
      run is judged on reporting that it was turned away as well.
      The verb `details-fit` is judged on its own report too: a details
      pane whose column runs off the right of the window frames exactly
      like one that fits, so the picture cannot answer it. So is
      `window-fill`: a maximised window fills the screen, leaving no
      desktop beside it for an unpainted edge to show against. And so is
      `commands-clear`, where the panel the reader emptied is the whole
      picture and the mark it is judged on is a corner of the band.
      `details-grow` and `wip-grow` (each with a `-squeeze` twin) pull the
      description box's grip past everything and are judged the same way,
      on what sits under the box still being inside the pane. The details
      argument is the row, and the row with a short message — where the
      grip is not offered — is the other half of that pair; the commit
      editor's argument is the description to type, and a long one is
      written for it when the run brings none, since that box starts
      empty.
      options:
        --repo <dir>      run against this repository
        --preset <name>   or against a fresh demo repo (default: basic)
                          Both repeat: one tab per repository, in order.
        --no-build        reuse the existing release binary
        --select          also set PG_AUTO_SELECT=1
        --quit-ms <n>     PG_AUTO_QUIT_MS (default 10000)
        --shot-dir <dir>  screenshot directory (default: temp, kept)
        --config-dir <d>  settings.toml / state.toml directory. Fresh per
                          run by default; name one to carry what a run
                          wrote into the next one.
        --restore         open the tabs the config directory remembers
                          instead of a named repository
        --allow-write-failure
                          a write git refused is what this verb shows, so
                          it does not sink the run (delete-branch-refused,
                          commands-fail, commands-clear, fetch-fail,
                          fetch-resume, push-retry, drop-commit on the
                          drop-stops preset, and push / publish-new-go
                          against an unreachable remote)
      The open-refused verbs (open-not-a-repo, open-bare, the -retry /
      -cancel ways out, and open-fail-tab for the road that keeps its
      tab) make their own folder when no argument names one: what they
      are about is a folder no repository can be.
      The verb `tab-widths` makes its own repositories too — its argument
      is how many (1..=16, default 8), and it names them at a spread of
      lengths, because a strip of equally-named tabs cannot show which
      of them gave way and which were left at their own width.
      `tab-mark` opens the same strip and puts the pointer on the tab its
      argument names (default 1), which is the only way a headless run
      reaches the half of the `✕` rule that hover answers.
      `identity` and `identity-half` are read as a pair, and are the only
      verbs whose write would land outside a demo repository — so they are
      handed a git configuration of their own (GIT_CONFIG_GLOBAL in the
      shot directory) and never see the one on this machine. The argument
      is what to type, as `<name>|<email>`. `identity` fills the screen and
      leaves it; `identity-half` saves against a configuration whose
      user.email holds two values, which git refuses to overwrite with one
      — so the name lands and the address does not, the same way a lost
      configuration lock leaves it. Both are judged on their own report:
      a screen still standing because the save did not take frames exactly
      like one nobody has answered yet.

  linux [--rebuild] [--shell] [--stage core|app] <command…>
      Run a command against this checkout on Ubuntu, in a container built
      from ci/linux/Dockerfile. On Linux it skips the container and runs
      the command where it stands.
        cargo xtask linux test -p platitude-core --test it
        cargo xtask linux verify-ui commit --preset basic
        cargo xtask linux bare
      A cargo command goes to cargo; an xtask verb goes to cargo xtask.
      `bare` is the one that is not either: it builds the release
      workspace-wide and starts it on an Ubuntu carrying only what a
      package would declare, which is the only check that the thing runs
      somewhere it was not built. Worth a place in a pre-merge sweep, not
      in a daily one — it answers rarely, and when it does the answer is
      about what a distribution has to ship.
      Which image it runs in follows what the command needs: core is
      Ubuntu and the toolchain, app adds Qt, a software GL stack and the
      fonts デザイン規約 names for Ubuntu. The build directory is a docker
      volume, so this target/ is untouched, and a verify-ui run is handed
      a host directory to leave its screenshot in.
      options:
        --rebuild        build the image again even if one already matches
        --shell          open a shell in the container instead
        --stage <name>   core or app, when the guess is not the one wanted

  hook <event>
      Claude Code hook handler (wired from .claude/settings.json; reads
      the hook payload from stdin). Events: pre-write, post-write,
      pre-shell (pre-git on branches that predate it), session-start.
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("check") => check::run(&args[1..]),
        Some("demo-repo") => demo::run(&args[1..]).map(|path| {
            // The path is the output: scripts consume `(cargo xtask ...)`.
            println!("{}", path.display());
        }),
        Some("verify-ui") => verify::run(&args[1..]),
        Some("linux") => linux::run(&args[1..]),
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

/// The workspace root, resolved at compile time from this crate's location.
/// A worktree builds its own task runner, so this is that worktree's root.
pub(crate) fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap_or(Path::new("."))
        .to_path_buf()
}

/// Runs a command to completion, capturing output; errors carry context.
pub(crate) fn run_captured(
    cmd: &mut std::process::Command,
) -> Result<std::process::Output, String> {
    let display = format!("{cmd:?}");
    cmd.output()
        .map_err(|e| format!("failed to spawn {display}: {e}"))
}
