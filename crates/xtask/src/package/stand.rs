//! The bundle started once, offscreen, from inside itself: judged as
//! `cargo xtask shipped` judges a build — the app said it started, then
//! that its window loaded, and it was still up a short stand after that —
//! and then by dyld's own account of the app's process: every image not
//! the system's came from inside the bundle.
//!
//! The stand `shipped` runs, written a second time rather than shared
//! (.claude/rules/structure.md §共通化: two places are left as they are).

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::bundle;
use crate::app_out::collect_marking;
use crate::wait::{Budget, Expired, LOOK_AGAIN, Wait, stood};

/// As long as `shipped` waits for a window to load, for its reason: a
/// load costs the engine compiling every file of the window when its disk
/// cache is cold — the bundle's always is — and this is many times that,
/// so a run that reaches it is a load that never ends.
const LOAD_CEILING: Duration = Duration::from_secs(300);

/// As long as `shipped` stands a window once it loaded: the event loop's
/// first turns are where a window that loaded can still fall over.
const STAND: Duration = Duration::from_secs(3);

/// The engine's line for a type that would not resolve — the one QML
/// warning `shipped` fails a build on.
const LOAD_FAILED: &str = "QQmlApplicationEngine failed to load component";

/// The app's first line (`main`): proof it got as far as running.
const STARTED: &str = "build tree";

/// The app's line once `Main.qml` loaded (`main`): the only proof the
/// bundle's QML is whole. Kept by hand, as `shipped` keeps it.
const LOADED: &str = "window loaded";

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

/// What the bundle said, and how its run ended.
struct Stood {
    /// The app's own process, which a line its children wrote does not name.
    pid: u32,
    /// Its stderr, then its stdout.
    lines: Vec<String>,
    ending: Ending,
}

/// How the bundle's run ended, as the wait saw it.
enum Ending {
    /// Still up a [`STAND`] after its window loaded: `loaded_at` into the
    /// run, for `stood_for`.
    StillUp {
        loaded_at: Duration,
        stood_for: Duration,
    },
    /// It ended on its own, `after` into the run; `code` is `None` for a
    /// signal.
    LeftEarly { code: Option<i32>, after: Duration },
    /// Its window had not loaded at the ceiling, and this runner killed it.
    OutOfTime(Expired),
}

impl Ending {
    /// How the run ended, as a verdict quotes it.
    fn account(&self) -> String {
        match self {
            Ending::StillUp {
                loaded_at,
                stood_for,
            } => format!(
                "it was still up {:.1}s in",
                (*loaded_at + *stood_for).as_secs_f32()
            ),
            Ending::LeftEarly { code, after } => format!(
                "it ended on its own {:.1}s in, exit {}",
                after.as_secs_f32(),
                code.map_or("signal".into(), |c| c.to_string())
            ),
            Ending::OutOfTime(_) => format!(
                "it was still up at the {}s ceiling, and was killed",
                LOAD_CEILING.as_secs()
            ),
        }
    }
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
    println!("{}", judge(&stood, app)?);
    Ok(())
}

fn run(cmd: &mut Command, main: &Path) -> Result<Stood, String> {
    let mut wait = Wait::new("the bundle", Budget::whole(LOAD_CEILING), LOOK_AGAIN);
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to start the bundle: {e}"))?;
    let pid = child.id();
    crate::budget::child_started(pid, &main.display().to_string());
    // Raised by the line itself, the moment it arrives.
    let loaded = Arc::new(AtomicBool::new(false));
    let out = child
        .stdout
        .take()
        .map(|pipe| collect_marking(pipe, Arc::clone(&loaded), said_loaded));
    let err = child
        .stderr
        .take()
        .map(|pipe| collect_marking(pipe, Arc::clone(&loaded), said_loaded));

    let ending = wait_out(&mut child, &loaded, &mut wait);
    // Killed on every road but its own exit, a failed look included: the
    // app has no watchdog to end it.
    if !matches!(ending, Ok(Ending::LeftEarly { .. })) {
        let _ = child.kill();
        let _ = child.wait();
    }
    let ending = ending?;

    let join = |h: Option<std::thread::JoinHandle<crate::app_out::Said>>| {
        h.and_then(|h| h.join().ok())
            .map(|said| said.lines)
            .unwrap_or_default()
    };
    Ok(Stood {
        pid,
        lines: join(err).into_iter().chain(join(out)).collect(),
        ending,
    })
}

/// Waits for the window to load, then stands it.
fn wait_out(child: &mut Child, loaded: &AtomicBool, wait: &mut Wait) -> Result<Ending, String> {
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return Ok(Ending::LeftEarly {
                code: status.code(),
                after: wait.elapsed(),
            });
        }
        if loaded.load(Ordering::SeqCst) {
            let loaded_at = wait.elapsed();
            // An exit before the stand ends is itself the answer.
            return match stood(STAND, LOOK_AGAIN, || {
                child.try_wait().map_err(|e| e.to_string()).transpose()
            }) {
                Ok(stood_for) => Ok(Ending::StillUp {
                    loaded_at,
                    stood_for,
                }),
                Err(ended) => Ok(Ending::LeftEarly {
                    code: ended?.code(),
                    after: wait.elapsed(),
                }),
            };
        }
        if let Err(expired) = wait.look_again("its window to load") {
            return Ok(Ending::OutOfTime(expired));
        }
    }
}

fn said_loaded(line: &str) -> bool {
    line.contains(LOADED)
}

/// The stand's verdict: what Qt and the app said first, then how the run
/// ended, then where its images came from. A pass needs the app's
/// [`LOADED`] among its lines.
fn judge(stood: &Stood, app: &Path) -> Result<String, String> {
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
            "the bundle never said `{STARTED}` ({}) — it did not get as far as starting (where \
             dyld stopped it, its own line above says why), so nothing above is about its QML",
            stood.ending.account()
        ));
    }
    let window_loaded = stood.lines.iter().any(|l| said_loaded(l));
    let (loaded_at, stood_for) = match &stood.ending {
        Ending::OutOfTime(expired) => {
            return Err(format!(
                "{expired} — it never said `{LOADED}`, so nothing says its QML is whole"
            ));
        }
        Ending::LeftEarly { .. } if window_loaded => {
            return Err(format!(
                "the bundle said `{LOADED}`, then {} — a window that is up stays up",
                stood.ending.account()
            ));
        }
        Ending::StillUp {
            loaded_at,
            stood_for,
        } if window_loaded => (*loaded_at, *stood_for),
        Ending::LeftEarly { .. } | Ending::StillUp { .. } => {
            return Err(format!(
                "the bundle never said `{LOADED}` ({}), so nothing says its QML is whole",
                stood.ending.account()
            ));
        }
    };
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
    Ok(format!(
        "the bundle said `{LOADED}` {:.1}s in and stood {:.1}s more, on {} image(s) of its own",
        loaded_at.as_secs_f32(),
        stood_for.as_secs_f32(),
        loaded.inside
    ))
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use super::{Ending, Stood, judge};
    use crate::wait::{Budget, Wait};

    const APP: &str = "/Users/runner/work/_temp/package/Platitude GG.app";

    /// What the bundle's process says, as dyld and the app write it: an
    /// image of its own, then the app's first line.
    fn started() -> Vec<String> {
        [
            "dyld[812]: <9F848759-9AB8-3BD2-96A1-C069DC1FFD43> /Users/runner/work/_temp/package/Platitude GG.app/Contents/MacOS/platitude-gg",
            "2026-10-05T00:13:24.294891Z  INFO platitude_gg: build tree tree=\"-\" tag=\"v0.0.0\" release=false commit=\"df976d24\"",
        ]
        .map(String::from)
        .to_vec()
    }

    const WINDOW_LOADED: &str = "2026-10-05T00:13:24.888380Z  INFO platitude_gg: window loaded";

    fn stand(lines: Vec<String>, ending: Ending) -> Stood {
        Stood {
            pid: 812,
            lines,
            ending,
        }
    }

    fn still_up() -> Ending {
        Ending::StillUp {
            loaded_at: Duration::from_millis(1_500),
            stood_for: Duration::from_secs(3),
        }
    }

    #[test]
    fn a_bundle_whose_window_never_said_it_loaded_is_red_whatever_dyld_says() {
        let expired = Wait::new("the bundle", Budget::whole(Duration::ZERO), Duration::ZERO)
            .check("its window to load")
            .expect_err("no budget at all");
        let red = judge(
            &stand(started(), Ending::OutOfTime(expired)),
            Path::new(APP),
        )
        .expect_err("never loaded");
        assert!(red.contains("never said `window loaded`"), "{red}");

        let left = Ending::LeftEarly {
            code: Some(1),
            after: Duration::from_secs(12),
        };
        let red = judge(&stand(started(), left), Path::new(APP)).expect_err("an exit");
        assert!(red.contains("never said `window loaded`"), "{red}");

        let red = judge(&stand(started(), still_up()), Path::new(APP)).expect_err("no line");
        assert!(red.contains("never said `window loaded`"), "{red}");
    }

    #[test]
    fn a_bundle_that_never_got_going_says_how_it_ended() {
        // dyld stopping the app before its first line ends it by a signal.
        let lines =
            vec!["dyld[812]: Library not loaded: @rpath/QtSvg.framework/Versions/A/QtSvg".into()];
        let aborted = Ending::LeftEarly {
            code: None,
            after: Duration::from_millis(100),
        };
        let red = judge(&stand(lines.clone(), aborted), Path::new(APP)).expect_err("no start");
        assert!(red.contains("never said `build tree`"), "{red}");
        assert!(red.contains("exit signal"), "{red}");

        let expired = Wait::new("the bundle", Budget::whole(Duration::ZERO), Duration::ZERO)
            .check("its window to load")
            .expect_err("no budget at all");
        let hung =
            judge(&stand(lines, Ending::OutOfTime(expired)), Path::new(APP)).expect_err("no start");
        assert!(hung.contains("ceiling, and was killed"), "{hung}");
        assert_eq!(hung.matches("the bundle").count(), 1, "{hung}");
    }

    #[test]
    fn a_bundle_whose_window_loaded_and_stood_is_judged_on_dyld_s_account() {
        let mut lines = started();
        lines.push(WINDOW_LOADED.into());
        let pass = judge(&stand(lines.clone(), still_up()), Path::new(APP)).expect("it stood");
        assert!(pass.contains("on 1 image(s) of its own"), "{pass}");

        lines.push(
            "dyld[812]: <2B3C4D5E-6F7A-3B2C-9D3E-4F5A6B7C8D9E> /Users/runner/work/_temp/qt/6.12.0/macos/lib/QtGui.framework/Versions/A/QtGui"
                .into(),
        );
        let red = judge(&stand(lines, still_up()), Path::new(APP)).expect_err("Qt from outside");
        assert!(red.contains("1 image(s) from outside itself"), "{red}");
    }
}
