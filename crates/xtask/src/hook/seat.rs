//! The seat roster guard: which worktrees a session may enter, the atomic
//! claim that keeps two sessions out of one seat, and the re-claim that
//! puts a claim back on a seat `land` set free.

use super::payload::string_field;
use crate::git_query;
use crate::seats::{self, SEAT_CLAIM, SEATS, worktree_root};

/// PreToolUse(EnterWorktree): a worktree name outside the seat roster
/// starts a cold target/ nobody will reuse (CLAUDE.md ビルド・テスト).
/// Entering an existing seat by path claims it here, atomically, with
/// `git worktree lock` — the survey a session read is a snapshot, and
/// two sessions told "a is free" would otherwise both settle in
/// (observed). Creating a missing seat needs no claim: the second
/// `git worktree add` of one letter fails by itself.
pub(super) fn pre_worktree(input: &str) -> Result<(), String> {
    let name = string_field(input, "name");
    let path = string_field(input, "path");
    if let Some(objection) = worktree_objection(name.as_deref(), path.as_deref()) {
        println!(
            "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
             \"permissionDecision\":\"ask\",\"permissionDecisionReason\":\
             \"{objection} Worktrees are six reusable seats, a-f: enter a \
             free one with path (the session greeting lists them), or create \
             a missing seat by passing its letter as name (CLAUDE.md \
             ビルド・テスト). A worktree outside the roster needs the user's \
             say-so.\"}}}}"
        );
        return Ok(());
    }
    let Some(cwd) = string_field(input, "cwd") else {
        return Ok(());
    };
    // Entering by name lands in the seat's existing tree just as surely as
    // entering by path — resolve it, or there is a door around the claim.
    let target = match (path, name) {
        (Some(path), _) => roster_seat(&path).is_some().then_some(path),
        (None, Some(name)) => existing_seat_path(&cwd, &name),
        (None, None) => None,
    };
    let Some(path) = target else {
        return Ok(());
    };
    let session = string_field(input, "session_id").unwrap_or_default();
    if let Claim::Held(reason) = lock_seat(&cwd, &path, &session) {
        println!(
            "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
             \"permissionDecision\":\"deny\",\"permissionDecisionReason\":\
             \"This seat is already claimed (locked: {}). Seats are first \
             come, first served and this refusal is the roster answering, \
             not an error to work around: take a different letter and \
             enter it the same way. There is nothing to survey first — \
             the claim is the check (CLAUDE.md ビルド・テスト).\"}}}}",
            printable(&reason)
        );
    }
    Ok(())
}

/// PostToolUse(EnterWorktree): the claim itself took in `pre_worktree`.
/// This is the line that makes the session say so out loud.
pub(super) fn post_worktree(input: &str) -> Result<(), String> {
    // The tool input's `path` is the seat the session just settled in;
    // `cwd` is the fallback for an entry by name, where the roster letter
    // is the only thing the payload carries.
    let where_ = string_field(input, "path").or_else(|| string_field(input, "cwd"));
    let Some(seat) = where_.as_deref().and_then(roster_seat) else {
        return Ok(());
    };
    println!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PostToolUse\",\
         \"additionalContext\":\"{}\"}}}}",
        announce(seat)
    );
    Ok(())
}

/// What a session owes the user the moment a seat becomes its own.
///
/// Sessions settle into a seat and never name it — often enough that the
/// user asked for it to be made certain rather than left to habit. The
/// letter is what everything downstream is read against: which tree a
/// change is in, which tree a build came out of, which tree took a
/// screenshot, which branch there is to land. A session that never says
/// it leaves the user guessing at all four.
fn announce(seat: &str) -> String {
    format!(
        "Seat {seat} is this session's now. Name it to the user in your very next \
         reply, in Japanese and by its letter (「席 {seat} を取った」) — a session \
         that never says which seat it took leaves the user unable to tell which \
         tree a change, a build or a screenshot came from (CLAUDE.md ビルド・テスト)."
    )
}

/// The roster letter `path` points into, if it is a seat's tree at all.
fn roster_seat(path: &str) -> Option<&'static str> {
    let root = worktree_root(path)?;
    let name = root.rsplit('/').next()?;
    SEATS.iter().find(|seat| **seat == name).copied()
}

/// The tree a roster letter already stands on, if it was ever created —
/// a name for a missing seat creates it fresh, and needs no claim here.
fn existing_seat_path(cwd: &str, name: &str) -> Option<String> {
    let listing = git_query(cwd, &["worktree", "list", "--porcelain"])?;
    seats::seat_entries(&listing)
        .into_iter()
        .find(|entry| entry.seat == name)
        .map(|entry| entry.tree.path)
}

/// How an attempt to claim a seat came out.
pub(super) enum Claim {
    /// Locked by us now, or in some state git could not judge — the tool
    /// call itself will surface whatever is actually wrong.
    OursOrMoot,
    /// Somebody holds it: the lock's reason, possibly empty.
    Held(String),
}

/// One atomic claim: `git worktree lock` refuses a second lock, so the
/// loser of a race is told here and not after settling in. What to make
/// of a seat somebody already holds is the caller's — a session entering
/// one has somewhere else to go, a session already sitting in one does
/// not.
pub(super) fn lock_seat(cwd: &str, seat_path: &str, session: &str) -> Claim {
    let reason = format!("{SEAT_CLAIM} {session}");
    let mut command = std::process::Command::new("git");
    command
        .arg("-C")
        .arg(cwd)
        // The "already locked" branch below reads git's message, and a
        // translated one would fall through to OursOrMoot — the allow
        // side. Pin the locale so the deny keeps its teeth.
        .env("LC_ALL", "C")
        .args(["worktree", "lock", "--reason", &reason, seat_path]);
    let Ok(output) = crate::run_captured(&mut command) else {
        return Claim::OursOrMoot;
    };
    if output.status.success() {
        return Claim::OursOrMoot;
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if let Some(rest) = stderr.split("already locked").nth(1) {
        let reason = rest
            .split_once("reason:")
            .map(|(_, reason)| reason.trim().to_string())
            .unwrap_or_default();
        return Claim::Held(reason);
    }
    Claim::OursOrMoot
}

/// A string sanitized for splicing into the hook's hand-built JSON:
/// everything that could end the string or the payload early is dropped.
fn printable(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '"' | '\\' => '\'',
            '\n' | '\r' | '\t' => ' ',
            other => other,
        })
        .collect()
}

/// The reason on this worktree's own lock, if it is locked at all.
pub(super) fn lock_reason(cwd: &str) -> Option<String> {
    let git_dir = git_query(cwd, &["rev-parse", "--path-format=absolute", "--git-dir"])?;
    let reason = std::fs::read_to_string(format!("{git_dir}/locked")).ok()?;
    Some(reason.trim().to_string())
}

/// SessionEnd: a seat claimed by this session is handed back, and the
/// pictures this session put on the shot board go with it. A lock
/// somebody else wrote stays — ending inside a seat that was never ours
/// is the collision case, not a reason to free it.
pub(super) fn session_end(input: &str) -> Result<(), String> {
    let session = string_field(input, "session_id").unwrap_or_default();
    if session.is_empty() {
        return Ok(());
    }
    // The board first, and by session rather than by seat: a session in
    // the primary checkout holds no seat to release and still leaves
    // pictures behind, and a seat outlives whoever sat in it (shots).
    if let Err(_unheard) = crate::shots::session_ended(&session) {
        // Nobody is left to tell — the session is over. What stayed on
        // the board is what `cargo xtask shots prune` is for.
    }
    let cwd = string_field(input, "cwd").unwrap_or_default();
    if roster_seat(&cwd).is_none() {
        return Ok(());
    }
    if lock_reason(&cwd).is_some_and(|reason| reason.contains(&session))
        && git_query(&cwd, &["worktree", "unlock", &cwd]).is_none()
    {
        // Nobody is left to tell; the stale mark in `cargo xtask seats`
        // is the fallback.
    }
    Ok(())
}

/// PostToolUse(Write|Edit): an edit inside a roster seat is work, and work
/// holds a claim. Seats come free mid-session — `land` releases the claim
/// the moment a seat's branch is on main (CLAUDE.md ビルド・テスト) — so
/// the next stretch of work claims the seat back at its first edit. Quiet
/// while the claim is already this session's; a note when the re-claim
/// takes; a warning when the seat belongs to somebody else.
pub(super) fn reclaim(input: &str, path: &str) -> Option<String> {
    let root = worktree_root(path)?;
    let name = root.rsplit('/').next()?.to_string();
    if !SEATS.contains(&name.as_str()) {
        return None;
    }
    let session = string_field(input, "session_id").unwrap_or_default();
    match standing(lock_reason(&root), &session) {
        Standing::Ours => None,
        Standing::Foreign(reason) => Some(collision(&name, &reason)),
        Standing::Free => match lock_seat(&root, &root, &session) {
            Claim::Held(reason) => Some(collision(&name, &reason)),
            // OursOrMoot cannot tell "claimed now" from "git could not
            // judge" — only a claim that verifiably took, *for this
            // session*, is announced (a lock that exists but names someone
            // else means the parse above missed a refusal).
            Claim::OursOrMoot => lock_reason(&root)
                .filter(|reason| reason.contains(&session))
                .map(|_| {
                    format!(
                        "Seat {name} stood unclaimed and this edit re-claimed it \
                     for the session (a landed seat comes unlocked; further \
                     work claims it back at its first edit — CLAUDE.md \
                     ビルド・テスト). {}",
                        announce(&name)
                    )
                }),
        },
    }
}

/// Where a seat's lock stands relative to this session. Pure so the tests
/// can ask.
enum Standing {
    /// No lock at all.
    Free,
    /// This session's claim — or a session id too empty to judge by,
    /// where fighting over the seat helps nobody.
    Ours,
    /// Somebody else's claim, or a lock a person wrote by hand.
    Foreign(String),
}

fn standing(reason: Option<String>, session: &str) -> Standing {
    match reason {
        None => Standing::Free,
        Some(reason) if session.is_empty() || reason.contains(session) => Standing::Ours,
        Some(reason) => Standing::Foreign(reason),
    }
}

/// The warning an edit into somebody else's seat rides out on.
fn collision(name: &str, reason: &str) -> String {
    format!(
        "This edit landed in seat {name}, which another session holds \
         (locked: {}). Two sessions in one seat trample each other's tree — \
         move to a free seat (`cargo xtask seats`) and carry over only your \
         own hunks (CLAUDE.md ビルド・テスト).",
        printable(reason)
    )
}

/// Why an EnterWorktree call is held, if it is. Pure so the tests can ask.
fn worktree_objection(name: Option<&str>, path: Option<&str>) -> Option<&'static str> {
    if path.is_some() || name.is_some_and(|name| SEATS.contains(&name)) {
        return None;
    }
    Some(match name {
        Some(_) => {
            "A worktree under a topical name is never reused, so its cold target/ build and its gigabytes are paid for one session."
        }
        None => {
            "A worktree under a generated name is never reused, so its cold target/ build and its gigabytes are paid for one session."
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{Standing, printable, roster_seat, standing, worktree_objection};

    #[test]
    fn knows_a_seat_path_from_the_rest() {
        assert_eq!(
            roster_seat("C:\\x\\platitude-gg\\.claude\\worktrees\\a"),
            Some("a")
        );
        assert_eq!(
            roster_seat("C:/x/platitude-gg/.claude/worktrees/b/crates"),
            Some("b")
        );
        assert_eq!(
            roster_seat("C:/x/platitude-gg/.claude/worktrees/tooltip"),
            None
        );
        assert_eq!(roster_seat("C:/x/platitude-gg"), None);
    }

    #[test]
    fn sanitizes_a_lock_reason_for_the_json_it_rides_in() {
        assert_eq!(
            printable("claude-seat abc\"def\\x\ny"),
            "claude-seat abc'def'x y"
        );
    }

    #[test]
    fn stands_a_lock_relative_to_the_session() {
        assert!(matches!(standing(None, "s1"), Standing::Free));
        assert!(matches!(
            standing(Some("claude-seat s1".into()), "s1"),
            Standing::Ours
        ));
        assert!(matches!(
            standing(Some("claude-seat s2".into()), "s1"),
            Standing::Foreign(_)
        ));
        assert!(matches!(
            standing(Some("parked by hand".into()), "s1"),
            Standing::Foreign(_)
        ));
        // An empty session id can match no claim — leave whatever holds.
        assert!(matches!(
            standing(Some("claude-seat s2".into()), ""),
            Standing::Ours
        ));
    }

    #[test]
    fn holds_worktree_names_outside_the_seat_roster() {
        assert!(worktree_objection(Some("feature-x"), None).is_some());
        assert!(worktree_objection(None, None).is_some());
        assert!(worktree_objection(Some("c"), None).is_none());
        assert!(worktree_objection(None, Some("C:/x/platitude-gg/.claude/worktrees/a")).is_none());
    }
}
