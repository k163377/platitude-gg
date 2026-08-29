//! The run itself: build, start the app offscreen, wait on it, and judge
//! what came back.

use std::collections::BTreeSet;
use std::io::BufRead;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::options::parse;
use super::outcome::Outcome;
use super::ownership::{claim_resource, fresh_shot_dir};
use super::repos::{body_for, folder_for, seed_merge_tool, tab_name_repos, tab_width_repos};
use super::shim::{
    SHIM_REAL, SHIM_VERSION, identity_answer, identity_seed, real_git, stage_old_git,
};
use super::verbs;

/// Grace after the app-side watchdog before the parent reaps a wedged GUI.
const GRACE_MS: u64 = 20_000;

#[expect(clippy::too_many_lines)]
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
    } else if opts.verb == "tab-name" {
        // A different strip entirely: names that collide, which the
        // ladder above deliberately has none of.
        tab_name_repos()?
    } else if opts.verb == "tab-drag" || opts.verb == "tab-hold" {
        // Same ladder of names, four of them: the order is what this one
        // is about, and four differently named tabs say an order a
        // picture can be read for. The argument names two of them.
        tab_width_repos("4")?
    } else if opts.verb == "tab-edge" {
        // A strip that has to overflow: eight of them, against a window
        // the verb puts down on its floor.
        tab_width_repos("8")?
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

    // Explicit resources are allowed to survive across sequential runs,
    // but never to be owned by two verify-ui processes at once. That would
    // mix Git writes, settings, or PNGs and can manufacture a false PASS.
    // Fresh preset repositories need no cross-process claim: their creator
    // already gave this run a private directory.
    let mut claimed = BTreeSet::new();
    let mut _resource_claims = Vec::new();
    if !opts.repo.is_empty() {
        for repo in &repos {
            if let Some(claim) = claim_resource(repo, "repository", &mut claimed)? {
                _resource_claims.push(claim);
            }
        }
    }

    // Written after the claim above, so a repository this run was handed
    // is owned before it is added to.
    seed_merge_tool(&opts.verb, &repos, &path)?;

    // The build's PATH, not the run's: the git shim below goes onto the
    // child's PATH only, so the build never sees it.
    let exe = crate::app_exe(&root, &path, opts.build, &[])?;

    let shot_dir = match &opts.shot_dir {
        Some(dir) => {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            if let Some(claim) = claim_resource(dir, "shot directory", &mut claimed)? {
                _resource_claims.push(claim);
            }
            dir.clone()
        }
        None => fresh_shot_dir(&opts.verb)?,
    };

    // A run of its own unless told otherwise. The app would refuse the
    // real files anyway once it sees a PG_* variable, but naming a
    // directory is what lets one run read what the last one wrote — and
    // what lets anyone look at the two files afterwards.
    let config_dir = match &opts.config_dir {
        Some(dir) => dir.clone(),
        None => shot_dir.join("config"),
    };
    std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
    if opts.config_dir.is_some()
        && let Some(claim) = claim_resource(&config_dir, "config directory", &mut claimed)?
    {
        _resource_claims.push(claim);
    }
    println!("config dir: {}", config_dir.display());

    // The verbs that need the configuration to say something before the
    // run starts: a window smaller than any floor the layout has. It is
    // what a file written before there was a floor looks like, and the
    // way in is the only place that can put it right. Written here rather
    // than by hand in the app, because the app never writes a shape it
    // could not take — so nothing inside it could produce this state.
    // `window-floor` is about the lift itself; the stepping verbs ride
    // the same seed for the height, since no demo repository has more
    // commits than the default window shows at once, and a graph with
    // nothing below the fold has no viewport rule to answer. The diff's
    // arrows want it for the same reason from the other side: no demo
    // file's diff is taller than a default window either, and a pane with
    // nothing to scroll answers every step the way a broken one would.
    if opts.verb == "window-floor"
        || opts.verb == "graph-step-edge"
        || opts.verb == "graph-step-far"
        || opts.verb == "diff-step"
        || opts.verb == "diff-step-edge"
    {
        let state = config_dir.join("state.toml");
        std::fs::write(
            &state,
            "version = 1\n\n[window]\nwidth = 320\nheight = 240\nmaximized = false\n",
        )
        .map_err(|e| format!("could not write {}: {e}", state.display()))?;
        println!("seeded window: 320x240 (under every floor)");
    }

    // The other side of the same idea: `details-fit` is about the pane's
    // right column, and half of what it claims only bites where the
    // author row is narrower than the block it stands in. At the default
    // 400 the row is already wider than the block, so it is clamped to
    // the block whether or not it was told to fill — and a row that
    // never filled answers exactly like one that did. Seeded wide, the
    // same run puts the hash plate 540px short of the pane's edge the
    // moment the fill is gone (2026-08-28 ユーザー報告).
    if opts.verb == "details-fit" {
        let state = config_dir.join("state.toml");
        std::fs::write(&state, "version = 1\n\n[layout]\ndetails_width = 800\n")
            .map_err(|e| format!("could not write {}: {e}", state.display()))?;
        println!("seeded layout: an 800px details pane (wider than the row asks for)");
    }
    // The band above the diff cuts its path at the head, and a band with
    // room for the whole path never cuts at all — so the run that has to
    // say the cut works has to be given a narrow band. Seeded from the
    // other side: the details pane is what the diff is left over from,
    // and 1100 of the window's 1440 leaves the band a couple of hundred
    // pixels, which every path in the presets is longer than.
    if opts.verb == "diff-band-sweep" {
        let state = config_dir.join("state.toml");
        std::fs::write(&state, "version = 1\n\n[layout]\ndetails_width = 1100\n")
            .map_err(|e| format!("could not write {}: {e}", state.display()))?;
        println!("seeded layout: an 1100px details pane (so the band has to cut)");
    }

    let arg = match opts.arg.is_empty() {
        true => match body_for(&opts.verb) {
            Some(body) => body,
            None => folder_for(&opts.verb, &shot_dir, &path)?,
        },
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
    // The three verbs named for it bring their own version, so that the
    // run reads `verify-ui old-git` and nothing else. Below any minimum
    // this app will ever have: minimums only go up.
    let old_git = match (opts.old_git.as_str(), opts.verb.as_str()) {
        ("", "old-git" | "old-git-card" | "old-git-fold") => "2.42.0",
        (asked, _) => asked,
    };
    let child_path = if old_git.is_empty() {
        path.clone()
    } else {
        let staged = stage_old_git(&shot_dir, &path)?;
        println!("git for this run: {old_git} (real git behind it)");
        staged
    };

    let mut cmd = Command::new(&exe);
    crate::app_env::clear_automation(&mut cmd);
    cmd.current_dir(&root)
        .env("PATH", &child_path)
        .env("QT_QPA_PLATFORM", "offscreen")
        .env("QT_FORCE_STDERR_LOGGING", "1")
        .env("PG_CONFIG_DIR", &config_dir)
        .env("PG_AUTO_WATCHDOG_MS", opts.watchdog_ms.to_string())
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
    if !old_git.is_empty() {
        // The two the copy on the front of PATH reads: what to answer
        // `--version` with, and who to hand the rest to. Both are set on
        // the app, so every git it starts inherits them.
        cmd.env(SHIM_VERSION, old_git)
            .env(SHIM_REAL, real_git(&path)?);
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
            .env("PG_AUTO_IDENTITY", identity_answer(&opts.verb, &opts.arg));
        if opts.verb == "identity-half" || opts.verb == "identity-tip" {
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
    let _held = if opts.verb == "solo" || opts.verb == "gate-sweep" {
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
    let deadline = Duration::from_millis(opts.watchdog_ms + GRACE_MS);
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

    let must_say = verbs::must_say(&opts.verb, &opts.arg);
    let outcome = Outcome {
        exit_ok: status.as_ref().is_some_and(|s| s.success()),
        saved: err_lines
            .iter()
            .chain(out_lines.iter())
            .any(|l| l.contains("screenshot saved=true")),
        timed_out,
        watchdog_expired: err_lines
            .iter()
            .chain(out_lines.iter())
            .any(|l| l.contains("auto-act watchdog expired")),
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
    // Onto the board as the run goes, not when somebody remembers: the
    // seat is read from the working directory there, so a picture that
    // is registered is a picture that says which tree took it. A pass is
    // not the condition — a failing run's picture is the one most worth
    // looking at.
    if !shots.is_empty() && !opts.no_board {
        let label = if opts.label.is_empty() {
            format!("{} {}", opts.verb, opts.arg).trim().to_string()
        } else {
            opts.label.clone()
        };
        match crate::shots::record(&label, &opts.verb, &shots) {
            Ok(page) => println!("board: {}", crate::shots::shown(&page)),
            // The board is not what this run is judging. Say the reason
            // and let the verdict stand on the pictures themselves.
            Err(message) => println!("board: not updated ({message})"),
        }
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
    if outcome.watchdog_expired {
        println!(
            "  the app's own watchdog ended this run — the verb never reached its \
             completion, and a run that wedged between the two grabs still leaves app.png \
             behind to pass on."
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
