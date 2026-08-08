//! `cargo xtask verify-ui` — one command from source to a judged
//! headless run.
//!
//! Wraps what CLAUDE.md prescribes for write-path verification: release
//! build (QML is embedded in the exe), offscreen QPA with an explicit
//! font dir, the PG_AUTO_* hooks, a bounded wait with a kill guard (never
//! an unbounded one — the lessons of the locked-screen hangs), and the
//! `screenshot saved=true` stderr line as the verdict.

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Grace on top of PG_AUTO_QUIT_MS before the run is killed: startup,
/// repository load, and the write itself happen inside this.
const GRACE_MS: u64 = 20_000;

struct Options {
    verb: String,
    arg: String,
    /// Repositories to open, in tab order. Both flags repeat, because a
    /// gesture on the tab strip needs a strip to land on — one tab can
    /// only show that a tab closed, never that the neighbour stayed.
    repo: Vec<PathBuf>,
    preset: Vec<String>,
    build: bool,
    select: bool,
    quit_ms: u64,
    shot_dir: Option<PathBuf>,
    /// Where the run keeps its settings and state. A fresh directory per
    /// run unless one is named, so a headless run never reads or writes
    /// the settings of whoever is sitting at this machine.
    config_dir: Option<PathBuf>,
    /// Let the app put back the tabs its config directory remembers,
    /// instead of being told which repository to open.
    restore: bool,
}

fn parse(args: &[String]) -> Result<Options, String> {
    let mut opts = Options {
        verb: String::new(),
        arg: String::new(),
        repo: Vec::new(),
        preset: Vec::new(),
        build: true,
        select: false,
        quit_ms: 10_000,
        shot_dir: None,
        config_dir: None,
        restore: false,
    };
    let mut positional: Vec<&str> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--repo" => opts
                .repo
                .push(PathBuf::from(it.next().ok_or("--repo needs a path")?)),
            "--preset" => opts
                .preset
                .push(it.next().ok_or("--preset needs a name")?.clone()),
            "--no-build" => opts.build = false,
            "--select" => opts.select = true,
            "--quit-ms" => {
                opts.quit_ms = it
                    .next()
                    .ok_or("--quit-ms needs a number")?
                    .parse()
                    .map_err(|e| format!("--quit-ms: {e}"))?;
            }
            "--shot-dir" => {
                opts.shot_dir = Some(PathBuf::from(it.next().ok_or("--shot-dir needs a path")?));
            }
            "--config-dir" => {
                opts.config_dir =
                    Some(PathBuf::from(it.next().ok_or("--config-dir needs a path")?));
            }
            "--restore" => opts.restore = true,
            other => positional.push(other),
        }
    }
    match positional.as_slice() {
        [] => return Err("verify-ui needs a verb (see `cargo xtask`)".into()),
        [verb] => opts.verb = (*verb).to_string(),
        [verb, arg] => {
            opts.verb = (*verb).to_string();
            opts.arg = (*arg).to_string();
        }
        more => return Err(format!("too many positional arguments: {more:?}")),
    }
    Ok(opts)
}

/// The workspace root, resolved at compile time from this crate's location.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap_or(Path::new("."))
        .to_path_buf()
}

pub fn run(args: &[String]) -> Result<(), String> {
    let opts = parse(args)?;
    let root = workspace_root();
    let path = crate::qt::path_with_qt()?;

    // Named repositories win outright; otherwise one fresh demo repository
    // per preset, in the order they were asked for — which is the order
    // the tabs come up in.
    let repos = if opts.repo.is_empty() {
        let presets: Vec<String> = if opts.preset.is_empty() {
            vec!["basic".into()]
        } else {
            opts.preset.clone()
        };
        let mut made = Vec::with_capacity(presets.len());
        for preset in &presets {
            let repo = crate::demo::create(preset, None)?;
            println!("demo repo ({preset}): {}", repo.display());
            made.push(repo);
        }
        made
    } else {
        opts.repo.clone()
    };

    if opts.build {
        println!("building (release)…");
        let status = Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(&root)
            .env("PATH", &path)
            .status()
            .map_err(|e| format!("failed to run cargo: {e}"))?;
        if !status.success() {
            return Err("cargo build --release failed".into());
        }
    }

    let exe = root.join("target").join("release").join(if cfg!(windows) {
        "platitude-gg.exe"
    } else {
        "platitude-gg"
    });
    if !exe.is_file() {
        return Err(format!(
            "{} not found — build first (or drop --no-build)",
            exe.display()
        ));
    }

    let shot_dir = match &opts.shot_dir {
        Some(dir) => dir.clone(),
        None => {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos();
            std::env::temp_dir()
                .join("pg-verify")
                .join(format!("{}-{nanos}", opts.verb))
        }
    };
    std::fs::create_dir_all(&shot_dir).map_err(|e| e.to_string())?;

    // A run of its own unless told otherwise. The app would refuse the
    // real files anyway once it sees a PG_* variable, but naming a
    // directory is what lets one run read what the last one wrote — and
    // what lets anyone look at the two files afterwards.
    let config_dir = match &opts.config_dir {
        Some(dir) => dir.clone(),
        None => shot_dir.join("config"),
    };
    std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
    println!("config dir: {}", config_dir.display());

    let opened = repos
        .iter()
        .map(|r| r.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    println!(
        "running: {} (arg: {}) against {}",
        opts.verb,
        if opts.arg.is_empty() { "-" } else { &opts.arg },
        if opts.restore {
            "the tabs it remembers"
        } else {
            &opened
        }
    );
    let mut cmd = Command::new(&exe);
    cmd.current_dir(&root)
        .env("PATH", &path)
        .env("QT_QPA_PLATFORM", "offscreen")
        .env("QT_FORCE_STDERR_LOGGING", "1")
        .env("PG_CONFIG_DIR", &config_dir)
        .env("PG_AUTO_QUIT_MS", opts.quit_ms.to_string())
        .env("PG_SHOT_DIR", &shot_dir)
        .env("PG_AUTO_ACT", &opts.verb)
        .env("PG_AUTO_ACT_ARG", &opts.arg)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if cfg!(windows) {
        // Offscreen Qt does not discover system fonts on Windows; without
        // this every glyph is a box (CLAUDE.md).
        cmd.env("QT_QPA_FONTDIR", "C:\\Windows\\Fonts");
    }
    // The automation hooks report through tracing at info; without this
    // their lines never reach the verdict output.
    if std::env::var_os("PG_LOG").is_none() {
        cmd.env("PG_LOG", "info");
    }
    if !opts.restore {
        // Naming a repository is what turns tab restoring off (Main.qml):
        // a run that is told what to open is not being asked what it
        // remembers. More than one opens a tab each, in this order —
        // joined rather than formatted, so a path git accepts but UTF-8
        // does not still reaches the app whole.
        let mut open = std::ffi::OsString::new();
        for (position, repo) in repos.iter().enumerate() {
            if position > 0 {
                open.push(";");
            }
            open.push(repo);
        }
        cmd.env("PG_AUTO_OPEN", &open);
    }
    if opts.select {
        cmd.env("PG_AUTO_SELECT", "1");
    }

    let started = Instant::now();
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to start the app: {e}"))?;
    let stdout = child.stdout.take().map(collect_lines);
    let stderr = child.stderr.take().map(collect_lines);

    // Bounded wait with a kill guard — never an unbounded wait or poll.
    let deadline = Duration::from_millis(opts.quit_ms + GRACE_MS);
    let mut timed_out = false;
    let status = loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(status) => break Some(status),
            None if started.elapsed() > deadline => {
                let _ = child.kill();
                timed_out = true;
                break child.wait().ok();
            }
            None => std::thread::sleep(Duration::from_millis(100)),
        }
    };

    let join = |h: Option<std::thread::JoinHandle<Vec<String>>>| {
        h.and_then(|h| h.join().ok()).unwrap_or_default()
    };
    let out_lines = join(stdout);
    let err_lines = join(stderr);
    let elapsed = started.elapsed();

    let saved = err_lines
        .iter()
        .chain(out_lines.iter())
        .any(|l| l.contains("screenshot saved=true"));
    let exit_ok = status.as_ref().is_some_and(|s| s.success());
    // Not part of the verdict — some verbs *exist* to walk a refusal path
    // (delete-branch on an unmerged branch) — but always worth eyes.
    let write_failures = err_lines
        .iter()
        .filter(|l| l.contains("write failed"))
        .count();

    for line in err_lines.iter().chain(out_lines.iter()) {
        println!("  | {line}");
    }
    let mut shots: Vec<PathBuf> = std::fs::read_dir(&shot_dir)
        .map(|it| {
            it.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "png"))
                .collect()
        })
        .unwrap_or_default();
    shots.sort();
    for shot in &shots {
        println!("shot: {}", shot.display());
    }

    let verdict_ok = exit_ok && saved && !timed_out;
    println!(
        "{}: {} in {:.1}s (exit {}, screenshot saved={}, write-failures {}{})",
        if verdict_ok { "PASS" } else { "FAIL" },
        opts.verb,
        elapsed.as_secs_f32(),
        status.map_or_else(
            || "?".into(),
            |s| s.code().map_or("signal".into(), |c| c.to_string())
        ),
        saved,
        write_failures,
        if timed_out { ", TIMED OUT" } else { "" },
    );
    if verdict_ok {
        Ok(())
    } else {
        Err(format!("verify-ui {} failed", opts.verb))
    }
}

/// Drains a pipe on its own thread, so a chatty child never blocks on a
/// full pipe while the parent waits for it to exit.
fn collect_lines<R: std::io::Read + Send + 'static>(
    reader: R,
) -> std::thread::JoinHandle<Vec<String>> {
    std::thread::spawn(move || {
        std::io::BufReader::new(reader)
            .lines()
            .map_while(Result::ok)
            .collect()
    })
}
