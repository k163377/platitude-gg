//! `cargo xtask check` — stage 2 (CLAUDE.md 確認は 3 段) with the host and
//! the container in parallel.
//!
//! The sides share nothing written: the host builds into this checkout's
//! target/, the container into its per-checkout volume on the engine
//! (linux.rs), and every demo repository gets its own temp directory.
//!
//! Within a side the steps stay sequential: they share its build
//! directory, so cargo would serialize them on its lock anyway, and a
//! build failure makes every later step noise.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::command::{self, Permission, Where};
use crate::wait::{Budget, LOOK_AGAIN, Wait};

pub(crate) static STAGE_TWO: command::Command = command::Command {
    id: "check.stage2",
    call: "check",
    purpose: "stage 2 in full, the host and the container in parallel",
    run_in: Where::Seat,
    needs: &["a --verb line for each verb the change touches"],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&STAGE_TWO];

/// How long a step may say nothing before it is killed and named: the long
/// steps keep talking, while a hang goes quiet first. Above the suite's own
/// `OVERALL_BUDGET` (900s), so that fires first and names the test. The
/// log growing renews it (`wait::Wait::saw`).
const QUIET_CEILING: Duration = Duration::from_secs(20 * 60);

/// The ceiling per step, for a hang that keeps talking. Also how long a
/// killed unit's leftover runs before it is reported; its room stays held
/// (`budget::Pool::leftover`).
pub(crate) const STEP_CEILING: Duration = Duration::from_secs(90 * 60);

pub fn run(args: &[String]) -> Result<(), String> {
    let mut verbs: Vec<String> = Vec::new();
    let mut at = 0;
    while let Some(arg) = args.get(at) {
        match arg.as_str() {
            "--verb" => {
                at += 1;
                verbs.push(args.get(at).ok_or("--verb needs a verify-ui verb")?.clone());
            }
            other => {
                return Err(format!(
                    "unknown option {other:?} (check takes --verb <v>…)"
                ));
            }
        }
        at += 1;
    }

    let root = crate::tree::workspace_root();
    // waits(measured): the run's wall clock, said at the end and judged by nothing
    let started = Instant::now();

    let words = |line: &[&str]| line.iter().map(|w| (*w).to_string()).collect::<Vec<_>>();
    // Not the `cargo xtask` alias: its `--quiet` swallows "Blocking waiting
    // for file lock", and a step blocked on a build lock would sit with an
    // empty log until the silence ceiling. `--locked` on every cargo here
    // (反映前テストの機械化.md「gate と check が撃つ cargo は全部 `--locked`」).
    let xtask = |line: &[&str]| {
        let mut step = words(&["cargo", "run", "--locked", "-p", "xtask", "--"]);
        step.extend(line.iter().map(|w| (*w).to_string()));
        step
    };
    let mut host_steps: Vec<Vec<String>> = vec![
        // What compiles nothing goes first: it answers in seconds, before
        // the minutes of compiling.
        xtask(&["structure"]),
        xtask(&["waits"]),
        xtask(&["versions"]),
        xtask(&["docs"]),
        xtask(&["qmltest"]),
        words(&["cargo", "fmt", "--all", "--", "--check"]),
        // `--all-features`: nothing else here compiles `automation` and
        // `memprobe`, both off by default. The arms they switch out are
        // compiled by the test line below.
        words(&[
            "cargo",
            "clippy",
            "--locked",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ]),
        words(&["cargo", "test", "--locked", "--workspace"]),
        // The verbs all build with the harness, and only the build without
        // it can fail to load its QML. The first verb rebuilds over the
        // featureless binary this leaves.
        xtask(&["shipped"]),
    ];
    let mut linux_steps: Vec<Vec<String>> = vec![
        xtask(&["linux", "test", "--locked", "-p", "platitude-core"]),
        // The other Qt build: these ask when a Canvas has painted, and
        // the two stacks paint through different software.
        xtask(&["linux", "qmltest"]),
        // The host's clippy never compiles #[cfg(not(windows))] code, which
        // CI's Linux and macOS jobs lint; `bare` compiles it too, but its
        // warnings are not errors.
        xtask(&[
            "linux",
            "clippy",
            "--locked",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ]),
    ];
    for (i, verb) in verbs.iter().enumerate() {
        // A --verb value is a whole verify-ui argument line ("co-authors 4
        // --preset co-authors"), given `--no-board` (`verify::suite_words`).
        // The first host run builds the release; the rest reuse it.
        let mut host = xtask(&["verify-ui"]);
        host.extend(crate::verify::suite_words(verb));
        if i > 0 {
            host.push("--no-build".to_string());
        }
        host_steps.push(host);
        // The container side too: a fingerprint check across the host
        // boundary is slow.
        let mut linux = xtask(&["linux", "verify-ui"]);
        linux.extend(crate::verify::suite_words(verb));
        if i > 0 {
            linux.push("--no-build".to_string());
        }
        linux_steps.push(linux);
    }
    // Last so it reuses the release the container's verify-ui just built.
    linux_steps.push(xtask(&["linux", "bare"]));

    let failures = both_sides(&root, &host_steps, &linux_steps)?;

    let minutes = started.elapsed().as_secs() / 60;
    let seconds = started.elapsed().as_secs() % 60;
    println!("check: {minutes}m{seconds:02}s wall clock");
    if verbs.is_empty() {
        // So a run without verbs cannot pass for the whole of stage 2.
        println!("check: no --verb given — verify-ui for the touched verbs still has to run");
    }
    if failures.is_empty() {
        println!("check: PASS");
        Ok(())
    } else {
        Err(format!("check failed: {}", failures.join(" / ")))
    }
}

/// Both sides at once, each on a thread of its own, and what came back
/// red. A side stops at its first failure (its later steps share the build
/// tree). Each step writes a log file, printed only on failure, so the
/// sides do not shred each other's lines. A file, not a pipe: reading a
/// pipe to EOF waits on every process that inherited it, so one straggler
/// would hold the check open; a file also keeps the tail of a step killed
/// at a ceiling.
///
/// One pool for the whole run, as the gate's (`gate::sides::run_sides`) —
/// not `budget::standalone`, which is for a command that is one unit.
fn both_sides(
    root: &Path,
    host_steps: &[Vec<String>],
    linux_steps: &[Vec<String>],
) -> Result<Vec<String>, String> {
    let pool = crate::budget::Pool::of(root, crate::budget::default_jobs())?;
    let seat = crate::budget::seat_of(root);
    let ground = Ground {
        root,
        pool: &pool,
        seat: &seat,
    };
    Ok(std::thread::scope(|scope| {
        let host = scope.spawn(|| run_side("host", &ground, host_steps));
        let linux = scope.spawn(|| run_side("linux", &ground, linux_steps));
        let mut failures = Vec::new();
        for handle in [host, linux] {
            match handle.join() {
                Ok(mut side_failures) => failures.append(&mut side_failures),
                Err(_) => failures.push("a side panicked".to_string()),
            }
        }
        failures
    }))
}

/// What both sides share: the tree, and the pool their steps are admitted
/// from.
#[derive(Clone, Copy)]
struct Ground<'a> {
    root: &'a Path,
    pool: &'a crate::budget::Pool,
    seat: &'a str,
}

fn run_side(side: &str, ground: &Ground<'_>, steps: &[Vec<String>]) -> Vec<String> {
    let root = ground.root;
    let logs = root.join("target").join("check-logs");
    if let Err(e) = std::fs::create_dir_all(&logs) {
        return vec![format!("{}: {e}", logs.display())];
    }
    for (index, step) in steps.iter().enumerate() {
        let display = step.join(" ");
        let log = logs.join(format!("{side}-{index:02}.log"));
        let room = ground
            .pool
            .admit_once_the_machine_is_free(&crate::budget::Ask {
                weight: crate::budget::weight_of(step, false),
                rank: crate::budget::Rank::Normal,
                seat: ground.seat,
                what: &display,
            });
        let room = match room {
            Ok(room) => room,
            Err(why) => return vec![format!("{display}: {why}")],
        };
        println!("[{side}] {display} … (log: {})", log.display());
        // waits(measured): the step's wall clock, said on its line and judged by nothing
        let at = Instant::now();
        let outcome = run_step(root, step, &log, &room, &|| false);
        drop(room);
        // Lossy: one localized byte in a linker or Qt line would
        // otherwise blank a failure's whole log.
        let text = String::from_utf8_lossy(&std::fs::read(&log).unwrap_or_default()).into_owned();
        let secs = at.elapsed().as_secs();
        match outcome {
            Ok(Stepped::Exited(true)) => {
                println!("[{side}] ok   {display} ({secs}s)");
                print_shots(side, &log);
            }
            // Nothing here asks a step to stop, so a stopped one is a
            // failure too.
            Ok(Stepped::Exited(false) | Stepped::Stopped) => {
                // The whole log: a cut one sends the reader to re-run it.
                println!("[{side}] FAIL {display} ({secs}s)");
                print!("{text}");
                return vec![display];
            }
            Err(why) => {
                println!("[{side}] FAIL {display} ({secs}s): {why}");
                for line in tail_of(&text).lines() {
                    println!("[{side}]      {line}");
                }
                return vec![format!("{display}: {why}")];
            }
        }
    }
    Vec::new()
}

/// Echoes where a verify-ui's PNGs landed: a green run still needs them
/// looked at (verify-ui skill).
pub(crate) fn print_shots(side: &str, log: &Path) {
    let text = String::from_utf8_lossy(&std::fs::read(log).unwrap_or_default()).into_owned();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("shot: ")
            || line.starts_with("screenshot: ")
            || line.contains("screenshots and settings:")
        {
            println!("[{side}]      {line}");
        }
    }
}

/// The end of a step's log, for a failure that has to say why in place.
pub(crate) fn log_tail(log: &Path) -> String {
    tail_of(&std::fs::read_to_string(log).unwrap_or_default())
}

/// The last lines of a step's output, as [`log_tail`] reads them.
pub(crate) fn tail_of(text: &str) -> String {
    const LINES: usize = 40;
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(LINES)..].join("\n")
}

/// How a step ended under [`run_step`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stepped {
    /// It exited, and this is whether it passed.
    Exited(bool),
    /// `stop` said so while it ran, and its process tree was ended — a
    /// gate that went red elsewhere (`gate::halt`).
    Stopped,
}

/// One step with both streams on its log file, watched. `Ok` is the step's
/// verdict or the stop it was ended for; `Err` is a ceiling, a spawn
/// failure or a Qt of another version on PATH (`crate::qt`). `stop` is
/// the gate's halt; everyone else hands `&|| false`. A
/// stop ends the step's process tree and nothing past it — a container the
/// step brought up keeps working — so hand `stop` only to a step whose
/// work ends with that tree or is reached by a mark (`gate::runner`).
pub(crate) fn run_step(
    root: &Path,
    step: &[String],
    log: &Path,
    room: &crate::budget::Admitted,
    stop: &dyn Fn() -> bool,
) -> Result<Stepped, String> {
    let out = std::fs::File::create(log).map_err(|e| format!("{}: {e}", log.display()))?;
    let err = out
        .try_clone()
        .map_err(|e| format!("{}: {e}", log.display()))?;
    // Announced as one build (`still`); the step runs under it and
    // announces nothing more.
    let _busy = crate::still::busy(root, &step.join(" "))?;
    let mut command = Command::new(&step[0]);
    command.args(&step[1..]).current_dir(root);
    // The pinned Qt for every cargo the step starts, whatever Qt the shell
    // that started the gate names (`crate::qt`). Where there is no Qt at
    // all, a step that needs none runs as it is and one that does fails on
    // its own; a qmake of another version stops the step here.
    if let Some(path) = crate::qt::path_with_qt_if_any()? {
        command.env("PATH", path);
    }
    crate::still::step(&mut command);
    // Every caller holds a ticket for this step (`room`), so what the step
    // starts takes no second one.
    crate::budget::under(&mut command);
    // So that a step ended at a ceiling takes its cargo's rustc with it,
    // and this side's build lock stays free (`reap`).
    crate::reap::own_group(&mut command);
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err))
        .spawn()
        .map_err(|e| e.to_string())?;
    // The ticket is this runner's but the load is the child's, which a
    // ledger outliving this process has to find (`budget::Pool::leftover`).
    room.started(child.id(), &step[0]);
    let mut wait = Wait::new(
        "the step",
        Budget::of(QUIET_CEILING, STEP_CEILING),
        LOOK_AGAIN,
    );
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return Ok(Stepped::Exited(status.success()));
        }
        if stop() {
            let (reaped, _ended) = crate::reap::reap(&mut child);
            // Into the step's log: it did not fail, it was ended.
            let note = format!(
                "\n[gate] ended here: the run went red elsewhere — {}\n",
                reaped.line()
            );
            if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(log) {
                use std::io::Write;
                let _unsaid = file.write_all(note.as_bytes());
            }
            return Ok(Stepped::Stopped);
        }
        let len = std::fs::metadata(log).map(|m| m.len()).unwrap_or(0);
        wait.saw(format!("{len} bytes of log"));
        if let Err(expired) = wait.look_again("its exit") {
            let (reaped, _ended) = crate::reap::reap(&mut child);
            return Err(format!(
                "killed at the ceiling — {expired}, and {} (its log has the tail; an empty \
                 log usually means it never got past a build lock another cargo holds)",
                reaped.line()
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Stepped, run_step};
    use crate::budget::{Ask, Pool, Rank};
    use crate::yard::Yard;

    /// A step that says it is running, then outlives any test: what ends
    /// it has to be the stop.
    fn sleeper() -> Vec<String> {
        let line: &[&str] = if cfg!(windows) {
            &[
                "powershell",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Write-Output ready; Start-Sleep -Seconds 600",
            ]
        } else {
            &["sh", "-c", "echo ready; sleep 600"]
        };
        line.iter().map(|word| (*word).to_string()).collect()
    }

    /// Stopped only once the step says it is running, so `Stopped` is a
    /// step ended mid-run, not one that never started; the log says so.
    #[test]
    fn a_step_told_to_stop_is_ended_where_it_stands() {
        let dir = Yard::new("check-stop");
        let room = Pool::at(&dir, 24)
            .admit(&Ask {
                weight: 1,
                rank: Rank::Normal,
                seat: "t",
                what: "the sleeper",
            })
            .unwrap();
        let log = dir.join("step.log");
        let running = || std::fs::read_to_string(&log).is_ok_and(|text| text.contains("ready"));
        let ended = run_step(&dir, &sleeper(), &log, &room, &running).unwrap();
        assert_eq!(ended, Stepped::Stopped);
        let text = std::fs::read_to_string(&log).unwrap();
        assert!(
            text.contains("ended here: the run went red elsewhere"),
            "{text}"
        );
    }
}
