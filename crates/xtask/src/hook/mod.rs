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
mod permit;
mod review;
mod seat;
mod still;
mod write;

/// What a command carries to say an explicit instruction asked for main to
/// move. It rides in the command itself so the transcript records the ask;
/// whether the user made it is the permit's to say (`permit`), read off
/// the user's own message.
const MAIN_ESCAPE: &str = "PGG_ALLOW_MAIN";

/// The same, for an instruction that asked for a rebase.
const REBASE_ESCAPE: &str = "PGG_ALLOW_REBASE";

/// The same, for an instruction that asked for a real window.
const WINDOW_ESCAPE: &str = "PGG_ALLOW_GUI";

/// The same, for an instruction that asked to kill runs beyond this tree.
const KILL_ESCAPE: &str = "PGG_ALLOW_KILL";

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

/// Stop: one JSON object at most. The chip guards ask first — how the
/// list is numbered, then what the reply says it is leaving behind
/// without putting it anywhere — and only while this turn has an ask
/// left: a block is put once, or the answer to it is met by the same
/// block again. When they have nothing to say, the seat's gate standing
/// goes to the user as a system message — a seat reported before its tip
/// was gated is what the user reads there.
fn stop(input: &str) -> Result<(), String> {
    if payload::bool_field(input, "stop_hook_active") != Some(true) {
        if chips::numbering(input)? {
            return Ok(());
        }
        if chips::leftovers(input)? {
            return Ok(());
        }
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

/// SessionEnd: the chip ledger goes with the session that wrote it.
///
/// The seat does not go with it. This event is handed to every open
/// conversation when the machine sleeps, and each one goes on working at
/// the next wake (shots/sweep.rs) — a claim released here comes off a
/// seat its session is still sitting in, and the session meets its own
/// tree as somebody else's on waking. The seat goes back where the work
/// does instead: landing the branch hands the letter to the roster
/// (`land::release_claim`), and a claim whose Claude process is gone is
/// litter the next `cargo xtask seat` lifts (`seats::claim_is_dead`).
fn session_end(input: &str) -> Result<(), String> {
    chips::session_end(input);
    Ok(())
}

/// PreToolUse(Bash|PowerShell): every shell line passes through here. One
/// decision per call — two JSON objects on stdout is not a payload — so the
/// guards run in order and the first refusal is the answer.
fn pre_shell(input: &str) -> Result<(), String> {
    let landing = match git::pre_git(input)? {
        git::Verdict::Refused => return Ok(()),
        git::Verdict::Landing => true,
        git::Verdict::Clear => false,
    };
    if still::pre_shell(input)?
        || attribution::pre_comment(input)?
        || kill::pre_kill(input)?
        || launch::pre_launch(input)?
    {
        return Ok(());
    }
    // Nothing refused: a landing goes through, and the permit it goes
    // through on is spent now — a land that stops still ran on it.
    if landing {
        permit::spend(input);
    }
    Ok(())
}
