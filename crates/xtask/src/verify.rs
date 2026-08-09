//! `cargo xtask verify-ui` — one command from source to a judged
//! headless run.
//!
//! Wraps what the verify-ui skill prescribes for write-path verification:
//! release build (QML is embedded in the exe), offscreen QPA with an
//! explicit font dir, the PG_AUTO_* hooks, a bounded wait with a kill
//! guard (never an unbounded one — the lessons of the locked-screen
//! hangs), and the `screenshot saved=true` stderr line as the verdict.

use std::io::BufRead;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Grace on top of PG_AUTO_QUIT_MS before the run is killed: startup,
/// repository load, and the write itself happen inside this.
const GRACE_MS: u64 = 20_000;

/// What the identity verbs type in when nothing else is asked for. Both
/// halves differ from anything the seed below holds, so a mark means the
/// write landed rather than that the value was already there.
const IDENTITY_ASKED: &str = "Ada Lovelace|ada@example.com";

/// The git configuration an identity run starts from, or `None` for every
/// verb that has nothing to do with one.
///
/// `identity-half` is the whole point of the pair. A `user.email` with two
/// values in the file refuses a plain set (measured: exit 5, `cannot
/// overwrite multiple values`) while the `user.name` written just before
/// it goes in — the same half-landed write a configuration lock lost
/// between the two calls leaves behind, and the only version of it that
/// can be produced on demand. Neither seed names a name, so the screen
/// asks for an identity on its own.
fn identity_seed(verb: &str) -> Option<&'static str> {
    match verb {
        "identity" => Some(""),
        "identity-half" => {
            Some("[user]\n\temail = personal@example.com\n\temail = second@example.com\n")
        }
        _ => None,
    }
}

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
    /// Whether a write git refused is part of what the verb is showing.
    allow_write_failure: bool,
}

/// What the run is judged on.
///
/// A refused write counts against it: the verb asked for one, and a
/// picture of the state it never reached proves nothing — a `commit` that
/// sent an empty message read as PASS for as long as this was only
/// printed. Verbs that exist to walk a refusal (`fetch-fail`,
/// `delete-branch-refused`) say so with `--allow-write-failure`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Outcome {
    exit_ok: bool,
    saved: bool,
    timed_out: bool,
    write_failures: usize,
    allow_write_failure: bool,
    /// What a verb whose failure the camera cannot see has to be caught
    /// saying. `solo` photographs a perfectly good ordinary window if the
    /// lock was never held, `details-fit` frames a pane whose content ran
    /// off the right of the window the same as one that fits, and
    /// `window-fill` is about a maximised window, which leaves no desktop
    /// beside itself for a short edge to show against — nor any way to
    /// photograph the frame it paints out past the screen.
    must_say: Option<&'static str>,
    said: bool,
}

impl Outcome {
    fn passed(self) -> bool {
        self.exit_ok
            && self.saved
            && !self.timed_out
            && !self.write_sank_it()
            && (self.must_say.is_none() || self.said)
    }

    /// Whether the failing writes are what the verdict turns on — the one
    /// reason worth a line of its own, since the run looks well otherwise.
    fn write_sank_it(self) -> bool {
        self.write_failures > 0 && !self.allow_write_failure
    }
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
        allow_write_failure: false,
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
            "--allow-write-failure" => opts.allow_write_failure = true,
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

pub fn run(args: &[String]) -> Result<(), String> {
    let opts = parse(args)?;
    let root = crate::workspace_root();
    let path = crate::qt::path_with_qt()?;

    // Named repositories win outright; otherwise one fresh demo repository
    // per preset, in the order they were asked for — which is the order
    // the tabs come up in.
    let repos = if !opts.repo.is_empty() {
        opts.repo.clone()
    } else if opts.verb == "tab-widths" {
        tab_width_repos(&opts.arg)?
    } else if opts.verb == "tab-mark" {
        // Same strip; the argument here names a tab in it rather than
        // how many there are.
        tab_width_repos("")?
    } else {
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

    let arg = match opts.arg.is_empty() {
        true => folder_for(&opts.verb, &shot_dir, &path)?,
        false => opts.arg.clone(),
    };

    let opened = repos
        .iter()
        .map(|r| r.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    println!(
        "running: {} (arg: {}) against {}",
        opts.verb,
        if arg.is_empty() { "-" } else { &arg },
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
        .env("PG_AUTO_ACT_ARG", &arg)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if cfg!(windows) {
        // Offscreen Qt does not discover system fonts on Windows; without
        // this every glyph is a box (verify-ui skill).
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

    // The identity screen is the one surface whose write lands outside the
    // demo repository — in the configuration of whoever is sitting at this
    // machine. So the run is given one of its own: `--global` follows
    // GIT_CONFIG_GLOBAL and the read-back resolves through the same file,
    // which exercises the whole feature without reading or touching a real
    // identity. The working directory goes with it, because git resolves
    // configuration from one and the repository this tree lives in would
    // otherwise get a say.
    if let Some(seed) = identity_seed(&opts.verb) {
        let config = shot_dir.join("gitconfig");
        std::fs::write(&config, seed).map_err(|e| e.to_string())?;
        cmd.current_dir(&shot_dir)
            .env("GIT_CONFIG_GLOBAL", &config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env(
                "PG_AUTO_IDENTITY",
                match opts.arg.is_empty() {
                    true => IDENTITY_ASKED,
                    false => opts.arg.as_str(),
                },
            );
        if opts.verb == "identity-half" {
            cmd.env("PG_AUTO_IDENTITY_SAVE", "1");
        }
        println!("identity config: {}", config.display());
    }

    // `solo` is the one verb the harness has to take part in: the window
    // it photographs is the one a *second* process puts up, so somebody
    // has to be the first. Holding the real lock — rather than setting a
    // flag that imitates the state — is what makes the picture proof of
    // the mechanism. The name is `settings::LOCK_FILE`; xtask depends on
    // std alone (CLAUDE.md), so it is spelled again here, and a drift
    // shows up as the run reporting `blocked=false` below.
    let _held = if opts.verb == "solo" {
        let path = config_dir.join("lock");
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|e| format!("could not open {}: {e}", path.display()))?;
        file.try_lock()
            .map_err(|e| format!("could not hold {}: {e}", path.display()))?;
        println!("holding: {}", path.display());
        Some(file)
    } else {
        None
    };

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

    let must_say = match opts.verb.as_str() {
        "solo" => Some("solo blocked=true"),
        "details-fit" => Some("details_fit fits=true"),
        // A pull that took more room than the pane had leaves the author
        // card drawn over the window's own footer, and that frames like a
        // pane that fits — the same blind spot details-fit answers for.
        // Only that half is judged here: whether the grip was offered at
        // all depends on the row, and the run where it stays away is half
        // of the pair.
        "details-grow" | "details-grow-squeeze" => Some("description_grow author=true"),
        // A dialog that stayed open because the write did not take looks
        // exactly like one nobody has answered yet, and "which half
        // landed" is not something a picture holds at all.
        "identity" => Some(
            "identity state=missing dialog=true nameSaved=false emailSaved=false unsaved=false",
        ),
        "identity-half" => {
            Some("state=ready dialog=true nameSaved=true emailSaved=false unsaved=true")
        }
        // `edge=` rides along in that report but is not judged: whether an
        // edge would land off the screen is a question about a real
        // monitor, and the offscreen platform has none to answer with.
        "window-fill" => Some("window_fill fills=true"),
        _ => None,
    };
    let outcome = Outcome {
        exit_ok: status.as_ref().is_some_and(|s| s.success()),
        saved: err_lines
            .iter()
            .chain(out_lines.iter())
            .any(|l| l.contains("screenshot saved=true")),
        timed_out,
        write_failures: err_lines
            .iter()
            .filter(|l| l.contains("write failed"))
            .count(),
        allow_write_failure: opts.allow_write_failure,
        must_say,
        said: must_say.is_none_or(|wanted| {
            err_lines
                .iter()
                .chain(out_lines.iter())
                .any(|l| l.contains(wanted))
        }),
    };

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

    println!(
        "{}: {} in {:.1}s (exit {}, screenshot saved={}, write-failures {}{})",
        if outcome.passed() { "PASS" } else { "FAIL" },
        opts.verb,
        elapsed.as_secs_f32(),
        status.map_or_else(
            || "?".into(),
            |s| s.code().map_or("signal".into(), |c| c.to_string())
        ),
        outcome.saved,
        outcome.write_failures,
        if timed_out { ", TIMED OUT" } else { "" },
    );
    if let Some(wanted) = outcome.must_say
        && !outcome.said
    {
        println!(
            "  the run never said `{wanted}` — and this verb's picture reads the same \
             whether it should have or not."
        );
    }
    if outcome.write_sank_it() {
        println!(
            "  git refused the write this verb asked for — the shot is of the state it \
             never reached. If the refusal is what the verb shows, say so with \
             --allow-write-failure."
        );
    }
    if outcome.passed() {
        Ok(())
    } else {
        Err(format!("verify-ui {} failed", opts.verb))
    }
}

/// Names of a spread of lengths, for the one verb whose subject is the
/// tab strip itself.
///
/// Both halves of the rule need seeing at once (デザイン規約 §ウィンドウの縁):
/// the short names keep their own width however crowded the strip gets,
/// and the long ones give way together. A row of repositories all called
/// `repo` can show neither.
const TAB_NAMES: [&str; 16] = [
    "ui",
    "platitude-gg",
    "core",
    "sealed-class-enumizer",
    "notes",
    "a-repository-with-a-rather-long-name",
    "docs",
    "another-repository-with-a-long-name",
    "spike",
    "yet-another-long-repository-name",
    "assets",
    "the-longest-repository-name-in-the-row",
    "ci",
    "one-more-long-repository-name-here",
    "tools",
    "a-final-repository-with-a-long-name",
];

/// The strip the `tab-widths` verb is run against: `count` repositories
/// (default 8) named off the ladder above.
///
/// The last one carries a real history — the tab opened last is the one
/// left in front, so that is the page under the strip in the picture.
/// The rest are bare of commits: what is being looked at is above them,
/// and building sixteen histories to photograph one band would be paying
/// for the wrong thing.
fn tab_width_repos(arg: &str) -> Result<Vec<PathBuf>, String> {
    let count: usize = if arg.is_empty() {
        8
    } else {
        arg.parse()
            .map_err(|_| format!("tab-widths takes a number of tabs, not {arg:?}"))?
    };
    if count == 0 || count > TAB_NAMES.len() {
        return Err(format!("tab-widths takes 1..={} tabs", TAB_NAMES.len()));
    }
    let mut made = Vec::with_capacity(count);
    for (position, name) in TAB_NAMES.iter().take(count).enumerate() {
        let preset = if position + 1 == count {
            "basic"
        } else {
            "empty"
        };
        let repo = crate::demo::create_named(preset, None, name)?;
        made.push(repo);
    }
    println!("demo repos (tab-widths): {count} named after their length");
    Ok(made)
}

/// The folder a verb needs handed to it, made on the spot.
///
/// The open-refused verbs are the only ones whose subject is a folder no
/// demo repository can be — being one is the whole point — so there is
/// nothing to name with `--preset`. Both sit one level down, so the
/// folder the second try opens at is a real one with the failed pick
/// inside it. Empty for every other verb: nothing is invented for a verb
/// that was simply run without its argument.
fn folder_for(
    verb: &str,
    shot_dir: &std::path::Path,
    path: &std::ffi::OsStr,
) -> Result<String, String> {
    let made = shot_dir.join("picked");
    match verb {
        "open-not-a-repo"
        | "open-not-a-repo-retry"
        | "open-not-a-repo-cancel"
        | "open-fail-tab"
        | "open-fail-tab-log" => {
            let dir = made.join("notes");
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            Ok(dir.display().to_string())
        }
        "open-bare" | "open-fail-tab-bare" => {
            let dir = made.join("origin.git");
            std::fs::create_dir_all(&made).map_err(|e| e.to_string())?;
            let status = Command::new("git")
                .args(["init", "--bare", "--quiet"])
                .arg(&dir)
                .env("PATH", path)
                .status()
                .map_err(|e| format!("failed to run git: {e}"))?;
            if !status.success() {
                return Err("git init --bare failed".into());
            }
            Ok(dir.display().to_string())
        }
        _ => Ok(String::new()),
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

#[cfg(test)]
mod tests {
    use super::{Outcome, parse};

    /// A run that reached the end and took its picture.
    const WELL: Outcome = Outcome {
        exit_ok: true,
        saved: true,
        timed_out: false,
        write_failures: 0,
        allow_write_failure: false,
        must_say: None,
        said: true,
    };

    #[test]
    fn a_verb_the_harness_stages_has_to_be_caught_saying_so() {
        // `solo` photographs an ordinary window if the lock was never
        // held, and an ordinary window takes a perfectly good picture.
        let quiet = Outcome {
            must_say: Some("solo blocked=true"),
            said: false,
            ..WELL
        };
        assert!(!quiet.passed());
        assert!(
            !quiet.write_sank_it(),
            "nothing was refused; the staging did not take"
        );
        assert!(
            Outcome {
                said: true,
                ..quiet
            }
            .passed()
        );
    }

    #[test]
    fn a_refused_write_sinks_the_run_however_good_the_picture() {
        assert!(WELL.passed());
        // The shape `verify-ui commit` came up in: the app started, quit
        // by itself and saved a screenshot, and git had refused the one
        // write the verb exists to make.
        let refused = Outcome {
            write_failures: 1,
            ..WELL
        };
        assert!(!refused.passed());
        assert!(refused.write_sank_it());
    }

    #[test]
    fn a_verb_that_walks_a_refusal_says_so_and_passes() {
        let expected = Outcome {
            write_failures: 3,
            allow_write_failure: true,
            ..WELL
        };
        assert!(expected.passed());
        assert!(!expected.write_sank_it());
    }

    #[test]
    fn the_other_ways_to_fail_are_not_blamed_on_the_write() {
        for broken in [
            Outcome {
                exit_ok: false,
                ..WELL
            },
            Outcome {
                saved: false,
                ..WELL
            },
            Outcome {
                timed_out: true,
                ..WELL
            },
        ] {
            assert!(!broken.passed(), "{broken:?}");
            assert!(!broken.write_sank_it(), "{broken:?}");
        }
    }

    #[test]
    fn the_flag_is_off_until_it_is_asked_for() {
        let plain = parse(&["commit".to_string()]).expect("verb only");
        assert!(!plain.allow_write_failure);
        let asked = parse(&[
            "fetch-fail".to_string(),
            "3".to_string(),
            "--allow-write-failure".to_string(),
        ])
        .expect("verb, arg and flag");
        assert!(asked.allow_write_failure);
        assert_eq!(asked.verb, "fetch-fail");
        assert_eq!(asked.arg, "3");
    }
}
