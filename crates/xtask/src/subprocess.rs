//! Running another program and reading what it said — the two shapes
//! every verb here uses, and the git one in particular.
//!
//! Not in main.rs, for the reason `tree` gives: a helper on the crate
//! root ties every module to every other in the gate's dependency graph.

/// Runs a command to completion, capturing output; errors carry context.
pub(crate) fn run_captured(
    cmd: &mut std::process::Command,
) -> Result<std::process::Output, String> {
    let display = format!("{cmd:?}");
    cmd.output()
        .map_err(|e| format!("failed to spawn {display}: {e}"))
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

/// The repository `dir` belongs to, shared by all of its worktrees, so
/// that a merge run from a worktree is recognised as the same repository
/// as the session that runs it, and a hold taken beside it is seen from
/// every seat.
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
    // tasklist exits 0 found or not; the filter's answer is the output.
    std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).contains(&format!("\"{pid}\"")))
        .unwrap_or(true)
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
/// honestly give. A pid is a name the machine hands out again the moment
/// its process is gone, so a claim a killed process left behind would
/// otherwise stand for as long as whatever inherited the number — a git,
/// a browser tab — and hold what it claimed until that stranger exits.
/// The image name tells the two apart.
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
    // `"xtask.exe","12345","Console","1","75,836 K"`: the image and the
    // pid are the first two fields, ahead of the one holding a comma.
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|line| {
            let mut fields = line.split(',').map(|field| field.trim().trim_matches('"'));
            let image = fields.next().unwrap_or_default().to_string();
            (fields.next().unwrap_or_default() == pid.to_string()).then_some(image)
        })
        .map_or(Behind::Nobody, Behind::Named)
}

/// The same, asking `ps` for the command name.
#[cfg(not(windows))]
fn behind(pid: u32) -> Behind {
    let Ok(out) = std::process::Command::new("ps")
        .args(["-o", "comm=", "-p", &pid.to_string()])
        .output()
    else {
        return Behind::Unanswerable;
    };
    // A pid nobody has answers with nothing and a non-zero exit.
    match String::from_utf8_lossy(&out.stdout).trim() {
        name if out.status.success() && !name.is_empty() => Behind::Named(name.to_string()),
        _ => Behind::Nobody,
    }
}

/// Whether the process `pid` names still exists **and is a program
/// `is_wanted` accepts** — the shape every claim that records its writer
/// is asked with. Answers "alive" when it could not ask, as
/// [`process_exists`] does: every caller asks this to decide whether
/// somebody else's claim may be broken.
fn image_matches(pid: u32, is_wanted: impl Fn(&str) -> bool) -> bool {
    match behind(pid) {
        Behind::Named(image) => is_wanted(&image),
        Behind::Nobody => false,
        Behind::Unanswerable => true,
    }
}

/// The program behind `pid`, for a claim to record beside the number it
/// is claiming from. None when nothing could be read, which leaves the
/// claim naming a bare number — the shape claims had before this, and
/// the one [`process_exists`] is the whole answer for.
///
/// The recording and the reading go through the same probe on purpose:
/// what a claim keeps is not the writer's idea of its own name but the
/// string this machine will hand back at that number, so the two are
/// comparable however the program was installed or spelled.
pub(crate) fn image_of(pid: u32) -> Option<String> {
    match behind(pid) {
        Behind::Named(image) => Some(image),
        Behind::Nobody | Behind::Unanswerable => None,
    }
}

/// Whether `pid` still names this task runner — for the claims nothing
/// but the runner ever writes (a verify-ui run's hold on a repository, a
/// shot directory, a config directory). The runner is the one program
/// this may name outright: it is asking after itself.
pub(crate) fn task_runner_exists(pid: u32) -> bool {
    image_matches(pid, is_task_runner)
}

/// Whether the program a claim recorded is still the one at `pid` — for
/// the claims written from somebody else's process, whose name this has
/// no business knowing (`seats::SEAT_CLAIM`, out of a session's
/// `CLAUDE_PID`). The claim carries the answer; this only compares.
pub(crate) fn image_still_at(pid: u32, recorded: &str) -> bool {
    image_matches(pid, |image| same_image(image, recorded))
}

/// Whether two spellings name one program. Both sides come from
/// [`behind`], so this is the tolerance a probe's own drift needs —
/// a path where a bare name was expected, and Windows' indifference to
/// case — and not a guess at what any particular program is called.
fn same_image(image: &str, recorded: &str) -> bool {
    !recorded.is_empty() && basename(image).eq_ignore_ascii_case(basename(recorded))
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

/// A pid no process on this machine can carry — for the tests that need a
/// claim whose process is gone. Spawning a child and reaping it hands its
/// number back to the kernel, which gives it out again; under a suite
/// forking git on every thread the number is somebody else's before the
/// assertion runs, and the test meets a live stranger where it looked for
/// a corpse. Windows hands out multiples of four only, and Linux stops at
/// `pid_max`, which is 2^22 at the most: odd, and above both, so nobody's.
#[cfg(test)]
pub(crate) const NO_SUCH_PID: u32 = 0x7FFF_FFFD;

#[cfg(test)]
mod tests {
    use super::{
        NO_SUCH_PID, image_of, image_still_at, is_task_runner, process_exists, same_image,
        task_runner_exists,
    };

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

    /// Two recordings of one program are one program, and nothing here
    /// knows what any of them is called — which is the point: whatever a
    /// session is installed as, its claim records that and this compares
    /// it. An empty recording is not a match with anything, so a claim
    /// that recorded nothing cannot be read as naming what it met.
    #[test]
    fn one_program_is_known_by_the_name_the_claim_recorded() {
        for (image, recorded) in [
            ("claude.exe", "claude.exe"),
            ("CLAUDE.EXE", "claude.exe"),
            ("node", "node"),
            ("node.exe", "C:\\Program Files\\nodejs\\node.exe"),
            ("/usr/local/bin/claude", "claude"),
        ] {
            assert!(same_image(image, recorded), "{image} vs {recorded}");
        }
        for (image, recorded) in [
            ("node.exe", "claude.exe"),
            ("git.exe", "claude.exe"),
            ("claude.exe", ""),
            ("", ""),
            ("claude.exe", "claude-code.exe"),
        ] {
            assert!(!same_image(image, recorded), "{image} vs {recorded}");
        }
    }

    /// This process is the runner's own test binary, and the pid no
    /// process can have is nobody's — whichever probe is asked. A live
    /// number is not by itself an answer: the runner reads as the runner
    /// and not as whatever else a claim recorded, which is the whole of
    /// what the image name adds.
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
