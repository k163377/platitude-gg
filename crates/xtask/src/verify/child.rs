//! The app itself: the environment one run hands it, the wait with a
//! kill guard at the end of it, and what came back.
//!
//! The app has a watchdog of its own ([`crate::verify::run`] passes it
//! in); this side allows [`GRACE_MS`] beyond it and then reaps, so a
//! wedged GUI cannot hold the run open. It reaps sooner only a run ordered
//! to hold with no deadline thread ([`ordered_hold`]) and one whose QML
//! would not load ([`qml_refused`]).

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use super::options::Options;
use super::shim::{
    OTHER_GIT, SHIM_NO_LFS, SHIM_REAL, SHIM_VERSION, identity_answer, identity_seed, real_git,
};
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
    /// What the staged git copy answers `--version` with (`--old-git` on
    /// PATH, `--other-git` beside the pictures); empty when neither.
    pub(super) shim_version: &'a str,
    /// Whether the copy on PATH answers `git lfs` as missing (`--no-lfs`).
    pub(super) shim_no_lfs: bool,
    pub(super) other_git: Option<&'a std::path::Path>,
    pub(super) repos: &'a [PathBuf],
    pub(super) opts: &'a Options,
}

/// What the run left behind.
pub(super) struct Ran {
    pub(super) out_lines: Vec<String>,
    pub(super) err_lines: Vec<String>,
    /// When each line above arrived, index for index
    /// (`crate::app_out::Said::at`): the silence before a run's exit
    /// account says where it stood ([`super::wedge`]).
    pub(super) out_at: Vec<Duration>,
    pub(super) err_at: Vec<Duration>,
    pub(super) status: Option<std::process::ExitStatus>,
    /// Whether this side reaped the app at the ceiling or at the station
    /// it was ordered to hold at ([`Self::held_at`]); `false` for a run
    /// that ended itself or was given up on ([`Self::gave_up`]).
    pub(super) timed_out: bool,
    /// The station a run ordered to hold ([`ordered_hold`]) was reaped at;
    /// `None` for every other run.
    pub(super) held_at: Option<String>,
    pub(super) elapsed: Duration,
    /// How long the app had said nothing when the run ended; `None` when
    /// it never spoke. Read for a run reaped at the ceiling
    /// ([`super::wedge`]).
    pub(super) quiet_for: Option<Duration>,
    /// What went with the app when the parent reaped it: the git it had
    /// running, counted (`reap::Reaped::line`). `None` for a run that
    /// ended itself.
    pub(super) reaped: Option<String>,
    /// Its threads, and a dump where one could be taken, looked at before
    /// this side reaped it at the ceiling or the held station
    /// (`super::look::look_at`); empty otherwise.
    pub(super) looked: Vec<String>,
    /// Why the parent gave up on a run already decided (QML that would
    /// not load); `None` otherwise.
    pub(super) gave_up: Option<String>,
}

/// Qt's warning when its engine could not load, or (parsed) could not
/// create, the root file (`qqmlapplicationengine.cpp`, v6.10.3);
/// `rootObjects()` stays empty, so no window is coming.
///
/// Matched on ASCII only: a Windows Qt writes its log in the local code
/// page (verify-ui skill).
fn qml_refused(line: &str) -> bool {
    line.contains("QQmlApplicationEngine failed to load component")
        || line.contains("QQmlApplicationEngine failed to create component")
}

const QML_REFUSED_ACCOUNT: &str = "the app said its QML would not load, so nothing was waiting to be photographed — \
     the lines above are Qt's own account of it";

/// Starts the app, waits it out, and answers with everything it said.
pub(super) fn run_app(start: &Start<'_>) -> Result<Ran, String> {
    let mut cmd = compose(start)?;
    let _held = hold_the_store(start.config_dir, &start.opts.verb)?;
    super::wedge::clear_any_account(start.shot_dir);

    let mut wait = Wait::new(
        format!("the app running {}", start.opts.verb),
        Budget::whole(Duration::from_millis(start.opts.watchdog_ms + GRACE_MS)),
        LOOK_AGAIN,
    );
    let app = cmd.get_program().to_string_lossy().into_owned();
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to start the app: {e}"))?;
    // A verb killed mid-run leaves the app standing, and its budget room
    // stays held while it does (`crate::budget::child_started`).
    crate::budget::child_started(child.id(), &app);
    // QML that will not load leaves a windowless app in its event loop
    // until the ceiling, and `gate::sides::verbs` would run such a tree's
    // verbs one at a time, each paying the full watchdog. Qt says so the
    // moment it happens, so the wait ends on that line.
    let unloadable = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    fn watch<R: std::io::Read + Send + 'static>(
        pipe: Option<R>,
        mark: &std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Option<std::thread::JoinHandle<crate::app_out::Said>> {
        pipe.map(|r| crate::app_out::collect_marking(r, std::sync::Arc::clone(mark), qml_refused))
    }
    let stdout = watch(child.stdout.take(), &unloadable);
    let stderr = watch(child.stderr.take(), &unloadable);

    let mut timed_out = false;
    let mut gave_up = None;
    let mut held_at = None;
    let mut reaped = None;
    let mut looked = Vec::new();
    let ordered = ordered_hold(start.opts);
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break Some(status);
        }
        // The ordered hold never returns, so it is reaped once the trail
        // reaches its station (`ordered_hold`).
        let held = ordered.filter(|station| {
            super::wedge::last_station(start.shot_dir).as_deref() == Some(*station)
        });
        // No look is taken: a wedge's diagnostics are for an unknown
        // reason, and this one's is in the lines the app already wrote.
        if unloadable.load(std::sync::atomic::Ordering::SeqCst) {
            gave_up = Some(QML_REFUSED_ACCOUNT.to_string());
            let (under, ended) = crate::reap::reap(&mut child);
            reaped = Some(under.line());
            break ended;
        }
        if held.is_none() && wait.look_again("its exit").is_ok() {
            continue;
        }
        // Before the reaping: past `exiting` the trail has run out, and
        // this is the only witness left (`super::look::look_at`).
        looked = super::look::look_at(
            child.id(),
            start.shot_dir,
            start.opts.fault_hang.is_empty(),
            start.opts.fault_stall_look,
        );
        // `reap` walks from the app to the git it was waiting on (a hook
        // that never returns, a fetch to nowhere). The app stays in this
        // runner's process group, so a Ctrl-C or the step's group kill
        // ends it too, store lock and all, before its own watchdog fires.
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
    // Either stream counts as the app still being there.
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
        gave_up,
    })
}

/// The station a run was ordered to hold at with no deadline thread to
/// end it there; the ceiling stays the backstop. A hold with the thread
/// up is left to it: that case reads the account the thread writes, which
/// a reaping from here would forestall (`super::faults`).
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
        shim_no_lfs,
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
    // Per build, so its runs share one compiled cache while a person's own
    // window and other seats' builds keep theirs (`platitude-app` qrc.rs).
    cmd.env("QML_DISK_CACHE_PATH", exe.with_file_name("qmlcache"));
    // `clear_automation` already dropped what the parent shell carried, so
    // an unset knob means no fault (`super::faults`).
    if !opts.fault_hang.is_empty() {
        cmd.env("PGG_FAULT_HANG", &opts.fault_hang);
    }
    if opts.fault_no_deadline {
        cmd.env("PGG_FAULT_NO_DEADLINE", "1");
    }
    if opts.fault_hold_act {
        cmd.env("PGG_FAULT_HOLD_ACT", "1");
    }
    // Raised by verb name: a run that needed a hand-typed flag is one the
    // census could not record (`super::verbs::window`).
    if HELD_WIP_ROW_VERBS.contains(&opts.verb.as_str()) {
        cmd.env("PGG_FAULT_HOLD_WIP_ROW", "1");
    }
    // The save hold: where a run asked for it, and unasked for
    // `HELD_SAVE_VERB` (`super::verbs::window`).
    let hold_save = if opts.fault_hold_save.is_empty() && opts.verb == HELD_SAVE_VERB {
        HELD_SAVE_STATION
    } else {
        opts.fault_hold_save.as_str()
    };
    if !hold_save.is_empty() {
        cmd.env("PGG_FAULT_HOLD_SAVE", hold_save);
    }
    // Set on the app so every git it starts inherits them, wherever the
    // staged copy stands (`--old-git` / `--no-lfs` on PATH, `--other-git`
    // where the settings point).
    if !shim_version.is_empty() {
        cmd.env(SHIM_VERSION, shim_version);
    }
    if shim_no_lfs {
        cmd.env(SHIM_NO_LFS, "1");
    }
    if !shim_version.is_empty() || shim_no_lfs {
        cmd.env(SHIM_REAL, real_git(path)?);
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
        // Naming repositories turns tab restoring off (Main.qml). Joined
        // as `OsString`, so a path that is not UTF-8 still arrives whole.
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
    // What the identity verbs' dialog is told to do over their seed.
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

/// Verbs started with the graph walking as if the first status had not
/// arrived — an arrangement no repository can be built into
/// (`platitude_app::harness::faults`).
const HELD_WIP_ROW_VERBS: &[&str] = &["wip-landing", "wip-landing-stopped"];

/// The verb whose subject is the exit waiting on a held configuration
/// save, and the station the save is held until: where `Hub::shutdown`
/// starts joining the writes still out, so the join lets it go.
pub(super) const HELD_SAVE_VERB: &str = "quit-save-held";
const HELD_SAVE_STATION: &str = "writes-joining";

/// Whether this verb's subject is a second process finding the settings
/// held. Every other run's config directory is its own, so a taken store
/// there is a broken run ([`super::outcome`]).
pub(super) fn stages_a_held_store(verb: &str) -> bool {
    verb == "solo" || verb == "gate-sweep"
}

/// Holds the real settings lock for [`stages_a_held_store`] verbs. The
/// name is `settings::LOCK_FILE` spelled again (xtask is std-only); a
/// drift shows as the run reporting `blocked=false`.
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
