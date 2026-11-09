//! Codex's native prompt and its recorded user message share one receipt.
//! Reading the same turn never reopens a spent permit or rejoins the line.

use super::payload::own_string_field as string;
use super::permit;
use crate::seats::{Held, Identity, Standing, primary_checkout, roster_letter, standing};

use super::codex_transcript as transcript;

pub(super) fn prompt(input: &str) -> Result<(), String> {
    if string(input, "hook_event_name").as_deref() != Some("UserPromptSubmit")
        || string(input, "agent_id").is_some()
    {
        return Ok(());
    }
    let field = |key| string(input, key).ok_or_else(|| format!("missing Codex {key}"));
    observe(
        &field("cwd")?,
        &field("session_id")?,
        &field("turn_id")?,
        &field("prompt")?,
        crate::land::line::now(),
    )
}

/// A fallback for hooks still built from main or not trusted yet. The
/// caller supplies no prompt: only this thread's on-disk user record can
/// authorize a landing. Reading again before the fast-forward also sees a
/// newer message when the native hook is unavailable.
pub(crate) fn sync(cwd: &str) -> Result<(), String> {
    let Ok(session) = std::env::var("CODEX_THREAD_ID") else {
        return Ok(());
    };
    if Identity::current(None).session != session {
        return Ok(());
    }
    let home = std::env::var_os("CODEX_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE")
                .or_else(|| std::env::var_os("HOME"))
                .map(|home| std::path::PathBuf::from(home).join(".codex"))
        })
        .ok_or("cannot locate Codex home")?;
    sync_from(cwd, &session, &home.join("sessions"))
}

pub(super) fn sync_from(
    cwd: &str,
    session: &str,
    sessions: &std::path::Path,
) -> Result<(), String> {
    let (turn, prompt, at) = transcript::latest(sessions, session)?;
    observe(cwd, session, &turn, &prompt, at)
}

fn observe(cwd: &str, session: &str, turn: &str, prompt: &str, at: u64) -> Result<(), String> {
    if session.is_empty() || turn.is_empty() {
        return Err("Codex message has no session or turn identity".into());
    }
    let Some(words) = permit::the_users_words(prompt) else {
        return Ok(());
    };
    let receipt = permit::kept_file(cwd, session, "codex-prompt").ok_or("no permit directory")?;
    let parent = receipt.parent().ok_or("no permit directory")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let _held = crate::locks::hold(&receipt.with_extension("codex-prompt-lock"))?;
    let key = format!("{turn}\t{}", crate::awake::digest(words.trim()));
    let mut seen = String::new();
    if let Ok(previous) = std::fs::read_to_string(&receipt) {
        let (time, prior) = previous
            .split_once('\n')
            .ok_or("invalid Codex prompt receipt")?;
        if prior.lines().next() == Some(&key) {
            return Ok(());
        }
        if prior.lines().any(|old| old == key) {
            return Err("this Codex message was superseded by another user message".into());
        }
        let time: u64 = time.parse().map_err(|_| "invalid Codex prompt time")?;
        if at < time {
            return Err(
                "Codex transcript precedes the latest native prompt; wait for its record".into(),
            );
        }
        seen.push('\n');
        seen.push_str(prior);
    }
    let seat = owned_seat(cwd, session)?;
    // Reserve the receipt before publishing authority: a failed write
    // cannot allow the same message to restore a consumed permit.
    permit::record_prompt_at(&seat, session, "", at)?;
    permit::write_whole(&receipt, &format!("{at}\n{key}{seen}")).map_err(|e| e.to_string())?;
    permit::record_prompt_at(&seat, session, words, at)?;
    Ok(())
}

fn owned_seat(cwd: &str, session: &str) -> Result<String, String> {
    let (_, trees) = primary_checkout(cwd)?;
    let me = Identity {
        session: session.into(),
        pid: None,
        host: None,
    };
    let seats: Vec<_> = trees
        .into_iter()
        .filter(|tree| {
            roster_letter(&tree.path).is_some()
                && matches!(
                    standing(
                        tree.locked.then(|| tree.reason.clone()),
                        &me,
                        Held::BySession
                    ),
                    Standing::Ours
                )
        })
        .collect();
    match seats.as_slice() {
        [seat] => Ok(seat.path.clone()),
        _ => Err("Codex landing needs exactly one seat claimed by this thread".into()),
    }
}
