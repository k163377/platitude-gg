//! Codex activity in the shared holder. Its namespace is separate because
//! older holders recognize only Claude owners and would remove Codex claims.
//! The body uses the shared activity protocol; no Claude state is copied.

use super::{
    Mark, Target, Whose, apply, claim::Writer, dir_of, file_word, now_ms, owner, remove, target,
};
use crate::command::{Command, Permission, Where};
use crate::seats::{Held, Identity, Standing, primary_checkout, roster_letter, standing};

pub(super) mod log;
pub(super) mod setup;
pub(super) use setup::{SETUP, UNDO};

pub(crate) static WORK: Command = Command {
    id: "awake.codex.work",
    call: "awake codex work",
    purpose: "refresh this Codex chat's working activity in the shared Windows sleep holder",
    run_in: Where::Seat,
    needs: &["CODEX_THREAD_ID and this chat's claimed seat"],
    permission: Permission::Plain,
};
pub(crate) static IDLE: Command = Command {
    id: "awake.codex.idle",
    call: "awake codex idle",
    purpose: "idle this Codex chat's sleep activity without releasing its seat",
    run_in: Where::Seat,
    needs: &["CODEX_THREAD_ID"],
    permission: Permission::Plain,
};
pub(super) fn run(args: &[String]) -> Result<(), String> {
    if matches!(args.first().map(String::as_str), Some("setup")) {
        return setup::run(&args[1..]);
    }
    let mark = match args {
        [word] if word == "work" => Mark::Working,
        [word] if word == "idle" => Mark::Idle,
        _ => return Err("awake codex accepts work, idle, or setup [--undo]".into()),
    };
    let session = std::env::var("CODEX_THREAD_ID").map_err(|_| "missing CODEX_THREAD_ID")?;
    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    record(&session, &cwd.to_string_lossy(), None, &[mark])
}

/// Seat ownership is the conversation's; the process only bounds sleep
/// activity after Codex exits. The session cwd can still be the primary
/// checkout, so locate the claimed seat from Git's listing.
pub(crate) fn record(
    session: &str,
    cwd: &str,
    agent: Option<&str>,
    marks: &[Mark<'_>],
) -> Result<(), String> {
    if !cfg!(windows) {
        return Ok(());
    }
    record_owned(
        session,
        cwd,
        agent,
        owner::codex_pid().ok_or("no Codex process in this command's ancestry")?,
        marks,
    )
}

fn record_owned(
    session: &str,
    cwd: &str,
    agent: Option<&str>,
    owner: u32,
    marks: &[Mark<'_>],
) -> Result<(), String> {
    if !cfg!(windows) {
        return Ok(());
    }
    let session = file_word(session).ok_or("missing Codex session identity")?;
    let dir = dir_of(cwd).ok_or("not inside a checkout of this repository")?;
    let agent = agent
        .map(|id| file_word(id).ok_or("missing Codex agent identity"))
        .transpose()?;
    let whose = agent.as_deref().map_or(Whose::Session, Whose::Agent);
    let (_, trees) = primary_checkout(cwd)?;
    let me = Identity {
        session: session.clone(),
        pid: None,
    };
    let seat = trees.iter().find(|tree| {
        roster_letter(&tree.path).is_some()
            && matches!(
                standing(
                    tree.locked.then(|| tree.reason.clone()),
                    &me,
                    Held::BySession
                ),
                Standing::Ours
            )
    });
    if seat.is_none() {
        return end_session(&dir, &session);
    }
    let writer = Writer {
        claude: owner,
        now_ms: now_ms(),
        seat: seat
            .and_then(|tree| roster_letter(&tree.path))
            .unwrap_or("-")
            .into(),
        who: agent.as_deref().unwrap_or("-").into(),
        // Claude's transcript reader cannot interpret Codex's transcripts.
        transcript: None,
    };
    apply_marks(&dir, &session, whose, &writer, marks)
}

pub(super) fn apply_marks(
    dir: &std::path::Path,
    session: &str,
    whose: Whose<'_>,
    writer: &Writer,
    marks: &[Mark<'_>],
) -> Result<(), String> {
    for mark in marks {
        // Item ids belong to a thread, including when child calls share
        // the parent's claim. Preserve that scope in the shared protocol.
        let id = match mark {
            Mark::ToolStart { id, .. } | Mark::ToolEnd(id) | Mark::Asked { id: Some(id), .. } => {
                Some(format!("{}.{}", writer.who, id))
            }
            _ => None,
        };
        let scoped = match (*mark, id.as_deref()) {
            (Mark::ToolStart { name, input, .. }, Some(id)) => Mark::ToolStart { id, name, input },
            (Mark::ToolEnd(_), Some(id)) => Mark::ToolEnd(id),
            (Mark::Asked { name, input, .. }, Some(id)) => Mark::Asked {
                id: Some(id),
                name,
                input,
            },
            _ => *mark,
        };
        match target(session, whose, *mark) {
            Some(Target::Session) => end_session(dir, session)?,
            Some(Target::Claim(name)) => {
                apply(dir, &format!("c{}", &name[1..]), writer, scoped)?;
            }
            None => {}
        }
    }
    Ok(())
}

fn end_session(dir: &std::path::Path, session: &str) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    let _held = super::lock(dir)?;
    for entry in std::fs::read_dir(dir)
        .map_err(|error| error.to_string())?
        .flatten()
    {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('c') && super::claim_of(&name) == Some(session) {
            remove(&entry.path())?;
        }
    }
    Ok(())
}
