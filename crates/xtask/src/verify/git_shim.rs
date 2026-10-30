//! The xtask process acting as Git on a verification run's PATH.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// Set on the app when `--old-git` asks for one: the version a copy of this
/// binary, standing on PATH under git's name, answers `--version` with.
pub(super) const SHIM_VERSION: &str = "PGG_SHIM_GIT_VERSION";
/// The git that copy passes everything else to.
pub(super) const SHIM_REAL: &str = "PGG_SHIM_REAL_GIT";
/// Set on the app when `--no-lfs` asks for it: the copy answers `git lfs`
/// as a git without Git LFS does, whatever this machine has.
pub(super) const SHIM_NO_LFS: &str = "PGG_SHIM_NO_LFS";
/// Set on the app when `--other-git` asks for one: where a second git
/// stands, staged beside the pictures and **off** PATH.
pub(super) const OTHER_GIT: &str = "PGG_OTHER_GIT";
/// Set on the app when `--slow-git` asks for one: the subcommand the copy
/// answers late, and by how much ([`Slow`]).
pub(super) const SHIM_SLOW: &str = "PGG_SHIM_SLOW";

/// One subcommand of git answered late: `<subcommand>:<ms>[:quiet]`.
/// `quiet` leaves the subcommand alone while the repository has a rebase
/// under way — so a status read during a rebase lands at its own pace and
/// the one after it lands late, the order a verb completing on the
/// answer alone photographs by chance (`AutoActPlanVerbs`).
pub(crate) struct Slow {
    pub(super) subcommand: String,
    pub(super) ms: u64,
    pub(super) quiet: bool,
}

impl Slow {
    pub(crate) fn parse(asked: &str) -> Result<Slow, String> {
        let mut parts = asked.split(':');
        let subcommand = parts
            .next()
            .filter(|sub| !sub.is_empty())
            .ok_or_else(|| format!("--slow-git takes <subcommand>:<ms>[:quiet]; got {asked:?}"))?;
        let ms = parts
            .next()
            .and_then(|ms| ms.parse::<u64>().ok())
            .ok_or_else(|| format!("--slow-git takes <subcommand>:<ms>[:quiet]; got {asked:?}"))?;
        let quiet = match parts.next() {
            None => false,
            Some("quiet") => true,
            Some(other) => {
                return Err(format!(
                    "--slow-git: the third part is `quiet` or nothing; got {other:?}"
                ));
            }
        };
        if parts.next().is_some() {
            return Err(format!(
                "--slow-git takes <subcommand>:<ms>[:quiet]; got {asked:?}"
            ));
        }
        Ok(Slow {
            subcommand: subcommand.to_string(),
            ms,
            quiet,
        })
    }

    /// Whether this call of git is the one to answer late.
    fn delays(&self, args: &[OsString]) -> bool {
        if subcommand(args).is_none_or(|sub| *sub != *self.subcommand) {
            return false;
        }
        !(self.quiet && rebase_under_way(&repository_of(args)))
    }
}

/// The repository a git call works in: `-C <dir>`, else the working
/// directory.
fn repository_of(args: &[OsString]) -> PathBuf {
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        if arg == "-C"
            && let Some(dir) = it.next()
        {
            return PathBuf::from(dir);
        }
    }
    std::env::current_dir().unwrap_or_default()
}

/// Whether git has a rebase under way in `repo` — the directories it
/// keeps one in (a plain repository's own `.git`; the demo repositories
/// are plain).
fn rebase_under_way(repo: &Path) -> bool {
    ["rebase-merge", "rebase-apply"]
        .iter()
        .any(|dir| repo.join(".git").join(dir).is_dir())
}

/// Stands in for git when this binary was copied onto a run's PATH under
/// git's name (`--old-git` / `--no-lfs` / `--slow-git`); `None` in every
/// other process, the xtask that set it up included. Only `--version`
/// (the old version), `lfs` (not there) and the one slow subcommand's
/// timing are answered here — the rest goes to the real git, so the app
/// sees a working install that is old, has no Git LFS, or answers late.
///
/// This binary because it is already built, and script work belongs in
/// xtask (CLAUDE.md §技術スタック).
pub fn git_shim() -> Option<ExitCode> {
    let version = std::env::var(SHIM_VERSION).ok();
    let no_lfs = std::env::var_os(SHIM_NO_LFS).is_some();
    let slow = std::env::var(SHIM_SLOW)
        .ok()
        .and_then(|asked| Slow::parse(&asked).ok());
    if version.is_none() && !no_lfs && slow.is_none() {
        return None;
    }
    let real = std::env::var_os(SHIM_REAL)?;
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    if let Some(version) = version
        && args.iter().any(|a| a == "--version")
    {
        println!("git version {version}");
        return Some(ExitCode::SUCCESS);
    }
    // git's own words and code for a subcommand it cannot find.
    if no_lfs && subcommand(&args).is_some_and(|sub| sub == "lfs") {
        eprintln!("git: 'lfs' is not a git command. See 'git --help'.");
        return Some(ExitCode::from(1));
    }
    if let Some(slow) = slow
        && slow.delays(&args)
    {
        // waits(timed): the one subcommand answers no sooner than asked (--slow-git) — the order
        // a slow read gives by chance, made the run's own; git's answer is the product's real time
        std::thread::sleep(std::time::Duration::from_millis(slow.ms));
    }
    // Stdio inherited: the app reads this output as git's.
    let status = Command::new(&real).args(&args).status();
    let code = match status {
        // 128 is git's own "fatal".
        Err(e) => {
            eprintln!("fatal: shim could not run {}: {e}", real.to_string_lossy());
            128
        }
        Ok(s) => s.code().unwrap_or(1),
    };
    Some(ExitCode::from(u8::try_from(code).unwrap_or(1)))
}

/// The subcommand git was asked to run: the first word after the global
/// options, two of which take the next word as their value (the app
/// leads every command with `-c <key>=<value>` pairs).
fn subcommand(args: &[OsString]) -> Option<&OsString> {
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        if arg == "-c" || arg == "-C" {
            it.next();
        } else if !arg.to_string_lossy().starts_with('-') {
            return Some(arg);
        }
    }
    None
}
