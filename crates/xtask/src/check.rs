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
use std::process::Command;
use std::time::Instant;

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
    let mut linux_steps: Vec<Vec<String>> = vec![words(&[
        "cargo",
        "xtask",
        "linux",
        "test",
        "-p",
        "platitude-core",
    ])];
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
        let mut linux = words(&["cargo", "xtask", "linux", "verify-ui"]);
        linux.extend(verb_words.iter().map(|w| (*w).to_string()));
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
/// (later steps of a side depend on the same build tree). Output is
/// captured and only printed for the step that failed, so two sides can
/// speak at once without shredding each other's lines; the start and
/// verdict lines carry the liveness.
fn run_side(side: &str, root: &Path, steps: &[Vec<String>]) -> Vec<String> {
    for step in steps {
        let display = step.join(" ");
        println!("[{side}] {display} …");
        let at = Instant::now();
        let out = match Command::new(&step[0])
            .args(&step[1..])
            .current_dir(root)
            .output()
        {
            Ok(out) => out,
            Err(e) => {
                println!("[{side}] FAIL {display}: {e}");
                return vec![format!("{display}: {e}")];
            }
        };
        let secs = at.elapsed().as_secs();
        if out.status.success() {
            println!("[{side}] ok   {display} ({secs}s)");
            // A green verify-ui is only half of Done — the PNGs still get
            // eyeballed (verify-ui skill), so where they landed survives
            // the capture.
            for text in [&out.stdout, &out.stderr] {
                for line in String::from_utf8_lossy(text).lines() {
                    let line = line.trim();
                    if line.starts_with("shot: ")
                        || line.starts_with("screenshot: ")
                        || line.contains("screenshots and settings:")
                    {
                        println!("[{side}]      {line}");
                    }
                }
            }
        } else {
            // The whole of both streams: a failure with its tail cut off
            // sends whoever reads it straight back here to re-run it.
            println!("[{side}] FAIL {display} ({secs}s)");
            print!("{}", String::from_utf8_lossy(&out.stdout));
            print!("{}", String::from_utf8_lossy(&out.stderr));
            return vec![display];
        }
    }
    Vec::new()
}
