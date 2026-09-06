//! The seat roster guard: which worktrees a session may enter, the atomic
//! claim that keeps two sessions out of one seat, and the re-claim that
//! puts a claim back on a seat that lost one while its session worked in
//! it.

use super::launch::resolve;
use super::payload::{deny, printable, string_field};
use crate::seats::{
    self, Held, Identity, RIG, SEATS, Standing, WorktreeBlock, claim_liveness, commits_in, in_rig,
    lock_reason, same_tree, standing, take_seat, worktree_blocks, worktree_root,
};
use crate::subprocess::git_query;

/// PreToolUse(EnterWorktree): a session enters the seat the roster gave
/// it, and no other.
///
/// Choosing a letter and then entering it is what this refuses. The
/// choosing was always done from a survey, and a survey is exactly the
/// thing two sessions can read the same way while only one of them can
/// be right; the claim behind `cargo xtask seat` is the only step that
/// ever settled it, so it is made the only step there is. A seat this
/// session already holds passes without a word, because that claim is
/// the proof this door asks for (CLAUDE.md ビルド・テスト).
pub(super) fn pre_worktree(input: &str) -> Result<(), String> {
    let name = string_field(input, "name");
    let path = string_field(input, "path");
    // The rig is entered by nobody, and a name for it must not make it
    // either: EnterWorktree creates the tree a name does not find.
    if name.as_deref() == Some(RIG) {
        deny(&rig_reason());
        return Ok(());
    }
    if let Some(objection) = worktree_objection(name.as_deref(), path.as_deref()) {
        println!(
            "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
             \"permissionDecision\":\"ask\",\"permissionDecisionReason\":\
             \"{objection} Worktrees are six reusable seats, a-f, and \
             `cargo xtask seat` is what hands one over — it claims a free \
             letter for this session and prints the path to enter \
             (CLAUDE.md ビルド・テスト). A worktree outside the roster \
             needs the user's say-so.\"}}}}"
        );
        return Ok(());
    }
    let Some(cwd) = string_field(input, "cwd") else {
        return Ok(());
    };
    // Entering by name lands in the seat's existing tree just as surely as
    // entering by path — resolve both, or there is a door around the claim.
    let target = match (path, name) {
        (Some(path), _) => named_tree(&cwd, &path),
        (None, Some(name)) => existing_seat_path(&cwd, &name),
        (None, None) => None,
    };
    let me = Identity::current(string_field(input, "session_id").as_deref());
    let entry = match target.as_deref() {
        Some(tree) if in_rig(tree) => Entry::Rig,
        Some(tree) if roster_seat(tree).is_some() => {
            Entry::Seat(standing_here(&cwd, tree, &me), tree.to_string())
        }
        Some(_) => Entry::OffRoster,
        None => Entry::Unresolved,
    };
    if let Some((decision, reason)) = entry_verdict(&entry, &me) {
        println!(
            "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
             \"permissionDecision\":\"{decision}\",\"permissionDecisionReason\":\
             \"{reason}\"}}}}"
        );
    }
    Ok(())
}

/// Where a seat stands for the session asking to enter it: the plain
/// standing, except that an unclaimed seat the session is already
/// standing in is claimed again rather than refused.
///
/// A claim can go missing under a session that never left its tree, and
/// the refusal that meets it on the way back in reads as somebody else's
/// seat — a session that takes a fresh letter over it leaves its commits
/// behind in the old one. Re-entering the tree the session is already
/// working in names no letter — it is the seat it already had — so the
/// claim is written back instead of the door being shut on it.
fn standing_here(cwd: &str, tree: &str, me: &Identity) -> Standing {
    let standing = standing(lock_reason(tree), me, Held::BySession);
    match reclaims_on_entry(cwd, tree, &standing) {
        true => take_seat(tree, tree, me, Held::BySession),
        false => standing,
    }
}

/// Whether this entry is a session going back into the tree it is already
/// working in, and finding no claim on it. Pure, so the one exception to
/// "seats are handed out, never chosen" can be asserted rather than
/// probed: everywhere else, an unclaimed letter is still a letter picked
/// out of a survey.
fn reclaims_on_entry(cwd: &str, tree: &str, standing: &Standing) -> bool {
    matches!(standing, Standing::Free)
        && worktree_root(cwd).is_some_and(|here| same_tree(&here, tree))
}

/// Why nobody enters or edits the rig: it is the measurement's, and it
/// moves under whoever is in it.
fn rig_reason() -> String {
    format!(
        "This is the measurement rig (.claude/worktrees/{RIG}): `cargo xtask perf --at <rev>` \
         switches it between commits and builds there under a claim of its own, and nobody \
         sits in it — a session inside would be working in a tree that moves under it, and \
         an edit there stops every measurement until somebody cleans it up by hand. Work in \
         a seat (`cargo xtask seat`) and measure the branch from there with `cargo xtask perf \
         --at <branch>`."
    )
}

/// Where an EnterWorktree call would land, once its path is resolved.
enum Entry {
    /// A roster seat, where its claim stands, and the seat's own tree.
    Seat(Standing, String),
    /// The measurement rig, which nobody enters (`seats::RIG`).
    Rig,
    /// A worktree of this repository outside the roster a-f.
    OffRoster,
    /// Nothing this hook could resolve to a tree of this repository.
    Unresolved,
}

/// The rule the entry door holds, apart from the git that resolved the
/// path: only a seat whose claim is already this session's is entered.
/// Pure so the rule the whole roster hangs on can be asserted.
fn entry_verdict(entry: &Entry, me: &Identity) -> Option<(&'static str, String)> {
    if let Entry::Rig = entry {
        return Some(("deny", rig_reason()));
    }
    // A tree of this repository that is not a seat is somebody's
    // unfinished branch, and going back to finish one is a real errand —
    // the roster has no seat to answer it with, so the user does.
    if let Entry::OffRoster = entry {
        return Some((
            "ask",
            "This worktree exists but is outside the seat roster a-f, so \
             nothing here claims it and nothing keeps a second session out \
             of it. Enter it only to carry on the branch it already holds; \
             for new work, `cargo xtask seat` hands this session a seat \
             (CLAUDE.md ビルド・テスト)."
                .to_string(),
        ));
    }
    if let Entry::Seat(Standing::Ours, _) = entry {
        return None;
    }
    Some((
        "deny",
        format!(
            "{} Seats are not chosen, they are handed out: run `cargo xtask \
             seat` and enter the path it prints. It claims a free letter behind \
             the lock — the only step that ever decided which session got a \
             seat — and hands back the one this session may enter, so there is \
             nothing to survey and nothing to pick (CLAUDE.md ビルド・テスト).",
            refusal(entry, me)
        ),
    ))
}

/// The half of the refusal that says what is wrong with this particular
/// call, so a session can tell "somebody is in there" from "you never
/// asked for a seat".
fn refusal(entry: &Entry, me: &Identity) -> String {
    match entry {
        Entry::Seat(Standing::Foreign(reason) | Standing::Stale(reason), tree) => format!(
            "This seat is claimed. The lock says: {}. This session is {}. {}{}",
            printable(reason),
            printable(&me.mark()),
            claim_liveness(reason),
            work_already_there(tree),
        ),
        Entry::Seat(Standing::Free, _) => {
            "This seat carries no claim for this session, and an unclaimed seat \
             is not the same as one that is yours: the claim is what keeps a \
             second session out."
                .to_string()
        }
        // The spelling that resolved to nothing is the spelling that
        // walked two sessions into seat e (2026-09-02): it is refused,
        // never waved through.
        _ => "This call names no seat this session holds — a path that resolved \
              to no worktree of this repository, or a roster letter with no tree \
              yet."
            .to_string(),
    }
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

/// PreToolUse(Write|Edit): where a write would land decides whether it
/// may. This is the door the whole seat mechanism hangs on — a session
/// only ever needs a seat because a write of its own was held here, and
/// what it is told is one command, never a letter to pick (CLAUDE.md
/// ビルド・テスト).
pub(super) fn write_objection(input: &str, path: &str) -> Option<(&'static str, String)> {
    let cwd = string_field(input, "cwd").unwrap_or_default();
    let me = Identity::current(string_field(input, "session_id").as_deref());
    let landing = match worktree_root(path) {
        Some(root) if in_rig(&root) => Landing::Rig,
        Some(root) => match roster_seat(&root) {
            Some(name) => Landing::Seat(
                name,
                standing(lock_reason(&root), &me, Held::BySession),
                root,
            ),
            // A worktree outside the roster is somebody's unfinished
            // branch, and the write door has nothing to say about it.
            None => Landing::Outside,
        },
        // Everything outside this repository is somebody else's business:
        // a memory file, a scratchpad, a sibling project.
        None if in_primary_checkout(&cwd, path) => Landing::Primary,
        None => Landing::Outside,
    };
    write_verdict(&landing, &me)
}

/// Where a write would land, once the path has been placed.
enum Landing {
    /// A roster seat, where its claim stands, and the seat's own tree.
    Seat(&'static str, Standing, String),
    /// The measurement rig, which nobody edits (`seats::RIG`).
    Rig,
    /// The primary checkout of this repository.
    Primary,
    /// Anywhere else at all.
    Outside,
}

/// The rule the write door holds, apart from the git that placed the
/// path. Pure so it can be asserted rather than probed by hand.
fn write_verdict(landing: &Landing, me: &Identity) -> Option<(&'static str, String)> {
    match landing {
        // A seat of this session's own, and everywhere outside this
        // repository, are nobody's business here. An unclaimed seat is
        // let through too: the post-write re-claim takes it back.
        Landing::Outside | Landing::Seat(_, Standing::Ours | Standing::Free, _) => None,
        // The rig is built and measured, never edited: an edit there is
        // what stops every measurement until somebody cleans it up.
        Landing::Rig => Some(("deny", rig_reason())),
        // A seat somebody else is in: refused outright, because the cost
        // of being wrong is the other session's afternoon. The post-write
        // note used to say this only after the file had been written.
        Landing::Seat(name, Standing::Foreign(reason) | Standing::Stale(reason), tree) => Some((
            "deny",
            format!(
                "This edit would write in seat {name}, which this session does \
                 not hold. The seat's lock says: {}. This session is {}. {}{} Run \
                 `cargo xtask seat` for a seat of this session's own and redo \
                 the edit there — do not name a letter (CLAUDE.md \
                 ビルド・テスト).",
                printable(reason),
                printable(&me.mark()),
                claim_liveness(reason),
                work_already_there(tree),
            ),
        )),
        Landing::Primary => Some((
            "ask",
            "This edit would write in the primary checkout, which is only ever \
         read: implementation, documents, settings and the shared session \
         rules all ride worktree branches, because parallel sessions keep \
         reaching for the same files and a direct commit to main collides \
         with theirs (CLAUDE.md ビルド・テスト / Git 運用). Run `cargo xtask \
         seat` — the roster claims a free letter for this session and prints \
         the path to enter with EnterWorktree — then redo the edit there. Do \
         not pick a letter and do not survey for one: a session that chooses \
         from a snapshot is choosing from something another session can \
         agree with and both be wrong. Allow this only if the user asked for \
         a direct change to the primary checkout in so many words."
                .to_string(),
        )),
    }
}

/// What a seat already carries that main does not, named so a session
/// meeting somebody else's claim can tell its own work from theirs.
///
/// A collision leaves nothing for `git status` to show — the other
/// session's work is already committed — and an empty status is what the
/// session sharing seat e read as proof that it was alone there
/// (2026-09-02).
fn work_already_there(tree: &str) -> String {
    let Some(ahead) = commits_in(tree, "main..HEAD").filter(|ahead| *ahead > 0) else {
        return String::new();
    };
    let subjects = git_query(tree, &["log", "--format=%h %s", "-3", "main..HEAD"])
        .unwrap_or_default()
        .lines()
        .collect::<Vec<_>>()
        .join("; ");
    format!(
        " The seat already carries {ahead} commit(s) main does not have, which a \
         clean `git status` says nothing about: {}.",
        printable(&subjects)
    )
}

/// Whether `path` would write into the primary checkout of the repository
/// this session sits in. The repository is asked from the session's own
/// directory, so a path outside it — a memory file, a scratchpad, a
/// sibling project — is never held.
fn in_primary_checkout(cwd: &str, path: &str) -> bool {
    if worktree_root(path).is_some() {
        return false;
    }
    let Some(listing) = git_query(cwd, &["worktree", "list", "--porcelain"]) else {
        return false;
    };
    // The listing's first entry is the primary checkout.
    worktree_blocks(&listing)
        .first()
        .is_some_and(|primary| under(&primary.path, path))
}

/// Whether `path` is `root` itself or something inside it. The boundary
/// is checked rather than the prefix: a sibling directory whose name
/// starts the same way is not inside it.
fn under(root: &str, path: &str) -> bool {
    let root = root.trim_end_matches('/');
    same_tree(root, path)
        || (path.len() > root.len()
            && path.as_bytes()[root.len()] == b'/'
            && same_tree(root, &path[..root.len()]))
}

/// PostToolUse(Write|Edit): an edit inside a roster seat is work, and work
/// holds a claim. A seat a session is working in should already carry
/// one, so this is the net under the ways a claim can still be missing:
/// released by hand, lifted as litter while the session's process was
/// gone, or written before the roster had the claim at all. Quiet while
/// the claim is already this session's; a note when the re-claim takes; a
/// warning when the seat belongs to somebody else.
pub(super) fn reclaim(input: &str, path: &str) -> Option<String> {
    let root = worktree_root(path)?;
    let name = root.rsplit('/').next()?.to_string();
    if !SEATS.contains(&name.as_str()) {
        return None;
    }
    let me = Identity::current(string_field(input, "session_id").as_deref());
    let held = matches!(
        standing(lock_reason(&root), &me, Held::BySession),
        Standing::Ours
    );
    match take_seat(&root, &root, &me, Held::BySession) {
        Standing::Ours if held => None,
        Standing::Ours => Some(format!(
            "Seat {name} stood unclaimed and this edit re-claimed it for the \
             session — a seat being worked in carries its session's claim, \
             and this one had come off (CLAUDE.md ビルド・テスト). {}",
            announce(&name)
        )),
        Standing::Foreign(reason) | Standing::Stale(reason) => {
            Some(collision(&name, &reason, &root, &me))
        }
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
fn collision(name: &str, reason: &str, tree: &str, me: &Identity) -> String {
    format!(
        "This edit landed in seat {name}, which this session does not hold. \
         The seat's lock says: {}. This session is {}. {}{} Two sessions in one \
         seat trample each other's tree and commit on top of one another — take \
         a seat of this session's own with `cargo xtask seat` and carry over \
         only your own hunks (CLAUDE.md ビルド・テスト).",
        printable(reason),
        printable(&me.mark()),
        claim_liveness(reason),
        work_already_there(tree),
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
    use super::{
        Entry, Landing, entry_verdict, reclaims_on_entry, roster_seat, tree_named, under,
        worktree_objection, write_verdict,
    };
    use crate::hook::payload::printable;
    use crate::seats::{Identity, Standing, WorktreeBlock};

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

    /// A session's marks, and a claim that is somebody else's.
    fn me() -> Identity {
        Identity {
            session: "mine".to_string(),
            pid: Some(std::process::id()),
            image: None,
            born: None,
        }
    }

    fn theirs() -> Standing {
        Standing::Foreign("claude-seat theirs".to_string())
    }

    /// A tree no git call can answer for, so the verdicts under test are
    /// the rule alone and not what a repository happened to hold.
    const NO_TREE: &str = "";

    #[test]
    fn only_a_seat_this_session_holds_is_entered() {
        let decision = |entry: &Entry| entry_verdict(entry, &me()).map(|(decision, _)| decision);
        assert_eq!(
            decision(&Entry::Seat(Standing::Ours, NO_TREE.into())),
            None,
            "the claim is the proof this door asks for"
        );
        assert_eq!(
            decision(&Entry::Seat(theirs(), NO_TREE.into())),
            Some("deny")
        );
        assert_eq!(
            decision(&Entry::Seat(
                Standing::Stale("claude-seat theirs pid 1".into()),
                NO_TREE.into()
            )),
            Some("deny"),
            "a seat whose claim died is the roster's to hand out, not this session's to take"
        );
        assert_eq!(
            decision(&Entry::Seat(Standing::Free, NO_TREE.into())),
            Some("deny"),
            "an unclaimed seat is not the same as one that is yours"
        );
        assert_eq!(decision(&Entry::OffRoster), Some("ask"));
        // The spelling that resolved to nothing is the spelling that
        // walked two sessions into seat e.
        assert_eq!(decision(&Entry::Unresolved), Some("deny"));
        // The rig is the measurement's, and it moves under whoever is in it.
        let (decision, reason) = entry_verdict(&Entry::Rig, &me()).expect("the rig is refused");
        assert_eq!(decision, "deny");
        assert!(reason.contains("perf --at"), "{reason}");
    }

    #[test]
    fn the_seat_a_session_stands_in_is_claimed_back_rather_than_refused() {
        let seat_e = "C:/x/platitude-gg/.claude/worktrees/e";
        assert!(
            reclaims_on_entry(seat_e, seat_e, &Standing::Free),
            "a claim can come off under a session that never left its tree, and \
             the refusal reads to it as somebody else's seat"
        );
        assert!(
            reclaims_on_entry(&format!("{seat_e}/crates/xtask"), seat_e, &Standing::Free),
            "the session stands wherever in the tree it was last working"
        );
        assert!(
            !reclaims_on_entry(
                "C:/x/platitude-gg/.claude/worktrees/a",
                seat_e,
                &Standing::Free
            ),
            "an unclaimed letter read from another seat is a letter chosen"
        );
        assert!(
            !reclaims_on_entry(PRIMARY, seat_e, &Standing::Free),
            "and so is one chosen from the primary checkout"
        );
        assert!(
            !reclaims_on_entry(seat_e, seat_e, &theirs()),
            "a claim somebody else holds is never written over"
        );
    }

    #[test]
    fn a_write_is_held_where_it_would_land_in_somebody_elses_tree() {
        let decision =
            |landing: &Landing| write_verdict(landing, &me()).map(|(decision, _)| decision);
        assert_eq!(
            decision(&Landing::Seat("a", Standing::Ours, NO_TREE.into())),
            None
        );
        assert_eq!(
            decision(&Landing::Seat("a", Standing::Free, NO_TREE.into())),
            None,
            "a seat whose claim came off is still this session's to write in; \
             the post-write re-claim takes it back"
        );
        assert_eq!(
            decision(&Landing::Seat("b", theirs(), NO_TREE.into())),
            Some("deny")
        );
        assert_eq!(
            decision(&Landing::Primary),
            Some("ask"),
            "the primary checkout is read-only, but a direct change the user asked \
             for in so many words is still theirs to allow"
        );
        assert_eq!(
            decision(&Landing::Outside),
            None,
            "a memory file, a scratchpad, a sibling project"
        );
        assert_eq!(
            decision(&Landing::Rig),
            Some("deny"),
            "the rig is built and measured, never edited"
        );
    }

    #[test]
    fn a_refusal_names_both_marks_so_the_reader_can_tell_them_apart() {
        let (_, reason) = entry_verdict(&Entry::Seat(theirs(), NO_TREE.into()), &me())
            .expect("somebody else's seat is refused");
        assert!(reason.contains("claude-seat theirs"), "{reason}");
        assert!(reason.contains("mine"), "{reason}");
        assert!(reason.contains("cargo xtask seat"), "{reason}");
    }

    #[test]
    fn a_path_beside_the_primary_checkout_is_not_inside_it() {
        assert!(under(PRIMARY, PRIMARY));
        assert!(under(PRIMARY, "C:/x/platitude-gg/CLAUDE.md"));
        assert!(under(PRIMARY, "C:/x/platitude-gg/crates/xtask/src/main.rs"));
        // The trap a plain prefix test falls into: a sibling whose name
        // begins with the checkout's would be held as one of its files.
        assert!(!under(PRIMARY, "C:/x/platitude-gg-notes/CLAUDE.md"));
        // What the guard must never hold: a memory file, a scratchpad, a
        // sibling project — everything outside the repository.
        assert!(!under(PRIMARY, "C:/Users/x/.claude/memory/note.md"));
        assert_eq!(
            under(PRIMARY, "C:/X/Platitude-GG/CLAUDE.md"),
            cfg!(windows),
            "one path spelled two ways is one path only where the OS says so"
        );
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
        // The rig sits beside the seats and is none of them.
        assert_eq!(roster_seat("C:/x/platitude-gg/.claude/worktrees/rig"), None);
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
