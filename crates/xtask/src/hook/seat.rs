//! The seat roster guard: which worktrees a session may enter or write in,
//! and the re-claim of a seat that lost its claim while its session
//! worked in it.

use super::launch::resolve;
use super::payload::{deny, printable, string_field};
use crate::seats::{
    self, Held, Identity, RIG, SEATS, Standing, WorktreeBlock, commits_in, how_claims_move, in_rig,
    lock_reason, same_tree, standing, take_seat, whose, worktree_blocks, worktree_root,
};
use crate::subprocess::git_query;

/// PreToolUse(EnterWorktree): a session enters only a seat whose claim is
/// already its own. A letter chosen from a survey is refused — two
/// sessions can read one survey the same way, and only the claim behind
/// `cargo xtask seat` settles it (CLAUDE.md ビルド・テスト).
pub(super) fn pre_worktree(input: &str) -> Result<(), String> {
    let name = string_field(input, "name");
    let path = string_field(input, "path");
    // The rig is refused by name too: EnterWorktree creates the tree a
    // name does not find.
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
    // A name lands in the seat's existing tree as surely as a path does;
    // resolve both, or the claim has a side door.
    let target = match (path, name) {
        (Some(path), _) => named_tree(&cwd, &path),
        (None, Some(name)) => existing_seat_path(&cwd, &name),
        (None, None) => None,
    };
    let me = Identity::current(string_field(input, "session_id").as_deref());
    let entry = match target.as_deref() {
        Some(tree) if in_rig(tree) => Entry::Rig,
        Some(tree) if seats::roster_letter(tree).is_some() => {
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

/// Where a seat stands for the session asking to enter it, except that an
/// unclaimed seat the session is already standing in is claimed again: a
/// claim can come off under a session that never left, and refusing it
/// would send the session to a fresh letter, leaving its commits behind.
fn standing_here(cwd: &str, tree: &str, me: &Identity) -> Standing {
    let standing = standing(lock_reason(tree), me, Held::BySession);
    match reclaims_on_entry(cwd, tree, &standing) {
        true => take_seat(tree, tree, me, Held::BySession),
        false => standing,
    }
}

/// Whether this entry goes back into the tree the session already works
/// in and finds no claim — the one exception to "seats are handed out".
/// Pure, for the tests.
fn reclaims_on_entry(cwd: &str, tree: &str, standing: &Standing) -> bool {
    matches!(standing, Standing::Free)
        && worktree_root(cwd).is_some_and(|here| same_tree(&here, tree))
}

/// Why nobody enters or edits the rig.
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

/// The entry door's rule, apart from the git that resolved the path. Pure,
/// for the tests.
fn entry_verdict(entry: &Entry, me: &Identity) -> Option<(&'static str, String)> {
    if let Entry::Rig = entry {
        return Some(("deny", rig_reason()));
    }
    // A tree off the roster is somebody's unfinished branch; finishing it
    // is an errand the roster cannot judge, so the user does.
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
            "{} Seats are handed out: run `cargo xtask \
             seat` and enter the path it prints. It claims a free letter behind \
             the lock — the only step that ever decided which session got a \
             seat — and hands back the one this session may enter \
             (CLAUDE.md ビルド・テスト).",
            refusal(entry, me)
        ),
    ))
}

/// What is wrong with this particular call: "somebody is in there" or
/// "you never asked for a seat".
fn refusal(entry: &Entry, me: &Identity) -> String {
    match entry {
        Entry::Seat(Standing::Foreign(reason) | Standing::Stale(reason), tree) => format!(
            "This seat is claimed: {}. This session is {}. {}{}",
            printable(&whose(reason)),
            printable(&me.mark()),
            how_claims_move(),
            work_already_there(tree),
        ),
        Entry::Seat(Standing::Free, _) => {
            "This seat carries no claim for this session, and the claim is \
             what makes a seat yours: it is what keeps a \
             second session out."
                .to_string()
        }
        // Letting an unresolved spelling through puts two sessions in one
        // seat.
        _ => "This call names no seat this session holds — a path that resolved \
              to no worktree of this repository, or a roster letter with no tree \
              yet."
            .to_string(),
    }
}

/// PostToolUse(EnterWorktree): the claim was settled in `pre_worktree`;
/// this marks the entry and has the session name its seat.
pub(super) fn post_worktree(input: &str) -> Result<(), String> {
    // An entry by name carries no `path`, so `cwd` is the fallback.
    let cwd = string_field(input, "cwd").unwrap_or_default();
    let entered = string_field(input, "path").and_then(|path| named_tree(&cwd, &path));
    let Some(seat) = entered
        .as_deref()
        .and_then(|root| seats::roster_letter(root))
        .or_else(|| seats::roster_letter(&cwd))
    else {
        return Ok(());
    };
    // Only here is the entry visible: a later shell line's cwd is not
    // where the session works (`seats::entry`).
    if let Some(root) = crate::tree::primary_root(&cwd)
        && let Some(session) = string_field(input, "session_id")
    {
        seats::entry::mark(&root, &session, seat);
    }
    println!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PostToolUse\",\
         \"additionalContext\":\"{}\"}}}}",
        announce(seat)
    );
    Ok(())
}

/// SessionEnd: the entry mark goes with the session; the seat stays
/// claimed (`seats::how_claims_move`).
pub(super) fn session_end(input: &str) {
    let cwd = string_field(input, "cwd").unwrap_or_default();
    if let Some(root) = crate::tree::primary_root(&cwd)
        && let Some(session) = string_field(input, "session_id")
    {
        seats::entry::forget(&root, &session);
    }
}

/// What a session owes the user the moment a seat becomes its own.
fn announce(seat: &str) -> String {
    format!(
        "Seat {seat} is this session's now. Name it to the user in your very next \
         reply, in Japanese and by its letter (「席 {seat} を取った」) — the letter \
         is how the user tells which tree a change, a build or a screenshot \
         came from (CLAUDE.md ビルド・テスト)."
    )
}

/// The worktree a tool's `path` names, resolved against the listing.
/// Judge seats only through this: the raw string calls
/// `.claude/worktrees/e` no seat, and an entry spelled that way would take
/// no claim and meet no refusal.
fn named_tree(cwd: &str, path: &str) -> Option<String> {
    let listing = git_query(cwd, &["worktree", "list", "--porcelain"])?;
    tree_named(&worktree_blocks(&listing), cwd, path)
}

/// The listing half of [`named_tree`], pure for the tests. A relative path
/// is tried against the session's directory and against the primary
/// checkout (the listing's first entry), which a session in one seat
/// writes another seat's path against.
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

/// PreToolUse(Bash|PowerShell): `cargo xtask seat takeover` runs only
/// behind its escape, written when the user asked (CLAUDE.md ビルド・テスト).
pub(super) fn pre_takeover(input: &str) -> Result<bool, String> {
    let Some(command) = string_field(input, "command") else {
        return Ok(false);
    };
    if !takes_over_unasked(&command) {
        return Ok(false);
    }
    deny(&format!(
        "`cargo xtask seat takeover` moves a seat away from the session that holds it, and a \
         letter changes hands only on the user's word (CLAUDE.md ビルド・テスト). If the \
         user asked for this letter in so many words, run the same command with {}=1 in \
         front of it; otherwise `cargo xtask seat` hands out a free letter, and when none is \
         free, stop and tell the user what stands in the way (`cargo xtask seats`).",
        super::approval::TAKEOVER_APPROVAL_FLAG
    ));
    Ok(true)
}

/// Whether a shell line runs the takeover verb without the escape.
/// `takeover` must be the seat verb's own operation: matched anywhere, it
/// would refuse a `seat release` whose comment mentions it.
fn takes_over_unasked(command: &str) -> bool {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    super::git::xtask_verb(&tokens, "seat")
        && seat_operation(&tokens) == Some("takeover")
        && !command.contains(super::approval::TAKEOVER_APPROVAL_FLAG)
}

/// The operation a `seat` line names: the first argument past the verb
/// that is neither an option nor an option's value.
fn seat_operation<'a>(tokens: &[&'a str]) -> Option<&'a str> {
    let mut rest = tokens.iter().skip_while(|token| **token != "seat").skip(1);
    while let Some(token) = rest.next() {
        if *token == "--dir" {
            rest.next();
        } else if !token.starts_with('-') {
            return Some(token);
        }
    }
    None
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
/// may — the door the seat mechanism hangs on (CLAUDE.md ビルド・テスト).
pub(super) fn write_objection(input: &str, path: &str) -> Option<(&'static str, String)> {
    let cwd = string_field(input, "cwd").unwrap_or_default();
    let me = Identity::current(string_field(input, "session_id").as_deref());
    let landing = match worktree_root(path) {
        Some(root) if in_rig(&root) => Landing::Rig,
        Some(root) => match seats::roster_letter(&root) {
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
/// path. Pure so it can be asserted.
fn write_verdict(landing: &Landing, me: &Identity) -> Option<(&'static str, String)> {
    match landing {
        // An unclaimed seat passes: the post-write re-claim takes it back.
        Landing::Outside | Landing::Seat(_, Standing::Ours | Standing::Free, _) => None,
        Landing::Rig => Some(("deny", rig_reason())),
        // Refused outright: a post-write note would come only after the
        // file is written in the other session's tree.
        Landing::Seat(name, Standing::Foreign(reason) | Standing::Stale(reason), tree) => Some((
            "deny",
            format!(
                "This edit would write in seat {name}, which this session does \
                 not hold. The seat is {}. This session is {}. {}{} Run \
                 `cargo xtask seat` for a seat of this session's own and redo \
                 the edit there (CLAUDE.md \
                 ビルド・テスト).",
                printable(&whose(reason)),
                printable(&me.mark()),
                how_claims_move(),
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
         the path to enter with EnterWorktree — then redo the edit there. \
         The letter comes from the claim: a session that chooses \
         from a snapshot is choosing from something another session can \
         agree with and both be wrong. Allow this only if the user asked for \
         a direct change to the primary checkout in so many words."
                .to_string(),
        )),
    }
}

/// What a seat already carries that main does not, so a session meeting
/// somebody else's claim can tell its own work from theirs — a clean
/// `git status` says nothing about committed work.
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

/// Whether `path` is inside the primary checkout of the repository the
/// session sits in (asked from `cwd`).
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
/// is checked: a sibling directory whose name starts the same way is
/// not inside it.
fn under(root: &str, path: &str) -> bool {
    let root = root.trim_end_matches('/');
    same_tree(root, path)
        || (path.len() > root.len()
            && path.as_bytes()[root.len()] == b'/'
            && same_tree(root, &path[..root.len()]))
}

/// PostToolUse(Write|Edit): an edit inside a roster seat holds a claim —
/// the net under a claim that came off (released by hand or by a land)
/// while its session kept working. Quiet when the claim is already this
/// session's; a note when the re-claim takes; a warning when the seat is
/// somebody else's.
pub(super) fn reclaim(input: &str, path: &str) -> Option<String> {
    let root = worktree_root(path)?;
    let name = seats::roster_letter(&root)?;
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
            announce(name)
        )),
        Standing::Foreign(reason) | Standing::Stale(reason) => {
            Some(collision(name, &reason, &root, &me))
        }
        // git could not judge the seat; the tool call surfaces what is wrong.
        Standing::Free => None,
    }
}

/// The warning an edit into somebody else's seat rides out on. Both marks
/// go in it: naming only the lock lets a session read a claim written in
/// the same minute as its own.
fn collision(name: &str, reason: &str, tree: &str, me: &Identity) -> String {
    format!(
        "This edit landed in seat {name}, which this session does not hold. \
         The seat is {}. This session is {}. {}{} Two sessions in one \
         seat trample each other's tree and commit on top of one another — take \
         a seat of this session's own with `cargo xtask seat` and carry over \
         only your own hunks (CLAUDE.md ビルド・テスト).",
        printable(&whose(reason)),
        printable(&me.mark()),
        how_claims_move(),
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
        Entry, Landing, entry_verdict, reclaims_on_entry, takes_over_unasked, tree_named, under,
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
        // Spelled against the primary checkout, where the session stood.
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
        }
    }

    fn theirs() -> Standing {
        Standing::Foreign("claude-seat theirs".to_string())
    }

    /// A tree no git call can answer for, so the verdicts under test are
    /// the rule alone.
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
            "a seat whose claim died is the roster's to hand out"
        );
        assert_eq!(
            decision(&Entry::Seat(Standing::Free, NO_TREE.into())),
            Some("deny"),
            "a seat is this session's by its claim alone"
        );
        assert_eq!(decision(&Entry::OffRoster), Some("ask"));
        assert_eq!(decision(&Entry::Unresolved), Some("deny"));
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
            "a claim somebody else holds stays theirs"
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
            "the rig is built and measured only"
        );
    }

    #[test]
    fn a_refusal_names_both_marks_so_the_reader_can_tell_them_apart() {
        let (_, reason) = entry_verdict(&Entry::Seat(theirs(), NO_TREE.into()), &me())
            .expect("somebody else's seat is refused");
        assert!(reason.contains("held by session theirs"), "{reason}");
        assert!(reason.contains("mine"), "{reason}");
        assert!(reason.contains("cargo xtask seat"), "{reason}");
        // The user's word is what moves a claim.
        assert!(reason.contains("PGG_ALLOW_TAKEOVER=1"), "{reason}");
        assert!(reason.contains("no process is asked"), "{reason}");
    }

    #[test]
    fn the_takeover_verb_goes_through_only_behind_the_flag() {
        for command in [
            "cargo xtask seat takeover b",
            "cargo run -p xtask -- seat takeover b",
            "cd .. && cargo xtask seat --dir /x/repo takeover b",
        ] {
            assert!(takes_over_unasked(command), "{command}");
        }
        for command in [
            "PGG_ALLOW_TAKEOVER=1 cargo xtask seat takeover b",
            "cargo xtask seat",
            "cargo xtask seat release",
            "cargo xtask seats",
            "git worktree lock --reason takeover x",
        ] {
            assert!(!takes_over_unasked(command), "{command}");
        }
    }

    #[test]
    fn a_path_beside_the_primary_checkout_is_not_inside_it() {
        assert!(under(PRIMARY, PRIMARY));
        assert!(under(PRIMARY, "C:/x/platitude-gg/CLAUDE.md"));
        assert!(under(PRIMARY, "C:/x/platitude-gg/crates/xtask/src/main.rs"));
        // A plain prefix test would hold this sibling.
        assert!(!under(PRIMARY, "C:/x/platitude-gg-notes/CLAUDE.md"));
        assert!(!under(PRIMARY, "C:/Users/x/.claude/memory/note.md"));
        assert_eq!(
            under(PRIMARY, "C:/X/Platitude-GG/CLAUDE.md"),
            cfg!(windows),
            "one path spelled two ways is one path only where the OS says so"
        );
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
