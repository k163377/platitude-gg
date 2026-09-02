//! The seat roster guard: which worktrees a session may enter, the atomic
//! claim that keeps two sessions out of one seat, and the re-claim that
//! puts a claim back on a seat `land` set free.

use super::launch::resolve;
use super::payload::string_field;
use crate::seats::{
    self, Identity, SEATS, Standing, WorktreeBlock, claim_liveness, lock_reason, standing,
    take_seat, unlock_seat, worktree_blocks, worktree_root,
};
use crate::subprocess::git_query;

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
        (Some(path), _) => named_tree(&cwd, &path).filter(|tree| roster_seat(tree).is_some()),
        (None, Some(name)) => existing_seat_path(&cwd, &name),
        (None, None) => None,
    };
    let Some(path) = target else {
        return Ok(());
    };
    let me = Identity::current(string_field(input, "session_id").as_deref());
    if let Standing::Foreign(reason) | Standing::Stale(reason) = take_seat(&cwd, &path, &me) {
        println!(
            "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
             \"permissionDecision\":\"deny\",\"permissionDecisionReason\":\
             \"This seat is already claimed. The lock says: {}. This session \
             is {}. {} Seats are first come, first served and this refusal is \
             the roster answering, not an error to work around: take a \
             different letter and enter it the same way. There is nothing to \
             survey first — the claim is the check (CLAUDE.md \
             ビルド・テスト).\"}}}}",
            printable(&reason),
            printable(&me.mark()),
            claim_liveness(&reason),
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
    let cwd = string_field(input, "cwd").unwrap_or_default();
    let entered = string_field(input, "path").and_then(|path| named_tree(&cwd, &path));
    let Some(seat) = entered
        .as_deref()
        .and_then(roster_seat)
        .or_else(|| roster_seat(&cwd))
    else {
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

/// The worktree a tool's `path` names, resolved the way git resolves it
/// and answered by the listing rather than by reading the string.
///
/// A path is only a seat's if this says so: `.claude/worktrees/e` and its
/// absolute spelling are one tree, and judging the raw string calls the
/// first one no seat at all. That is how two sessions came to share seat
/// e — the entry that spelled it relatively took no claim and met no
/// refusal, and worked on top of the other's commits for eight minutes
/// (observed 2026-09-02).
fn named_tree(cwd: &str, path: &str) -> Option<String> {
    let listing = git_query(cwd, &["worktree", "list", "--porcelain"])?;
    tree_named(&worktree_blocks(&listing), cwd, path)
}

/// The listing half of [`named_tree`], pure so the tests can ask.
///
/// Two spellings are tried, because a relative path is written against
/// wherever the session stands: the session's own directory, and the
/// primary checkout, which is the listing's first entry and what a
/// session sitting in one seat writes another seat's path against.
fn tree_named(trees: &[WorktreeBlock], cwd: &str, path: &str) -> Option<String> {
    let candidates = [resolve(cwd, path), resolve(&trees.first()?.path, path)];
    trees
        .iter()
        .find(|tree| {
            candidates
                .iter()
                .any(|candidate| same_tree(candidate, &tree.path))
        })
        .map(|tree| tree.path.clone())
}

/// Whether two paths name one tree. Windows spells a path in whatever
/// case the writer used, so the comparison there is case-blind.
fn same_tree(left: &str, right: &str) -> bool {
    let trim = |path: &str| path.replace('\\', "/").trim_end_matches('/').to_string();
    let (left, right) = (trim(left), trim(right));
    if cfg!(windows) {
        left.eq_ignore_ascii_case(&right)
    } else {
        left == right
    }
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
    // The unlock must name the worktree by its top-level path — git
    // resolves the argument by exact real path, so a session that ended
    // standing in a subdirectory would fail it silently.
    let root = worktree_root(&cwd).unwrap_or(cwd);
    let me = Identity::current(Some(&session));
    if matches!(standing(lock_reason(&root), &me), Standing::Ours) && !unlock_seat(&root, &root) {
        // Nobody is left to tell; the claim's dead pid is what the next
        // session reads it by.
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
    let me = Identity::current(string_field(input, "session_id").as_deref());
    let held = matches!(standing(lock_reason(&root), &me), Standing::Ours);
    match take_seat(&root, &root, &me) {
        Standing::Ours if held => None,
        Standing::Ours => Some(format!(
            "Seat {name} stood unclaimed and this edit re-claimed it for the \
             session (a landed seat comes unlocked; further work claims it \
             back at its first edit — CLAUDE.md ビルド・テスト). {}",
            announce(&name)
        )),
        Standing::Foreign(reason) | Standing::Stale(reason) => Some(collision(&name, &reason, &me)),
        // git could not judge the seat at all; the tool call itself will
        // surface whatever is actually wrong with it.
        Standing::Free => None,
    }
}

/// The warning an edit into somebody else's seat rides out on.
///
/// Both marks go in it, and the verdict on the other one's process with
/// them. A warning that names only the lock leaves the reader to date it
/// against their own arrival, and a lock written in the same minute then
/// reads as their own — which is how the session that shared seat e
/// talked itself out of eight of these (2026-09-02).
fn collision(name: &str, reason: &str, me: &Identity) -> String {
    format!(
        "This edit landed in seat {name}, which this session does not hold. \
         The seat's lock says: {}. This session is {}. {} Two sessions in one \
         seat trample each other's tree and commit on top of one another — \
         move to a free seat and carry over only your own hunks (CLAUDE.md \
         ビルド・テスト).",
        printable(reason),
        printable(&me.mark()),
        claim_liveness(reason),
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
    use super::{printable, roster_seat, tree_named, worktree_objection};
    use crate::seats::WorktreeBlock;

    const PRIMARY: &str = "C:/x/platitude-gg";

    fn listing() -> Vec<WorktreeBlock> {
        [PRIMARY, "C:/x/platitude-gg/.claude/worktrees/a"]
            .into_iter()
            .chain(["C:/x/platitude-gg/.claude/worktrees/e"])
            .map(|path| WorktreeBlock {
                path: path.to_string(),
                branch: String::new(),
                locked: false,
                reason: String::new(),
            })
            .collect()
    }

    #[test]
    fn a_seat_spelled_relatively_is_the_same_seat() {
        let trees = listing();
        let seat_e = "C:/x/platitude-gg/.claude/worktrees/e".to_string();
        // The shape that put two sessions in seat e: the entry spelled
        // the path against the primary checkout, where the session stood.
        assert_eq!(
            tree_named(&trees, PRIMARY, ".claude/worktrees/e"),
            Some(seat_e.clone())
        );
        // The same letter written from inside another seat, and the
        // absolute spelling with the separators Windows hands over.
        assert_eq!(
            tree_named(
                &trees,
                "C:/x/platitude-gg/.claude/worktrees/a",
                ".claude/worktrees/e"
            ),
            Some(seat_e.clone())
        );
        assert_eq!(
            tree_named(
                &trees,
                PRIMARY,
                "C:\\x\\platitude-gg\\.claude\\worktrees\\e"
            ),
            Some(seat_e)
        );
        // A path that is no tree of this repository stays unjudged, and
        // the caller has to treat that as "not a seat I may enter".
        assert_eq!(tree_named(&trees, PRIMARY, ".claude/worktrees/z"), None);
        assert_eq!(tree_named(&trees, PRIMARY, "crates/xtask"), None);
    }

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
        // Why nothing may be judged by the raw string a tool was given:
        // a seat spelled relatively reads as no seat here, so a path
        // goes through `named_tree` before it reaches this.
        assert_eq!(roster_seat(".claude/worktrees/e"), None);
    }

    #[test]
    fn sanitizes_a_lock_reason_for_the_json_it_rides_in() {
        assert_eq!(
            printable("claude-seat abc\"def\\x\ny"),
            "claude-seat abc'def'x y"
        );
    }

    #[test]
    fn holds_worktree_names_outside_the_seat_roster() {
        assert!(worktree_objection(Some("feature-x"), None).is_some());
        assert!(worktree_objection(None, None).is_some());
        assert!(worktree_objection(Some("c"), None).is_none());
        assert!(worktree_objection(None, Some("C:/x/platitude-gg/.claude/worktrees/a")).is_none());
    }
}
