//! `cargo xtask shipped` — the one run of the build nobody else here makes.
//!
//! Every other command builds the app with the verification harness
//! (`crate::tree::HARNESS_FEATURE`), because every other command drives it. The
//! shipped binary is the build without it, and the way it breaks is
//! particular: a QML file in `platitude.ui` that names a type from
//! `platitude.auto` resolves perfectly in a harness build and fails to
//! load `Main.qml` in this one — no window at all, from a line that reads
//! well.
//!
//! `cargo xtask structure` catches that statically ([`crate::structure`]
//! §modules); this catches everything it cannot, by starting the real
//! thing offscreen and reading what the QML engine says.
//!
//! **Bounded and reaped**, like every other app this runner starts: it
//! watches the window stand for a few seconds, long enough for the engine
//! to have finished complaining, then kills. There is no watchdog inside
//! a shipped build to do it — that is a harness knob, and this is the
//! build without one. The stand is the one wait in this runner whose end
//! is the answer rather than a failure (`wait::stood`): what is asked of
//! the window is that it stays, and one still there at the end of the
//! stretch stood for at least that long.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::app_out::collect;
use crate::wait::{LOOK_AGAIN, stood};

/// How long the window is watched standing. The engine reports a failed
/// load during `load_qml_from_file`, which is before the event loop
/// starts, so this only has to outlast the runtime, the store and the
/// first paint.
const STAND_MS: u64 = 8_000;

/// What the engine says when a type would not resolve. Everything else it
/// says about QML is a warning some run may legitimately draw.
const LOAD_FAILED: &str = "QQmlApplicationEngine failed to load component";

/// What the app says before anything else can go wrong (`main`), and so
/// what says it got as far as running at all.
const STARTED: &str = "build tree";

pub fn run(args: &[String]) -> Result<(), String> {
    let mut build = true;
    for arg in args {
        match arg.as_str() {
            "--no-build" => build = false,
            other => return Err(format!("shipped does not take {other:?}")),
        }
    }
    let root = crate::tree::workspace_root();
    // The release build without the harness is a compile like any other,
    // and is counted like one (`crate::budget`) whether a gate asked for
    // it or a session did.
    let _room = crate::budget::standalone(
        &root,
        if build {
            crate::budget::COMPILE
        } else {
            crate::budget::LIGHT
        },
        crate::budget::Rank::Normal,
        "shipped",
    )?;
    let path = crate::qt::path_with_qt()?;
    let exe = crate::tree::shipped_exe(&root, &path, build)?;

    // A directory of its own, so a run here never reads or writes the
    // settings of the person at this machine.
    let config = crate::keepsakes::keepsake_dir("shipped")?;
    let mut cmd = Command::new(&exe);
    // Nothing of the parent's automation reaches it: this run is about
    // the build that answers none of it, and a stray `PG_AUTO_ACT` in the
    // shell would be a run that is not the one being judged.
    crate::app_env::clear_automation(&mut cmd);
    cmd.current_dir(&config)
        .env("PATH", &path)
        .env("QT_QPA_PLATFORM", "offscreen")
        .env("QT_FORCE_STDERR_LOGGING", "1")
        .env("PG_CONFIG_DIR", &config)
        .env("PG_LOG", "info")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if cfg!(windows) {
        cmd.env("QT_QPA_FONTDIR", "C:\\Windows\\Fonts");
    }
    println!("running the shipped build offscreen: {}", config.display());

    // waits(measured): how long the build stood, for the verdict's wording — `stood`
    // is what decides
    let started = Instant::now();
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to start the app: {e}"))?;
    // What this unit is doing, for a ledger that outlives this process
    // (`crate::budget::child_started`).
    crate::budget::child_started(child.id(), &exe.display().to_string());
    let out = child.stdout.take().map(collect);
    let err = child.stderr.take().map(collect);

    // Watched standing, with a kill guard. A shipped build has no
    // watchdog of its own, so the stretch is the whole of what ends it —
    // and an exit before then is itself the answer.
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
    let lines: Vec<String> = join(err).into_iter().chain(join(out)).collect();
    for line in &lines {
        println!("  | {line}");
    }

    let failed_to_load: Vec<&String> = lines.iter().filter(|l| l.contains(LOAD_FAILED)).collect();
    if !failed_to_load.is_empty() {
        return Err(format!(
            "the shipped build could not load its QML — {} line(s) above. A type in \
             `platitude.ui` that only exists in `platitude.auto` is the usual cause \
             (.claude/rules/app-ui.md §QML モジュール)",
            failed_to_load.len()
        ));
    }
    if !lines.iter().any(|l| l.contains(STARTED)) {
        return Err(format!(
            "the shipped build never said `{STARTED}` — it did not get as far as \
             starting, so nothing above is about its QML"
        ));
    }
    if let Some(status) = left_early {
        return Err(format!(
            "the shipped build ended on its own after {:.1}s (exit {}) — a window that \
             is up stays up",
            started.elapsed().as_secs_f32(),
            status.code().map_or("signal".into(), |c| c.to_string())
        ));
    }
    println!(
        "PASS: the shipped build came up and stood for {:.1}s with its QML whole",
        started.elapsed().as_secs_f32()
    );
    Ok(())
}
