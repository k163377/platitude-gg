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

/// Whether the process `pid` names still exists **and is a program
/// `is_wanted` accepts** — the shape every claim that records its writer
/// is asked with. A pid is a name the machine hands out again the moment
/// its process is gone, so a claim a killed process left behind would
/// otherwise stand for as long as whatever inherited the number — a git,
/// a browser tab — and hold what it claimed until that stranger exits.
/// The image name tells the two apart. Answers "alive" when it could not
/// ask, as [`process_exists`] does.
#[cfg(windows)]
fn image_matches(pid: u32, is_wanted: fn(&str) -> bool) -> bool {
    let Ok(out) = std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
        .output()
    else {
        return true;
    };
    // `"xtask.exe","12345","Console","1","75,836 K"`: the image and the
    // pid are the first two fields, ahead of the one holding a comma.
    String::from_utf8_lossy(&out.stdout).lines().any(|line| {
        let mut fields = line.split(',').map(|field| field.trim().trim_matches('"'));
        let image = fields.next().unwrap_or_default();
        fields.next().unwrap_or_default() == pid.to_string() && is_wanted(image)
    })
}

/// The same, asking `ps` for the command name.
#[cfg(not(windows))]
fn image_matches(pid: u32, is_wanted: fn(&str) -> bool) -> bool {
    let Ok(out) = std::process::Command::new("ps")
        .args(["-o", "comm=", "-p", &pid.to_string()])
        .output()
    else {
        return true;
    };
    // A pid nobody has answers with nothing and a non-zero exit.
    out.status.success() && is_wanted(String::from_utf8_lossy(&out.stdout).trim())
}

/// Whether `pid` still names this task runner — for the claims nothing
/// but the runner ever writes (a verify-ui run's hold on a repository, a
/// shot directory, a config directory).
pub(crate) fn task_runner_exists(pid: u32) -> bool {
    image_matches(pid, is_task_runner)
}

/// Whether `pid` still names a Claude session — for the seat claims a
/// session writes out of its `CLAUDE_PID` (`seats::SEAT_CLAIM`), which
/// no other program ever writes.
pub(crate) fn claude_session_exists(pid: u32) -> bool {
    image_matches(pid, is_claude_session)
}

/// Whether an image name is this runner's, however it is spelled: cargo's
/// `xtask.exe`, a test binary's `xtask-<hash>`, a landing's
/// `xtask-inflight-<pid>` (`land::step_out_of_the_build_slot`).
fn is_task_runner(image: &str) -> bool {
    let name = image.rsplit(['/', '\\']).next().unwrap_or(image);
    name.get(..5)
        .is_some_and(|head| head.eq_ignore_ascii_case("xtask"))
}

/// Whether an image name is a Claude session's. Claude Code runs as
/// `claude.exe` here (measured against every claim on the roster) and as
/// `claude` where `ps` gives the name back; a session hosted under some
/// other image — an npm install's `node` among them — is not one this
/// knows, and its claim would read as litter.
fn is_claude_session(image: &str) -> bool {
    let name = image.rsplit(['/', '\\']).next().unwrap_or(image);
    name.eq_ignore_ascii_case("claude") || name.eq_ignore_ascii_case("claude.exe")
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
        NO_SUCH_PID, claude_session_exists, is_claude_session, is_task_runner, process_exists,
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

    #[test]
    fn a_session_is_known_by_its_image_name_on_either_platform() {
        for image in [
            "claude.exe",
            "CLAUDE.EXE",
            "claude",
            "C:\\Users\\x\\AppData\\Local\\claude\\claude.exe",
            "/usr/local/bin/claude",
        ] {
            assert!(is_claude_session(image), "{image}");
        }
        for image in [
            "node.exe",
            "git.exe",
            "xtask.exe",
            "",
            "claud",
            "claude-code.exe",
        ] {
            assert!(!is_claude_session(image), "{image}");
        }
    }

    /// This process is the runner's own test binary, and the pid no
    /// process can have is nobody's — whichever probe is asked. The
    /// runner is not a session, so a live number is not by itself an
    /// answer either: that is the whole of what the image name adds.
    #[test]
    fn this_process_is_alive_and_the_pid_nobody_can_have_is_not() {
        let me = std::process::id();
        assert!(process_exists(me));
        assert!(task_runner_exists(me));
        assert!(!claude_session_exists(me));
        assert!(!process_exists(NO_SUCH_PID));
        assert!(!task_runner_exists(NO_SUCH_PID));
        assert!(!claude_session_exists(NO_SUCH_PID));
    }
}
