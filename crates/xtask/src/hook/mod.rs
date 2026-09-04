//! Claude Code hook handlers (`cargo xtask hook <event>`).
//!
//! Wired from .claude/settings.json. Each handler reads the hook's JSON
//! payload from stdin and answers on stdout; printing nothing means "no
//! objection".

use std::io::Read;

mod attribution;
mod chips;
mod commit;
mod git;
mod greeting;
mod kill;
mod launch;
mod payload;
mod review;
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
        "pre-chip" => chips::pre_spawn(&input),
        "post-chip" => chips::post_chip(&input),
        "stop" => stop(&input),
        "pre-worktree" => seat::pre_worktree(&input),
        "post-worktree" => seat::post_worktree(&input),
        "prompt-submit" => prompt_submit(&input),
        "session-start" => greeting::session_start(&input),
        "session-end" => session_end(&input),
        other => Err(format!("unknown hook event: {other:?}")),
    }
}

/// Stop: one JSON object at most. The chip guard's block comes first;
/// when it has nothing to say, the seat's gate standing goes to the user
/// as a system message — a seat reported before its tip was gated is
/// what the user reads there.
fn stop(input: &str) -> Result<(), String> {
    if chips::stop(input)? {
        return Ok(());
    }
    let cwd = payload::string_field(input, "cwd").unwrap_or_default();
    if let Some(standing) = crate::gate::standing(&cwd) {
        println!(
            "{{\"systemMessage\":\"{}\"}}",
            standing.replace('"', "'").replace('\n', " ")
        );
    }
    Ok(())
}

/// UserPromptSubmit: a prompt is the plainest sign a session is alive,
/// and it is the answer to a SessionEnd that fired over a sleep — the
/// runs that end marked are somebody's again (shots/sweep.rs). Silent
/// either way: what this handler prints becomes part of the prompt, and
/// the review guard is what has something to say there.
fn prompt_submit(input: &str) -> Result<(), String> {
    if let Err(_unheard) =
        crate::shots::session_seen(&payload::string_field(input, "session_id").unwrap_or_default())
    {
        // A board that could not be written is not a reason to hold up
        // the prompt: the runs stand until their mark runs out anyway.
    }
    review::prompt_submit(input)
}

/// SessionEnd: everything a session leaves behind for itself alone —
/// the seat it claimed, and the chip ledger its numbering was judged
/// against.
fn session_end(input: &str) -> Result<(), String> {
    chips::session_end(input);
    seat::session_end(input)
}

/// PreToolUse(Bash|PowerShell): every shell line passes through here. One
/// decision per call — two JSON objects on stdout is not a payload — so the
/// guards run in order and the first refusal is the answer.
fn pre_shell(input: &str) -> Result<(), String> {
    if git::pre_git(input)? {
        return Ok(());
    }
    if attribution::pre_comment(input)? {
        return Ok(());
    }
    if kill::pre_kill(input)? {
        return Ok(());
    }
    launch::pre_launch(input)
}
