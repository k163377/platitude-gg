//! The run itself: build, start the app offscreen, wait on it, and judge
//! what came back.

use std::io::BufRead;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::options::parse;
use super::outcome::Outcome;
use super::repos::{body_for, folder_for, tab_width_repos};
use super::shim::{
    SHIM_REAL, SHIM_VERSION, identity_answer, identity_seed, real_git, stage_old_git,
};

/// Grace on top of PG_AUTO_QUIT_MS before the run is killed: startup,
/// repository load, and the write itself happen inside this.
const GRACE_MS: u64 = 20_000;

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
    cmd.current_dir(&root)
        .env("PATH", &child_path)
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
        // A pull that took more room than the pane had leaves what sits
        // under the box — the author card, the commit button — drawn over
        // the window's own footer, and that frames like a pane that fits:
        // the same blind spot details-fit answers for. Only that half is
        // judged here: whether the grip was offered at all depends on the
        // message, and the run where it stays away is half of the pair.
        "details-grow" | "details-grow-squeeze" | "wip-grow" | "wip-grow-squeeze" => {
            Some("description_grow keeps=true")
        }
        // Half of these the picture holds — which row is lit, and whose
        // commit fills the right-hand pane. The other half it cannot: in a
        // history short enough to fit on screen, a viewport that followed
        // the new commit and one that never moved frame the same way.
        "revert-commit" | "cherry-pick" | "merge-branch" => {
            Some("tip_landed follows=true onscreen=true")
        }
        // The caret is the whole of these two, and a hook that never
        // reached the box leaves a picture of the resting colour --
        // which is a real state, and the other half of each pair.
        "edit-message-focus" => Some("message_focus pane=details focused=true"),
        "wip-message-focus" => Some("message_focus pane=wip focused=true"),
        // A dialog that stayed open because the write did not take looks
        // exactly like one nobody has answered yet, and "which half
        // landed" is not something a picture holds at all.
        "identity" => Some(
            "identity state=missing dialog=true nameSaved=false emailSaved=false unsaved=false",
        ),
        "identity-half" => {
            Some("state=ready dialog=true nameSaved=true emailSaved=false unsaved=true")
        }
        // The three forced tooltips: a hook that never reached its
        // target photographs the resting state, which is a real state.
        // `tip=` is the attached ToolTip's own visible — the output
        // side, as everywhere.
        "identity-tip" => Some("identity_tip unsaved=true badge=true tip=true"),
        "signature-tip" => Some("signature_tip code=E tip=true"),
        "stash-tip" => Some("stash_tip blocked=true tip=true"),
        // The fourth: an elided row whose hover says the whole name.
        // One verb serves both panes (the argument picks one), so the
        // wanted line names neither — `tree=` echoes which view the
        // argument asked for (`-tree` keeps the tree, where row 0 is
        // the elided folder chain), `tip=` the shared instance's own
        // visible.
        "path-tip" if opts.arg.ends_with("-tree") => Some("tree=true tip=true"),
        "path-tip" => Some("tree=false tip=true"),
        // An emptied panel with a red mark over it and an emptied panel
        // with a quiet one frame the same from the waist down: the panel
        // is the picture, and the mark is 12 pixels of it in a corner.
        // `was=` is judged with it — a refusal that never landed leaves
        // a mark that was never red, and clearing nothing would pass.
        "commands-clear" => Some("commands_clear was=true wrong=false"),
        // Recovery is the show, and the picture can only hold its quiet
        // half: a band that failed and healed ends the run looking like
        // one that never failed at all. `was=`/`hadline=` are the red
        // half — without them, a fetch that never failed raised no line,
        // and taking down nothing would pass as recovery.
        "fetch-recover" => {
            Some("fetch_recover was=true hadline=true wrong=false line=false failures=0")
        }
        // `edge=` rides along in that report but is not judged: whether an
        // edge would land off the screen is a question about a real
        // monitor, and the offscreen platform has none to answer with.
        "window-fill" => Some("window_fill fills=true"),
        // A window held at its floor and one let past it frame alike — the
        // picture is of the panes either way, and the one that went past
        // simply has a pane outside the frame, where a screenshot cannot
        // follow. What the floor came to is a number or it is nothing.
        "window-floor" => Some("window_floor fits=true"),
        // The graph column pulled past its floor: the clamp has to land
        // on the floor exactly, and the floor is the message tick
        // brought up against lane 0's co-author badge without touching
        // it (GraphPane.graphColWMin). The number is the token
        // arithmetic spelled out — it moves only when those tokens do,
        // and a clamp that stopped anywhere else photographs just as
        // neatly, since the gap in question is one pixel of the frame.
        "graph-min" => Some("graph_min w=21 min=21"),
        // A drag carried past one of a divider's bounds. The badge is 12
        // pixels in the middle of a pane, and a run where the hook never
        // reached the divider photographs a window that looks entirely
        // well — so the refusal is said out loud. `line=` rides with it
        // because the two are a pair: the boundary still moves the other
        // way, and one that withdrew its line would be answering a
        // different question (that is `graph-divider`'s squeezed half).
        // One wanted line for every case: `line=` is already whichever
        // divider has the hand, so the argument does not change it.
        "divider-refuse" => Some("divider_refuse refuses=true line=true"),
        // The three states themselves. A run where one of them never stood
        // photographs a band that was never crowded, and that picture
        // cannot be told from a band that gave the crowd room — so the
        // crowd has to be said out loud.
        //
        // Neither the fit nor which of the group's three shapes landed is
        // judged: this verb takes a width, and the widths that show the
        // last shape are below the floor a hand can drag the window to
        // (`fits=false` is what was asked for there). The floor itself is
        // `window-floor`'s question.
        "badges" => Some("op=true conflicts=true identity=true"),
        // The card, opened. `rows=` is the half the picture cannot carry
        // on its own: a card with one row and a card with three frame the
        // same way once it is cropped to the band, and which rows arrived
        // is the whole question the group raises when it gives way.
        "badges-hover" => Some("card=true rows=op,conflicts,identity"),
        // The fourth badge. A run whose shim never reached PATH reads the
        // git this machine has, wears no badge, and photographs an
        // ordinary window — which is exactly what an ordinary window looks
        // like. `badge=` is the band's own reading, so the whole path from
        // `git --version` to the row is what passes or fails here.
        "old-git" => Some("old-git badge=true"),
        // And the card it opens. `rows=` is not judged: the other three
        // rows come and go with the machine (a container with no identity
        // configured stands one of them), and only this row is the verb's.
        "old-git-card" => Some("old-git badge=true card=true"),
        // The band folded with nothing red standing in it. Two halves, and
        // the picture holds neither on its own: a mark that never came up
        // frames as a band with room to spare, and the colour of three
        // dots is not something a cropped screenshot settles an argument
        // about. `tint=` is named rather than spelled in hex — what is
        // being judged is which rule painted it (規約 §状態: 最も重い状態が
        // 決める), and a red mark over a lone warning is the way that rule
        // fails silently.
        "old-git-fold" => Some("mark=true tint=warning"),
        // Walking the graph with the arrows. The picture holds which row
        // is lit and whose commit fills the right-hand pane, but not the
        // three things that make the walk work: that the keyboard was on
        // the list at all, that the settle behind a held key landed the
        // selection, and that the viewport carried the row it stepped
        // onto. A walk that moved nothing frames as a graph sitting still,
        // which is what a graph does most of the time.
        "graph-step" => Some(
            "landing=in held=false back=false refused=0 focused=true diff=false onscreen=true selected=true",
        ),
        // The other two landings, which no picture holds: a row brought in
        // flush against the bottom one step at a time, and a row centered
        // because the one it stepped off was nowhere on screen. Both frame
        // as a graph with a lit row somewhere in it.
        "graph-step-edge" => Some("landing=edge held=false back=false refused=0 focused=true"),
        "graph-step-far" => Some("landing=center held=false back=false refused=0 focused=true"),
        // The refusing halves. `back=true` is the whole of them — nothing
        // moved — and it is worth nothing without `refused=`, since a walk
        // that was never attempted leaves the same row lit. Read them
        // beside a plain `graph-step`: a step that always refuses passes
        // these two on its own.
        "graph-step-named" => Some("held=false back=true refused=1"),
        "graph-step-dirty" => Some("held=true back=true refused=1"),
        // A diff opened over the graph. `focused=false` is the mechanism —
        // Qt leaves active focus on a pane it has just swapped away, and
        // the keys go on arriving there — and `diff=true` is what the
        // report was about: a step behind the diff moves the selection,
        // and moving the selection closes the diff, so the screen jumps
        // back to the graph. A picture of the diff still standing is also
        // a picture of a run where the arrow was never pressed.
        "graph-step-diff" => Some("back=true refused=1 focused=false diff=true"),
        // The diff's own arrows, where the picture is the weakest witness
        // in the app: a diff scrolled two rows and a diff never scrolled
        // at all are the same photograph of the same file. Everything that
        // matters is in the line. `focused=true` is the arrival taking the
        // keyboard — nothing pressed this pane, so a false here means the
        // arrows would have been dead in a real window — and `moved=true`
        // with `atEnd=false stopped=false` is a walk that had somewhere to
        // go and went there.
        "diff-step" => Some("moved=true atEnd=false stopped=false focused=true"),
        // And the end it stops at rather than wraps past. `stopped=true`
        // is the refusal itself: the walk asks for twenty rows, gets as
        // far as the bottom, and the rest answer false. Without `moved=`
        // beside it a pane that refused every step from the start — never
        // on screen, never focused — would read the same.
        "diff-step-edge" => Some("moved=true atEnd=true stopped=true focused=true"),
        // Everything that acts on a row of a diff. The failure these
        // share is the one no camera catches: a pane the rows never
        // reached — because the read was still out, or because the file
        // was not dirty in the first place — is a pane with nothing in
        // it, and so is a file with nothing to show. The verb then names
        // a row that is not there, picks no lines, or holds a button
        // nobody drew, and none of it writes, so the run came back green
        // with an empty picture. `ready=` is the pane's own answer to
        // "were the rows here when I acted", and it is worth spelling
        // per verb: the wanted line
        // names the act, so a run whose hook never reached the diff at
        // all cannot borrow another verb's report to pass on.
        "diff-file" => Some("diff_row act=diff-file ready=true"),
        "line-tools" => Some("diff_row act=line-tools ready=true"),
        "hunk-tools" => Some("diff_row act=hunk-tools ready=true"),
        "stage-hunk" => Some("diff_row act=stage-hunk ready=true"),
        "stage-line" => Some("diff_row act=stage-line ready=true"),
        "keep-place" => Some("diff_row act=keep-place ready=true"),
        "discard-hunk" => Some("diff_row act=discard-hunk ready=true"),
        "discard-hunk-go" => Some("diff_row act=discard-hunk-go ready=true"),
        // These two answer with a count of their own, so the count is
        // what they are judged on rather than the line above — it says
        // the same thing (nothing can be picked out of rows that are not
        // there) and says it where the reader is already looking. Only
        // "none at all" fails: a first hunk carrying a single changed
        // line reports `got=1`, and that is the fixture choosing a
        // heading that names a hunk, not the wiring failing to name one.
        "pick-lines" | "stage-lines" => Some("picked_lines any=true"),
        // The colours that land behind the rows, and the place they must
        // not cost. Neither half is a picture: a diff whose colours never
        // came frames as a language the set has no rules for, and a view
        // thrown back to the top frames as one nobody had scrolled. `at=`
        // is `scrollTo`'s own number read back after the swap — 400 or
        // the reader lost their place.
        "colour-place" => Some("colour_place coloured=true at=400"),
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
