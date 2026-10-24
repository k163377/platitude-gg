//! Claude Code hook handlers (`cargo xtask hook <event>`), wired from
//! .claude/settings.json: the git, launches, kills, seats, chips, board
//! window and other sessions a session touches only when the user asks,
//! and the language of what the user reads (`language`). Main moves on the
//! permit the user's own `/land` opens (`permit`), and on that alone.
//!
//! Each handler reads the hook's JSON payload from stdin and answers on
//! stdout; printing nothing means "no objection".

use std::io::Read;

pub(crate) mod approval;
mod attribution;
mod awake;
pub(crate) mod board;
mod chips;
mod codex;
mod commit;
mod contact;
mod dump;
mod git;
mod greeting;
mod kill;
mod language;
mod launch;
mod payload;
pub(crate) mod permit;
mod python;
mod repeat;
mod review;
#[cfg(test)]
mod runner;
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
    if event == "codex-awake" {
        codex::awake(&input);
        return Ok(());
    }
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
        "pre-tool" => pre_tool(&input),
        "post-tool" => post_tool(&input),
        "stop" => stop(&input),
        "pre-worktree" => seat::pre_worktree(&input),
        "post-worktree" => seat::post_worktree(&input),
        "prompt-submit" => prompt_submit(&input),
        "session-start" => greeting::session_start(&input),
        "session-end" => session_end(&input),
        // A session that loaded an older settings.json still names these,
        // for a tool that no longer exists; `pre-tool` / `post-tool` carry
        // the chip guard.
        "pre-chip" | "post-chip" => Ok(()),
        "stop-failure" | "permission-request" | "permission-denied" | "elicitation"
        | "elicitation-result" | "subagent-start" | "subagent-stop" => Ok(()),
        other => Err(format!("unknown hook event: {other:?}")),
    }
}

/// PreToolUse, of any tool: the guards on tools no matcher names — the
/// other sessions (`contact`), the board (`board`), the wrap-up box's
/// language (`language`) and the chips. One decision per call.
fn pre_tool(input: &str) -> Result<(), String> {
    if contact::pre_tool(input) || board::pre_tool(input) || language::pre_tool(input) {
        return Ok(());
    }
    chips::pre_spawn(input)
}

/// PostToolUse and PostToolUseFailure, of any tool: the chips' notes and
/// the language's. Two JSON objects on stdout are read as plain text, so
/// every note rides in one, named for the event that asked.
fn post_tool(input: &str) -> Result<(), String> {
    let mut notes = chips::post_chip(input)?;
    notes.extend(language::post_tool(input).map(str::to_string));
    if !notes.is_empty() {
        let event = payload::own_string_field(input, "hook_event_name")
            .unwrap_or_else(|| "PostToolUse".to_string());
        println!(
            "{{\"hookSpecificOutput\":{{\"hookEventName\":\"{}\",\"additionalContext\":\"{}\"}}}}",
            payload::printable(&event),
            payload::printable(&notes.join(" "))
        );
    }
    Ok(())
}

/// UserPromptSubmit. A side session's report that is refused is answered
/// by the refusal alone — one JSON object on stdout.
fn prompt_submit(input: &str) -> Result<(), String> {
    let prompt = payload::string_field(input, "prompt").unwrap_or_default();
    if contact::prompt_submit(input, &prompt) {
        // A refused prompt starts no turn, and no Stop comes to end the one
        // `awake::prompt` marked for it.
        awake::stopped(input);
        return Ok(());
    }
    board::prompt_submit(input, &prompt);
    review::prompt_submit(input)
}

/// Stop. Repository state cannot tell whether the request is complete;
/// that is the session's to check against the user's instructions. The
/// reasons to go on — the chips' order, the closing's language — are
/// judged together on the turn's first Stop: the next one comes with
/// `stop_hook_active` and judges neither. The user's notice waits for the
/// Stop that ends the turn, so a turn that goes on does not show it twice.
fn stop(input: &str) -> Result<(), String> {
    let first = payload::bool_field(input, "stop_hook_active") != Some(true);
    let chips = if first {
        chips::numbering(input)?
    } else {
        None
    };
    let reasons = chips
        .into_iter()
        .chain(language::stop(input).map(str::to_string))
        .collect::<Vec<_>>();
    if !reasons.is_empty() {
        println!(
            "{{\"decision\":\"block\",\"reason\":\"{}\"}}",
            payload::printable(&reasons.join(" "))
        );
        return Ok(());
    }
    let cwd = payload::string_field(input, "cwd").unwrap_or_default();
    let message = [permit::unmet(input), crate::gate::standing(&cwd)]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    if !message.is_empty() {
        println!("{{\"systemMessage\":\"{}\"}}", payload::printable(&message));
    }
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
