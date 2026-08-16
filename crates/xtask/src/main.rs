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
mod land;
mod linux;
mod perf;
mod qt;
mod seats;
mod structure;
mod verify;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
cargo xtask <command>

commands:
  check [--verb <v>]...
      Stage-2 verification (CLAUDE.md 確認は 3 段), with the host and the
      container running in parallel: structure, fmt, clippy and the
      workspace tests here, while the container runs test -p
      platitude-core, verify-ui
      for each --verb, and bare. The two sides write to different build
      trees (target/ vs the docker volume), so the wall clock is
      whichever side finishes last. Each --verb also runs verify-ui on
      the host, so one flag covers the verb on both OSes; its value is
      a whole verify-ui argument line, quoted when the verb needs its
      preset or argument beside it (--verb 'co-authors 4 --preset
      co-authors'). Without --verb the summary says the touched verbs
      still have to run — it never passes for the whole of stage 2 on
      its own.

  structure
      The per-file line ceilings of .claude/rules/structure.md (src 500,
      tests 1000, physical lines) over crates/**/*.rs and *.qml — first
      step of `check`, and a second or two on its own. Three standings:
      a file the ledger (.claude/rules-refs/structure.md 分割しない判断)
      gives a written reason not to split has no ceiling; a file already
      over when this went in is pinned by crates/xtask/structure-baseline.txt
      at the length it had, free to shrink (the pin follows it down) and
      not to grow; everything else meets the ceiling as written, so a file
      that crosses it for the first time fails the run that sees it. The
      other half of that § — fn 100 lines — is clippy's too_many_lines,
      raised to warn in the workspace lints, and an existing long function
      carries #[expect(clippy::too_many_lines)] until it is cut up.

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
        deep      more commits than the graph loads at once: the window
                  cut, which nothing shorter can show (graph-tail)
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
        --watchdog-ms <n> maximum run time; never chooses the shot (default 120000)
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
                          fetch-recover, fetch-resume, push-retry,
                          drop-commit on the drop-stops preset, and push /
                          publish-new-go against an unreachable remote)
        --old-git <ver>   run the app against a git that answers --version
                          with <ver> and passes everything else to the real
                          one, so an installation older than the supported
                          minimum can be photographed working. Implied by
                          the verbs old-git and old-git-card.
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

  perf --repo <path> [--label <name>] [--runs <n>] [--breakdown]
      The measurement behind ci/baseline/perf-windows-x64.md, run the
      same way every time: release build, a real window (offscreen
      reports neither memory nor fps honestly), the PG_AUTO_* hooks,
      WorkingSet and private bytes sampled every 100ms for their
      maximum, and a deadline with a kill guard. The first run is
      discarded — the record is a warm-cache number.
      --breakdown builds with the `memprobe` feature and adds
      PG_MEM_REPORT=1, then prints the largest `mem report` line the run
      produced: live Rust heap, the models and the session parts holding
      it, and what none of them account for. That remainder plus the
      process total is what separates the toolkit's bytes from ours.
      options:
        --runs <n>        kept runs after the discarded first (default 3)
        --watchdog-ms <n> outer hang ceiling (default 300000)
        --no-scroll       leave the scroll benchmark out
        --no-select       do not select a row or open a diff
        --no-open         start with no repository at all — the window and
                          nothing in it. Subtracting this from a run that
                          opened an empty repository leaves the cost of
                          putting the page up, which is otherwise
                          indistinguishable from the toolkit's own floor.
        --no-build        use the release binary already built

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

  land [<branch>]
      Put a branch on main — the one sanctioned way (CLAUDE.md Git 運用;
      the pre-shell hook asks for PG_ALLOW_MAIN=1 in front, which is how
      the transcript records that the user asked). Reads where main is
      checked out before moving anything: merges in the primary checkout
      when it sits on main; fast-forwards the ref (and reattaches a
      detached primary) when main is checked out nowhere; refuses the
      ambiguous rest with what to do instead. Bare `land` from a seat
      lands the seat's own branch. A landed branch's seat is handed
      back: its claude-seat claim is released once the commits are on
      main, and the next edit there claims it back (a lock written by
      hand stays).

  kill
      Reap this tree's app processes — the ones holding this tree's exe
      against the next link, or its store lock against the next window —
      and nobody else's. The pre-shell hook points image-name kills
      (taskkill /IM, Stop-Process -Name), which reach every seat and the
      user's own window, at this instead.

  launch [--no-build]
      Real-window start for 「起動して」 asks (the verify-ui skill's fast
      path): reap this tree's stale runs, build release, start detached,
      and confirm it outlived its first second. Run it with
      PG_ALLOW_GUI=1 in front — a real window is the user's ask. The app
      keeps a per-tree settings store on its own, so seats never fight
      over one instance lock.

  seats
      Where the six worktree seats a-f stand right now, one line each:
      branch, whether HEAD sits at main's tip, commits ahead of main
      (main..HEAD), uncommitted changes (status --porcelain lines), and
      how long since the seat's own index was written. The session
      greeting reads the same survey, but only once, when the session
      starts — a seat that looked free then can hold another session's
      work minutes later, so this is the line to read before entering
      one (CLAUDE.md ビルド・テスト). Ends with how to read the columns,
      and a locked seat carries the mark past them.

  hook <event>
      Claude Code hook handler (wired from .claude/settings.json; reads
      the hook payload from stdin). Events: pre-write, post-write,
      pre-shell, pre-worktree, session-start, session-end.
";

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
        Some("land") => land::run(&args[1..]),
        Some("kill") => gui::kill(&args[1..]),
        Some("launch") => gui::launch(&args[1..]),
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
