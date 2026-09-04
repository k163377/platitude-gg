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
