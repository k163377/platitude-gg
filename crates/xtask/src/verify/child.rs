//! The app itself: the environment one run hands it, the wait with a
//! kill guard at the end of it, and what came back.
//!
//! **Never an unbounded wait or poll.** The app has a watchdog of its
//! own ([`crate::verify::run`] passes it in); this side allows it
//! [`GRACE_MS`] beyond that and then reaps, so a wedged GUI cannot hold
//! the run open. The ceiling is the run's, the wait is `crate::wait`'s.
//! The one run ended before its ceiling is the one ordered to hold at a
//! station with no deadline thread to end it: the trail says when it is
//! there, and it is reaped on that word ([`ordered_hold`]).

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use super::options::Options;
use super::shim::{OTHER_GIT, SHIM_REAL, SHIM_VERSION, identity_answer, identity_seed, real_git};
use crate::wait::{Budget, LOOK_AGAIN, Wait};

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
    /// When each of the lines above arrived, at the same index as the
    /// line it belongs to (`crate::app_out::Said::at`). What a run that
    /// ended itself is read off: the account it wrote on the way out is
    /// a line here like any other, so the silence that says where it
    /// stood is the one before that line rather than the one after it
    /// ([`super::wedge`]).
    pub(super) out_at: Vec<Duration>,
    pub(super) err_at: Vec<Duration>,
    pub(super) status: Option<std::process::ExitStatus>,
    /// Whether this side reaped the app — at the ceiling, or at the
    /// station it was ordered to hold at ([`Self::held_at`]). A run that
    /// ended itself, its own deadline thread included, is `false` here.
    pub(super) timed_out: bool,
    /// The station the app was found held at, for the run ordered to
    /// hold there with no deadline thread of its own ([`ordered_hold`]):
    /// what the reaping was on the word of, rather than the ceiling.
    /// `None` for every other run.
    pub(super) held_at: Option<String>,
    pub(super) elapsed: Duration,
    /// How long the app had said nothing when the run ended. `None` where
    /// it never said anything at all — the whole run is the silence then.
    /// What a run reaped at the ceiling is read off ([`super::wedge`]).
    pub(super) quiet_for: Option<Duration>,
    /// What went with the app when the parent reaped it: the git it had
    /// running, counted (`reap::Reaped::line`). `None` for a run that
    /// ended itself.
    pub(super) reaped: Option<String>,
    /// What a look at the app while it still stood could say — its
    /// threads, and a dump of it where one could be taken — for a run
    /// reaped at the ceiling; empty for one that ended itself
    /// (`super::wedge::look_at`).
    pub(super) looked: Vec<String>,
}

/// Starts the app, waits it out, and answers with everything it said.
pub(super) fn run_app(start: &Start<'_>) -> Result<Ran, String> {
    let mut cmd = compose(start)?;
    let _held = hold_the_store(start.config_dir, &start.opts.verb)?;
    super::wedge::clear_any_account(start.shot_dir);

    // Bounded wait with a kill guard — never an unbounded wait or poll.
    let mut wait = Wait::new(
        format!("the app running {}", start.opts.verb),
        Budget::whole(Duration::from_millis(start.opts.watchdog_ms + GRACE_MS)),
        LOOK_AGAIN,
    );
    let app = cmd.get_program().to_string_lossy().into_owned();
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to start the app: {e}"))?;
    // What this unit is doing, where the run is a session's own rather
    // than a gate's step: a verb killed at the wrong moment leaves the
    // app standing, and the room is held while it does
    // (`crate::budget::child_started`). Silent under a gate, whose steps
    // say it through the ticket they were handed.
    crate::budget::child_started(child.id(), &app);
    let stdout = child.stdout.take().map(crate::app_out::collect);
    let stderr = child.stderr.take().map(crate::app_out::collect);

    let mut timed_out = false;
    let mut held_at = None;
    let mut reaped = None;
    let mut looked = Vec::new();
    let ordered = ordered_hold(start.opts);
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break Some(status);
        }
        // A run ordered to hold at a station, with no deadline thread of
        // its own, is this side's to end — and the trail says when it is
        // there. The hold never returns, so every second between that
        // mark and the ceiling would be wall clock paid for no answer
        // (`ordered_hold`).
        let held = ordered.filter(|station| {
            super::wedge::last_station(start.shot_dir).as_deref() == Some(*station)
        });
        if held.is_none() && wait.look_again("its exit").is_ok() {
            continue;
        }
        // Taken while the app still stands, since past `exiting` the
        // trail has run out and these are the only witnesses left
        // (`super::wedge::look_at`).
        looked = super::wedge::look_at(
            child.id(),
            start.shot_dir,
            start.opts.fault_hang.is_empty(),
            start.opts.fault_stall_look,
        );
        // The app takes the git it was waiting on with it — a hook
        // that never returns, a fetch to nowhere — which `reap`
        // reaches by walking from the app rather than leaving it to
        // the step's own ceiling. The app is left in this runner's
        // own group, so that a signal aimed at the runner from
        // outside (a Ctrl-C, the step's group kill) ends it here
        // instead of leaving it holding this run's store lock until
        // its own watchdog fires. What ran out is reported off the
        // run itself (`super::wedge`: the ceiling, the silence, what
        // went with it), so the wait's own words are not repeated.
        let (under, ended) = crate::reap::reap(&mut child);
        reaped = Some(under.line());
        timed_out = true;
        held_at = held.map(str::to_string);
        break ended;
    };

    let join =
        |h: Option<std::thread::JoinHandle<crate::app_out::Said>>| h.and_then(|h| h.join().ok());
    let (out, err) = (join(stdout), join(stderr));
    let elapsed = wait.elapsed();
    // The later of the two streams: either counts as the app still having
    // been there.
    let spoke_at = [
        out.as_ref().and_then(crate::app_out::Said::last),
        err.as_ref().and_then(crate::app_out::Said::last),
    ]
    .into_iter()
    .flatten()
    .max();
    let (out, err) = (out.unwrap_or_default(), err.unwrap_or_default());
    Ok(Ran {
        out_at: out.at,
        err_at: err.at,
        out_lines: out.lines,
        err_lines: err.lines,
        status,
        timed_out,
        held_at,
        elapsed,
        quiet_for: spoke_at.map(|at| elapsed.saturating_sub(at)),
        reaped,
        looked,
    })
}

/// The station a run was ordered to hold at where no deadline thread will
/// end it there: the one run this side ends on the trail's word rather
/// than the ceiling's. **The ceiling stays, as the backstop** — a run
/// that never reaches its station is still reaped at it. A hold with the
/// deadline thread up is left to the thread: what that case reads is the
/// account the thread writes, which a reaping from here would forestall
/// (`super::faults`).
fn ordered_hold(opts: &Options) -> Option<&str> {
    (opts.fault_no_deadline && !opts.fault_hang.is_empty()).then_some(opts.fault_hang.as_str())
}

/// The environment one run hands the app: where it reads its git identity
/// and its settings from, and every `PGG_*` knob the verb asked for.
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
        .env("PGG_CONFIG_DIR", config_dir)
        .env("PGG_AUTO_WATCHDOG_MS", opts.watchdog_ms.to_string())
        .env("PGG_SHOT_DIR", shot_dir)
        .env("PGG_AUTO_ACT", &opts.verb)
        .env("PGG_AUTO_ACT_ARG", arg)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if cfg!(windows) {
        // Offscreen Qt does not discover system fonts on Windows; without
        // this every glyph is a box (verify-ui skill).
        cmd.env("QT_QPA_FONTDIR", "C:\\Windows\\Fonts");
    }
    // Set only where they were asked for: `clear_automation` above has
    // already taken whatever the parent shell carried, so an unset knob
    // here is an app that is not being made to wedge (`super::faults`).
    if !opts.fault_hang.is_empty() {
        cmd.env("PGG_FAULT_HANG", &opts.fault_hang);
    }
    if opts.fault_no_deadline {
        cmd.env("PGG_FAULT_NO_DEADLINE", "1");
    }
    if opts.fault_hold_act {
        cmd.env("PGG_FAULT_HOLD_ACT", "1");
    }
    // The saves held until a station where a run asked for it — and,
    // unasked, for the verb whose subject that is: its identity save is
    // held until the shutdown joins it, which is what the run reads
    // (`super::verbs::window`).
    let hold_save = if opts.fault_hold_save.is_empty() && opts.verb == HELD_SAVE_VERB {
        HELD_SAVE_STATION
    } else {
        opts.fault_hold_save.as_str()
    };
    if !hold_save.is_empty() {
        cmd.env("PGG_FAULT_HOLD_SAVE", hold_save);
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
    if std::env::var_os("PGG_LOG").is_none() {
        cmd.env("PGG_LOG", "info");
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
        cmd.env("PGG_AUTO_OPEN", &open);
    }
    if opts.select {
        cmd.env("PGG_AUTO_SELECT", "1");
    }
    if !opts.scroll_to.is_empty() {
        cmd.env("PGG_SCROLL_TO", &opts.scroll_to);
    }
    if opts.system_title_bar {
        cmd.env("PGG_SYSTEM_TITLE_BAR", "1");
    }
    super::perf::configure(&mut cmd, &opts.verb, arg)?;
    // The screen the identity verbs are about: the seed written above is
    // theirs, and this is what the dialog standing on it is told to do.
    if identity_seed(&opts.verb).is_some() {
        cmd.env("PGG_AUTO_IDENTITY", identity_answer(&opts.verb, &opts.arg));
        if opts.verb == "identity-half"
            || opts.verb == "identity-tip"
            || opts.verb == HELD_SAVE_VERB
        {
            cmd.env("PGG_AUTO_IDENTITY_SAVE", "1");
        }
    }

    Ok(cmd)
}

/// The verb whose subject is the exit waiting for a configuration save
/// the hub holds, and the station its save is held until: the one the
/// shutdown reaches as it starts joining the writes still out
/// (`Hub::shutdown`), so the join is what lets the save go.
pub(super) const HELD_SAVE_VERB: &str = "quit-save-held";
const HELD_SAVE_STATION: &str = "writes-joining";

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
fn hold_the_store(config_dir: &Path, verb: &str) -> Result<Option<crate::locks::Locked>, String> {
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
    Ok(Some(crate::locks::Locked::new(file)))
}
