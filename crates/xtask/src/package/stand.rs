//! The bundle started once, offscreen, from inside itself: judged as
//! `cargo xtask shipped` judges a build — the QML loaded, the app said it
//! started, it was still up when the stretch ended — and then by dyld's
//! own account of the app's process: every image not the system's came
//! from inside the bundle.
//!
//! The stand `shipped` runs, written a second time rather than shared
//! (.claude/rules/structure.md §共通化: two places are left as they are).

use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use super::bundle;
use crate::app_out::collect;
use crate::wait::{LOOK_AGAIN, stood};

/// As long as `shipped` stands a build: the engine reports a failed load
/// before the event loop, so this only has to outlast the runtime, the
/// store and the first paint.
const STAND_MS: u64 = 8_000;

/// The engine's line for a type that would not resolve — the one QML
/// warning `shipped` fails a build on.
const LOAD_FAILED: &str = "QQmlApplicationEngine failed to load component";

/// The app's first line (`main`): proof it got as far as running.
const STARTED: &str = "build tree";

/// What would let Qt or dyld look outside the bundle, or send dyld's
/// account somewhere other than stderr.
const OUTSIDE: [&str; 7] = [
    "QT_PLUGIN_PATH",
    "QT_QPA_PLATFORM_PLUGIN_PATH",
    "QML_IMPORT_PATH",
    "QML2_IMPORT_PATH",
    "DYLD_LIBRARY_PATH",
    "DYLD_FRAMEWORK_PATH",
    "DYLD_PRINT_TO_FILE",
];

/// What the bundle said and did in the stretch it was given.
struct Stood {
    /// The app's own process, which a line its children wrote does not name.
    pid: u32,
    /// Its stderr, then its stdout.
    lines: Vec<String>,
    ran_for: Duration,
    /// Its exit, where it ended before the stretch did.
    left_early: Option<ExitStatus>,
}

/// Starts the bundle's `main` binary offscreen with nothing pointing it
/// outside — and without `hidden`, a variable no code of the app's needs to
/// see — and judges the stand.
pub(super) fn stand(app: &Path, main: &Path, hidden: Option<&str>) -> Result<(), String> {
    let config = crate::keepsakes::keepsake_dir("package")?;
    let mut cmd = Command::new(main);
    crate::app_env::clear_automation(&mut cmd);
    for name in OUTSIDE.into_iter().chain(hidden) {
        cmd.env_remove(name);
    }
    cmd.current_dir(&config)
        .env("QT_QPA_PLATFORM", "offscreen")
        .env("QT_FORCE_STDERR_LOGGING", "1")
        .env("PGG_CONFIG_DIR", &config)
        .env("PGG_LOG", "info")
        .env("QML_DISK_CACHE_PATH", config.join("qmlcache"))
        .env("DYLD_PRINT_LIBRARIES", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    println!("running the bundle offscreen: {}", config.display());
    let stood = run(&mut cmd, main)?;
    // dyld's account of every image is a few hundred lines; what else it
    // says (a library not loaded) is shown with the app's own.
    for line in stood
        .lines
        .iter()
        .filter(|line| bundle::image_loaded(line).is_none())
    {
        println!("  | {line}");
    }
    judge(&stood, app)
}

fn run(cmd: &mut Command, main: &Path) -> Result<Stood, String> {
    // waits(measured): how long the bundle stood, for the verdict's wording — `stood`
    // is what decides
    let started = Instant::now();
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to start the bundle: {e}"))?;
    let pid = child.id();
    crate::budget::child_started(pid, &main.display().to_string());
    let out = child.stdout.take().map(collect);
    let err = child.stderr.take().map(collect);

    // An exit before the stretch ends is itself the answer.
    let left_early = match stood(Duration::from_millis(STAND_MS), LOOK_AGAIN, || {
        child.try_wait().map_err(|e| e.to_string()).transpose()
    }) {
        Ok(_stood_for) => {
            let _ = child.kill();
            let _ = child.wait();
            None
        }
        Err(ended) => Some(ended?),
    };

    let join = |h: Option<std::thread::JoinHandle<crate::app_out::Said>>| {
        h.and_then(|h| h.join().ok())
            .map(|said| said.lines)
            .unwrap_or_default()
    };
    Ok(Stood {
        pid,
        lines: join(err).into_iter().chain(join(out)).collect(),
        ran_for: started.elapsed(),
        left_early,
    })
}

fn judge(stood: &Stood, app: &Path) -> Result<(), String> {
    let failed_to_load = stood
        .lines
        .iter()
        .filter(|l| l.contains(LOAD_FAILED))
        .count();
    if failed_to_load > 0 {
        return Err(format!(
            "the bundle could not load its QML — {failed_to_load} line(s) above. A QML module \
             the bundle does not carry is the usual cause: macdeployqt's account above says \
             what it deployed (-qmldir)"
        ));
    }
    if !stood.lines.iter().any(|l| l.contains(STARTED)) {
        return Err(format!(
            "the bundle never said `{STARTED}` — it did not get as far as starting (where dyld \
             stopped it, its own line above says why), so nothing above is about its QML"
        ));
    }
    if let Some(status) = stood.left_early {
        return Err(format!(
            "the bundle ended on its own after {:.1}s (exit {}) — a window that is up stays up",
            stood.ran_for.as_secs_f32(),
            status.code().map_or("signal".into(), |c| c.to_string())
        ));
    }
    let loaded = bundle::loaded(&stood.lines, &app.display().to_string(), stood.pid);
    if loaded.inside == 0 {
        return Err(
            "dyld named no image of the app's from inside the bundle, so nothing says where its \
             Qt came from"
                .into(),
        );
    }
    if !loaded.outside.is_empty() {
        return Err(format!(
            "the bundle loaded {} image(s) from outside itself and the system — it stands only \
             on this machine:\n  {}",
            loaded.outside.len(),
            loaded.outside.join("\n  ")
        ));
    }
    println!(
        "the bundle stood for {:.1}s with its QML whole, on {} image(s) of its own",
        stood.ran_for.as_secs_f32(),
        loaded.inside
    );
    Ok(())
}
