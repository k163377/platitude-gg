//! Claude Code hook handlers (`cargo xtask hook <event>`), wired from
//! .claude/settings.json: the git, launches, kills, seats and chips a
//! session touches only when the user asks. Main moves on the permit the
//! user's own message opens (`permit`), and on that alone.
//!
//! Each handler reads the hook's JSON payload from stdin and answers on
//! stdout; printing nothing means "no objection".

use std::io::Read;

mod attribution;
mod chips;
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

/// The escape for a commit in the primary checkout the user asked for in
/// so many words (`commit`). A landing does not read it — the permit does.
const MAIN_APPROVAL_FLAG: &str = "PGG_ALLOW_MAIN";

/// The same, for an instruction that asked for a rebase.
const REBASE_APPROVAL_FLAG: &str = "PGG_ALLOW_REBASE";

/// The same, for an instruction that asked for a real window.
pub(crate) const GUI_APPROVAL_FLAG: &str = "PGG_ALLOW_GUI";

/// The same, for an instruction that approved stopping processes outside this worktree.
const PROCESS_STOP_APPROVAL_FLAG: &str = "PGG_ALLOW_KILL";

/// The same, for an instruction that asked for a seat to be taken over
/// from whoever holds it (`seats::takeover`).
pub(crate) const TAKEOVER_APPROVAL_FLAG: &str = "PGG_ALLOW_TAKEOVER";

/// Every escape this hook reads — what a flag spelled elsewhere is held to.
pub(crate) const APPROVAL_FLAGS: [&str; 5] = [
    MAIN_APPROVAL_FLAG,
    REBASE_APPROVAL_FLAG,
    GUI_APPROVAL_FLAG,
    PROCESS_STOP_APPROVAL_FLAG,
    TAKEOVER_APPROVAL_FLAG,
];

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
