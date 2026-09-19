//! What one step of a gate is actually started from: the task runner it
//! begins at, the line it is begun as, and the log that start leaves.
//!
//! Kept out of [`super`], which is about what a branch owes main and what
//! may be stamped for it. Which binary a step begins at is a question of
//! its own — the answer is a copy, and where a copy comes from, whom it
//! may overwrite and what happens when there is none are all this file.

use std::path::Path;

use super::{census, default_jobs, rank, seat_of};

/// The tests' switch: with it set no step runs at all (`execute_step`).
pub(super) const FAKE_LOG: &str = "PGG_GATE_FAKE_LOG";

/// The tests' switch for a step another tree stamped while this one
/// waited for room ([`run_one`]).
pub(super) const FAKE_STAMP: &str = "PGG_GATE_FAKE_STAMP";

/// Where a faked run records the steps it was handed `--no-build`, so a
/// test can see which of a block's verbs were told to reuse a release
/// and which were left to build one ([`verbs`]). Beside the fake log,
/// whose own lines are the ids and nothing else — every test reads that
/// one as a set of ids, and widening it would rewrite all of them.
fn no_build_log(fake: &str) -> String {
    format!("{fake}.no-build")
}

/// One step, through `check`'s watched runner, its xtask launcher
/// swapped for the runner copy ([`launched`]). With `PGG_GATE_FAKE_LOG`
/// set the step is not run at all: its id is appended to that file and
/// it passes, unless `PGG_GATE_FAKE_FAIL` names it — which is how the
/// tests watch selection and caching without a toolchain in the
/// throwaway repository. `PGG_GATE_FAKE_REWRITE` names a step that
/// rewrites the census the way a passing verb does: one line put in,
/// once, so the run after the commit of it finds nothing to move.
pub(super) fn execute_step(
    dir: &Path,
    id: &str,
    command: &[String],
    log: &Path,
    runner: Option<&Path>,
    room: &crate::budget::Admitted,
) -> Result<(), String> {
    if let Ok(fake) = std::env::var(FAKE_LOG) {
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
        if command.iter().any(|word| word == "--no-build") {
            let mut told = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(no_build_log(&fake))
                .map_err(|e| format!("{fake}: {e}"))?;
            told.write_all(format!("{id}\n").as_bytes())
                .map_err(|e| e.to_string())?;
        }
        let failing = std::env::var("PGG_GATE_FAKE_FAIL").unwrap_or_default();
        if failing.split(',').any(|f| f == id) {
            return Err("failed on purpose (PGG_GATE_FAKE_FAIL)".into());
        }
        if let Some(line) = id.strip_prefix("verify ")
            && std::env::var("PGG_GATE_FAKE_REWRITE").is_ok_and(|step| step == id)
        {
            // The verb's own line with one more name on it — the shape
            // of a run that met a component it had not before.
            let path = dir.join(census::FILE);
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
        return Ok(());
    }
    match crate::check::run_step(dir, &launched(command, runner), log, room) {
        Ok(true) => Ok(()),
        Ok(false) => Err(format!(
            "exited non-zero (log: {})\n{}",
            log.display(),
            crate::check::log_tail(log)
        )),
        Err(why) => Err(why),
    }
}

/// What a runner copy is called, beside the logs; the pid follows.
const RUNNER: &str = "xtask-runner-";

/// The task runner the steps start from: this program, built from the
/// tree once and copied beside the logs.
///
/// A step that is one of xtask's own verbs is spelled `cargo run -p
/// xtask -- …` in the plan, and cargo holds `target/debug` for the
/// length of any build there — so beside the checks ([`side`]) every
/// verb's launcher would wait out whatever `cargo test` was compiling,
/// and even with nothing beside them but each other a fifth of a gate's
/// host verb logs showed the wait on the build directory (none from the
/// copy). Built with cargo all the same, so that it is the
/// tree's code — under `land` the rebase has just brought sources in —
/// and copied, because the landing has renamed
/// the slot away from under this very process and the slot is what cargo
/// rebuilds. What earlier gates left is taken away first; a copy a
/// process of theirs still holds stays, its name carrying their pid.
pub(super) fn runner(
    dir: &Path,
    logs: &Path,
    jobs: usize,
    landing: bool,
) -> Result<std::path::PathBuf, String> {
    let exe = format!("xtask{}", std::env::consts::EXE_SUFFIX);
    // A compile like any other, and out of the same budget: this is the
    // one the gate runs before its sides, so a machine full of seats
    // would otherwise start every gate with an uncounted cargo.
    let pool = crate::budget::Pool::of(dir, jobs.max(default_jobs()))?;
    let room = pool.admit_once_the_machine_is_free(&crate::budget::Ask {
        weight: crate::budget::COMPILE,
        rank: rank(landing),
        seat: &seat_of(dir),
        what: "the task runner's build",
    })?;
    // Through the same road every step takes (`check::run_step`): the
    // announcement to a measurement, the ceiling, and the tree kill at it.
    // A build with no ceiling would sit on a rustc holding this tree's
    // build lock with the gate's own lock held, and the next gate here
    // would be refused by a live pid saying nothing.
    let build_log = logs.join(format!("{RUNNER}build-{}.log", std::process::id()));
    let build = ["cargo", "build", "--locked", "-p", "xtask"].map(String::from);
    match crate::check::run_step(dir, &build, &build_log, &room) {
        Ok(true) => {}
        Ok(false) => {
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

/// The command as it is started: a step that is one of this program's
/// own verbs — `cargo run --locked -p xtask -- <verb>…`, which the plan
/// keeps spelling so that a stamp's key names one line on every machine
/// — starts from the runner copy, and any other step as spelled.
fn launched(command: &[String], runner: Option<&Path>) -> Vec<String> {
    const THROUGH_CARGO: [&str; 6] = ["cargo", "run", "--locked", "-p", "xtask", "--"];
    let through_cargo = command.len() >= THROUGH_CARGO.len()
        && command
            .iter()
            .zip(THROUGH_CARGO)
            .all(|(word, spelled)| word == spelled);
    match runner {
        Some(runner) if through_cargo => std::iter::once(runner.display().to_string())
            .chain(command[THROUGH_CARGO.len()..].iter().cloned())
            .collect(),
        _ => command.to_vec(),
    }
}

/// Whether a red verb's log says the app itself did not build — cargo's
/// own line, or the runner's when it reports the build (`tree::app_exe`).
pub(super) fn app_did_not_build(log: &Path) -> bool {
    let text = String::from_utf8_lossy(&std::fs::read(log).unwrap_or_default()).into_owned();
    text.contains("could not compile") || text.contains("cargo build --release failed")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{app_did_not_build, launched};

    fn words(line: &[&str]) -> Vec<String> {
        line.iter().map(|w| (*w).to_string()).collect()
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
                Some(runner)
            ),
            words(&[
                "C:/x/target/gate-logs/xtask-runner-7.exe",
                "verify-ui",
                "wip",
                "--no-build"
            ])
        );
        let test = words(&["cargo", "test", "--locked", "-p", "xtask", "--lib"]);
        assert_eq!(launched(&test, Some(runner)), test);
        let verb = words(&["cargo", "run", "--locked", "-p", "xtask", "--", "structure"]);
        assert_eq!(
            launched(&verb, None),
            verb,
            "without a copy the plan's spelling stands"
        );
        // An older spelling, without the lock, is not this runner's line
        // any more: it is started as spelled, and cargo answers for it.
        let unlocked = words(&["cargo", "run", "-p", "xtask", "--", "structure"]);
        assert_eq!(launched(&unlocked, Some(runner)), unlocked);
    }

    #[test]
    fn a_build_that_failed_is_read_off_the_verbs_log() {
        let dir = std::env::temp_dir().join(format!("pgg-gate-build-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
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
        let _ = std::fs::remove_dir_all(&dir);
    }
}
