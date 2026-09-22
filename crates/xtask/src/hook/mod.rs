//! Claude Code hook handlers (`cargo xtask hook <event>`).
//!
//! The git this repository holds until the user asks for it — a landing,
//! a rebase, a commit in the primary checkout — and the launches, kills,
//! seats and chips a session handles only when asked. Main moves on
//! the permit the user's own message opens (`permit`), and on that
//! alone.
//!
//! Two guards hold a line for what it costs rather than for what it
//! touches: a wait asked again and again (`repeat`) and a file printed
//! whole (`dump`) are both read back by every call that follows them,
//! and the conversation is what every call is charged.
//!
//! Wired from .claude/settings.json. Each handler reads the hook's JSON
//! payload from stdin and answers on stdout; printing nothing means "no
//! objection".

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

/// What a command carries to say the user asked, in so many words, for a
/// commit in the primary checkout — the one exception the commit guard
/// makes (`commit`). A landing carries nothing of the kind: the permit
/// answers for it, read off the user's own message.
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

/// Every escape this hook reads — the five above, which
/// is what a flag spelled elsewhere is held to.
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

/// Stop: check chip numbering once, then report the permit and gate
/// standing. Repository state cannot establish whether the request is
/// complete; the session checks that against the user's instructions.
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

/// SessionEnd: the chip ledger goes with the session that wrote it.
///
/// The seat stays. This event is handed to every open
/// conversation when the machine sleeps, and each one goes on working at
/// the next wake (shots/sweep.rs) — a claim released here comes off a
/// seat its session is still sitting in, and the session meets its own
/// tree as somebody else's on waking. The seat goes back where the work
/// does: landing the branch hands the letter to the roster
/// (`land::release_claim`), the session hands it back itself (`cargo
/// xtask seat release`), or the user has it taken over (`seats::takeover`).
fn session_end(input: &str) -> Result<(), String> {
    chips::session_end(input);
    repeat::session_end(input);
    Ok(())
}

/// PreToolUse(Bash|PowerShell): every shell line passes through here. One
/// decision per call — two JSON objects on stdout is not a payload — so the
/// guards run in order and the first refusal is the answer.
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
