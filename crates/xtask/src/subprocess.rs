//! Running another program and reading what it said — the shapes every
//! verb here uses, and the git one in particular.

use std::fs::File;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::time::Duration;

use crate::wait::{Budget, LOOK_AGAIN, Wait};

/// Runs a command to completion, capturing output; errors carry context.
pub(crate) fn run_captured(
    cmd: &mut std::process::Command,
) -> Result<std::process::Output, String> {
    let display = format!("{cmd:?}");
    cmd.output()
        .map_err(|e| format!("failed to spawn {display}: {e}"))
}

/// What a bounded diagnostic came back with.
#[derive(Debug)]
pub(crate) enum Answer {
    /// It ended on its own: how, and what it had written by then to the
    /// file it was given for stdout — nothing where it was given none.
    Ended { status: ExitStatus, stdout: String },
    /// It stood past its ceiling and was ended here, after this long —
    /// or could not be, which is said, with the pid that may then still
    /// be running.
    OutOfTime {
        pid: u32,
        after: Duration,
        ended: Result<(), String>,
    },
    /// It could not be started, or asked after.
    Unstarted(String),
}

/// Runs `command` to its end or to `ceiling`, whichever comes first, and
/// says which. A diagnostic past its ceiling is ended and waited for
/// before this answers, so whatever runs after it never runs beside a
/// diagnostic still holding what it was looking at.
///
/// **Stdout goes to a file, never a pipe**: a pipe ends only when every
/// write handle closes, so a child of the diagnostic that inherited its
/// stdout and outlives it would hold the answer for as long as it lives.
/// `said` is that file; `None` for a diagnostic whose words are not
/// wanted.
pub(crate) fn bounded(
    what: &str,
    command: Command,
    ceiling: Duration,
    said: Option<&Path>,
) -> Answer {
    run_bounded(what, command, ceiling, said, false)
}

/// [`bounded`], with the diagnostic's stderr written into the same file
/// as its stdout — for a program whose refusal on stderr is the answer
/// (the container engine, `linux::container`). Not the default: a listing
/// parsed off the file (`verify::look`) must not have a warning
/// interleaved with its rows.
pub(crate) fn bounded_both_streams(
    what: &str,
    command: Command,
    ceiling: Duration,
    said: &Path,
) -> Answer {
    run_bounded(what, command, ceiling, Some(said), true)
}

fn run_bounded(
    what: &str,
    mut command: Command,
    ceiling: Duration,
    said: Option<&Path>,
    stderr_too: bool,
) -> Answer {
    let (stdout, stderr) = match said.map(File::create) {
        None => (Stdio::null(), Stdio::null()),
        Some(Ok(file)) if stderr_too => match file.try_clone() {
            Ok(twin) => (Stdio::from(file), Stdio::from(twin)),
            Err(error) => {
                return Answer::Unstarted(format!(
                    "{what} could not be given a file for what it says: {error}"
                ));
            }
        },
        Some(Ok(file)) => (Stdio::from(file), Stdio::null()),
        Some(Err(error)) => {
            return Answer::Unstarted(format!(
                "{what} could not be given a file for what it says: {error}"
            ));
        }
    };
    command.stdin(Stdio::null()).stdout(stdout).stderr(stderr);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => return Answer::Unstarted(format!("{what} could not be started: {error}")),
    };
    let mut wait = Wait::new(what, Budget::whole(ceiling), LOOK_AGAIN);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = said
                    .and_then(|path| std::fs::read(path).ok())
                    .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                    .unwrap_or_default();
                return Answer::Ended { status, stdout };
            }
            Ok(None) => {}
            Err(error) => {
                return Answer::Unstarted(format!("{what} could not be asked after: {error}"));
            }
        }
        if wait.look_again("its exit").is_err() {
            // `kill` only asks; the wait is what says the process is gone.
            let ended = child
                .kill()
                .and_then(|()| child.wait())
                .map(|_| ())
                .map_err(|error| error.to_string());
            return Answer::OutOfTime {
                pid: child.id(),
                after: wait.elapsed(),
                ended,
            };
        }
    }
}

/// Runs git in `dir`, answering its trimmed stdout with backslashes
/// forward, and None when it fails at all — callers treat a repository
/// that cannot answer as one they are not judging.
pub(crate) fn git_query(dir: &str, arguments: &[&str]) -> Option<String> {
    if dir.is_empty() {
        return None;
    }
    let mut command = std::process::Command::new("git");
    command.arg("-C").arg(dir).args(arguments);
    let output = run_captured(&mut command).ok()?;
    output.status.success().then(|| {
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .replace('\\', "/")
    })
}

/// The git directory `dir` belongs to, shared by all of its worktrees.
pub(crate) fn common_git_dir(dir: &str) -> Option<String> {
    git_query(
        dir,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
}

/// Whether the process `pid` names still exists. A probe that could not
/// answer says "alive": every caller asks this to decide whether somebody
/// else's claim may be broken, and breaking a live one costs more than
/// leaving a dead one standing.
#[cfg(windows)]
pub(crate) fn process_exists(pid: u32) -> bool {
    !matches!(behind(pid), Behind::Nobody)
}

/// The same, where a signal-less kill is the question.
#[cfg(not(windows))]
pub(crate) fn process_exists(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|status| status.success())
        .unwrap_or(true)
}

/// What is behind a pid right now, in the three answers a probe can
/// honestly give. A pid is handed out again once its process is gone, so
/// a dead claim would otherwise stand for as long as whatever inherited
/// the number; the image name tells the two apart.
enum Behind {
    /// The program at that number, as this probe spells it.
    Named(String),
    /// The probe answered, and nobody is at that number.
    Nobody,
    /// The probe could not be asked at all.
    Unanswerable,
}

/// Who holds `pid`, asked of `tasklist`.
#[cfg(windows)]
fn behind(pid: u32) -> Behind {
    let Ok(out) = std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
        .output()
    else {
        return Behind::Unanswerable;
    };
    listed_by_tasklist(
        out.status.success(),
        &String::from_utf8_lossy(&out.stdout),
        &String::from_utf8_lossy(&out.stderr),
        pid,
    )
}

/// What `tasklist` said, as one of the three answers.
///
/// **A probe that could not ask is not a process that is gone**: a
/// refused, missing or policy-restricted `tasklist` also matches nothing,
/// and reading that as nobody hands out a running process's claim. So a
/// non-zero exit, or anything on stderr, is `Unanswerable`.
#[cfg(windows)]
fn listed_by_tasklist(ok: bool, stdout: &str, stderr: &str, pid: u32) -> Behind {
    if !ok || !stderr.trim().is_empty() {
        return Behind::Unanswerable;
    }
    // `"xtask.exe","12345","Console","1","75,836 K"`: the image and the
    // pid are the first two fields, ahead of the one holding a comma.
    stdout
        .lines()
        .find_map(|line| {
            let mut fields = line.split(',').map(|field| field.trim().trim_matches('"'));
            let image = fields.next().unwrap_or_default().to_string();
            (fields.next().unwrap_or_default() == pid.to_string()).then_some(image)
        })
        .map_or(Behind::Nobody, Behind::Named)
}

/// The same, asking `ps` for the state and the command name.
///
/// **A zombie is nobody**: it holds nothing, and in the container PID 1
/// is cargo, which reaps nothing it did not start — read as alive, it
/// would hold a machine claim for as long as the container lived.
#[cfg(not(windows))]
fn behind(pid: u32) -> Behind {
    let Ok(out) = std::process::Command::new("ps")
        .args(["-o", "stat=,comm=", "-p", &pid.to_string()])
        .output()
    else {
        return Behind::Unanswerable;
    };
    // A pid nobody has answers with nothing and a non-zero exit.
    let answer = String::from_utf8_lossy(&out.stdout);
    let answer = answer.trim();
    if !out.status.success() || answer.is_empty() {
        return Behind::Nobody;
    }
    let mut fields = answer.split_whitespace();
    let state = fields.next().unwrap_or_default();
    let name = fields.next().unwrap_or_default();
    if state.starts_with('Z') || name.is_empty() {
        return Behind::Nobody;
    }
    Behind::Named(name.to_string())
}

/// Whether the process `pid` names still exists and is a program
/// `is_wanted` accepts. Answers "alive" when it could not ask, as
/// [`process_exists`] does.
fn image_matches(pid: u32, is_wanted: impl Fn(&str) -> bool) -> bool {
    match behind(pid) {
        Behind::Named(image) => is_wanted(&image),
        Behind::Nobody => false,
        Behind::Unanswerable => true,
    }
}

/// The program behind `pid`, for the tests only: the probes ask after a
/// name they already have ([`image_still_at`]) or after this very program
/// ([`task_runner_exists`]), never for a name to record.
#[cfg(test)]
pub(crate) fn image_of(pid: u32) -> Option<String> {
    match behind(pid) {
        Behind::Named(image) => Some(image),
        Behind::Nobody | Behind::Unanswerable => None,
    }
}

/// Whether anything is still *running* at `pid` — for a claim with no
/// name to compare against (`budget::Pool::leftover`). Stricter than
/// [`process_exists`]: a process that has exited and not been waited on
/// holds nothing, and [`behind`] reads it as nobody.
pub(crate) fn running_at(pid: u32) -> bool {
    image_matches(pid, |_| true)
}

/// Whether `pid` still names this task runner — for the claims nothing
/// but the runner ever writes. The runner is the one program this may
/// name outright: it is asking after itself.
pub(crate) fn task_runner_exists(pid: u32) -> bool {
    image_matches(pid, is_task_runner)
}

/// Whether the program a claim recorded is still the one at `pid` — for
/// claims about somebody else's process (the budget's tickets), whose
/// name this has no business knowing; it only compares.
pub(crate) fn image_still_at(pid: u32, recorded: &str) -> bool {
    image_matches(pid, |image| same_image(image, recorded))
}

/// Whether two spellings name one program. The tolerance is a path, case
/// and the executable suffix ([`stem`]) — never a guess at what any
/// particular program is called.
fn same_image(image: &str, recorded: &str) -> bool {
    !recorded.is_empty() && stem(image).eq_ignore_ascii_case(stem(recorded))
}

/// Whether an image name is this runner's, however it is spelled: cargo's
/// `xtask.exe`, a test binary's `xtask-<hash>`, a landing's
/// `xtask-inflight-<pid>` (`land::step_out_of_the_build_slot`).
fn is_task_runner(image: &str) -> bool {
    basename(image)
        .get(..5)
        .is_some_and(|head| head.eq_ignore_ascii_case("xtask"))
}

/// An image name with whatever path a probe put in front of it taken off.
fn basename(image: &str) -> &str {
    image.rsplit(['/', '\\']).next().unwrap_or(image)
}

/// The same, with the executable suffix off as well: a claim that records
/// what a launcher spelled has `cargo` where the machine hands back
/// `cargo.exe` (`budget::Pool::leftover`). A name that is nothing but a
/// suffix is left whole: two dotfiles are not one program.
fn stem(image: &str) -> &str {
    let base = basename(image);
    match base.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem,
        _ => base,
    }
}

/// A program a caller spelled itself, written in the vocabulary a probe
/// on this machine answers in — for a claim that has to record a name
/// without paying a process to ask.
///
/// **Linux keeps only the first fifteen characters of a program's name**
/// (`TASK_COMM_LEN` less its terminator): recorded whole, a runner copy
/// `xtask-runner-12345` never matches the `xtask-runner-1` that `ps`
/// answers, and reads as a stranger's on every step. Windows and macOS do
/// not truncate.
pub(crate) fn as_probed(program: &str) -> String {
    let stem = stem(program);
    if !cfg!(target_os = "linux") {
        return stem.to_string();
    }
    const COMM: usize = 15;
    stem.char_indices()
        .nth(COMM)
        .map_or(stem, |(at, _)| &stem[..at])
        .to_string()
}

/// A pid no process on this machine can carry — for the tests that need a
/// claim whose process is gone. A reaped child's pid will not do: the
/// kernel hands it out again, and under a suite forking git it is a live
/// stranger's before the assertion runs. Windows hands out multiples of
/// four only, and Linux's `pid_max` is 2^22 at the most: odd, and above
/// both.
#[cfg(test)]
pub(crate) const NO_SUCH_PID: u32 = 0x7FFF_FFFD;

#[cfg(test)]
mod tests {
    use super::{
        NO_SUCH_PID, image_of, image_still_at, is_task_runner, process_exists, same_image,
        task_runner_exists,
    };

    #[test]
    #[cfg(windows)]
    fn a_refused_listing_is_not_a_process_that_is_gone() {
        use super::{Behind, listed_by_tasklist};
        let named = |seen: Behind| matches!(seen, Behind::Named(_));
        assert!(
            named(listed_by_tasklist(
                true,
                "\"xtask.exe\",\"1234\",\"Console\",\"1\",\"75,836 K\"\n",
                "",
                1234
            )),
            "the ordinary answer"
        );
        assert!(
            matches!(
                listed_by_tasklist(
                    true,
                    "INFO: No tasks are running which match the specified criteria.\n",
                    "",
                    1234
                ),
                Behind::Nobody
            ),
            "a pid nobody has is answered, and the answer is nobody"
        );
        for (ok, stdout, stderr) in [
            (false, "", "ERROR: Access is denied.\n"),
            (false, "", ""),
            (true, "", "ERROR: The RPC server is unavailable.\n"),
        ] {
            assert!(
                matches!(
                    listed_by_tasklist(ok, stdout, stderr, 1234),
                    Behind::Unanswerable
                ),
                "a refused listing read as a process that is gone ({ok}, {stderr:?})"
            );
        }
    }

    #[test]
    fn the_runner_is_known_by_its_image_name_however_cargo_spelled_it() {
        for image in [
            "xtask.exe",
            "XTASK.EXE",
            "xtask",
            "xtask-4aadee3d869c602a.exe",
            "xtask-inflight-31336.exe",
            "C:\\x\\target\\debug\\xtask.exe",
            "/x/target/debug/xtask",
        ] {
            assert!(is_task_runner(image), "{image}");
        }
        for image in [
            "git.exe",
            "ping.exe",
            "cargo.exe",
            "",
            "xtas",
            "notxtask.exe",
        ] {
            assert!(!is_task_runner(image), "{image}");
        }
    }

    /// An empty recording matches nothing, so a claim that recorded
    /// nothing cannot be read as naming what it met.
    #[test]
    fn one_program_is_known_by_the_name_the_claim_recorded() {
        for (image, recorded) in [
            ("claude.exe", "claude.exe"),
            ("CLAUDE.EXE", "claude.exe"),
            ("node", "node"),
            ("node.exe", "C:\\Program Files\\nodejs\\node.exe"),
            ("/usr/local/bin/claude", "claude"),
            // What a launcher spelled against what the loader ran.
            ("cargo.exe", "cargo"),
            ("docker.exe", "docker"),
        ] {
            assert!(same_image(image, recorded), "{image} vs {recorded}");
        }
        for (image, recorded) in [
            ("node.exe", "claude.exe"),
            ("git.exe", "claude.exe"),
            ("claude.exe", ""),
            ("", ""),
            ("claude.exe", "claude-code.exe"),
            // Nothing but a suffix.
            (".bashrc", ".profile"),
        ] {
            assert!(!same_image(image, recorded), "{image} vs {recorded}");
        }
    }

    /// A live number is not by itself an answer: the runner reads as the
    /// runner and not as whatever else a claim recorded.
    #[test]
    fn this_process_is_alive_and_the_pid_nobody_can_have_is_not() {
        let me = std::process::id();
        let mine = image_of(me).expect("this process is behind its own pid");
        assert!(is_task_runner(&mine), "{mine}");
        assert!(process_exists(me));
        assert!(task_runner_exists(me));
        assert!(image_still_at(me, &mine));
        assert!(!image_still_at(me, "claude.exe"));
        assert!(!process_exists(NO_SUCH_PID));
        assert!(!task_runner_exists(NO_SUCH_PID));
        assert!(!image_still_at(NO_SUCH_PID, &mine));
        assert_eq!(image_of(NO_SUCH_PID), None);
    }
}
