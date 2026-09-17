//! `cargo xtask corpus --probe`: what the reads behind a click cost as
//! processes, and what git's own speed-ups would give back — measured
//! against the corpus under the product's own conditions, so the record
//! (ci/baseline/git-slots-windows-x64.md §プロセス再利用) is taken the
//! way the application would spend it.
//!
//! Three questions, each a table:
//!
//! - **`status`**: as the application runs it (`--no-optional-locks`,
//!   `GIT_OPTIONAL_LOCKS=0`), then with the untracked cache, with
//!   fsmonitor, and with both — in a working copy of the corpus, since
//!   both write the index once to switch on, and the corpus itself is
//!   read only.
//! - **object reads**: `cat-file blob` a process at a time, against one
//!   resident `cat-file --batch` answering the same reads on stdin.
//! - **the existence probe** before a blob read (`rev-parse --verify`,
//!   `preview::blob_is_there`), against a resident `--batch-check`.
//!
//! This is a probe: the product keeps stdin closed (`scratch`), and the
//! resident process is a design the record decides on.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// What every application invocation carries (`process::executor`):
/// spelled again here because xtask depends on std alone.
const FIXED_ARGS: [&str; 9] = [
    "-c",
    "color.ui=false",
    "-c",
    "core.quotepath=false",
    "-c",
    "log.showSignature=false",
    "-c",
    "diff.autoRefreshIndex=false",
    "--no-optional-locks",
];
const FIXED_ENV: [(&str, &str); 4] = [
    ("LC_ALL", "C"),
    ("GIT_TERMINAL_PROMPT", "0"),
    ("GIT_OPTIONAL_LOCKS", "0"),
    ("GIT_EDITOR", "true"),
];

const STATUS: [&str; 5] = ["status", "--porcelain=v2", "-z", "--branch", "-uall"];

/// Rounds of each timing; the table reads min and median.
const ROUNDS: usize = 5;
/// Object reads per variant.
const READS: usize = 20;

pub(super) fn run(corpus: &Path, copy: &Path) -> Result<(), String> {
    if !copy.join(".git").exists() {
        return Err(format!(
            "{} is not there: stand a copy first (cargo xtask corpus --copies 1)",
            copy.display()
        ));
    }
    let (oid, path) = newest_file(corpus)?;
    println!("probe: {}", corpus.display());
    println!("  index-writing variants run in {}", copy.display());
    println!("  object read: {oid}:{path}");
    status_table(copy)?;
    object_table(corpus, &format!("{oid}:{path}"))?;
    Ok(())
}

/// The newest commit's first openable file — the same read `perf` opens.
fn newest_file(at: &Path) -> Result<(String, String), String> {
    let raw = git(
        at,
        &[],
        &[
            "log",
            "-1",
            "--format=commit %H",
            "--raw",
            "--no-abbrev",
            "HEAD",
        ],
    )?;
    let mut commit = String::new();
    for line in raw.lines() {
        if let Some(oid) = line.strip_prefix("commit ") {
            commit = oid.to_string();
        } else if let Some(fields) = line.strip_prefix(':')
            && let Some((_, path)) = fields.split_once('\t')
        {
            return Ok((commit, path.to_string()));
        }
    }
    Err("HEAD changes no file to open".into())
}

fn status_table(copy: &Path) -> Result<(), String> {
    println!("status (min / median of {ROUNDS}, wall clock, product conditions):");
    let base = rounds(|| timed(copy, &[], &STATUS));
    say("as the application runs it", &base);

    // The untracked cache: switched on in the index once (a write the
    // application never makes), then filled by one status that may write
    // the index — which is the one the product's conditions forbid — so
    // what is measured after is a cache the product would find already
    // there and read as is.
    git(copy, &[], &["update-index", "--untracked-cache"])?;
    git(
        copy,
        &[("GIT_OPTIONAL_LOCKS", "1")],
        &["status", "--porcelain=v2", "-uall"],
    )?;
    let cached = rounds(|| timed(copy, &[], &STATUS));
    say(
        "untracked cache on, filled once with the index writable",
        &cached,
    );
    git(copy, &[], &["update-index", "--no-untracked-cache"])?;

    // fsmonitor: the daemon starts with the first status that names it
    // and answers what changed since the token in the index. Under the
    // product's conditions the token is never written back, so every
    // later status asks "since the first look" — the accumulated answer,
    // which is the steady state the application would be in.
    let fsmonitor = [("-c", "core.fsmonitor=true")];
    let fsmonitor_args: Vec<&str> = fsmonitor.iter().flat_map(|(a, b)| [*a, *b]).collect();
    let mut starting = fsmonitor_args.clone();
    starting.extend(STATUS);
    let first = timed_env(copy, &[("GIT_OPTIONAL_LOCKS", "1")], &starting);
    println!(
        "  fsmonitor: the first status, daemon starting, index writable: {} ms",
        first.as_millis()
    );
    let monitored = rounds(|| timed(copy, &[], &starting));
    say("fsmonitor on, token from that first look", &monitored);

    git(copy, &[], &["update-index", "--untracked-cache"])?;
    git(copy, &[("GIT_OPTIONAL_LOCKS", "1")], &starting)?;
    let both = rounds(|| timed(copy, &[], &starting));
    say("fsmonitor and the untracked cache, both filled once", &both);
    git(copy, &[], &["update-index", "--no-untracked-cache"])?;
    // Stopped, so nothing this probe started outlives it.
    if let Err(error) = git(copy, &[], &["fsmonitor--daemon", "stop"]) {
        println!("  (fsmonitor daemon stop: {error})");
    }
    Ok(())
}

fn object_table(corpus: &Path, spec: &str) -> Result<(), String> {
    println!("object reads ({READS} of {spec}; min / median per read, wall clock):");
    let one_shot = (0..READS)
        .map(|_| timed(corpus, &[], &["cat-file", "blob", spec]))
        .collect::<Vec<_>>();
    say("cat-file blob, a process each", &one_shot);
    let probe = (0..READS)
        .map(|_| timed(corpus, &[], &["rev-parse", "--verify", "-q", spec]))
        .collect::<Vec<_>>();
    say(
        "rev-parse --verify -q, a process each (the existence probe)",
        &probe,
    );
    let show = (0..READS)
        .map(|_| timed(corpus, &[], &["show", "--no-patch", "--format=%H", "HEAD"]))
        .collect::<Vec<_>>();
    say("show --no-patch, a process each (the smallest read)", &show);

    let mut batch = Resident::start(corpus, "--batch")?;
    let mut answered = Vec::new();
    for _ in 0..READS {
        // waits(measured): one resident read's round trip, the table's own number
        let began = Instant::now();
        let bytes = batch.ask(spec)?;
        answered.push(began.elapsed());
        if bytes == 0 {
            return Err("the resident cat-file answered nothing".into());
        }
    }
    say("cat-file --batch, one resident process", &answered);
    println!(
        "  resident cat-file --batch: {} after {READS} reads",
        batch.footprint()
    );
    drop(batch);

    let mut check = Resident::start(corpus, "--batch-check")?;
    let mut checked = Vec::new();
    for _ in 0..READS {
        // waits(measured): one resident check's round trip, the table's own number
        let began = Instant::now();
        check.ask(spec)?;
        checked.push(began.elapsed());
    }
    say("cat-file --batch-check, one resident process", &checked);
    // waits(measured): the missing-object check's round trip, printed beside the table
    let began = Instant::now();
    let missing = check.ask("HEAD:no/such/path")?;
    println!(
        "  --batch-check on a path that is not there: {} bytes back in {} ms (exit code unchanged)",
        missing,
        began.elapsed().as_millis()
    );
    Ok(())
}

/// One resident `git cat-file --batch*`, asked a spec at a time on stdin.
struct Resident {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
    contents: bool,
}

impl Resident {
    fn start(at: &Path, mode: &str) -> Result<Self, String> {
        let mut command = Command::new("git");
        command
            .current_dir(at)
            .args(FIXED_ARGS)
            .args(["cat-file", mode])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        for (key, value) in FIXED_ENV {
            command.env(key, value);
        }
        let mut child = command
            .spawn()
            .map_err(|e| format!("could not start git cat-file {mode}: {e}"))?;
        let stdin = child.stdin.take().ok_or("no stdin")?;
        let stdout = BufReader::new(child.stdout.take().ok_or("no stdout")?);
        Ok(Self {
            child,
            stdin,
            stdout,
            contents: mode == "--batch",
        })
    }

    /// Asks one spec and reads the whole answer back: the header line,
    /// and for `--batch` the object and the newline after it. Answers how
    /// many bytes the object was (zero for a missing one).
    fn ask(&mut self, spec: &str) -> Result<usize, String> {
        writeln!(self.stdin, "{spec}").map_err(|e| e.to_string())?;
        self.stdin.flush().map_err(|e| e.to_string())?;
        let mut header = String::new();
        self.stdout
            .read_line(&mut header)
            .map_err(|e| e.to_string())?;
        let fields: Vec<&str> = header.split_whitespace().collect();
        let size = match fields.as_slice() {
            [_, _, size] => size.parse::<usize>().unwrap_or(0),
            _ => 0,
        };
        if self.contents && size > 0 {
            let mut body = vec![0u8; size + 1];
            self.stdout
                .read_exact(&mut body)
                .map_err(|e| e.to_string())?;
        }
        Ok(size)
    }

    /// What the resident process holds, as the OS reports it.
    fn footprint(&self) -> String {
        let pid = self.child.id();
        let out = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
            .output();
        match out {
            Ok(out) if out.status.success() => {
                let text = String::from_utf8_lossy(&out.stdout);
                text.split(',')
                    .next_back()
                    .map(|mem| mem.trim().trim_matches('"').to_string())
                    .unwrap_or_default()
            }
            _ => "footprint not read (tasklist)".into(),
        }
    }
}

impl Drop for Resident {
    fn drop(&mut self) {
        if self.child.kill().is_err() {
            println!("  (the resident cat-file had already ended)");
        }
        if let Err(error) = self.child.wait() {
            println!("  (the resident cat-file could not be reaped: {error})");
        }
    }
}

fn rounds(mut once: impl FnMut() -> Duration) -> Vec<Duration> {
    (0..ROUNDS).map(|_| once()).collect()
}

fn say(what: &str, timings: &[Duration]) {
    let mut sorted: Vec<u128> = timings.iter().map(Duration::as_millis).collect();
    sorted.sort_unstable();
    let min = sorted.first().copied().unwrap_or_default();
    let median = sorted.get(sorted.len() / 2).copied().unwrap_or_default();
    let max = sorted.last().copied().unwrap_or_default();
    println!("  {what}: {min} / {median} ms (max {max})");
}

fn timed(at: &Path, env: &[(&str, &str)], args: &[&str]) -> Duration {
    timed_env(at, env, args)
}

fn timed_env(at: &Path, env: &[(&str, &str)], args: &[&str]) -> Duration {
    // waits(measured): one process's round trip, the table's own number
    let began = Instant::now();
    if let Err(error) = git(at, env, args) {
        println!("  (git {} failed: {error})", args.join(" "));
    }
    began.elapsed()
}

/// One application-shaped invocation: the fixed arguments and
/// environment, `env` on top of them.
fn git(at: &Path, env: &[(&str, &str)], args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("git");
    command.current_dir(at).args(FIXED_ARGS).args(args);
    for (key, value) in FIXED_ENV {
        command.env(key, value);
    }
    for (key, value) in env {
        command.env(key, value);
    }
    let out = command
        .output()
        .map_err(|e| format!("failed to run git {args:?}: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}
