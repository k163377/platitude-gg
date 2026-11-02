//! `cargo xtask shipped` — the one run of the build nobody else here makes.
//!
//! Every other command builds with the verification harness
//! (`crate::app_build::HARNESS_FEATURE`). A QML file in `platitude.ui` that
//! names a type from `platitude.auto` resolves in a harness build and
//! fails to load `Main.qml` in this one — no window at all.
//! `cargo xtask structure` catches that statically ([`crate::structure`]
//! §modules); this catches the rest by starting the real thing offscreen
//! and reading what the QML engine says.
//!
//! The run waits for the app's word that its window loaded ([`LOADED`]):
//! a load takes as long as the machine makes it, and a stretch of clock
//! that ends first says nothing about the QML. Then the window stands
//! ([`STAND`]) — the one wait whose end is the answer (`wait::stood`): a
//! window still there at the end stood that long. This runner then ends
//! it: a shipped build has no watchdog (that is a harness knob). The end is
//! the stop an IDE sends, which the window must answer as its own close —
//! Ctrl+C through a console on Windows ([`console`]), SIGINT elsewhere
//! ([`signal`]); a failed run ends with a kill.

use std::ffi::OsStr;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::app_out::{Said, collect_marking};
use crate::command::{self, Permission, Where};
use crate::wait::{Budget, Expired, LOOK_AGAIN, Wait, stood};

#[cfg(windows)]
mod console;
#[cfg(unix)]
mod signal;

pub(crate) static SHIPPED: command::Command = command::Command {
    id: "shipped.build",
    call: "shipped",
    purpose: "the build without the verification harness — the one nobody else makes",
    run_in: Where::Seat,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&SHIPPED];

/// How long the window's QML may take to load. A load costs the engine
/// compiling every file of the window when its disk cache is cold, on a
/// machine that may be running a gate's verbs beside it; this is many
/// times that, so a run that reaches it is a load that never ends.
const LOAD_CEILING: Duration = Duration::from_secs(300);

/// How long the window must stay up once it loaded: the event loop's
/// first turns — the first paint, the startup's own asks — are where a
/// window that loaded can still fall over.
const STAND: Duration = Duration::from_secs(3);

/// The engine's line for a type that would not resolve — the only QML
/// warning that fails the run; the others a run may legitimately draw.
const LOAD_FAILED: &str = "QQmlApplicationEngine failed to load component";

/// The app's first line (`main`): proof it got as far as running.
const STARTED: &str = "build tree";

/// The app's line once `Main.qml` loaded (`main`): the only proof the
/// window's QML is whole — the warnings a load draws, or their absence,
/// say nothing of whether it ended. Kept by hand beside the app's (xtask
/// links no app crate, and no file may read a crate root): a rewording on
/// either side is a run that waits out its ceiling.
const LOADED: &str = "window loaded";

/// The app's line once a stop from outside reached its close
/// (`app_backend/lifecycle.rs`), kept by hand as [`LOADED`] is.
const DELIVERED: &str = "a stop from outside requested window close";

/// How long a window asked to close may take to end: the settings flush
/// and `Hub::shutdown` over an empty store, many times over.
const CLOSE_CEILING: Duration = Duration::from_secs(30);

/// How the run ended, as the wait saw it.
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

pub fn run(args: &[String]) -> Result<(), String> {
    let mut build = true;
    for arg in args {
        match arg.as_str() {
            "--no-build" => build = false,
            other => return Err(format!("shipped does not take {other:?}")),
        }
    }
    #[cfg(windows)]
    if let Some(ran) = console::run_as_parent() {
        return ran;
    }
    let root = crate::tree::workspace_root();
    // Counted like any compile, whether a gate asked for it or a session.
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
    let exe =
        crate::app_build::shipped_exe(&root, &path, crate::app_build::BuildEnv::default(), build)?;

    #[cfg(windows)]
    if !console::is_host() {
        drop(_room);
        return console::run_host();
    }
    #[cfg(windows)]
    console::protect()?;

    // Its own directory, never the settings of the person at this machine.
    let config = crate::keepsakes::keepsake_dir("shipped")?;
    println!("running the shipped build offscreen: {}", config.display());

    let mut wait = Wait::new("the shipped build", Budget::whole(LOAD_CEILING), LOOK_AGAIN);
    let mut window = Started::spawn(app_command(&exe, &path, &config), &exe)?;
    let ending = wait_out(&mut window.child, &window.loaded, &mut wait);
    let stopped = if matches!(ending, Ok(Ending::StillUp { .. })) {
        stop(&mut window.child)
    } else {
        Ok(())
    };
    let lines = window.finish(matches!(ending, Ok(Ending::LeftEarly { .. })));
    let ending = ending?;
    for line in &lines {
        println!("  | {line}");
    }
    println!("{}", verdict(&lines, &ending)?);
    stopped?;
    check_delivery(&lines)?;
    #[cfg(windows)]
    {
        console::check_left_cargo(&exe)?;
        console::check_other_parent(&exe)?;
    }
    #[cfg(unix)]
    signal::check_ignored_hangup(&exe, &path)?;
    Ok(())
}

/// A window this run started, its output read as it arrives.
struct Started {
    child: Child,
    /// Raised by the line itself, the moment it arrives.
    loaded: Arc<AtomicBool>,
    out: Option<JoinHandle<Said>>,
    err: Option<JoinHandle<Said>>,
}

impl Started {
    fn spawn(mut cmd: Command, exe: &Path) -> Result<Self, String> {
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("failed to start the app: {e}"))?;
        crate::budget::child_started(child.id(), &exe.display().to_string());
        let loaded = Arc::new(AtomicBool::new(false));
        let out = child
            .stdout
            .take()
            .map(|pipe| collect_marking(pipe, Arc::clone(&loaded), said_loaded));
        let err = child
            .stderr
            .take()
            .map(|pipe| collect_marking(pipe, Arc::clone(&loaded), said_loaded));
        Ok(Self {
            child,
            loaded,
            out,
            err,
        })
    }

    /// What it said, stderr first. Killed on every road but its own exit
    /// (`left`), a failed look included: a shipped build has no watchdog to
    /// end it. A window that answered its stop has already gone, and the
    /// kill finds nothing.
    fn finish(mut self, left: bool) -> Vec<String> {
        if !left {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        let join = |h: Option<JoinHandle<Said>>| {
            h.and_then(|h| h.join().ok())
                .map(|said| said.lines)
                .unwrap_or_default()
        };
        join(self.err).into_iter().chain(join(self.out)).collect()
    }
}

/// The stop an IDE sends the window it waits on.
#[cfg(windows)]
fn stop(child: &mut Child) -> Result<(), String> {
    close_after(child, "a waiting cargo's Ctrl+C", console::send)
}

/// The stop an IDE sends the window it waits on.
#[cfg(unix)]
fn stop(child: &mut Child) -> Result<(), String> {
    let pid = child.id();
    close_after(child, "an IDE's SIGINT", || {
        signal::send(pid, signal::SIGINT)
    })
}

/// Sends a window that stood its `stop`, and waits for a normal exit.
fn close_after(
    child: &mut Child,
    stop: &str,
    send: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    send()?;
    let mut wait = Wait::new(
        "the shipped build",
        Budget::whole(CLOSE_CEILING),
        LOOK_AGAIN,
    );
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            if !status.success() {
                return Err(format!("{stop} ended the shipped build abruptly: {status}"));
            }
            println!("PASS: {stop} closed the shipped build normally");
            return Ok(());
        }
        wait.look_again(&format!("a normal exit after {stop}"))
            .map_err(|e| e.to_string())?;
    }
}

/// The close the stop asked for went the window's own way.
fn check_delivery(lines: &[String]) -> Result<(), String> {
    if !lines.iter().any(|line| line.contains(DELIVERED)) {
        return Err("the stop did not reach the window's close route".into());
    }
    Ok(())
}

/// The shipped build as every run of this command starts it: offscreen,
/// on a settings directory of its own. Its output goes where the caller
/// says.
fn app_command(exe: &Path, path: &OsStr, config: &Path) -> Command {
    let mut cmd = Command::new(exe);
    // A stray `PGG_AUTO_ACT` in the shell would make this a run other
    // than the one being judged.
    crate::app_env::clear_automation(&mut cmd);
    cmd.current_dir(config)
        .env("PATH", path)
        .env("QT_QPA_PLATFORM", "offscreen")
        .env("QT_FORCE_STDERR_LOGGING", "1")
        .env("PGG_CONFIG_DIR", config)
        .env("PGG_LOG", "info");
    crate::qt::offscreen_fonts(&mut cmd);
    // Apart from the cache the person's own windows use (`verify::child`).
    cmd.env("QML_DISK_CACHE_PATH", exe.with_file_name("qmlcache"));
    cmd
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

/// The run's verdict: what Qt and the app said first, then how it ended.
/// A pass needs the app's [`LOADED`] among its lines.
fn verdict(lines: &[String], ending: &Ending) -> Result<String, String> {
    let failed_to_load = lines.iter().filter(|l| l.contains(LOAD_FAILED)).count();
    if failed_to_load > 0 {
        return Err(format!(
            "the shipped build could not load its QML — {failed_to_load} line(s) above. A type \
             in `platitude.ui` that only exists in `platitude.auto` is the usual cause \
             (.claude/rules/app-ui.md「QML モジュールは 2 つ」)"
        ));
    }
    if !lines.iter().any(|l| l.contains(STARTED)) {
        return Err(format!(
            "the shipped build never said `{STARTED}` ({}) — it did not get as far as \
             starting, so nothing above is about its QML",
            ending.account()
        ));
    }
    let loaded = lines.iter().any(|l| said_loaded(l));
    match ending {
        Ending::OutOfTime(expired) => Err(format!(
            "{expired} — it never said `{LOADED}`, so nothing says its QML is whole"
        )),
        Ending::LeftEarly { .. } if loaded => Err(format!(
            "the shipped build said `{LOADED}`, then {} — a window that is up stays up",
            ending.account()
        )),
        Ending::StillUp {
            loaded_at,
            stood_for,
        } if loaded => Ok(format!(
            "PASS: the shipped build said `{LOADED}` {:.1}s in and stood {:.1}s more",
            loaded_at.as_secs_f32(),
            stood_for.as_secs_f32()
        )),
        Ending::LeftEarly { .. } | Ending::StillUp { .. } => Err(format!(
            "the shipped build never said `{LOADED}` ({}), so nothing says its QML is whole",
            ending.account()
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Ending, verdict};
    use crate::wait::{Budget, Wait};

    /// Two of the app's lines, as it writes them.
    const BUILD_TREE: &str = "2026-10-05T00:13:24.294891Z  INFO platitude_gg: build tree \
                              tree=\"a\" tag=\"v0.0.0\" release=false commit=\"df976d24\"";
    const WINDOW_LOADED: &str = "2026-10-05T00:13:24.888380Z  INFO platitude_gg: window loaded";

    fn said(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|line| line.to_string()).collect()
    }

    fn still_up() -> Ending {
        Ending::StillUp {
            loaded_at: Duration::from_millis(1_500),
            stood_for: Duration::from_secs(3),
        }
    }

    fn out_of_time() -> Ending {
        Ending::OutOfTime(
            Wait::new(
                "the shipped build",
                Budget::whole(Duration::ZERO),
                Duration::ZERO,
            )
            .check("its window to load")
            .expect_err("no budget at all"),
        )
    }

    #[test]
    fn a_window_that_loaded_and_stood_passes_and_nothing_else_does() {
        let pass = verdict(&said(&[BUILD_TREE, WINDOW_LOADED]), &still_up()).expect("it stood");
        assert!(pass.starts_with("PASS"), "{pass}");

        let unsaid = verdict(&said(&[BUILD_TREE]), &still_up()).expect_err("no line, no pass");
        assert!(unsaid.contains("never said `window loaded`"), "{unsaid}");

        let red = verdict(&said(&[BUILD_TREE]), &out_of_time()).expect_err("never loaded");
        assert!(red.contains("never said `window loaded`"), "{red}");
        assert!(
            red.contains("still waiting for its window to load"),
            "{red}"
        );
    }

    #[test]
    fn an_exit_is_red_and_says_whether_the_window_had_loaded() {
        let left = Ending::LeftEarly {
            code: Some(1),
            after: Duration::from_secs(12),
        };
        let before = verdict(&said(&[BUILD_TREE]), &left).expect_err("an exit");
        assert!(before.contains("never said `window loaded`"), "{before}");
        assert!(before.contains("exit 1"), "{before}");
        let after = verdict(&said(&[BUILD_TREE, WINDOW_LOADED]), &left).expect_err("an exit");
        assert!(after.contains("said `window loaded`, then"), "{after}");
    }

    #[test]
    fn qts_word_that_the_qml_would_not_load_comes_before_how_the_run_ended() {
        let lines = said(&[
            BUILD_TREE,
            "QQmlApplicationEngine failed to load component",
            "qrc:/qt/qml/platitude/ui/NoSuchRoot.qml: No such file or directory",
            "2026-10-05T00:06:38.670499Z ERROR platitude_gg: the window's QML would not load \
             url=qrc:/qt/qml/platitude/ui/NoSuchRoot.qml",
        ]);
        let left = Ending::LeftEarly {
            code: Some(1),
            after: Duration::from_millis(100),
        };
        let red = verdict(&lines, &left).expect_err("the load failed");
        assert!(red.contains("could not load its QML — 1 line(s)"), "{red}");
    }

    #[test]
    fn a_build_that_never_got_going_says_how_it_ended() {
        // A library the loader could not find (STATUS_DLL_NOT_FOUND): no
        // line at all, and the exit is all there is to say.
        let unstarted = Ending::LeftEarly {
            code: Some(-1_073_741_515),
            after: Duration::from_millis(100),
        };
        let never = verdict(&[], &unstarted).expect_err("it never started");
        assert!(never.contains("never said `build tree`"), "{never}");
        assert!(never.contains("exit -1073741515"), "{never}");

        let hung = verdict(&[], &out_of_time()).expect_err("it never started");
        assert!(hung.contains("ceiling, and was killed"), "{hung}");
        assert_eq!(hung.matches("the shipped build").count(), 1, "{hung}");
    }
}
