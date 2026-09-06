//! The app itself: the environment one run hands it, the wait with a
//! kill guard at the end of it, and what came back.
//!
//! **Never an unbounded wait or poll.** The app has a watchdog of its
//! own ([`crate::verify::run`] passes it in); this side allows it
//! [`GRACE_MS`] beyond that and then reaps, so a wedged GUI cannot hold
//! the run open.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::options::Options;
use super::shim::{OTHER_GIT, SHIM_REAL, SHIM_VERSION, identity_answer, identity_seed, real_git};

/// Grace after the app-side watchdog before the parent reaps a wedged GUI.
const GRACE_MS: u64 = 20_000;

/// Everything the run worked out before there was a process.
pub(super) struct Start<'a> {
    pub(super) exe: &'a Path,
    pub(super) shot_dir: &'a Path,
    pub(super) config_dir: &'a Path,
    /// The gitconfig this run reads its identity from.
    pub(super) config: &'a Path,
    /// The PATH the *child* gets, which carries the git shim when one
    /// was staged; `path` is the one without it.
    pub(super) child_path: &'a OsString,
    pub(super) path: &'a OsString,
    pub(super) arg: &'a str,
    /// The version the staged copy of this binary answers `--version`
    /// with, whether it stands on PATH (`--old-git`) or beside the
    /// pictures (`--other-git`); empty where neither was asked for.
    pub(super) shim_version: &'a str,
    pub(super) other_git: Option<&'a std::path::Path>,
    pub(super) repos: &'a [PathBuf],
    pub(super) opts: &'a Options,
}

/// What the run left behind.
pub(super) struct Ran {
    pub(super) out_lines: Vec<String>,
    pub(super) err_lines: Vec<String>,
    pub(super) status: Option<std::process::ExitStatus>,
    pub(super) timed_out: bool,
    pub(super) elapsed: Duration,
    /// How long the app had said nothing when the run ended. `None` where
    /// it never said anything at all — the whole run is the silence then.
    /// What a run reaped at the ceiling is read off ([`super::wedge`]).
    pub(super) quiet_for: Option<Duration>,
    /// What went with the app when the parent reaped it: the git it had
    /// running, counted (`reap::Reaped::line`). `None` for a run that
    /// ended itself.
    pub(super) reaped: Option<String>,
}

/// Starts the app, waits it out, and answers with everything it said.
pub(super) fn run_app(start: &Start<'_>) -> Result<Ran, String> {
    let mut cmd = compose(start)?;
    // So that an app reaped at the ceiling takes the git it was waiting
    // on with it — a hook that never returns, a fetch to nowhere — rather
    // than leaving that process to the step's own ceiling (`reap`). The
    // group is the app's own, so on unix a signal that ends this runner
    // from outside (Ctrl-C, the step's group kill) does not reach the
    // app: it is left to its own ceiling, which its native watchdog holds
    // it to (`--watchdog-ms` plus its grace) whether or not its event
    // loop is turning.
    crate::reap::own_group(&mut cmd);
    let _held = hold_the_store(start.config_dir, &start.opts.verb)?;
    super::wedge::clear_any_account(start.shot_dir);

    let started = Instant::now();
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to start the app: {e}"))?;
    let stdout = child.stdout.take().map(crate::app_out::collect);
    let stderr = child.stderr.take().map(crate::app_out::collect);

    // Bounded wait with a kill guard — never an unbounded wait or poll.
    let deadline = Duration::from_millis(start.opts.watchdog_ms + GRACE_MS);
    let mut timed_out = false;
    let mut reaped = None;
    let status = loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(status) => break Some(status),
            None if started.elapsed() > deadline => {
                let (under, ended) = crate::reap::reap(&mut child);
                reaped = Some(under.line());
                timed_out = true;
                break ended;
            }
            None => std::thread::sleep(Duration::from_millis(100)),
        }
    };

    let join =
        |h: Option<std::thread::JoinHandle<crate::app_out::Said>>| h.and_then(|h| h.join().ok());
    let (out, err) = (join(stdout), join(stderr));
    let elapsed = started.elapsed();
    // The later of the two streams: either counts as the app still having
    // been there.
    let spoke_at = [
        out.as_ref().and_then(|s| s.last),
        err.as_ref().and_then(|s| s.last),
    ]
    .into_iter()
    .flatten()
    .max();
    let unwrap_lines =
        |said: Option<crate::app_out::Said>| said.map(|s| s.lines).unwrap_or_default();
    Ok(Ran {
        out_lines: unwrap_lines(out),
        err_lines: unwrap_lines(err),
        status,
        timed_out,
        elapsed,
        quiet_for: spoke_at.map(|at| elapsed.saturating_sub(at)),
        reaped,
    })
}

/// The environment one run hands the app: where it reads its git identity
/// and its settings from, and every `PG_*` knob the verb asked for.
fn compose(start: &Start<'_>) -> Result<Command, String> {
    let Start {
        exe,
        shot_dir,
        config_dir,
        config,
        child_path,
        path,
        arg,
        shim_version,
        other_git,
        repos,
        opts,
    } = *start;

    let mut cmd = Command::new(exe);
    crate::app_env::clear_automation(&mut cmd);
    cmd.current_dir(shot_dir)
        .env("GIT_CONFIG_GLOBAL", config)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("PATH", child_path)
        .env("QT_QPA_PLATFORM", "offscreen")
        .env("QT_FORCE_STDERR_LOGGING", "1")
        .env("PG_CONFIG_DIR", config_dir)
        .env("PG_AUTO_WATCHDOG_MS", opts.watchdog_ms.to_string())
        .env("PG_SHOT_DIR", shot_dir)
        .env("PG_AUTO_ACT", &opts.verb)
        .env("PG_AUTO_ACT_ARG", arg)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if cfg!(windows) {
        // Offscreen Qt does not discover system fonts on Windows; without
        // this every glyph is a box (verify-ui skill).
        cmd.env("QT_QPA_FONTDIR", "C:\\Windows\\Fonts");
    }
    // The two the staged copy reads to be a git: what to answer
    // `--version` with, and who to hand the rest to. Both are set on the
    // app, so every git it starts inherits them — which is how the copy
    // works whether it stands on PATH (`--old-git`) or somewhere only the
    // settings box points at (`--other-git`, whose place the app is told).
    if !shim_version.is_empty() {
        cmd.env(SHIM_VERSION, shim_version)
            .env(SHIM_REAL, real_git(path)?);
    }
    if let Some(other) = other_git {
        cmd.env(OTHER_GIT, other);
    }
    // The automation hooks report through tracing at info; without this
    // their lines never reach the verdict output.
    if std::env::var_os("PG_LOG").is_none() {
        cmd.env("PG_LOG", "info");
    }
    if !opts.restore {
        // Naming a repository is what turns tab restoring off (Main.qml):
        // a run that is told what to open is not being asked what it
        // remembers. More than one opens a tab each, in this order —
        // joined rather than formatted, so a path git accepts but UTF-8
        // does not still reaches the app whole.
        let mut open = std::ffi::OsString::new();
        for (position, repo) in repos.iter().enumerate() {
            if position > 0 {
                open.push(";");
            }
            open.push(repo);
        }
        cmd.env("PG_AUTO_OPEN", &open);
    }
    if opts.select {
        cmd.env("PG_AUTO_SELECT", "1");
    }
    if opts.system_title_bar {
        cmd.env("PG_SYSTEM_TITLE_BAR", "1");
    }
    super::perf::configure(&mut cmd, &opts.verb, arg)?;
    // The screen the identity verbs are about: the seed written above is
    // theirs, and this is what the dialog standing on it is told to do.
    if identity_seed(&opts.verb).is_some() {
        cmd.env("PG_AUTO_IDENTITY", identity_answer(&opts.verb, &opts.arg));
        if opts.verb == "identity-half" || opts.verb == "identity-tip" {
            cmd.env("PG_AUTO_IDENTITY_SAVE", "1");
        }
    }

    Ok(cmd)
}

/// Whether this verb's subject is a second process finding the settings
/// held. Every other run has a store nobody else can be in — its config
/// directory is made for it and thrown away after — so one that reports
/// the store taken is a broken run, and [`super::outcome`] reads this to
/// tell the two apart.
pub(super) fn stages_a_held_store(verb: &str) -> bool {
    verb == "solo" || verb == "gate-sweep"
}

/// Holds the settings lock for the two verbs whose subject is a *second*
/// process finding it held.
///
/// Holding the real lock — rather than setting a flag that imitates the
/// state — is what makes the picture proof of the mechanism. The name is
/// `settings::LOCK_FILE`; xtask depends on std alone (CLAUDE.md), so it
/// is spelled again here, and a drift shows up as the run reporting
/// `blocked=false`.
fn hold_the_store(config_dir: &Path, verb: &str) -> Result<Option<std::fs::File>, String> {
    if !stages_a_held_store(verb) {
        return Ok(None);
    }
    let path = config_dir.join("lock");
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|e| format!("could not open {}: {e}", path.display()))?;
    file.try_lock()
        .map_err(|e| format!("could not hold {}: {e}", path.display()))?;
    println!("holding: {}", path.display());
    Ok(Some(file))
}
