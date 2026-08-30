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

/// How long a step may say nothing before it is killed and named. Silence,
/// not wall time: the long steps (a first container image build, a release
/// link) all keep talking, while every hang this has to catch — a test
/// deadlocked past its own backstops, a cargo blocked on another cargo's
/// build lock, a docker CLI waiting out a wedged daemon — goes quiet first.
/// Generous on purpose: the suite's own 900s backstop must fire before
/// this one so the failure carries a test name, and a link is the longest
/// legitimately silent stretch.
const QUIET_CEILING: Duration = Duration::from_secs(20 * 60);

/// The absolute ceiling per step, for a hang that keeps talking.
const STEP_CEILING: Duration = Duration::from_secs(90 * 60);

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

    let root = crate::workspace_root();
    let started = Instant::now();

    let words = |line: &[&str]| line.iter().map(|w| (*w).to_string()).collect::<Vec<_>>();
    let mut host_steps: Vec<Vec<String>> = vec![
        // First because it is the cheapest thing here that can fail — it
        // builds nothing and answers in a second or two, and a line ceiling
        // is not worth finding out about after ten minutes of compiling.
        words(&["cargo", "xtask", "structure"]),
        words(&["cargo", "fmt", "--all", "--", "--check"]),
        words(&[
            "cargo",
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ]),
        words(&["cargo", "test", "--workspace"]),
    ];
    let mut linux_steps: Vec<Vec<String>> = vec![
        words(&["cargo", "xtask", "linux", "test", "-p", "platitude-core"]),
        // The same line as the host's clippy above, because the host's
        // cannot answer for it: a name reachable only under
        // #[cfg(not(windows))] is not compiled on Windows at all, so an
        // unused import or an orphaned fn behind that cfg passes here and
        // fails the Linux and macOS jobs the first time CI runs
        // (.github/workflows/ci.yml runs this across the whole matrix).
        // `bare` is the only other thing on this side that compiles the
        // app for Linux, and it is a release build whose warnings are not
        // errors and which nobody reads.
        words(&[
            "cargo",
            "xtask",
            "linux",
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ]),
    ];
    for (i, verb) in verbs.iter().enumerate() {
        // A --verb value is a whole verify-ui argument line — some verbs
        // only mean anything with their preset or argument beside them
        // ("co-authors 4 --preset co-authors").
        let verb_words: Vec<&str> = verb.split_whitespace().collect();
        // The first host run builds the release; the rest reuse it.
        let mut host = words(&["cargo", "xtask", "verify-ui"]);
        host.extend(verb_words.iter().map(|w| (*w).to_string()));
        if i > 0 {
            host.push("--no-build".to_string());
        }
        host_steps.push(host);
        // The container side reuses its first build too: a fingerprint
        // check across the host boundary is measurably slow, and paying
        // it once per verb bought nothing.
        let mut linux = words(&["cargo", "xtask", "linux", "verify-ui"]);
        linux.extend(verb_words.iter().map(|w| (*w).to_string()));
        if i > 0 {
            linux.push("--no-build".to_string());
        }
        linux_steps.push(linux);
    }
    // Last so it reuses the release the container's verify-ui just built.
    linux_steps.push(words(&["cargo", "xtask", "linux", "bare"]));

    let host_root = root.clone();
    let host = std::thread::spawn(move || run_side("host", &host_root, &host_steps));
    let linux_root = root.clone();
    let linux = std::thread::spawn(move || run_side("linux", &linux_root, &linux_steps));

    let mut failures: Vec<String> = Vec::new();
    for handle in [host, linux] {
        match handle.join() {
            Ok(mut side_failures) => failures.append(&mut side_failures),
            Err(_) => failures.push("a side panicked".to_string()),
        }
    }

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
/// A file rather than a pipe, so this always comes back: reading a pipe to
/// EOF waits on every process that inherited its write end, and one
/// straggler a killed or finished child left behind (a wedged git, an
/// orphaned test binary) would hold the whole check open after every test
/// had already answered. The file also survives a hang — when a step is
/// killed at a ceiling, its tail says what the step was doing, which a
/// pipe lost in a buffer cannot.
fn run_side(side: &str, root: &Path, steps: &[Vec<String>]) -> Vec<String> {
    let logs = root.join("target").join("check-logs");
    if let Err(e) = std::fs::create_dir_all(&logs) {
        return vec![format!("{}: {e}", logs.display())];
    }
    for (index, step) in steps.iter().enumerate() {
        let display = step.join(" ");
        println!("[{side}] {display} …");
        let at = Instant::now();
        let log = logs.join(format!("{side}-{index:02}.log"));
        let outcome = run_step(root, step, &log);
        let text = std::fs::read_to_string(&log).unwrap_or_default();
        let secs = at.elapsed().as_secs();
        match outcome {
            Ok(true) => {
                println!("[{side}] ok   {display} ({secs}s)");
                // A green verify-ui is only half of Done — the PNGs still
                // get eyeballed (verify-ui skill), so where they landed
                // survives the capture.
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
            Ok(false) => {
                // The whole log: a failure with its tail cut off sends
                // whoever reads it straight back here to re-run it.
                println!("[{side}] FAIL {display} ({secs}s)");
                print!("{text}");
                return vec![display];
            }
            Err(why) => {
                println!("[{side}] FAIL {display} ({secs}s): {why}");
                let tail: Vec<&str> = text.lines().rev().take(40).collect();
                for line in tail.into_iter().rev() {
                    println!("[{side}]      {line}");
                }
                return vec![format!("{display}: {why}")];
            }
        }
    }
    Vec::new()
}

/// One step against its log file: spawned with both streams on the file,
/// watched rather than awaited. `Ok` is the step's own verdict; `Err` is a
/// ceiling or a spawn failure — the reasons a check used to sit forever.
fn run_step(root: &Path, step: &[String], log: &Path) -> Result<bool, String> {
    let out = std::fs::File::create(log).map_err(|e| format!("{}: {e}", log.display()))?;
    let err = out
        .try_clone()
        .map_err(|e| format!("{}: {e}", log.display()))?;
    let mut child = Command::new(&step[0])
        .args(&step[1..])
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err))
        .spawn()
        .map_err(|e| e.to_string())?;
    let started = Instant::now();
    let mut grew = Instant::now();
    let mut seen = 0u64;
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return Ok(status.success());
        }
        let len = std::fs::metadata(log).map(|m| m.len()).unwrap_or(0);
        if len != seen {
            seen = len;
            grew = Instant::now();
        }
        let (quiet, whole) = (grew.elapsed(), started.elapsed());
        if quiet >= QUIET_CEILING || whole >= STEP_CEILING {
            let _ = child.kill();
            let _ = child.wait();
            let ceiling = if quiet >= QUIET_CEILING {
                format!("said nothing for {} minutes", quiet.as_secs() / 60)
            } else {
                format!("ran for {} minutes", whole.as_secs() / 60)
            };
            // Killing the immediate child cannot reach its survivors —
            // grandchildren keep running and may hold this side's build
            // lock, which the message owns up to rather than letting the
            // next run's stall look unrelated.
            return Err(format!(
                "killed at the ceiling: the step {ceiling} (its log has the tail; \
                 survivors of the killed command may still hold this side's build lock)"
            ));
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}
