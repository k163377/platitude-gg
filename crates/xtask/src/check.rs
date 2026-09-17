//! `cargo xtask check` — stage-2 verification (CLAUDE.md 確認は 3 段) with
//! the host and the container running in parallel.
//!
//! The two sides share nothing that is written: the host builds into this
//! checkout's target/ while the container builds into its per-checkout
//! docker volume (linux.rs), and every demo repository lands in its own
//! temp directory. So the wall clock is whichever side finishes last,
//! not the sum — the container chain alone runs for minutes, and running
//! it beside the host chain gives that time back. Parallel sessions were
//! already isolated a boundary further out (worktree target/ + per-checkout
//! volume); this is the same idea inside one session.
//!
//! Within a side the steps stay sequential on purpose: they share that
//! side's build directory, so cargo would serialize them on its lock
//! anyway, and a build failure makes every later step of that side noise.

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

/// How long a step may say nothing before it is killed and named. Silence,
/// not wall time: the long steps (a first container image build, a release
/// link) all keep talking, while every hang this has to catch — a test
/// deadlocked past its own backstops, a cargo blocked on another cargo's
/// build lock, a docker CLI waiting out a wedged daemon — goes quiet first.
/// Generous on purpose: the suite's own 900s backstop must fire before
/// this one so the failure carries a test name, and a link is the longest
/// legitimately silent stretch. The log growing is what renews it
/// (`wait::Wait::saw`).
const QUIET_CEILING: Duration = Duration::from_secs(20 * 60);

/// The absolute ceiling per step, for a hang that keeps talking. Also
/// how long a killed unit's leftovers may hold the machine's budget,
/// which is the same question asked from the other side: past the
/// longest a step may run, what is at that number is not that step
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
    // The `cargo xtask` alias runs `--quiet`, which swallows cargo's
    // "Blocking waiting for file lock" line — and a step blocked on this
    // side's own build lock would then sit with an empty log until the
    // silence ceiling calls it a hang. Spelled out unquieted here, so the
    // lock line (and the compile lines, which are liveness) reach the log.
    // `--locked` on every cargo here, as on the gate's: a cargo that would
    // rewrite `Cargo.lock` says so and stops (反映前テストの機械化.md §gate).
    let xtask = |line: &[&str]| {
        let mut step = words(&["cargo", "run", "--locked", "-p", "xtask", "--"]);
        step.extend(line.iter().map(|w| (*w).to_string()));
        step
    };
    let mut host_steps: Vec<Vec<String>> = vec![
        // First because it is the cheapest thing here that can fail — it
        // builds nothing and answers in a second or two, and a length backstop
        // is not worth finding out about after ten minutes of compiling.
        xtask(&["structure"]),
        // Same reasoning, same cost: a naked wait is a hang the suite
        // cannot name, and this answers before anything compiles.
        xtask(&["waits"]),
        // And again: a torn markdown block compiles nothing and shows
        // nothing in the source, so it is worth a second before the ten
        // minutes.
        xtask(&["docs"]),
        // The QtTest files, which compile nothing of the app either: the
        // product's QML is staged into an import tree and handed to Qt's
        // own runner, and a whole file answers in a fraction of a second.
        xtask(&["qmltest"]),
        words(&["cargo", "fmt", "--all", "--", "--check"]),
        // `--all-features`, because the code a feature switches off is
        // code nothing else here compiles: the verification harness
        // (`automation`) and the counting allocator (`memprobe`) are
        // both off by default, and a warning inside either would
        // otherwise reach nobody until a release run. The arms those two
        // features switch *out* are compiled by the test line below,
        // which runs on the default set.
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
        // Before the verbs, and for the reason they cannot answer for it:
        // every one of them builds the app *with* the harness, and the
        // build without it is the only one that can fail to load its QML
        // at all. It leaves a featureless binary behind, which the first
        // verb below rebuilds over.
        xtask(&["shipped"]),
    ];
    let mut linux_steps: Vec<Vec<String>> = vec![
        xtask(&["linux", "test", "--locked", "-p", "platitude-core"]),
        // The other Qt build: these ask when a Canvas has painted, and
        // the two stacks paint through different software.
        xtask(&["linux", "qmltest"]),
        // The same line as the host's clippy above, because the host's
        // cannot answer for it: a name reachable only under
        // #[cfg(not(windows))] is not compiled on Windows at all, so an
        // unused import or an orphaned fn behind that cfg passes here and
        // fails the Linux and macOS jobs the first time CI runs
        // (.github/workflows/ci.yml runs this across the whole matrix).
        // `bare` is the only other thing on this side that compiles the
        // app for Linux, and it is a release build whose warnings are not
        // errors and which nobody reads.
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
        // A --verb value is a whole verify-ui argument line — some verbs
        // only mean anything with their preset or argument beside them
        // ("co-authors 4 --preset co-authors") — and it comes back with
        // `--no-board`, because a suite's pictures are not the ones
        // anybody asked to look at (`verify::suite_words`).
        // The first host run builds the release; the rest reuse it.
        let mut host = xtask(&["verify-ui"]);
        host.extend(crate::verify::suite_words(verb));
        if i > 0 {
            host.push("--no-build".to_string());
        }
        host_steps.push(host);
        // The container side reuses its first build too: a fingerprint
        // check across the host boundary is measurably slow, and paying
        // it once per verb bought nothing.
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
        // Said out loud so a run without verbs cannot pass for the whole
        // of stage 2 — Done needs the verbs the change touched, on both
        // OSes (verify-ui skill).
        println!("check: no --verb given — verify-ui for the touched verbs still has to run");
    }
    if failures.is_empty() {
        println!("check: PASS");
        Ok(())
    } else {
        Err(format!("check failed: {}", failures.join(" / ")))
    }
}

/// Runs one side's steps in order, stopping the side at its first failure
/// (later steps of a side depend on the same build tree). Output goes to a
/// log file per step and is only printed for the step that failed, so two
/// sides can speak at once without shredding each other's lines; the start
/// and verdict lines carry the liveness.
///
/// A file, so this always comes back: reading a pipe to
/// EOF waits on every process that inherited its write end, and one
/// straggler a killed or finished child left behind (a wedged git, an
/// orphaned test binary) would hold the whole check open after every test
/// had already answered. The file also survives a hang — when a step is
/// killed at a ceiling, its tail says what the step was doing, which a
/// pipe lost in a buffer cannot.
/// Both sides at once, each on a thread of its own, and what came back
/// red.
///
/// One pool for the whole run, as the gate builds one for its sides
/// (`gate::run_sides`): this verb drives steps, and a runner's steps are
/// units of the machine's budget the same way a gate's are. Not the
/// standalone road — that one is for a command that *is* one unit and
/// holds one ticket for its whole life (`budget::standalone`).
fn both_sides(
    root: &Path,
    host_steps: &[Vec<String>],
    linux_steps: &[Vec<String>],
) -> Result<Vec<String>, String> {
    let pool = crate::budget::Pool::of(root, crate::gate::default_jobs())?;
    let seat = crate::gate::seat_of(root);
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

/// What both of this verb's sides stand on: the tree, and the machine's
/// budget their steps are admitted out of.
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
        // One step is one unit of the machine, here as in the gate
        // (`crate::budget`): this verb is the older road to the same
        // work, and a machine full of seats counts what runs on it
        // however it was started.
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
        let outcome = run_step(root, step, &log, &room);
        drop(room);
        // Lossy: one localized byte in a linker or Qt line would
        // otherwise blank a failure's whole log.
        let text = String::from_utf8_lossy(&std::fs::read(&log).unwrap_or_default()).into_owned();
        let secs = at.elapsed().as_secs();
        match outcome {
            Ok(true) => {
                println!("[{side}] ok   {display} ({secs}s)");
                print_shots(side, &log);
            }
            Ok(false) => {
                // The whole log: a failure with its tail cut off sends
                // whoever reads it straight back here to re-run it.
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

/// A green verify-ui is only half of Done — the PNGs still get eyeballed
/// (verify-ui skill), so where they landed survives the capture.
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

/// The end of a step's log, for a failure that has to say why in place: a
/// build or a test that stopped says so in its last lines.
pub(crate) fn log_tail(log: &Path) -> String {
    tail_of(&std::fs::read_to_string(log).unwrap_or_default())
}

/// The last lines of a step's output, as [`log_tail`] reads them.
pub(crate) fn tail_of(text: &str) -> String {
    const LINES: usize = 40;
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(LINES)..].join("\n")
}

/// One step against its log file: spawned with both streams on the file,
/// watched. `Ok` is the step's own verdict; `Err` is a
/// ceiling or a spawn failure — the reasons a check used to sit forever.
/// The gate runs its steps through here too.
pub(crate) fn run_step(
    root: &Path,
    step: &[String],
    log: &Path,
    room: &crate::budget::Admitted,
) -> Result<bool, String> {
    let out = std::fs::File::create(log).map_err(|e| format!("{}: {e}", log.display()))?;
    let err = out
        .try_clone()
        .map_err(|e| format!("{}: {e}", log.display()))?;
    // Every step is a cargo of its own, announced as one (`still`); the
    // step itself is under that announcement and says nothing more.
    let _busy = crate::still::busy(root, &step.join(" "))?;
    let mut command = Command::new(&step[0]);
    command.args(&step[1..]).current_dir(root);
    crate::still::step(&mut command);
    // Every caller of this is itself one unit of the machine's budget —
    // the gate's step under its own ticket, `check`'s under no budget at
    // all — so what the step starts is under that and takes no second
    // ticket of its own (`crate::budget`).
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
    // What this step is, as far as the machine's budget is concerned:
    // the ticket is this runner's, but the load is the child's, and a
    // ledger that outlives this process has to know which number to ask
    // after — and what to expect at it (`budget::Pool::leftover`).
    room.started(child.id(), &step[0]);
    let mut wait = Wait::new(
        "the step",
        Budget::of(QUIET_CEILING, STEP_CEILING),
        LOOK_AGAIN,
    );
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return Ok(status.success());
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
