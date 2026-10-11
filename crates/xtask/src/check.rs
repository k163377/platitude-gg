//! One step under watch, both streams on a log file of its own: the quiet
//! and the whole ceiling, and the stop a gate's halt hands it. The gate
//! (`gate::runner`) and `replay` start every step through here.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::wait::{Budget, LOOK_AGAIN, Wait};

/// How long a step may say nothing before it is killed and named: the long
/// steps keep talking, while a hang goes quiet first. Above the suite's own
/// `OVERALL_BUDGET` (900s), so that fires first and names the test. The
/// log growing renews it (`wait::Wait::saw`).
const QUIET_CEILING: Duration = Duration::from_secs(20 * 60);

/// The ceiling per step, for a hang that keeps talking. Also how long a
/// killed unit's leftover runs before it is reported; its room stays held
/// (`budget::Pool::leftover`).
pub(crate) const STEP_CEILING: Duration = Duration::from_secs(90 * 60);

/// Echoes where a verify-ui's PNGs landed: a green run still needs them
/// looked at (verify-ui skill).
pub(crate) fn print_shots(side: &str, log: &Path) {
    let text = String::from_utf8_lossy(&std::fs::read(log).unwrap_or_default()).into_owned();
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

/// The end of a step's log, for a failure that has to say why in place.
pub(crate) fn log_tail(log: &Path) -> String {
    tail_of(&std::fs::read_to_string(log).unwrap_or_default())
}

/// The last lines of a step's output, as [`log_tail`] reads them.
pub(crate) fn tail_of(text: &str) -> String {
    const LINES: usize = 40;
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(LINES)..].join("\n")
}

/// How a step ended under [`run_step`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stepped {
    /// It exited, and this is whether it passed.
    Exited(bool),
    /// `stop` said so while it ran, and its process tree was ended — a
    /// gate that went red elsewhere (`gate::halt`).
    Stopped,
}

/// One step with both streams on its log file, watched. `Ok` is the step's
/// verdict or the stop it was ended for; `Err` is a ceiling, a spawn
/// failure or a Qt of another version on PATH (`crate::qt`). `stop` is
/// the gate's halt; everyone else hands `&|| false`. A
/// stop ends the step's process tree and nothing past it — a container the
/// step brought up keeps working — so hand `stop` only to a step whose
/// work ends with that tree or is reached by a mark (`gate::runner`).
pub(crate) fn run_step(
    root: &Path,
    step: &[String],
    log: &Path,
    room: &crate::budget::Admitted,
    stop: &dyn Fn() -> bool,
) -> Result<Stepped, String> {
    let out = std::fs::File::create(log).map_err(|e| format!("{}: {e}", log.display()))?;
    let err = out
        .try_clone()
        .map_err(|e| format!("{}: {e}", log.display()))?;
    // Announced as one build (`still`); the step runs under it and
    // announces nothing more.
    let _busy = crate::still::busy(root, &step.join(" "))?;
    let mut command = Command::new(&step[0]);
    command.args(&step[1..]).current_dir(root);
    // The pinned Qt for every cargo the step starts, whatever Qt the shell
    // that started the gate names (`crate::qt`). Where there is no Qt at
    // all, a step that needs none runs as it is and one that does fails on
    // its own; a qmake of another version stops the step here.
    if let Some(path) = crate::qt::path_with_qt_if_any()? {
        command.env("PATH", path);
    }
    crate::still::step(&mut command);
    // Every caller holds a ticket for this step (`room`), so what the step
    // starts takes no second one.
    crate::budget::under(&mut command);
    // So that a step ended at a ceiling takes its cargo's rustc with it,
    // and this side's build lock stays free (`reap`).
    crate::reap::own_group(&mut command);
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err))
        .spawn()
        .map_err(|e| e.to_string())?;
    // The ticket is this runner's but the load is the child's, which a
    // ledger outliving this process has to find (`budget::Pool::leftover`).
    room.started(child.id(), &step[0]);
    let mut wait = Wait::new(
        "the step",
        Budget::of(QUIET_CEILING, STEP_CEILING),
        LOOK_AGAIN,
    );
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return Ok(Stepped::Exited(status.success()));
        }
        if stop() {
            let (reaped, _ended) = crate::reap::reap(&mut child);
            // Into the step's log: it did not fail, it was ended.
            let note = format!(
                "\n[gate] ended here: the run went red elsewhere — {}\n",
                reaped.line()
            );
            if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(log) {
                use std::io::Write;
                let _unsaid = file.write_all(note.as_bytes());
            }
            return Ok(Stepped::Stopped);
        }
        let len = std::fs::metadata(log).map(|m| m.len()).unwrap_or(0);
        wait.saw(format!("{len} bytes of log"));
        if let Err(expired) = wait.look_again("its exit") {
            let (reaped, _ended) = crate::reap::reap(&mut child);
            return Err(format!(
                "killed at the ceiling — {expired}, and {} (its log has the tail; an empty \
                 log usually means it never got past a build lock another cargo holds)",
                reaped.line()
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Stepped, run_step};
    use crate::budget::{Ask, Pool, Rank};
    use crate::yard::Yard;

    /// A step that says it is running, then outlives any test: what ends
    /// it has to be the stop.
    fn sleeper() -> Vec<String> {
        let line: &[&str] = if cfg!(windows) {
            &[
                "powershell",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Write-Output ready; Start-Sleep -Seconds 600",
            ]
        } else {
            &["sh", "-c", "echo ready; sleep 600"]
        };
        line.iter().map(|word| (*word).to_string()).collect()
    }

    /// Stopped only once the step says it is running, so `Stopped` is a
    /// step ended mid-run, not one that never started; the log says so.
    #[test]
    fn a_step_told_to_stop_is_ended_where_it_stands() {
        let dir = Yard::new("check-stop");
        let room = Pool::at(&dir, 24)
            .admit(&Ask {
                weight: 1,
                rank: Rank::Normal,
                seat: "t",
                what: "the sleeper",
            })
            .unwrap();
        let log = dir.join("step.log");
        let running = || std::fs::read_to_string(&log).is_ok_and(|text| text.contains("ready"));
        let ended = run_step(&dir, &sleeper(), &log, &room, &running).unwrap();
        assert_eq!(ended, Stepped::Stopped);
        let text = std::fs::read_to_string(&log).unwrap();
        assert!(
            text.contains("ended here: the run went red elsewhere"),
            "{text}"
        );
    }
}
