//! What one step of a gate is actually started from: the task runner it
//! begins at, the line it is begun as, and the log that start leaves.
//!
//! Kept out of [`super`] (what a branch owes main): where a runner copy
//! comes from, whom it may overwrite and what happens when there is none.

use std::path::Path;

use super::plan::Required;
use super::sides::rank;
use super::{Ground, census};

/// The tests' switch: with it set no step runs at all (`execute_step`).
pub(super) const FAKE_LOG: &str = "PGG_GATE_FAKE_LOG";

/// The tests' switch for a step another tree stamped while this one
/// waited for room ([`run_one`](super::step::run_one)).
pub(super) const FAKE_STAMP: &str = "PGG_GATE_FAKE_STAMP";

/// Where a faked run records the steps it was handed `--no-build`
/// ([`super::sides::verbs`]). A file of its own: every test reads the
/// fake log as a set of ids.
fn no_build_log(fake: &str) -> String {
    format!("{fake}.no-build")
}

/// The tests' switch for a step still running when the run goes red
/// elsewhere: a faked step it names holds until the halt reaches it
/// ([`faked`]); faked steps otherwise end the instant they start.
pub(super) const FAKE_HOLD: &str = "PGG_GATE_FAKE_HOLD";

/// A step `PGG_GATE_FAKE_FAIL` names goes red only once the step this
/// names has started ([`faked`]): the sides are two threads, and the red
/// could otherwise end the held step at its door.
pub(super) const FAKE_FAIL_AFTER: &str = "PGG_GATE_FAKE_FAIL_AFTER";

/// How a step that was started came back, short of red.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Finished {
    Green,
    /// Ended by the run's halt before it could answer (`super::halt`).
    Halted,
}

/// One step, through `check`'s watched runner, its xtask launcher
/// swapped for the runner copy ([`launched`]). With `PGG_GATE_FAKE_LOG`
/// set the step is not run at all ([`faked`]).
pub(super) fn execute_step(
    ground: &Ground<'_>,
    id: &str,
    command: &[String],
    log: &Path,
    room: &crate::budget::Admitted,
) -> Result<Finished, String> {
    if let Ok(fake) = std::env::var(FAKE_LOG) {
        return faked(ground, id, command, &fake);
    }
    // A verb going into the gate's container carries a mark: what a stop
    // is addressed to when the step is ended out here.
    let mark = ground
        .container
        .filter(|_| into_the_container(command))
        .map(|_| step_mark(ground.run, log));
    // Ended by the halt only where everything it started ends with it. A
    // `linux` line with a container of its own would go on working past
    // its launcher, so it is left to finish, holding its room.
    let stoppable = mark.is_some() || !starts_its_own_container(command);
    match crate::check::run_step(
        ground.dir,
        &launched(
            command,
            ground.runner,
            ground.copy,
            ground.container,
            mark.as_deref(),
        ),
        log,
        room,
        &|| stoppable && ground.halt.raised(),
    ) {
        Ok(crate::check::Stepped::Exited(true)) => Ok(Finished::Green),
        // Ended out here for the run's halt: what it started inside the
        // container is reached by its mark, as at a ceiling below.
        Ok(crate::check::Stepped::Stopped) => {
            if let (Some(container), Some(mark)) = (ground.container, &mark) {
                let said = crate::linux::container::stop_step(container, mark, ground.logs)
                    .unwrap_or_else(|line| line);
                println!("[{}] {id}: inside the container, {said}", ground.name);
            }
            Ok(Finished::Halted)
        }
        Ok(crate::check::Stepped::Exited(false)) => Err(format!(
            "exited non-zero (log: {})\n{}",
            log.display(),
            crate::check::log_tail(log)
        )),
        // A ceiling out here reaps only the host side; the container's
        // end is reached by its mark, said in the same failure.
        Err(why) => match (ground.container, mark) {
            (Some(container), Some(mark)) => Err(format!(
                "{why}; inside the container, {}",
                // Either way it came back, the line is what the reader
                // needs: the step is red already.
                crate::linux::container::stop_step(container, &mark, ground.logs)
                    .unwrap_or_else(|line| line)
            )),
            _ => Err(why),
        },
    }
}

/// A step of the tests' faked runs ([`FAKE_LOG`]): its id appended to
/// that file, and then green — or red where `PGG_GATE_FAKE_FAIL` names
/// it, held where [`FAKE_HOLD`] does. `PGG_GATE_FAKE_REWRITE` names a
/// step that rewrites the census the way a passing verb does, once, so
/// the run after its commit finds nothing to move.
fn faked(
    ground: &Ground<'_>,
    id: &str,
    command: &[String],
    fake: &str,
) -> Result<Finished, String> {
    use std::io::Write;
    // Both sides append from their own thread: one write per line,
    // under one lock, or the ids interleave mid-word. Let go before
    // a held step waits, which is for the other side's red.
    static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());
    {
        let _turn = ONE_AT_A_TIME.lock().map_err(|e| e.to_string())?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(fake)
            .map_err(|e| format!("{fake}: {e}"))?;
        file.write_all(format!("{id}\n").as_bytes())
            .map_err(|e| e.to_string())?;
        if command.iter().any(|word| word == "--no-build") {
            let mut told = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(no_build_log(fake))
                .map_err(|e| format!("{fake}: {e}"))?;
            told.write_all(format!("{id}\n").as_bytes())
                .map_err(|e| e.to_string())?;
        }
    }
    let failing = std::env::var("PGG_GATE_FAKE_FAIL").unwrap_or_default();
    if failing.split(',').any(|f| f == id) {
        if let Ok(after) = std::env::var(FAKE_FAIL_AFTER) {
            let mut wait = crate::wait::Wait::new(
                format!("{after} to start before {id} goes red"),
                crate::wait::Budget::whole(crate::check::STEP_CEILING),
                crate::wait::LOOK_AGAIN,
            );
            while !std::fs::read_to_string(fake)
                .unwrap_or_default()
                .lines()
                .any(|started| started == after)
            {
                wait.look_again("the step to start")
                    .map_err(|expired| expired.to_string())?;
            }
        }
        return Err("failed on purpose (PGG_GATE_FAKE_FAIL)".into());
    }
    if std::env::var(FAKE_HOLD).is_ok_and(|held| held == id) {
        let mut wait = crate::wait::Wait::new(
            format!("the held step {id}"),
            crate::wait::Budget::whole(crate::check::STEP_CEILING),
            crate::wait::LOOK_AGAIN,
        );
        while !ground.halt.raised() {
            wait.look_again("the run's halt")
                .map_err(|expired| expired.to_string())?;
        }
        return Ok(Finished::Halted);
    }
    if let Some(line) = id.strip_prefix("verify ")
        && std::env::var("PGG_GATE_FAKE_REWRITE").is_ok_and(|step| step == id)
    {
        // The verb's own line with one more name on it.
        let path = ground.dir.join(census::FILE);
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let rewritten: String = text
            .lines()
            .map(|held| {
                if held.starts_with(&format!("{line}\t")) && !held.ends_with(" Theme") {
                    format!("{held} Theme\n")
                } else {
                    format!("{held}\n")
                }
            })
            .collect();
        if rewritten != text {
            std::fs::write(&path, rewritten).map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    Ok(Finished::Green)
}

/// Whether a plan's step is a `linux` line, which brings up a container
/// of its own (`docker run --rm`) unless it goes into the gate's
/// container by its mark ([`into_the_container`]).
fn starts_its_own_container(command: &[String]) -> bool {
    command.iter().any(|word| word == "linux")
}

/// The mark a step's processes carry inside the gate's container: this
/// run's name and the step's log name, which together name one step of
/// one gate and are letters, digits and dashes (`linux::runner::spelled`).
fn step_mark(run: &str, log: &Path) -> String {
    format!(
        "{run}-{}",
        log.file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default()
    )
}

/// Whether a plan's step is one that starts from the copy inside the
/// container — `linux` and then one of this program's own verbs.
fn into_the_container(command: &[String]) -> bool {
    command
        .iter()
        .position(|word| word == "linux")
        .is_some_and(|at| crate::linux::a_runner_verb(&command[at + 1..]))
}

/// The container the Linux side will have if its preparation
/// ([`linux_runner`]) goes through, on a host that is not Linux. Named
/// here and nowhere else: the side execs into it only once the
/// preparation came back green, and takes it down afterwards either way —
/// a start that failed half way may have left it.
pub(super) fn container_expected(ground: &Ground<'_>, steps: &[&Required]) -> Option<String> {
    if cfg!(target_os = "linux") || ground.runner.is_none() || !a_copy_is_wanted(steps) {
        return None;
    }
    Some(crate::linux::container::container_of(
        ground.dir, ground.run,
    ))
}

/// Takes the gate's container down once its side is over, and says
/// what became of it. Never a failure of the gate: a container that
/// would not go is said, and leaves on its own or with the next gate
/// (`linux::container::remove_container`).
pub(super) fn dismiss_container(ground: &Ground<'_>, name: &str) {
    println!(
        "[{}] the gate's container: {}",
        ground.name,
        crate::linux::container::remove_container(ground.dir, name)
    );
}

/// What a runner copy is called, beside the logs; the pid follows.
const RUNNER: &str = "xtask-runner-";

/// The task runner the steps start from: this program, built from the
/// tree once and copied beside the logs, so no `cargo run` launcher waits
/// on cargo's lock of `target/debug`
/// (反映前テストの機械化.md §群の並走と動詞の並列).
///
/// Built from the tree because under `land` the rebase has just brought
/// sources in; copied because the landing has renamed the slot away from
/// under this very process. What earlier gates left is taken away first;
/// a copy a process of theirs still holds stays, its name carrying their
/// pid.
pub(super) fn runner(
    dir: &Path,
    logs: &Path,
    jobs: usize,
    landing: bool,
) -> Result<std::path::PathBuf, String> {
    let exe = format!("xtask{}", std::env::consts::EXE_SUFFIX);
    // A compile out of the same budget as any other, or every gate would
    // start with an uncounted cargo.
    let pool = crate::budget::Pool::of(dir, jobs.max(crate::budget::default_jobs()))?;
    let room = pool.admit_once_the_machine_is_free(&crate::budget::Ask {
        weight: crate::budget::COMPILE,
        rank: rank(landing),
        seat: &crate::budget::seat_of(dir),
        what: "the task runner's build",
    })?;
    // Through `check::run_step` for its ceiling: a build with none would
    // sit holding this tree's build lock and the gate's own, and the next
    // gate here would be refused by a live pid saying nothing.
    let build_log = logs.join(format!("{RUNNER}build-{}.log", std::process::id()));
    let build = ["cargo", "build", "--locked", "-p", "xtask"].map(String::from);
    match crate::check::run_step(dir, &build, &build_log, &room, &|| false) {
        Ok(crate::check::Stepped::Exited(true)) => {}
        // Nothing asks this build to stop: it comes before either side.
        Ok(crate::check::Stepped::Exited(false) | crate::check::Stepped::Stopped) => {
            return Err(format!(
                "the task runner did not build in {}:\n{}",
                dir.display(),
                crate::check::log_tail(&build_log)
            ));
        }
        // A ceiling, a cargo already announced on this tree, or a spawn
        // that failed: the step's own words say which.
        Err(why) => {
            return Err(format!(
                "the task runner's build in {} did not run to its end: {why}",
                dir.display()
            ));
        }
    }
    if let Ok(entries) = std::fs::read_dir(logs) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(RUNNER) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    let built = dir.join("target").join("debug").join(&exe);
    let copy = logs.join(format!(
        "{RUNNER}{}{}",
        std::process::id(),
        std::env::consts::EXE_SUFFIX
    ));
    std::fs::copy(&built, &copy).map_err(|e| {
        format!(
            "could not copy {} to {}: {e}",
            built.display(),
            copy.display()
        )
    })?;
    Ok(copy)
}

/// The Linux side's own copy of the task runner, prepared before the
/// side's first step and named after this run — or `None` when no step
/// would start from one ([`a_copy_is_wanted`]; the plan's look suffices,
/// as a queued step's second look can only take work away).
///
/// Without it every container verb resolves /work's manifests and lock
/// across the mount before it begins; with it they start from a binary,
/// as [`runner`] does out here.
///
/// A failure here is the side's failure, with no road back to
/// `cargo xtask` (`linux::runner`). A run that goes red while this waits
/// for room gets `None`; once it builds it builds to its end, being a
/// container of its own ([`execute_step`]).
pub(super) fn linux_runner(
    ground: &Ground<'_>,
    steps: &[&Required],
) -> Result<Option<String>, String> {
    // Under the tests' faked steps there is no runner and nothing runs.
    let Some(host) = ground.runner else {
        return Ok(None);
    };
    if !a_copy_is_wanted(steps) {
        return Ok(None);
    }
    let name = ground.run.to_string();
    // Out of the same budget as [`runner`]'s own build: a container's
    // cargo is this machine's cargo.
    let Some(room) = ground.pool.admit_unless(
        &crate::budget::Ask {
            weight: crate::budget::COMPILE,
            rank: ground.rank,
            seat: ground.seat,
            what: "the container's task runner",
        },
        &|| ground.halt.raised(),
    )?
    else {
        return Ok(None);
    };
    ground.waited.add(room.waited);
    // One name, written over by the next gate: a tree runs one gate at a
    // time (`lanes::sole`).
    let log = ground.logs.join("linux-runner.log");
    println!(
        "[linux] run    the container's task runner … (log: {})",
        log.display()
    );
    // This process's pid, the one in the tree's gate note (`lanes::sole`):
    // the step runs only if that live gate named it
    // (`linux::runner::owned_by_the_gate`), since this checkout's volume
    // and its one preparation container are not free to take beside a gate.
    let line = [
        host.display().to_string(),
        "linux".to_string(),
        "runner".to_string(),
        name.clone(),
        "--gate".to_string(),
        std::process::id().to_string(),
    ];
    match crate::check::run_step(ground.dir, &line, &log, &room, &|| false) {
        Ok(crate::check::Stepped::Exited(true)) => Ok(Some(name)),
        Ok(crate::check::Stepped::Stopped) => Ok(None),
        Ok(crate::check::Stepped::Exited(false)) => Err(format!(
            "the container's task runner did not build (log: {})\n{}",
            log.display(),
            crate::check::log_tail(&log)
        )),
        Err(why) => Err(format!(
            "the container's task runner did not run to its end: {why}"
        )),
    }
}

/// Whether any uncached step of this side is one of this program's own
/// verbs in the container. Pure, so asking for a container nobody needed
/// is a test and not a run.
fn a_copy_is_wanted(steps: &[&Required]) -> bool {
    steps.iter().any(|r| {
        !r.cached
            && r.step
                .command
                .iter()
                .position(|word| word == "linux")
                .is_some_and(|at| crate::linux::a_runner_verb(&r.step.command[at + 1..]))
    })
}

/// The command as it is started: `cargo run --locked -p xtask -- <verb>…`
/// (kept so in the plan, so a stamp's key names one line on every
/// machine) starts from the runner copy, any other step as spelled.
///
/// A container verb of this program's own is also handed the copy
/// prepared in there ([`linux_runner`]) and, where the gate has a
/// container, its name and this step's mark, so the line goes in by exec
/// (`linux::container::exec_in`). A `linux test` in there stays cargo's.
fn launched(
    command: &[String],
    runner: Option<&Path>,
    copy: Option<&str>,
    container: Option<&str>,
    mark: Option<&str>,
) -> Vec<String> {
    const THROUGH_CARGO: [&str; 6] = ["cargo", "run", "--locked", "-p", "xtask", "--"];
    let through_cargo = command.len() >= THROUGH_CARGO.len()
        && command
            .iter()
            .zip(THROUGH_CARGO)
            .all(|(word, spelled)| word == spelled);
    let Some(runner) = runner.filter(|_| through_cargo) else {
        return command.to_vec();
    };
    let verb = &command[THROUGH_CARGO.len()..];
    let mut line = vec![runner.display().to_string()];
    match copy {
        Some(name)
            if verb.first().is_some_and(|word| word == "linux")
                && crate::linux::a_runner_verb(&verb[1..]) =>
        {
            line.extend([
                "linux".to_string(),
                "--runner".to_string(),
                name.to_string(),
            ]);
            if let (Some(container), Some(mark)) = (container, mark) {
                line.extend([
                    "--container".to_string(),
                    container.to_string(),
                    "--step".to_string(),
                    mark.to_string(),
                ]);
            }
            line.extend(verb[1..].iter().cloned());
        }
        _ => line.extend(verb.iter().cloned()),
    }
    line
}

/// Whether a red verb's log says the app itself did not build — cargo's
/// own line, or the runner's when it reports the build (`app_build::app_exe`).
pub(super) fn app_did_not_build(log: &Path) -> bool {
    let text = String::from_utf8_lossy(&std::fs::read(log).unwrap_or_default()).into_owned();
    text.contains("could not compile") || text.contains("cargo build --release failed")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::super::plan::{Side, Step};
    use super::{Required, a_copy_is_wanted, app_did_not_build, launched};

    fn words(line: &[&str]) -> Vec<String> {
        line.iter().map(|w| (*w).to_string()).collect()
    }

    fn owed(line: &str, cached: bool) -> Required {
        Required {
            step: Step {
                id: line.to_string(),
                side: Side::Linux,
                always: false,
                builds_app: false,
                release: false,
                command: words(&["cargo", "run", "--locked", "-p", "xtask", "--"])
                    .into_iter()
                    .chain(line.split_whitespace().map(String::from))
                    .collect(),
                inputs: std::collections::BTreeSet::new(),
            },
            key: "k".to_string(),
            cached,
        }
    }

    #[test]
    fn a_copy_is_asked_for_only_when_a_step_would_start_from_one() {
        let verb = owed("linux verify-ui wip", false);
        let stamped = owed("linux verify-ui wip", true);
        let cargo = owed("linux test -p platitude-core", false);
        let bare = owed("linux bare", false);
        assert!(a_copy_is_wanted(&[&verb]));
        assert!(a_copy_is_wanted(&[&cargo, &stamped, &verb]));
        assert!(
            !a_copy_is_wanted(&[&stamped]),
            "every verb answered by a stamp: nothing starts from a copy"
        );
        assert!(
            !a_copy_is_wanted(&[&cargo, &bare]),
            "cargo's own work in there needs no copy"
        );
        assert!(!a_copy_is_wanted(&[]));
    }

    #[test]
    fn an_xtask_step_starts_from_the_runner_and_a_cargo_step_as_spelled() {
        let runner = Path::new("C:/x/target/gate-logs/xtask-runner-7.exe");
        assert_eq!(
            launched(
                &words(&[
                    "cargo",
                    "run",
                    "--locked",
                    "-p",
                    "xtask",
                    "--",
                    "verify-ui",
                    "wip",
                    "--no-build"
                ]),
                Some(runner),
                None,
                None,
                None
            ),
            words(&[
                "C:/x/target/gate-logs/xtask-runner-7.exe",
                "verify-ui",
                "wip",
                "--no-build"
            ])
        );
        let test = words(&["cargo", "test", "--locked", "-p", "xtask", "--lib"]);
        assert_eq!(launched(&test, Some(runner), None, None, None), test);
        let verb = words(&["cargo", "run", "--locked", "-p", "xtask", "--", "structure"]);
        assert_eq!(
            launched(&verb, None, None, None, None),
            verb,
            "without a copy the plan's spelling stands"
        );
        // A spelling without `--locked` is not this runner's line.
        let unlocked = words(&["cargo", "run", "-p", "xtask", "--", "structure"]);
        assert_eq!(
            launched(&unlocked, Some(runner), None, None, None),
            unlocked
        );
    }

    #[test]
    fn a_container_verb_names_this_runs_copy_and_a_cargo_in_there_does_not() {
        let runner = Path::new("C:/x/target/gate-logs/xtask-runner-7.exe");
        let verb = words(&[
            "cargo",
            "run",
            "--locked",
            "-p",
            "xtask",
            "--",
            "linux",
            "verify-ui",
            "wip",
            "--no-build",
        ]);
        assert_eq!(
            launched(&verb, Some(runner), Some("1758-40"), None, None),
            words(&[
                "C:/x/target/gate-logs/xtask-runner-7.exe",
                "linux",
                "--runner",
                "1758-40",
                "verify-ui",
                "wip",
                "--no-build"
            ])
        );
        // No copy prepared: the line is what it always was.
        assert_eq!(
            launched(&verb, Some(runner), None, None, None),
            words(&[
                "C:/x/target/gate-logs/xtask-runner-7.exe",
                "linux",
                "verify-ui",
                "wip",
                "--no-build"
            ])
        );
        // A cargo command in there is cargo's work whatever is prepared.
        for line in [
            words(&[
                "cargo",
                "run",
                "--locked",
                "-p",
                "xtask",
                "--",
                "linux",
                "test",
                "-p",
                "platitude-core",
            ]),
            words(&[
                "cargo", "run", "--locked", "-p", "xtask", "--", "linux", "bare",
            ]),
        ] {
            let started = launched(&line, Some(runner), Some("1758-40"), None, None);
            assert!(
                !started.contains(&"--runner".to_string()),
                "{started:?} is cargo's line"
            );
        }
    }

    /// Only a container verb: a cargo line in there is as spelled,
    /// whatever is up.
    #[test]
    fn a_container_verb_goes_into_the_gates_container_under_its_mark() {
        let runner = Path::new("C:/x/target/gate-logs/xtask-runner-7.exe");
        let verb = words(&[
            "cargo",
            "run",
            "--locked",
            "-p",
            "xtask",
            "--",
            "linux",
            "verify-ui",
            "wip",
            "--no-build",
        ]);
        assert_eq!(
            launched(
                &verb,
                Some(runner),
                Some("1758-40"),
                Some("pgg-linux-gate-c-1758-40"),
                Some("1758-40-linux-12")
            ),
            words(&[
                "C:/x/target/gate-logs/xtask-runner-7.exe",
                "linux",
                "--runner",
                "1758-40",
                "--container",
                "pgg-linux-gate-c-1758-40",
                "--step",
                "1758-40-linux-12",
                "verify-ui",
                "wip",
                "--no-build"
            ])
        );
        let cargo = words(&[
            "cargo",
            "run",
            "--locked",
            "-p",
            "xtask",
            "--",
            "linux",
            "test",
            "-p",
            "platitude-core",
        ]);
        let started = launched(
            &cargo,
            Some(runner),
            Some("1758-40"),
            Some("pgg-linux-gate-c-1758-40"),
            Some("1758-40-linux-03"),
        );
        assert!(!started.contains(&"--container".to_string()), "{started:?}");
        assert!(super::into_the_container(&verb));
        assert!(!super::into_the_container(&cargo));
        // The mark: this run and this log's name, spelled as a name.
        let mark = super::step_mark("1758-40", Path::new("C:/x/target/gate-logs/linux-12.log"));
        assert_eq!(mark, "1758-40-linux-12");
    }

    #[test]
    fn a_build_that_failed_is_read_off_the_verbs_log() {
        let dir = crate::yard::Yard::new("gate-build");
        let log = dir.join("host-08.log");
        std::fs::write(
            &log,
            "building (release, automation)…\nerror[E0425]: cannot find value\n\
             error: could not compile `platitude-app` (bin \"platitude-gg\") due to 1 previous error\n",
        )
        .expect("a log");
        assert!(app_did_not_build(&log));
        std::fs::write(
            &log,
            "building (release, automation)…\nFAIL: wip in 2.1s (exit 1)\n",
        )
        .expect("a log");
        assert!(
            !app_did_not_build(&log),
            "a verb red on its own account is not a build that failed"
        );
        assert!(
            !app_did_not_build(&dir.join("host-99.log")),
            "no log, no build to have failed"
        );
    }
}
