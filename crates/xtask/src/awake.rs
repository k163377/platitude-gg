//! Keeps this machine from idle-sleeping while any session is at work,
//! and leaves the sleep to the power plan once every one of them has
//! stopped or waits on a person, with nothing left running. Windows only;
//! elsewhere every call is a no-op.
//!
//! A session's hooks keep a claim under `<primary>/.awake/`, and each of
//! its subagents one of its own, so that no event of one overwrites the
//! other's state (`hook::awake` says which event writes what). Compatible
//! builds share the [`CLAIM_FORMAT`] protocol and the same claims: their
//! events update one activity record under the lock. A claim is
//! `working` from a prompt to the turn's stop, `idle` after it, and keeps
//! each tool call from its start to its end — the session's claim keeps
//! the calls of its subagents too — with whether a person is asked about
//! it. A working claim counts, unless it is the session's and its
//! transcript ends on the user interrupting the turn (which runs no Stop):
//!
//! - while one of its own calls runs: open, with no result in its
//!   session's transcripts yet, no person asked about it, and not a
//!   subagent's call (`Agent`, whose subagent's claim shows its work — once
//!   there is one in the shared protocol) — whichever call ended or was
//!   answered before, and however long it runs;
//! - while none of its calls is open — the model is replying, which runs
//!   no hook — for [`WORKING_TTL`] after its last hook.
//!
//! A subagent stopped from outside runs no SubagentStop: its claim goes,
//! and its open calls end, once its session's transcript records its stop
//! after its last hook — a background task's notice naming it, or the
//! result of the call it runs under (the holder says how).
//!
//! A notice written into a session's transcript since its claim's last
//! hook — a background task's end, another session's message — starts a
//! turn that no hook of this repository is sure to hear begin: the holder
//! rewrites the claim `working` as of the notice, and it counts as any
//! working claim does — replies and calls alike — until the turn's Stop.
//!
//! An MCP server's pending request for a person's input names neither a
//! call nor a subagent, only its server: each holds up one of the
//! session's running calls on that server, whichever — none of them shows
//! running — so a server with more calls running than requests pending
//! still works, and one with none left over waits on the person. A
//! request's answer takes off that request alone.
//!
//! A turn whose open calls all wait on a person — a permission prompt, a
//! question of its own, a server's request for input — counts for
//! nothing. No hook says when a person lets a call through, nor when a
//! tool the desktop app runs asks a person itself (its browser's leave to
//! open a file), so the holder reads what those leave unsaid, best-effort,
//! off the desktop app's own log (`desktop`): a permission request logged
//! for a call it ties to one open call alone holds that call up until its
//! answer, and a grant sets the call running again. Without such a line —
//! no log, a request it cannot tie to one call, the supplement switched
//! off — a call that nothing shows running (an MCP tool, a fetch) is not
//! seen from its grant to its end, and one the app asks about itself
//! counts as running while it waits. The log is read after the fact:
//! neither is protected before the reading that sees it.
//!
//! Whatever the state, a claim counts while a process of one of its shell
//! calls runs — a child of the Claude process started between the call's
//! start and its end, the session's or a subagent's, however it went to
//! the background (asked to, timed out, or sent there by hand); a child
//! the process already had, or started during another tool's call, is no
//! work — and until the wake-up it scheduled is due, since that cannot
//! fire on a sleeping machine.
//!
//! One holder per [`REVISION`] (`holder-r<n>`), a PowerShell with no
//! window, reads the shared protocol's claims while any stands, asks
//! `ES_SYSTEM_REQUIRED` — never the display, which the power plan turns off
//! as usual — while any counts, lets go while none does, and quits once no
//! compatible claim is left. Every change to the directory is made
//! under its `lock`, by the hooks and the holders of every revision alike
//! — a byte-range lock both sides speak (`File::lock` over the whole file,
//! `FileStream.Lock` over its first byte) — so a holder quitting and a
//! session claiming cannot pass each other: whichever comes second sees
//! what the first wrote. The lock is held for the directory's files alone:
//! a hook starts a holder, and a holder asks after processes and reads
//! transcripts, with it let go, so no seat's slow step holds up another's.
//! The holder validates the snapshot and applies its OS request under the
//! lock; changed activity discards the old verdict and keeps protection
//! until the next reading. An incompatible claim is never overwritten.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::command::{Command, Permission, Where};
use crate::locks::Locked;

mod claim;
mod desktop;
mod holder;

pub(crate) use claim::digest;
use claim::{Claim, Writer};

pub(crate) static STATUS: Command = Command {
    id: "awake.status",
    call: "awake",
    purpose: "which sessions keep this machine from idle-sleeping, and whether the holder holds it",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static LOG_ON: Command = Command {
    id: "awake.log.on",
    call: "awake log on",
    purpose: "enable the best-effort Desktop permission log supplement",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};
pub(crate) static LOG_OFF: Command = Command {
    id: "awake.log.off",
    call: "awake log off",
    purpose: "disable the Desktop log supplement and retain hook-only sleep control",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};
pub(crate) static COMMANDS: &[&Command] = &[&STATUS, &LOG_ON, &LOG_OFF];

/// In the primary checkout, so every seat's sessions share one holder per
/// [`REVISION`] (the `.chips` / `.permits` layout).
const DIR: &str = ".awake";
/// The directory's lock. Shared by name, so it is never removed.
const LOCK: &str = "lock";
/// What a holder's name is called ahead of its revision (`holder-r<n>`):
/// its pid, whether it holds the machine, and when it read the claims that
/// verdict comes from, rewritten at every reading — its age is the
/// holder's heartbeat. A hook starting a holder first names
/// it `<the hook's pid> starting` — a fresh name, which keeps every other
/// hook from starting one — and starts it outside the lock.
const HOLDER: &str = "holder";
const CLAIM: &str = "claim";
/// What a hook reserving the holder's name writes after its own pid.
const STARTING: &str = "starting";
/// While this file stands, holders leave the desktop app's log unread and
/// go by the hooks alone. The holder's script names it too.
const DESKTOP_OFF: &str = "desktop-log.off";
/// What a holder's last reading of the desktop app's log is called ahead
/// of its revision (`desktop-r<n>`). The holder's script names it too.
const DESKTOP_SEEN: &str = "desktop-r";
/// Between a session's id and a subagent's in the name of the subagent's
/// claim.
const AGENT: char = '~';
/// The holder implementation, independent of the activity protocol.
/// A script change raises this; compatible builds still update the same
/// claims, so a Stop or a question reaches every holder reading them.
const REVISION: u32 = 3;
/// The shared activity protocol, including claim names and bodies. Do not
/// raise this for a holder change: a protocol change needs a migration,
/// not a second independent copy of a session's current state.
const CLAIM_FORMAT: u32 = 1;

/// How long a claim counts with nothing but its last word to go on: a
/// `working` one with no call open — a reply that calls no tool runs no
/// hook — from its last hook, or from the notice that started its turn.
/// Set far past the length of a reply. A turn the user interrupts is read
/// off the transcript instead; this bounds one whose transcript cannot be
/// read. A call still open is never timed out: it counts until its end,
/// its result, or its subagent's end is written.
#[cfg(windows)]
const WORKING_TTL: Duration = Duration::from_secs(60 * 60);
/// A heartbeat older than this is a holder that died without clearing
/// its name; the next claim written starts another.
const HOLDER_STALE: Duration = Duration::from_secs(3 * 15);
/// How long past its scheduled time a wake-up still holds the machine:
/// time for the wake-up's turn to reach a hook that writes `working`.
const WAKE_GRACE: u64 = 5 * 60;

/// What a hook says of a session, or of one of its subagents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mark<'a> {
    Working,
    /// A prompt: working, and the calls of the turn before end.
    Prompt,
    /// The turn stopped, and its calls end.
    Idle,
    /// Working, and a wake-up is scheduled this many seconds from now.
    WakeIn(u64),
    /// Working, and the scheduled wake-up is called off.
    NoWake,
    /// A tool call starts: its id, its tool, and a digest of its input.
    ToolStart {
        id: &'a str,
        name: &'a str,
        input: &'a str,
    },
    /// The tool call ends — its processes may run on in the background.
    ToolEnd(&'a str),
    /// A person is asked about a call: by its id, or by its tool and input
    /// when the hook names no id.
    Asked {
        id: Option<&'a str>,
        name: &'a str,
        input: &'a str,
    },
    /// The MCP server so named in tool names asks a person for input
    /// (`pending`) — by the request's id when it has one — or has the
    /// answer. The request names no call: it holds up one of the server's
    /// running calls until its answer, and changes none.
    Elicited {
        server: &'a str,
        id: Option<&'a str>,
        pending: bool,
    },
    /// The writer's calls that never ended — turned down, interrupted —
    /// end: its turn is over, and they started all they will.
    CallsEnd,
    /// The claim goes: the subagent stopped, or the session ended — which
    /// takes its subagents' claims with it.
    Gone,
}

/// Whose event a hook reports: its session's, or one of its subagents'.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Whose<'a> {
    Session,
    Agent(&'a str),
}

/// Writes what a hook says, and starts the holder if none beats.
/// Advisory: a hook has nobody to tell, and a claim that cannot be written
/// only leaves the sleep to the power plan.
pub(crate) fn mark(
    session: &str,
    whose: Whose<'_>,
    cwd: &str,
    transcript: Option<&str>,
    mark: Mark<'_>,
) {
    if !cfg!(windows) {
        return;
    }
    let (Some(dir), Some(session)) = (dir_of(cwd), file_word(session)) else {
        return;
    };
    // Without the Claude process there is nothing the holder can outlive
    // the claim by.
    let Some(claude) = claude_pid() else {
        return;
    };
    // A subagent's hooks name its session's transcript (measured).
    let transcript =
        transcript
            .filter(|path| !path.trim().is_empty())
            .and_then(|path| match whose {
                Whose::Session => Some(path.to_string()),
                Whose::Agent(agent) => agent_transcript(path, agent),
            });
    let writer = Writer {
        claude,
        now_ms: now_ms(),
        seat: crate::seats::roster_letter(cwd).unwrap_or("-").to_string(),
        who: match whose {
            Whose::Session => "-".to_string(),
            Whose::Agent(agent) => file_word(agent).unwrap_or_else(|| "-".to_string()),
        },
        transcript,
    };
    let result = match target(&session, whose, mark) {
        Some(Target::Session) => end_session(&dir, &session),
        Some(Target::Claim(name)) => apply(&dir, &name, &writer, mark).map(drop),
        None => Ok(()),
    };
    if let Err(_unheard) = result {
        // Nobody to tell from inside a hook.
    }
}

/// A subagent's own transcript, beside its session's: `<session>.jsonl`
/// keeps its subagents' as `<session>/subagents/agent-<id>.jsonl`. None
/// for an id with no characters a file name takes.
fn agent_transcript(session_transcript: &str, agent: &str) -> Option<String> {
    let agent = file_word(agent)?;
    let path = Path::new(session_transcript)
        .with_extension("")
        .join("subagents")
        .join(format!("agent-{agent}.jsonl"));
    Some(path.display().to_string())
}

/// Where a mark lands.
#[derive(Debug, PartialEq, Eq)]
enum Target {
    /// This claim file.
    Claim(String),
    /// Every claim of the session: its own and its subagents'.
    Session,
}

/// The session's own claim for its own events and for every tool call — a
/// subagent's too, since a process it started outlives it; a subagent's
/// claim for the subagent's other events. None for a subagent with no
/// usable id.
fn target(session: &str, whose: Whose<'_>, mark: Mark<'_>) -> Option<Target> {
    let on_calls = matches!(
        mark,
        Mark::ToolStart { .. }
            | Mark::ToolEnd(_)
            | Mark::Asked { .. }
            | Mark::Elicited { .. }
            | Mark::CallsEnd
    );
    match (whose, mark) {
        (Whose::Session, Mark::Gone) => Some(Target::Session),
        (Whose::Session, _) => Some(Target::Claim(claim_name(session, None))),
        (Whose::Agent(_), _) if on_calls => Some(Target::Claim(claim_name(session, None))),
        (Whose::Agent(agent), _) => {
            file_word(agent).map(|agent| Target::Claim(claim_name(session, Some(&agent))))
        }
    }
}

/// The claim's change under the lock — a holder quitting sees the claim or
/// has already cleared its name, a claim read to be changed is the one
/// standing, and two sessions claiming at once reserve one holder's name —
/// then, when no holder of this [`REVISION`] beat, the holder's start
/// outside it: WMI takes a while to start one, and every seat's hooks and
/// holders wait on the lock. A start that fails leaves the reservation to
/// go stale, so the hooks after it try again at most that often. Answers
/// the pid of the holder this call started, if it started one.
fn apply(dir: &Path, name: &str, writer: &Writer, mark: Mark<'_>) -> Result<Option<u32>, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("could not make {}: {e}", dir.display()))?;
    let hook = std::process::id();
    let reservation = format!("{hook} {STARTING}");
    {
        let _held = lock(dir)?;
        let path = dir.join(name);
        let before = match std::fs::read_to_string(&path) {
            Ok(text) => Some(
                Claim::parse(&text)
                    .ok_or("the existing activity does not read; it was not overwritten")?,
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(format!("could not read {}: {error}", path.display())),
        };
        let Some(claim) = writer.claim(mark, before.as_ref()) else {
            remove(&path)?;
            return Ok(None);
        };
        std::fs::write(&path, claim.line())
            .map_err(|e| format!("could not write {}: {e}", path.display()))?;
        if holder_fresh(dir) {
            return Ok(None);
        }
        std::fs::write(holder_path(dir), &reservation)
            .map_err(|e| format!("could not reserve the holder's name: {e}"))?;
    }
    let pid = holder::spawn(dir, hook).ok_or("could not start the holder")?;
    // A holder that has read already named itself; one that has not reads
    // its pid here or takes the reservation it was started under.
    let _held = lock(dir)?;
    if std::fs::read_to_string(holder_path(dir)).is_ok_and(|said| said.trim() == reservation) {
        std::fs::write(holder_path(dir), pid.to_string())
            .map_err(|e| format!("could not name the holder: {e}"))?;
    }
    Ok(Some(pid))
}

/// This [`REVISION`]'s holder's name.
fn holder_path(dir: &Path) -> PathBuf {
    dir.join(format!("{HOLDER}-r{REVISION}"))
}

/// SessionEnd: the session's claims go together — its subagents', and
/// every revision's, since the session's end is theirs too.
fn end_session(dir: &Path, session: &str) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    let _held = lock(dir)?;
    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("could not read {}: {e}", dir.display()))?;
    for entry in entries.flatten() {
        if claim_of(&entry.file_name().to_string_lossy()) == Some(session) {
            remove(&entry.path())?;
        }
    }
    Ok(())
}

/// The session a claim's name is of — its own or a subagent's, of any
/// revision; None for a name that is no claim's.
fn claim_of(name: &str) -> Option<&str> {
    let (revision, whose) = name.strip_prefix('r')?.split_once('.')?;
    if revision.is_empty() || !revision.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let whose = whose.strip_suffix(&format!(".{CLAIM}"))?;
    Some(
        whose
            .split_once(AGENT)
            .map_or(whose, |(session, _)| session),
    )
}

fn remove(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            Err(format!("could not remove {}: {e}", path.display()))
        }
        _ => Ok(()),
    }
}

fn lock(dir: &Path) -> Result<Locked, String> {
    let path = dir.join(LOCK);
    let file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|e| format!("could not open {}: {e}", path.display()))?;
    file.lock()
        .map_err(|e| format!("could not lock {}: {e}", path.display()))?;
    Ok(Locked::new(file))
}

/// Whether this [`REVISION`]'s holder has beaten within [`HOLDER_STALE`].
fn holder_fresh(dir: &Path) -> bool {
    std::fs::metadata(holder_path(dir))
        .and_then(|meta| meta.modified())
        .is_ok_and(|at| {
            // A beat stamped ahead of this clock is a fresh one.
            SystemTime::now()
                .duration_since(at)
                .map_or(true, |age| age < HOLDER_STALE)
        })
}

fn dir_of(cwd: &str) -> Option<PathBuf> {
    Some(crate::tree::primary_root(cwd)?.join(DIR))
}

/// An id kept to the characters a file name takes on every OS; None for
/// an id with none of them.
fn file_word(id: &str) -> Option<String> {
    let word: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    (!word.is_empty()).then_some(word)
}

/// Names belong to the activity protocol, not the holder implementation.
fn claim_name(session: &str, agent: Option<&str>) -> String {
    match agent {
        None => format!("r{CLAIM_FORMAT}.{session}.{CLAIM}"),
        Some(agent) => format!("r{CLAIM_FORMAT}.{session}{AGENT}{agent}.{CLAIM}"),
    }
}

/// The Claude process the hook runs under, which Claude Code names in
/// the environment of everything it starts.
fn claude_pid() -> Option<u32> {
    std::env::var("CLAUDE_PID").ok()?.trim().parse().ok()
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

/// `cargo xtask awake`: the claims as written, and whether the holder
/// holds the machine. Whether each claim counts is the holder's to decide.
pub fn run(args: &[String]) -> Result<(), String> {
    if !args.is_empty() && args != ["log", "on"] && args != ["log", "off"] {
        return Err(format!("awake accepts log on or log off (got {args:?})"));
    }
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let dir = dir_of(&cwd.to_string_lossy()).ok_or("not inside a checkout of this repository")?;
    if !args.is_empty() {
        return configure_log(&dir, &args[1]);
    }
    // Read under the lock every rewrite happens under: a file caught
    // between a rewrite's truncation and its write reads empty.
    let _held = if dir.exists() {
        Some(lock(&dir)?)
    } else {
        None
    };
    let mut holders: Vec<(String, String)> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let revision = name.strip_prefix(&format!("{HOLDER}-r"))?.to_string();
            Some((revision, std::fs::read_to_string(entry.path()).ok()?))
        })
        .collect();
    holders.sort();
    if holders.is_empty() {
        println!("holder: none — nothing keeps this machine awake");
    }
    for (revision, said) in &holders {
        let mut words = said.split_whitespace();
        let (pid, verdict) = (words.next().unwrap_or("?"), words.next().unwrap_or("-"));
        let note = if *revision != REVISION.to_string() {
            " (another holder build: it reads the shared activity with a script of its own)"
        } else if !holder_fresh(&dir) {
            " (stale — the next claim written starts another)"
        } else {
            ""
        };
        println!("holder r{revision}: pid {pid}, {verdict}{note}");
        if let Ok(seen) = std::fs::read_to_string(dir.join(format!("{DESKTOP_SEEN}{revision}"))) {
            println!("  desktop app's log, last read: {}", seen.trim());
        }
    }
    let now = now_secs();
    let (mut others, mut torn) = (0, 0);
    let own = format!("r{CLAIM_FORMAT}.");
    let mut claims: Vec<(String, Claim)> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            claim_of(&name)?;
            let Some(whose) = name.strip_prefix(&own) else {
                others += 1;
                return None;
            };
            let whose = whose.strip_suffix(&format!(".{CLAIM}"))?.to_string();
            let claim = std::fs::read_to_string(entry.path())
                .ok()
                .and_then(|text| Claim::parse(&text));
            if claim.is_none() {
                torn += 1;
            }
            Some((whose, claim?))
        })
        .collect();
    claims.sort_by(|left, right| (&left.1.seat, &left.0).cmp(&(&right.1.seat, &right.0)));
    for (whose, claim) in &claims {
        let wake = if claim.wake > now {
            format!("  wake-up due in {}s", claim.wake - now)
        } else {
            String::new()
        };
        let open: Vec<_> = claim.calls.iter().filter(|call| call.end == 0).collect();
        let asked = open.iter().filter(|call| call.asked).count();
        println!(
            "  seat {} {:<7} {}s ago  claude pid {}  {} call(s) open, {asked} asked, \
             {} request(s) for input  {whose}{wake}",
            claim.seat,
            claim.state.word(),
            now.saturating_sub(claim.at / 1000),
            claim.claude,
            open.len(),
            claim.requests.len()
        );
    }
    if others > 0 {
        println!("  {others} claim(s) of another activity protocol — compatibility is not assumed");
    }
    if torn > 0 {
        println!("  {torn} claim(s) that do not read");
    }
    if claims.is_empty() && others == 0 && torn == 0 {
        println!("  no claims");
    }
    Ok(())
}

/// `cargo xtask awake log on|off`: whether holders read the desktop app's
/// log. Each holder takes it up at its next reading.
fn configure_log(dir: &Path, mode: &str) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("could not make {}: {e}", dir.display()))?;
    let _held = lock(dir)?;
    let path = dir.join(DESKTOP_OFF);
    if mode == "off" {
        std::fs::write(&path, "disabled\n")
            .map_err(|e| format!("could not write {}: {e}", path.display()))?;
    } else {
        remove(&path)?;
    }
    println!(
        "the desktop app's log supplement is {mode}; each holder takes it up at its next reading"
    );
    Ok(())
}

#[cfg(test)]
mod tests;
