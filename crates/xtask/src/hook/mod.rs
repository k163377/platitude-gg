//! Claude Code hook handlers (`cargo xtask hook <event>`), wired from
//! .claude/settings.json: the git, launches, kills, seats and chips a
//! session touches only when the user asks. Main moves on the permit the
//! user's own message opens (`permit`), and on that alone.
//!
//! Each handler reads the hook's JSON payload from stdin and answers on
//! stdout; printing nothing means "no objection".

use std::io::Read;

pub(crate) mod approval;
mod attribution;
mod awake;
mod chips;
#[cfg(test)]
mod cloud;
mod commit;
mod dump;
mod git;
mod greeting;
mod kill;
mod launch;
mod payload;
pub(crate) mod permit;
mod python;
mod repeat;
mod review;
mod seat;
mod shell;
mod still;
mod write;

pub(crate) static HOOK: crate::command::Command = crate::command::Command {
    id: "hook.event",
    call: "hook <event>",
    purpose: "the Claude Code hook handler, wired from .claude/settings.json",
    run_in: crate::command::Where::Either,
    needs: &["the hook payload on stdin — the harness runs this, not a session"],
    permission: crate::command::Permission::Plain,
};

pub(crate) static COMMANDS: &[&crate::command::Command] = &[&HOOK];

pub fn run(args: &[String]) -> Result<(), String> {
    let event = args.first().map(String::as_str).unwrap_or("");
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| format!("failed to read hook payload: {e}"))?;
    // Whether the session is at work (`awake`) is written before any
    // handler answers, so a refusal or an early return leaves it said.
    match event {
        "prompt-submit" => awake::prompt(&input),
        "pre-tool" => awake::tool_start(&input),
        "post-tool" | "permission-denied" => awake::after_tool(&input),
        "permission-request" => awake::permission_request(&input),
        "elicitation" => awake::elicitation(&input, true),
        "elicitation-result" => awake::elicitation(&input, false),
        "stop" | "stop-failure" => awake::stopped(&input),
        "subagent-start" => awake::subagent_start(&input),
        "subagent-stop" => awake::subagent_stop(&input),
        _ => {}
    }
    match event {
        "pre-write" => write::pre_write(&input),
        "post-write" => write::post_write(&input),
        "pre-shell" => pre_shell(&input),
        "pre-chip" => chips::pre_spawn(&input),
        "post-chip" => chips::post_chip(&input),
        "stop" => stop(&input),
        "pre-worktree" => seat::pre_worktree(&input),
        "post-worktree" => seat::post_worktree(&input),
        "prompt-submit" => review::prompt_submit(&input),
        "session-start" => greeting::session_start(&input),
        "session-end" => session_end(&input),
        "stop-failure" | "pre-tool" | "post-tool" | "permission-request" | "permission-denied"
        | "elicitation" | "elicitation-result" | "subagent-start" | "subagent-stop" => Ok(()),
        other => Err(format!("unknown hook event: {other:?}")),
    }
}

/// Stop. Repository state cannot tell whether the request is complete;
/// that is the session's to check against the user's instructions.
fn stop(input: &str) -> Result<(), String> {
    if payload::bool_field(input, "stop_hook_active") != Some(true) && chips::numbering(input)? {
        return Ok(());
    }
    let cwd = payload::string_field(input, "cwd").unwrap_or_default();
    let message = [permit::unmet(input), crate::gate::standing(&cwd)]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    if message.is_empty() {
        return Ok(());
    }
    println!("{{\"systemMessage\":\"{}\"}}", payload::printable(&message));
    Ok(())
}

/// SessionEnd: the session's ledgers and entry mark go with it; its seat
/// claim stays. The machine's sleep hands this event to every open
/// conversation, and each goes on working at the next wake, so a claim
/// released here would come off a seat its session still sits in
/// (`seats::how_claims_move`).
fn session_end(input: &str) -> Result<(), String> {
    chips::session_end(input);
    repeat::session_end(input);
    seat::session_end(input);
    awake::session_end(input);
    Ok(())
}

/// PreToolUse(Bash|PowerShell). One decision per call — two JSON objects
/// on stdout is not a payload — so the first refusal is the answer.
fn pre_shell(input: &str) -> Result<(), String> {
    let _refused = git::pre_git(input)?
        || seat::pre_takeover(input)?
        || still::pre_shell(input)?
        || attribution::pre_comment(input)?
        || kill::pre_kill(input)?
        || launch::pre_launch(input)?
        || python::pre_shell(input)?
        || dump::pre_shell(input)?
        || repeat::pre_shell(input)?;
    Ok(())
}
