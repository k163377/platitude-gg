//! Claude Code hook handlers (`cargo xtask hook <event>`).
//!
//! Wired from .claude/settings.json. Each handler reads the hook's JSON
//! payload from stdin and answers on stdout; printing nothing means "no
//! objection".

use std::io::Read;

mod commit;
mod git;
mod greeting;
mod kill;
mod launch;
mod payload;
mod seat;
mod write;

/// What a command carries to say an explicit instruction asked for main to
/// move. It rides in the command itself so the transcript records the ask.
const MAIN_ESCAPE: &str = "PG_ALLOW_MAIN";

/// The same, for an instruction that asked for a rebase.
const REBASE_ESCAPE: &str = "PG_ALLOW_REBASE";

/// The same, for an instruction that asked for a real window.
const WINDOW_ESCAPE: &str = "PG_ALLOW_GUI";

/// The same, for an instruction that asked to kill runs beyond this tree.
const KILL_ESCAPE: &str = "PG_ALLOW_KILL";

pub fn run(args: &[String]) -> Result<(), String> {
    let event = args.first().map(String::as_str).unwrap_or("");
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| format!("failed to read hook payload: {e}"))?;
    match event {
        "pre-write" => write::pre_write(&input),
        "post-write" => write::post_write(&input),
        "pre-shell" => pre_shell(&input),
        "pre-worktree" => seat::pre_worktree(&input),
        "post-worktree" => seat::post_worktree(&input),
        "session-start" => greeting::session_start(&input),
        "session-end" => seat::session_end(&input),
        other => Err(format!("unknown hook event: {other:?}")),
    }
}

/// PreToolUse(Bash|PowerShell): every shell line passes through here. One
/// decision per call — two JSON objects on stdout is not a payload — so the
/// guards run in order and the first refusal is the answer.
fn pre_shell(input: &str) -> Result<(), String> {
    if git::pre_git(input)? {
        return Ok(());
    }
    if kill::pre_kill(input)? {
        return Ok(());
    }
    launch::pre_launch(input)
}
