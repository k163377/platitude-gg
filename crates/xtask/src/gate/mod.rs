//! `cargo xtask gate` — the pre-merge tests, chosen by machine.
//!
//! What a branch owes main is read off its diff: the files it changed,
//! everything that reads them ([`graph`]), and the tests *in* that reach
//! — unit tests by module path, the integration binary by top-level
//! module, the verify-ui verbs by the census of what each one shows
//! ([`census`]) — plus the crate-wide checks (clippy, shipped, bare) for
//! the crates the reach enters ([`plan`]). Each step is cached by the
//! object ids of what it reads ([`stamp`]), so a second run of one commit
//! runs nothing, and a rebase reruns only what main's move touched. A
//! commit whose every owed step is green is stamped, and the
//! `reference-transaction` hook lets `refs/heads/main` move onto stamped
//! commits only ([`hooks`]). The host's verb runs rewrite their own
//! census lines as they go, so the file follows the change that moved it;
//! a gate that finds it rewritten stops before stamping, because the
//! commit that passed has to be the one holding it.
//!
//! `--host-only` is the daily tier (CLAUDE.md 確認は 3 段: no container);
//! its stamps are reused by the full run, which then owes the container
//! side alone. `--all` counts every file as changed — stage 2 in full,
//! what `check` used to be — and `--verb <line>` adds verify-ui lines to
//! run (and record) besides the census's.

mod census;
mod deps;
mod graph;
mod hooks;
mod plan;
mod stamp;

use std::path::Path;

use plan::{Plan, Required, Side};
use stamp::{CommitStamp, Store};

pub(crate) use census::{names_in, page_settled_in, record};
pub(crate) use hooks::{SESSION, SKIP, install};

pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("verdict") => {
            let [old, new] = &args[1..] else {
                return Err("gate verdict takes <old> <new>".into());
            };
            let dir = std::env::current_dir().map_err(|e| e.to_string())?;
            hooks::verdict(&dir, old, new)
        }
        Some("install") => {
            let dir = match args.get(1..3) {
                Some([flag, path]) if flag == "--dir" => std::path::PathBuf::from(path),
                _ => std::env::current_dir().map_err(|e| e.to_string())?,
            };
            println!("{}", install(&dir)?);
            Ok(())
        }
        Some("deps") => deps::run(&args[1..]),
        _ => gate(args),
    }
}

/// What the command line asks of the gate.
struct Options {
    /// The tree to gate: this workspace unless `--dir` names another (a
    /// seat from the primary, a throwaway repository in the tests).
    dir: std::path::PathBuf,
    main_ref: String,
    host_only: bool,
    all: bool,
    fresh: bool,
    dry_run: bool,
    verbs: Vec<String>,
}

fn options(args: &[String]) -> Result<Options, String> {
    let mut opts = Options {
        dir: crate::tree::workspace_root(),
        main_ref: "main".to_string(),
        host_only: false,
        all: false,
        fresh: false,
        dry_run: false,
        verbs: Vec::new(),
    };
    let mut at = 0;
    while let Some(arg) = args.get(at) {
        match arg.as_str() {
            "--host-only" => opts.host_only = true,
            "--all" => opts.all = true,
            "--fresh" => opts.fresh = true,
            "--dry-run" => opts.dry_run = true,
            "--dir" => {
                at += 1;
                opts.dir = std::path::PathBuf::from(args.get(at).ok_or("--dir needs a path")?);
            }
            "--main" => {
                at += 1;
                opts.main_ref = args.get(at).ok_or("--main needs a ref")?.clone();
            }
            "--verb" => {
                at += 1;
                opts.verbs
                    .push(args.get(at).ok_or("--verb needs a verify-ui line")?.clone());
            }
            other => {
                return Err(format!(
                    "unknown option {other:?} (gate takes --host-only, --all, --fresh, \
                     --dry-run, --dir <tree>, --main <ref>, --verb <line>…)"
                ));
            }
        }
        at += 1;
    }
    Ok(opts)
}

fn gate(args: &[String]) -> Result<(), String> {
    let opts = options(args)?;
    let plan = plan::make(
        &opts.dir,
        &plan::Ask {
            main_ref: &opts.main_ref,
            host_only: opts.host_only,
            all: opts.all,
            fresh: opts.fresh,
            extra_verbs: &opts.verbs,
        },
    )?;
    print!("{}", plan::describe(&plan));
    if opts.dry_run {
        return Ok(());
    }
    execute(&plan)
}

/// The gate for `land`: the seat's tree, both sides, the census's verbs.
pub(crate) fn for_landing(seat: &Path, main_ref: &str) -> Result<(), String> {
    let plan = plan::make(
        seat,
        &plan::Ask {
            main_ref,
            host_only: false,
            all: false,
            fresh: false,
            extra_verbs: &[],
        },
    )?;
    print!("{}", plan::describe(&plan));
    execute(&plan)
}

/// Runs what is not cached, one thread per side, and stamps the commit
/// when both sides came back with nothing red. A side stops at its first
/// failure; the other side finishes, so its green steps are stamped and
/// need not run again.
fn execute(plan: &Plan) -> Result<(), String> {
    let here = plan.dir.display().to_string();
    let dirty = crate::subprocess::git_query(&here, &["status", "--porcelain"])
        .ok_or("git status failed")?;
    if !dirty.is_empty() {
        return Err(format!(
            "the tree has uncommitted changes, and a stamp names a commit — commit or stash \
             first:\n{dirty}"
        ));
    }
    if !plan.uncovered.is_empty() {
        return Err(format!(
            "no verb shows these components, so the gate cannot pass them:\n{}\nRun a verb \
             that brings each one up (`cargo xtask verify-ui <verb> …`, or `gate --verb \
             <line>`) — a passing run records what it showed in {}, and from then on the gate \
             picks that verb by itself.",
            plan.uncovered
                .iter()
                .map(|f| format!("  {f}"))
                .collect::<Vec<_>>()
                .join("\n"),
            census::FILE
        ));
    }
    // What the census said before the verbs ran, so that a line one of
    // them rewrote can be told from the file as it was committed.
    let census_before = std::fs::read(plan.dir.join(census::FILE)).unwrap_or_default();
    let store = Store::open(&plan.dir)?;
    let host: Vec<&Required> = plan
        .required
        .iter()
        .filter(|r| r.step.side == Side::Host)
        .collect();
    let linux: Vec<&Required> = plan
        .required
        .iter()
        .filter(|r| r.step.side == Side::Linux)
        .collect();
    let started = std::time::Instant::now();
    let failures: Vec<String> = std::thread::scope(|scope| {
        let host = scope.spawn(|| side("host", &plan.dir, &store, &host));
        let linux = scope.spawn(|| side("linux", &plan.dir, &store, &linux));
        let mut failures = Vec::new();
        for handle in [host, linux] {
            match handle.join() {
                Ok(mut side_failures) => failures.append(&mut side_failures),
                Err(_) => failures.push("a side panicked".to_string()),
            }
        }
        failures
    });
    let secs = started.elapsed().as_secs();
    println!("gate: {}m{:02}s wall clock", secs / 60, secs % 60);
    let head = plan.head.chars().take(10).collect::<String>();
    // A verb that passed rewrote its census line whether or not another
    // step went red, so this is said on both roads out. A tree left dirty
    // without a word is the next gate refusing to run over a change
    // nobody made — which is the whole complaint the recording answers.
    let rewrote = if std::fs::read(plan.dir.join(census::FILE)).unwrap_or_default() != census_before
    {
        format!(
            " The verbs rewrote {}: it is generated, so review the diff and commit it (never by \
             hand), and the gate can stamp the commit that holds it.",
            census::FILE
        )
    } else {
        String::new()
    };
    if !failures.is_empty() {
        return Err(format!(
            "gate failed: {} — nothing stamped for {head}.{rewrote}",
            failures.join(" / ")
        ));
    }
    // Green, and the tree that passed is no longer the commit: a stamp
    // names one, so the file has to be in it before a stamp is written.
    if !rewrote.is_empty() {
        return Err(format!(
            "the verbs passed and the tree moved with them — nothing stamped for {head}.{rewrote}"
        ));
    }
    let stamp = CommitStamp {
        main: plan.main.clone(),
        base: plan.base.clone(),
        onto_main: plan.onto_main,
        full: !plan.host_only,
        steps: plan
            .required
            .iter()
            .map(|r| {
                format!(
                    "{} {}",
                    r.step.id,
                    if r.key.is_empty() { "-" } else { &r.key }
                )
            })
            .collect(),
    };
    store.mark_commit(&plan.head, &stamp)?;
    println!(
        "gate: PASS — {head} stamped ({}, {})",
        if stamp.full { "full" } else { "host-only" },
        if stamp.onto_main {
            "on main"
        } else {
            "off main"
        }
    );
    Ok(())
}

fn side(name: &str, dir: &Path, store: &Store, steps: &[&Required]) -> Vec<String> {
    let logs = dir.join("target").join("gate-logs");
    if let Err(e) = std::fs::create_dir_all(&logs) {
        return vec![format!("{}: {e}", logs.display())];
    }
    // Whether a verb of this side has built the release in this very
    // invocation. Only then may the next verb skip the build: a cached
    // verb's build happened in whatever tree took the stamp, and the
    // binary here may be older than the tree.
    let mut built = false;
    for (index, required) in steps.iter().enumerate() {
        let id = &required.step.id;
        if required.cached {
            println!("[{name}] cached {id}");
            continue;
        }
        let log = logs.join(format!("{name}-{index:02}.log"));
        println!("[{name}] run    {id} … (log: {})", log.display());
        let at = std::time::Instant::now();
        let mut command = required.step.command.clone();
        if required.step.builds_app && built {
            command.push("--no-build".to_string());
        }
        let outcome = execute_step(dir, &required.step.id, &command, &log);
        let secs = at.elapsed().as_secs();
        match outcome {
            Ok(()) => {
                println!("[{name}] ok     {id} ({secs}s)");
                built |= required.step.builds_app;
                crate::check::print_shots(name, &log);
                if !required.step.always
                    && let Err(why) = store.mark_step(
                        &required.key,
                        &format!("{id}\n{}\n", required.step.command.join(" ")),
                    )
                {
                    return vec![format!("{id}: green but not stamped: {why}")];
                }
            }
            Err(why) => {
                println!("[{name}] FAIL   {id} ({secs}s): {why}");
                return vec![id.clone()];
            }
        }
    }
    Vec::new()
}

/// One step, through `check`'s watched runner. With `PG_GATE_FAKE_LOG`
/// set the step is not run at all: its id is appended to that file and
/// it passes, unless `PG_GATE_FAKE_FAIL` names it — which is how the
/// tests watch selection and caching without a toolchain in the
/// throwaway repository.
fn execute_step(dir: &Path, id: &str, command: &[String], log: &Path) -> Result<(), String> {
    if let Ok(fake) = std::env::var("PG_GATE_FAKE_LOG") {
        use std::io::Write;
        // Both sides append from their own thread: one write per line,
        // under one lock, or the ids interleave mid-word.
        static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _turn = ONE_AT_A_TIME.lock().map_err(|e| e.to_string())?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&fake)
            .map_err(|e| format!("{fake}: {e}"))?;
        file.write_all(format!("{id}\n").as_bytes())
            .map_err(|e| e.to_string())?;
        let failing = std::env::var("PG_GATE_FAKE_FAIL").unwrap_or_default();
        if failing.split(',').any(|f| f == id) {
            return Err("failed on purpose (PG_GATE_FAKE_FAIL)".into());
        }
        return Ok(());
    }
    match crate::check::run_step(dir, command, log) {
        Ok(true) => Ok(()),
        Ok(false) => {
            let text =
                String::from_utf8_lossy(&std::fs::read(log).unwrap_or_default()).into_owned();
            let tail: Vec<&str> = text.lines().rev().take(40).collect();
            Err(format!(
                "exited non-zero (log: {})\n{}",
                log.display(),
                tail.into_iter().rev().collect::<Vec<_>>().join("\n")
            ))
        }
        Err(why) => Err(why),
    }
}

/// Stop hook: where the seat's gate stands, for the user's eyes. A seat
/// ahead of main whose tip carries no full stamp is work reported before
/// it was gated — said as a system message, not a block, so a turn that
/// ends mid-work is not held to a gate it was never claiming.
pub(crate) fn standing(cwd: &str) -> Option<String> {
    let root = crate::seats::worktree_root(cwd)?;
    let head = crate::subprocess::git_query(&root, &["rev-parse", "HEAD"])?;
    let ahead = crate::seats::commits_in(&root, "main..HEAD")?;
    if ahead == 0 {
        return None;
    }
    let store = Store::open(Path::new(&root)).ok()?;
    let seat = root.rsplit('/').next().unwrap_or_default();
    Some(match store.commit(&head) {
        None => format!(
            "gate: seat {seat} is {ahead} commit(s) ahead of main and its tip is not gated — \
             `cargo xtask gate` before 「マージ可」."
        ),
        Some(found) if !found.full => format!(
            "gate: seat {seat} is {ahead} commit(s) ahead of main; its tip is gated on the host \
             side only — the full `cargo xtask gate` still owes the container side."
        ),
        Some(found) if !found.onto_main => format!(
            "gate: seat {seat} is {ahead} commit(s) ahead of main; its tip was gated off main — \
             `land` will rebase and gate again."
        ),
        Some(_) => {
            format!("gate: seat {seat} is {ahead} commit(s) ahead of main, tip gated in full.")
        }
    })
}
