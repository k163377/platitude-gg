//! The worktree seat roster: the survey (`cargo xtask seats`) the
//! session-start greeting shares, and the letters it hands out. Whose a
//! seat is, is the claim's to say (`claim`).
//!
//! The greeting reports where the seats stood when the session began, and
//! that snapshot goes stale: a seat it called free was measured minutes
//! later holding another session's 13 uncommitted files.
//! This module is the one place a seat is measured, so the command and
//! the greeting cannot drift apart — and the command is the live answer
//! to read before entering a seat (CLAUDE.md ビルド・テスト).
//!
//! A letter changes hands three ways. `cargo xtask seat` takes a letter
//! nobody holds; `land` and `seat release` hand one back; `seat takeover`
//! moves one on the user's word. Nothing else moves a claim — not a
//! process ending, not a session ending — because nothing else knows
//! whether the conversation behind it goes on (`claim`).

use std::time::SystemTime;

mod claim;
pub(crate) mod commands;
mod survey;

pub(crate) use claim::{
    Held, Identity, Standing, is_a_seat_claim, lock_reason, standing, take_seat, unlock_seat, whose,
};
pub(crate) use commands::COMMANDS;
pub(crate) use survey::{Seat, format_age, run, survey};
// Built only by the greeting's tests: the survey is the one thing that
// measures a seat, and everything else reads what it measured.
#[cfg(test)]
pub(crate) use survey::SeatState;

/// The reusable worktree seats. Sessions rotate through these six —
/// a topical worktree is never reused, so
/// every one paid a cold target/ build and kept the gigabytes afterwards
/// (CLAUDE.md ビルド・テスト).
pub(crate) const SEATS: [&str; 6] = ["a", "b", "c", "d", "e", "f"];

/// The directory every worktree of this repository sits under.
const WORKTREES: &str = "/.claude/worktrees/";

/// How a seat claim moves, for every refusal that meets one. The reader
/// is a session that wants the letter, and what it can do about a claim
/// is nothing — or tell the user, whose word is what moves it.
pub(crate) fn how_claims_move() -> String {
    format!(
        "A seat claim stands until its branch lands, its session hands it back \
         (`{}`), or the user has it taken over (`{}`); no process is asked, so \
         neither a restart of the app nor a session ending lifts it.",
        commands::RELEASE.line(),
        commands::TAKEOVER.line()
    )
}

/// A seat this session holds, whether it just took it or already had it.
pub(crate) struct Assigned {
    pub seat: &'static str,
    pub path: String,
    /// What the roster did to the seat on the way in, if anything.
    pub note: String,
    /// Whether the session was already holding it before this ran.
    pub held: bool,
}

impl Assigned {
    /// What the session is told. An instruction: this path
    /// is the one EnterWorktree argument that will be let through, because
    /// the claim behind it is already this session's.
    pub(crate) fn report(&self) -> String {
        let standing = if self.held {
            "was already this session's"
        } else {
            "is this session's now"
        };
        format!(
            "seat {} {standing}{}\nenter it with EnterWorktree path={}",
            self.seat, self.note, self.path
        )
    }
}

/// `cargo xtask seat`: the roster hands this session a seat.
///
/// The letter is the roster's answer, because a letter named is one
/// chosen from a survey, and a survey is a snapshot two sessions can
/// agree on while both are wrong about it. Only the lock ever decided
/// which of them got the seat, so the choosing happens here, behind
/// that lock: letters are tried until one is claimed, and the letter
/// comes back as an answer (CLAUDE.md ビルド・テスト).
pub fn take(args: &[String]) -> Result<(), String> {
    let (root, args) = rooted(args)?;
    let me = Identity::current(None);
    match args {
        [] => {
            println!("{}", assign(&root, &me)?.report());
            Ok(())
        }
        [verb] if verb == "release" => release(&root, &me, None),
        [verb, letter] if verb == "release" => release(&root, &me, Some(letter)),
        [verb, letter] if verb == "takeover" => takeover(&root, &me, letter),
        _ => Err(format!(
            "seat takes no arguments, `release [<letter>]` to hand a seat of this \
             session's back, or `takeover <letter>` to take one over on the user's \
             word (got {args:?})"
        )),
    }
}

/// Where this call is being made from: `--dir <path>` when it is given,
/// and the working directory otherwise.
///
/// It says both which repository's roster is being asked and where the
/// session asking stands, and the roster turns on the second as much as
/// the first (`assign`). The working directory is what answers that, not
/// the tree this binary was built in (`tree::workspace_root`): the two
/// agree when `cargo xtask` is run from inside a seat and part company
/// the moment it is not, and the other doors a seat has all read the
/// directory the session is actually in. `--dir` is for a roster that is
/// not the one around this call — the suite's sandboxes — and `land`
/// takes the same flag.
fn rooted(args: &[String]) -> Result<(String, &[String]), String> {
    match args.split_first() {
        Some((flag, rest)) if flag == "--dir" => {
            let (path, rest) = rest.split_first().ok_or("--dir needs a path")?;
            Ok((path.replace('\\', "/"), rest))
        }
        _ => {
            let here = std::env::current_dir()
                .map_err(|e| format!("this call has no working directory to read: {e}"))?;
            Ok((slashed(&here), args))
        }
    }
}

/// `cargo xtask seat release [<letter>]`: a seat this session holds goes
/// back to the roster.
///
/// For handing a seat back without landing it — work abandoned, or a
/// stretch that ends in a branch somebody else will merge. A landing
/// hands its seat back already (`land::release_claim`), and the
/// SessionEnd the machine's sleep hands out lifts nothing, because that
/// event reaches every open conversation (CLAUDE.md ビルド・テスト).
/// What the seat still carries is named on the way out: the roster hands
/// a letter with work in it to nobody else, so a release leaves that
/// work for a reader to land, take over or drop — and for the releasing
/// session to pick up again by asking from that tree
/// (`claim_where_it_stands`). The letter is needed only when the session
/// holds more than one — a takeover on top of its own — and stands in
/// neither.
fn release(root: &str, me: &Identity, letter: Option<&str>) -> Result<(), String> {
    me.require_session()?;
    let (primary, trees) = primary_checkout(root)?;
    let entries = seat_entries_of(trees);
    let mine: Vec<&SeatEntry> = entries
        .iter()
        .filter(|entry| is_ours(&entry.tree, me))
        .collect();
    let letters = || {
        mine.iter()
            .map(|entry| entry.seat)
            .collect::<Vec<_>>()
            .join(", ")
    };
    let held = match letter {
        Some(letter) => mine
            .iter()
            .find(|entry| entry.seat == letter)
            .copied()
            .ok_or_else(|| match mine.is_empty() {
                true => format!("this session does not hold seat {letter} — it holds no seat"),
                false => format!(
                    "this session does not hold seat {letter} — it holds {}",
                    letters()
                ),
            })?,
        None => match mine.as_slice() {
            [] => return Err("this session holds no seat, so there is none to release".into()),
            [one] => one,
            _ => mine
                .iter()
                .find(|entry| {
                    worktree_root(root).is_some_and(|here| same_tree(&entry.tree.path, &here))
                })
                .copied()
                .ok_or_else(|| {
                    format!(
                        "this session holds seats {} — name the one to release (`cargo xtask \
                         seat release <letter>`)",
                        letters()
                    )
                })?,
        },
    };
    if !unlock_seat(&primary, &held.tree.path) {
        return Err(format!(
            "the claim on seat {} did not release — `git worktree unlock {}` by hand",
            held.seat, held.tree.path
        ));
    }
    println!(
        "seat {} released{}",
        held.seat,
        left_behind(&held.tree.path)
    );
    Ok(())
}

/// What a released seat still carries, said on the way out so that the
/// roster's refusal to hand it to anybody is not a surprise later. A
/// figure git could not answer for is reported as unknown — the seat is
/// released either way, and a silent zero would be
/// the one reading that needs no reply.
fn left_behind(path: &str) -> String {
    let (Some(ahead), Some(dirty)) = (commits_in(path, "main..HEAD"), dirty_lines(path)) else {
        return " — git could not say what it still carries; `cargo xtask seats` reads the roster"
            .to_string();
    };
    if ahead == 0 && dirty == 0 {
        return String::new();
    }
    format!(
        " — it still carries {ahead} commit(s) main does not have and {dirty} \
         uncommitted file(s), so the roster hands it to nobody else until those \
         land, are taken over, or go. Asking for a seat from that tree takes it \
         back, work and all"
    )
}

/// `cargo xtask seat takeover <letter>`: the letter goes to this session
/// whoever holds it, tree and board untouched.
///
/// The one way a claim moves without its own session, and it moves on
/// the user's word alone: the pre-shell hook lets the verb through only
/// behind the escape flag (hook/seat.rs), which a session writes for an
/// instruction that asked and for nothing else (CLAUDE.md
/// ビルド・テスト). Nothing about the letter is judged here, because
/// the user already has — a live claim, a session that is gone, an
/// unclaimed letter carrying commits or uncommitted files: this session
/// inherits all of it as it stands. Nothing is put at main's tip and
/// the board is not swept, because the work goes on rather than ending
/// (`start_at_main` is the other case). A letter whose tree went away
/// grows one back on its own branch, and a letter never made is made.
fn takeover(root: &str, me: &Identity, letter: &str) -> Result<(), String> {
    me.require_session()?;
    let seat = SEATS
        .into_iter()
        .find(|name| *name == letter)
        .ok_or_else(|| format!("{letter} is no roster letter — the roster is a-f"))?;
    let (primary, trees) = primary_checkout(root)?;
    let entries = seat_entries_of(trees);
    let taken = match entries.iter().find(|entry| entry.seat == seat) {
        Some(entry) if has_a_directory(&entry.tree.path) => {
            take_over_tree(&primary, &entry.tree.path, seat, me)?
        }
        Some(entry) => grow_back(&primary, seat, me, Some(&entry.tree))?,
        None => grow_back(&primary, seat, me, None)?,
    };
    println!("{}", taken.report());
    Ok(())
}

/// Whether a listed tree is on disk. A directory can go while git still
/// lists the tree — the listing then calls it prunable — and such a
/// letter is treeless for every purpose here: nothing can be measured
/// or entered there, and the registration only stands in the way of
/// growing the tree back.
fn has_a_directory(path: &str) -> bool {
    std::path::Path::new(path).is_dir()
}

/// Clears git's record of a tree whose directory is gone, so a tree can
/// be made at that path again. The lock comes off first: a locked tree
/// is one git keeps the record of however long it is missing.
fn clear_missing_registration(primary: &str, path: &str) {
    unlock_seat(primary, path);
    let _ = crate::subprocess::git_query(primary, &["worktree", "remove", "--force", path]);
}

/// A letter with a tree, moved to this session. The displaced claim is
/// named, because a takeover of a live session's seat is the one thing
/// here that costs somebody something, and the user asked for it by
/// letter, not by session.
fn take_over_tree(
    primary: &str,
    path: &str,
    seat: &'static str,
    me: &Identity,
) -> Result<Assigned, String> {
    // What stood on the seat before this, to put back if the claim this
    // session writes will not go on.
    let displaced = lock_reason(path);
    let was = match standing(displaced.clone(), me, Held::BySession) {
        Standing::Ours => {
            return Ok(Assigned {
                seat,
                path: path.to_string(),
                note: format!(", and {}", carrying(path)),
                held: true,
            });
        }
        Standing::Free => "unclaimed".to_string(),
        Standing::Foreign(reason) | Standing::Stale(reason) => {
            // A person's lock is nobody's to take: it was written by
            // hand, and comes off by hand.
            if !is_a_seat_claim(&reason) {
                return Err(format!(
                    "seat {seat} is locked by hand ({reason}) — a person's lock is nobody's \
                     to take over; `git worktree unlock {path}` by hand first"
                ));
            }
            if !unlock_seat(primary, path) {
                return Err(format!(
                    "the claim on seat {seat} would not come off — `git worktree unlock \
                     {path}` by hand"
                ));
            }
            whose(&reason)
        }
    };
    match take_seat(primary, path, me, Held::BySession) {
        Standing::Ours => Ok(Assigned {
            seat,
            path: path.to_string(),
            note: format!(
                " (it was {was}); {} — nothing was moved to main's tip, and the board keeps \
                 this letter's pictures",
                carrying(path)
            ),
            held: false,
        }),
        Standing::Foreign(reason) | Standing::Stale(reason) => Err(format!(
            "seat {seat} was claimed by somebody else in the same moment ({}) — ask the user \
             again",
            whose(&reason)
        )),
        // The claim came off and this session's would not go on. Put
        // back what was displaced: a takeover that got no letter must
        // not leave the letter belonging to nobody.
        Standing::Free => Err(format!(
            "seat {seat} would not take this session's claim{} — `cargo xtask seats` reads \
             the roster",
            put_back(primary, path, displaced.as_deref())
        )),
    }
}

/// Writes a displaced claim back onto a seat, for a takeover that lifted
/// one and then could not put its own on. Answers what the reader needs
/// to know about the letter it leaves behind.
fn put_back(primary: &str, path: &str, displaced: Option<&str>) -> String {
    let Some(reason) = displaced.filter(|reason| is_a_seat_claim(reason)) else {
        return String::new();
    };
    match crate::subprocess::git_query(primary, &["worktree", "lock", "--reason", reason, path]) {
        Some(_) => format!(
            ", and the claim it displaced ({}) is back on it",
            whose(reason)
        ),
        None => format!(
            ", and the claim it displaced ({}) could not be put back either — the letter is \
             now nobody's",
            whose(reason)
        ),
    }
}

/// What a seat carries, said where it changes hands as it stands.
fn carrying(path: &str) -> String {
    match (commits_in(path, "main..HEAD"), dirty_lines(path)) {
        (Some(ahead), Some(dirty)) => format!(
            "it carries {ahead} commit(s) main does not have and {dirty} uncommitted file(s)"
        ),
        _ => "git could not say what it carries".to_string(),
    }
}

/// A letter with no tree, on the user's word: the tree comes back on the
/// letter's own branch when that branch still stands — with whatever
/// commits it carries, which is the point — and is made at main's tip
/// when nothing of the letter is left at all. Locked in the same step
/// that makes it, as a letter the roster creates is (`create_seat`).
/// `registered` is the block git still lists for the letter when its
/// directory went without the record going with it — whose claim that
/// record carries is named on the way out, since a tree that is gone is
/// still a letter somebody was working.
fn grow_back(
    primary: &str,
    seat: &'static str,
    me: &Identity,
    registered: Option<&WorktreeBlock>,
) -> Result<Assigned, String> {
    let branch = format!("worktree-{seat}");
    let path = format!("{primary}{WORKTREES}{seat}");
    let reason = me.reason();
    let was = match registered {
        Some(entry) if entry.locked => {
            // A person's lock is nobody's to take here either, and a
            // record is all that is left to read it from.
            if !is_a_seat_claim(&entry.reason) {
                return Err(format!(
                    "seat {seat} is locked by hand ({}) — a person's lock is nobody's to take \
                     over; `git worktree unlock {}` by hand first",
                    entry.reason, entry.path
                ));
            }
            clear_missing_registration(primary, &entry.path);
            format!(" (it was {})", whose(&entry.reason))
        }
        Some(entry) => {
            clear_missing_registration(primary, &entry.path);
            String::new()
        }
        None => String::new(),
    };
    let branch_stands = crate::subprocess::git_query(
        primary,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ],
    )
    .is_some();
    if !branch_stands {
        crate::subprocess::git_query(
            primary,
            &[
                "worktree", "add", "--lock", "--reason", &reason, "-B", &branch, &path, "main",
            ],
        )
        .ok_or_else(|| format!("git could not make a tree for {branch} at {path}"))?;
        return Ok(Assigned {
            seat,
            path,
            note: format!("{was}, newly created — the roster had not made this letter yet"),
            held: false,
        });
    }
    crate::subprocess::git_query(
        primary,
        &[
            "worktree", "add", "--lock", "--reason", &reason, &path, &branch,
        ],
    )
    .ok_or_else(|| format!("git could not put a tree for {branch} back at {path}"))?;
    let ahead = commits_in(&path, "main..HEAD").map_or("?".to_string(), |ahead| ahead.to_string());
    Ok(Assigned {
        seat,
        path,
        note: format!(
            "{was}, grown back on {branch} with its {ahead} commit(s) main does not have — \
             nothing was moved to main's tip, and the board keeps this letter's pictures"
        ),
        held: false,
    })
}

/// The seat this session holds, taking one if it holds none.
///
/// Every refusal on the way is a seat somebody else is in, so the loop
/// walks the whole roster. What it hands back is a tree ready to be
/// worked in and claimed: a letter it took from the roster is empty at
/// main's tip, and the tree the session is already standing in is
/// whatever that session left in it (`claim_where_it_stands`).
pub(crate) fn assign(cwd: &str, me: &Identity) -> Result<Assigned, String> {
    me.require_session()?;
    let (primary, trees) = primary_checkout(cwd)?;
    let entries = seat_entries_of(trees);
    let here = worktree_root(cwd);
    if let Some(held) = held_seat(&entries, me, here.as_deref()) {
        return Ok(held);
    }
    // The tree the session is standing in comes first, and on terms of
    // its own: a landed seat is empty at main's tip with its claim
    // handed back (`land`), and a session that goes on working there
    // wants that tree — its warm target/, and whatever it has put in it
    // since. A tree somebody else holds is passed over like any other;
    // the claim is still what decides.
    let standing_in = here.as_deref().and_then(|root| {
        entries
            .iter()
            .find(|entry| same_tree(&entry.tree.path, root))
    });
    if let Some(entry) = standing_in
        && let Some(taken) = claim_where_it_stands(&primary, &entry.tree.path, entry.seat, me)
    {
        return Ok(taken);
    }
    // A letter that refused the claim above is somebody else's, and the
    // walk would only ask it the same question again.
    let asked = standing_in.map(|entry| entry.seat);
    for name in spread_order()
        .into_iter()
        .filter(|name| Some(*name) != asked)
    {
        let taken = match entries.iter().find(|entry| entry.seat == name) {
            Some(entry) if has_a_directory(&entry.tree.path) => {
                claim_existing(&primary, &entry.tree.path, name, me)
            }
            Some(entry) => create_seat(&primary, name, me, Some(&entry.tree)),
            None => create_seat(&primary, name, me, None),
        };
        if let Some(taken) = taken {
            return Ok(taken);
        }
    }
    Err(format!(
        "every seat a-f is held or carries work — no seat is free to hand out. Stop and tell \
         the user what stands in the way: a letter changes hands only on the user's word \
         (`{}`), and the roster ends at f (CLAUDE.md ビルド・テスト)\n{}",
        commands::TAKEOVER.line(),
        survey::in_the_way(&primary)
    ))
}

/// Whether the claim the listing shows on a letter is this session's.
///
/// Read from the listing rather than from the tree, because a lock is
/// kept in the repository and `lock_reason` asks for it from inside the
/// worktree: a letter whose directory went missing would answer "no
/// claim" to its own session, which would then be handed a different
/// letter and could not even release the one it holds.
fn is_ours(block: &WorktreeBlock, me: &Identity) -> bool {
    let reason = block.locked.then(|| block.reason.clone());
    matches!(standing(reason, me, Held::BySession), Standing::Ours)
}

/// The seat this session already holds, if it holds one. Asking twice
/// gives the same answer. A session holding more than one — a takeover
/// on top of its own — is answered with the one it stands in, and
/// otherwise with the first.
fn held_seat(entries: &[SeatEntry], me: &Identity, here: Option<&str>) -> Option<Assigned> {
    let mine: Vec<&SeatEntry> = entries
        .iter()
        .filter(|entry| is_ours(&entry.tree, me))
        .collect();
    let entry = mine
        .iter()
        .find(|entry| here.is_some_and(|here| same_tree(&entry.tree.path, here)))
        .or_else(|| mine.first())?;
    Some(Assigned {
        seat: entry.seat,
        path: entry.tree.path.clone(),
        note: String::new(),
        held: true,
    })
}

/// Claims one seat that already exists. Everything is judged again after
/// the lock takes: what the listing said was a snapshot, and the state
/// that decides whether this seat can be worked is the state behind the
/// claim. Only an unclaimed seat is ever locked here (`claim::standing`
/// lifts no seat claim), so a seat that turns out to hold somebody's
/// work is handed back as it was found: unclaimed.
fn claim_existing(
    primary: &str,
    path: &str,
    seat: &'static str,
    me: &Identity,
) -> Option<Assigned> {
    if !matches!(
        take_seat(primary, path, me, Held::BySession),
        Standing::Ours
    ) {
        return None;
    }
    // Commits main does not have are a merge waiting to happen, and
    // uncommitted files are somebody's afternoon: neither is this
    // session's to start on top of.
    if !nothing_to_lose(path, seat) {
        unlock_seat(primary, path);
        return None;
    }
    Some(Assigned {
        seat,
        path: path.to_string(),
        note: start_at_main(path, seat),
        held: false,
    })
}

/// Claims the seat the session is standing in, whatever it carries.
///
/// Emptiness is what the roster asks of a letter before handing it to
/// somebody, and it is a question about *somebody else's* work. There is
/// nobody else here: what this tree holds is what this session was doing
/// in it, and a claim comes off a tree its session never left every time
/// that session lands or hands the letter back. So the work is left
/// exactly where it is, board and branch alike, and only an empty seat
/// at main's tip begins a fresh stretch.
///
/// This is the rule the other doors hold: a session standing in an
/// unclaimed tree claims it back by entering it
/// (`hook::seat::reclaims_on_entry`), by editing in it
/// (`hook::seat::reclaim`), by putting a picture on the board from it
/// (`held_by_this_run`) and by starting there (`hook::greeting`). Asking
/// the roster is the fifth door and the only one that answers before a
/// session has done anything, so a rule of its own here is a session
/// handed a different letter while its own work sits in one the roster
/// can then hand to nobody.
fn claim_where_it_stands(
    primary: &str,
    path: &str,
    seat: &'static str,
    me: &Identity,
) -> Option<Assigned> {
    if !matches!(
        take_seat(primary, path, me, Held::BySession),
        Standing::Ours
    ) {
        return None;
    }
    Some(Assigned {
        seat,
        path: path.to_string(),
        note: match nothing_to_lose(path, seat) {
            true => start_at_main(path, seat),
            false => format!(
                ", and {} — nothing was moved to main's tip, and the board keeps this \
                 letter's pictures",
                carrying(path)
            ),
        },
        held: false,
    })
}

/// Whether beginning a fresh stretch in this seat would throw nothing
/// away — the one question `start_at_main` may not be asked without.
///
/// Three things, not two. That the tree holds nothing is two of them:
/// HEAD at main's tip, and no uncommitted file. The third is the
/// letter's own branch, asked after separately because that is the name
/// `start_at_main` resets: a seat left detached at main's tip over a
/// branch that carries commits reads as clean by its tree alone, and
/// resetting the branch there is the work gone. A figure git would not
/// give is not a zero — what cannot be measured is left alone.
fn nothing_to_lose(path: &str, seat: &str) -> bool {
    commits_in(path, "main..HEAD") == Some(0)
        && dirty_lines(path) == Some(0)
        && commits_in(path, &format!("main..worktree-{seat}")) == Some(0)
}

/// Puts a claimed seat on its own letter's branch at main's tip, which is
/// where a stretch of work begins (CLAUDE.md ビルド・テスト). Safe only
/// because the caller proved the seat merged and clean behind its lock.
fn start_at_main(path: &str, seat: &str) -> String {
    // Whatever this letter still has on the shot board belongs to the
    // work that just ended here, and a stretch of work beginning is the
    // one moment the board can be sure of that (shots/sweep.rs). Said
    // out loud when there was anything to take: the session beginning
    // here is often the session that took those pictures.
    let swept = match crate::shots::seat_freed(seat) {
        Ok((gone, _)) if gone > 0 => format!(", and {gone} run(s) of this letter left the board"),
        _ => String::new(),
    };
    let branch = format!("worktree-{seat}");
    let on_branch = crate::subprocess::git_query(path, &["rev-parse", "--abbrev-ref", "HEAD"])
        .is_some_and(|head| head == branch);
    if on_branch && commits_in(path, "HEAD..main") == Some(0) {
        return swept;
    }
    match crate::subprocess::git_query(path, &["switch", "-C", &branch, "main"]) {
        Some(_) => format!("{swept}, started at main's tip on {branch}"),
        None => format!(
            "{swept}, but it could not be put at main's tip — run `git switch -C {branch} main` \
             there before working"
        ),
    }
}

/// Creates a letter the roster never made, locked in the same step that
/// makes it: `worktree add --lock` leaves no moment between the tree
/// existing and being claimed for a second session to arrive in.
/// `registered` is the block git still lists for the letter when its
/// directory went without the record going with it.
fn create_seat(
    primary: &str,
    seat: &'static str,
    me: &Identity,
    registered: Option<&WorktreeBlock>,
) -> Option<Assigned> {
    let branch = format!("worktree-{seat}");
    // A letter with no tree can still own a branch, left behind when its
    // worktree was removed. Reusing that name is only safe once main has
    // its commits; a branch still carrying work is skipped, and a
    // takeover is what grows its tree back (`grow_back`).
    if survey::stranded_work(primary, seat).is_some() {
        return None;
    }
    if let Some(registered) = registered {
        // What went missing is the tree, not the word that says whose the
        // letter is: a claim on a letter whose directory is gone is still
        // its session's, and clearing the record would lift it. The claim
        // is read from the listing, because a lock is read out of the
        // tree that is not there any more (`lock_reason`).
        let ours = matches!(
            standing(Some(registered.reason.clone()), me, Held::BySession),
            Standing::Ours
        );
        if registered.locked && !ours {
            return None;
        }
        clear_missing_registration(primary, &registered.path);
    }
    let path = format!("{primary}{WORKTREES}{seat}");
    crate::subprocess::git_query(
        primary,
        &[
            "worktree",
            "add",
            "--lock",
            "--reason",
            &me.reason(),
            "-B",
            &branch,
            &path,
            "main",
        ],
    )?;
    // A letter whose tree was removed can still have pictures standing
    // under its name, and the work they argued for went with the tree
    // (shots/sweep.rs).
    crate::shots::seat_reused(seat);
    Some(Assigned {
        seat,
        path,
        note: ", newly created — the roster had not made this letter yet".to_string(),
        held: false,
    })
}

/// The letters to try, in an order that differs run to run. The lock is
/// what actually decides, so this only keeps sessions started in one
/// burst from queueing on one letter and paying a refusal each.
fn spread_order() -> [&'static str; SEATS.len()] {
    let offset = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.subsec_nanos() as usize);
    std::array::from_fn(|index| SEATS[(index + offset) % SEATS.len()])
}

/// Uncommitted changes in `dir`, untracked files included. --no-optional-
/// locks because a plain status opportunistically rewrites the index it
/// refreshed, and the survey reports index mtimes.
pub(crate) fn dirty_lines(dir: &str) -> Option<usize> {
    crate::subprocess::git_query(dir, &["--no-optional-locks", "status", "--porcelain"])
        .map(|status| status.lines().filter(|line| !line.is_empty()).count())
}

/// A run being put on the board from a roster seat holds the seat for
/// the session that took it, as an edit does (hook/seat.rs `reclaim`).
///
/// The seat a session lands in is handed back (`land::release_claim`),
/// and the session often goes on there — a picture of what landed is the
/// usual next thing — while a letter nobody holds is one `seat` hands to
/// the next session, whose fresh stretch sweeps this letter's runs off
/// the board (`start_at_main`). So the run takes the claim back first.
/// Quiet when the claim is already this session's, and only a word when
/// it is somebody else's: the run is then a picture of their tree, which
/// is what a landing's gate takes when it lands their branch. A run with
/// no session behind it — CI's — claims nothing.
pub(crate) fn held_by_this_run(seat: &str) -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    let root = worktree_root(&slashed(&cwd))?;
    let me = Identity::current(None);
    if me.session.trim().is_empty() {
        return None;
    }
    // The tree is here to be asked, this run being taken in it.
    let held = matches!(
        standing(lock_reason(&root), &me, Held::BySession),
        Standing::Ours
    );
    match take_seat(&root, &root, &me, Held::BySession) {
        Standing::Ours if held => None,
        Standing::Ours => Some(format!(
            "board: seat {seat} stood unclaimed, and this run claimed it back for the session"
        )),
        Standing::Foreign(reason) | Standing::Stale(reason) => Some(format!(
            "board: seat {seat} is {} — this run pictures a tree that is not this session's",
            whose(&reason)
        )),
        Standing::Free => None,
    }
}

/// How many commits `git rev-list --count` sees in `range`, run in `dir`.
pub(crate) fn commits_in(dir: &str, range: &str) -> Option<u32> {
    crate::subprocess::git_query(dir, &["rev-list", "--count", range])
        .and_then(|count| count.parse().ok())
}

/// One roster seat as `git worktree list --porcelain` shows it: the
/// listing's own block, and the roster letter its path names.
#[derive(Debug, PartialEq)]
pub(crate) struct SeatEntry {
    pub seat: &'static str,
    pub tree: WorktreeBlock,
}

/// One tree of `git worktree list --porcelain`, as the listing gives it.
///
/// Nothing is filtered and the order is the listing's, because the first
/// entry is the primary checkout and `land` turns on that.
#[derive(Debug, PartialEq)]
pub(crate) struct WorktreeBlock {
    /// The worktree's path, slashes forward, for `git -C`.
    pub path: String,
    /// Branch name, empty when HEAD is detached.
    pub branch: String,
    pub locked: bool,
    /// The lock's reason, empty when unlocked or given none.
    pub reason: String,
}

/// Every tree the listing shows, in listing order.
pub(crate) fn worktree_blocks(listing: &str) -> Vec<WorktreeBlock> {
    let mut blocks = Vec::new();
    for block in listing.split("\n\n") {
        let mut path = None;
        let mut branch = String::new();
        let mut locked = false;
        let mut reason = String::new();
        for line in block.lines() {
            if let Some(rest) = line.strip_prefix("worktree ") {
                path = Some(rest.replace('\\', "/"));
            } else if let Some(rest) = line.strip_prefix("branch refs/heads/") {
                branch = rest.to_string();
            } else if let Some(rest) = line.strip_prefix("locked") {
                locked = true;
                reason = rest.trim().to_string();
            }
        }
        if let Some(path) = path {
            blocks.push(WorktreeBlock {
                path,
                branch,
                locked,
                reason,
            });
        }
    }
    blocks
}

/// Every roster seat the listing shows, in listing order.
pub(crate) fn seat_entries(listing: &str) -> Vec<SeatEntry> {
    seat_entries_of(worktree_blocks(listing))
}

fn seat_entries_of(trees: Vec<WorktreeBlock>) -> Vec<SeatEntry> {
    trees
        .into_iter()
        .filter_map(|block| {
            let seat = roster_letter(&block.path)?;
            Some(SeatEntry { seat, tree: block })
        })
        .collect()
}

/// The roster letter `path` points into, if it is a seat's tree at all —
/// the letters being the directory names under the worktree directory,
/// and a path inside a seat being that seat's as much as its root is.
pub(crate) fn roster_letter(path: &str) -> Option<&'static str> {
    let root = worktree_root(path)?;
    let name = root.rsplit('/').next()?;
    SEATS.iter().find(|seat| **seat == name).copied()
}

/// Where a repository keeps its seats. A repository with no such
/// directory has no roster, whatever its branches are called.
pub(crate) fn roster_dir(primary: &str) -> String {
    format!("{}{WORKTREES}", primary.trim_end_matches('/'))
}

/// The same, with the repository whose roster that letter belongs to —
/// for a reader that has to tell this checkout's seats from the same
/// layout somewhere else (hook/git.rs).
pub(crate) fn seat_in_repository(path: &str) -> Option<(String, &'static str)> {
    let letter = roster_letter(path)?;
    let root = worktree_root(path)?;
    let repository = root.strip_suffix(&format!("{WORKTREES}{letter}"))?;
    Some((repository.to_string(), letter))
}

/// The worktree `cwd` sits in: the path down to the directory named under
/// .claude/worktrees/, and None for the primary checkout.
pub(crate) fn worktree_root(cwd: &str) -> Option<String> {
    let cwd = cwd.replace('\\', "/");
    let at = cwd.find(WORKTREES)? + WORKTREES.len();
    if at >= cwd.len() {
        return None;
    }
    let end = cwd[at..].find('/').map_or(cwd.len(), |slash| at + slash);
    Some(cwd[..end].to_string())
}

/// The measurement rig: the one tree under the roster's directory that is
/// no seat. `cargo xtask perf --at <rev>` puts the commit it was asked for
/// on the rig, builds it there and measures that build, so the seat that
/// asked keeps its own target/ and its uncommitted work, and the build
/// being measured is a commit anybody can name again (`perf::rig`).
/// Nobody sits in it and nothing is edited there: the entry and write
/// hooks refuse it, and a dirty rig refuses every measurement until it
/// is cleaned by hand (hook/seat.rs).
pub(crate) const RIG: &str = "rig";

/// Where the rig stands, written against the primary checkout the way
/// every seat path is.
pub(crate) fn rig_path(primary: &str) -> String {
    format!("{}{WORKTREES}{RIG}", primary.trim_end_matches('/'))
}

/// Whether `path` is the rig's tree or something inside it.
pub(crate) fn in_rig(path: &str) -> bool {
    worktree_root(path).is_some_and(|root| root.rsplit('/').next() == Some(RIG))
}

/// Whether two paths name one tree. The filesystem answers it, because
/// the two are spelled by different mouths: git writes forward slashes
/// and the long name, while a process is handed the spelling it was
/// started with — an 8.3 short name, a junction, either case. A path the
/// filesystem cannot resolve falls back to its text, with the separators
/// levelled and, on Windows, the case.
pub(crate) fn same_tree(left: &str, right: &str) -> bool {
    if let (Ok(left), Ok(right)) = (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
        return left == right;
    }
    let trim = |path: &str| path.replace('\\', "/").trim_end_matches('/').to_string();
    let (left, right) = (trim(left), trim(right));
    if cfg!(windows) {
        left.eq_ignore_ascii_case(&right)
    } else {
        left == right
    }
}

/// A path as the listing spells it: slashes forward, for `git -C` and
/// for comparing against what git said.
pub(crate) fn slashed(path: &std::path::Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// The primary checkout — the listing's first entry, where seats are
/// created and the tree every seat path is written against — and the
/// listing it came from.
pub(crate) fn primary_checkout(cwd: &str) -> Result<(String, Vec<WorktreeBlock>), String> {
    let listing = crate::subprocess::git_query(cwd, &["worktree", "list", "--porcelain"])
        .ok_or("git worktree list failed — is git on PATH and this a repository?")?;
    let trees = worktree_blocks(&listing);
    let primary = trees
        .first()
        .ok_or("git worktree list named no tree at all")?
        .path
        .clone();
    Ok((primary, trees))
}

#[cfg(test)]
mod tests {
    use super::{SeatEntry, WorktreeBlock, roster_letter, seat_entries};

    #[test]
    fn knows_a_seat_path_from_the_rest() {
        assert_eq!(
            roster_letter("C:\\x\\platitude-gg\\.claude\\worktrees\\a"),
            Some("a")
        );
        assert_eq!(
            roster_letter("C:/x/platitude-gg/.claude/worktrees/b/crates"),
            Some("b")
        );
        assert_eq!(
            roster_letter("C:/x/platitude-gg/.claude/worktrees/tooltip"),
            None
        );
        // The rig sits beside the seats and is none of them.
        assert_eq!(
            roster_letter("C:/x/platitude-gg/.claude/worktrees/rig"),
            None
        );
        assert_eq!(roster_letter("C:/x/platitude-gg"), None);
        // Why a path goes through `named_tree` before it reaches this:
        // the raw string a tool was given can spell a seat relatively,
        // and that reads as no seat here.
        assert_eq!(roster_letter(".claude/worktrees/e"), None);
    }

    /// The rig is the one tree under the roster's directory the roster
    /// never hands out, and a name that merely starts with its is not it.
    #[test]
    fn knows_the_rig_from_the_seats() {
        assert!(super::in_rig("C:/x/platitude-gg/.claude/worktrees/rig"));
        assert!(super::in_rig(
            "C:\\x\\platitude-gg\\.claude\\worktrees\\rig\\crates\\xtask"
        ));
        assert!(!super::in_rig(
            "C:/x/platitude-gg/.claude/worktrees/rigging"
        ));
        assert!(!super::in_rig("C:/x/platitude-gg/.claude/worktrees/a"));
        assert!(!super::in_rig("C:/x/platitude-gg"));
        assert_eq!(
            super::rig_path("C:/x/platitude-gg/"),
            "C:/x/platitude-gg/.claude/worktrees/rig"
        );
        assert!(!super::SEATS.contains(&super::RIG));
        let listing = format!(
            "worktree C:/x/platitude-gg\nHEAD 1111\nbranch refs/heads/main\n\n\
             worktree {}\nHEAD 2222\ndetached\n",
            super::rig_path("C:/x/platitude-gg")
        );
        assert!(seat_entries(&listing).is_empty());
    }

    #[test]
    fn one_tree_spelled_two_ways_is_one_tree() {
        assert!(super::same_tree("C:/x/rig/", "C:\\x\\rig"));
        assert_eq!(super::same_tree("C:/x/RIG", "C:/x/rig"), cfg!(windows));
        assert!(!super::same_tree("C:/x/rig", "C:/x/rigging"));
        assert_eq!(super::slashed(std::path::Path::new("C:\\x\\y")), "C:/x/y");
    }

    #[test]
    fn reads_seats_out_of_a_worktree_listing() {
        let listing = "worktree C:/x/platitude-gg\nHEAD 1111\nbranch refs/heads/main\n\n\
                       worktree C:/x/platitude-gg/.claude/worktrees/a\nHEAD 2222\nbranch refs/heads/worktree-a\n\n\
                       worktree C:/x/platitude-gg/.claude/worktrees/tooltip\nHEAD 3333\nbranch refs/heads/worktree-tooltip\n\n\
                       worktree C:/x/platitude-gg/.claude/worktrees/b\nHEAD 4444\nbranch refs/heads/worktree-b\nlocked claude session b (pid 1)\n";
        let seats = seat_entries(listing);
        assert_eq!(
            seats,
            vec![
                SeatEntry {
                    seat: "a",
                    tree: WorktreeBlock {
                        path: "C:/x/platitude-gg/.claude/worktrees/a".to_string(),
                        branch: "worktree-a".to_string(),
                        locked: false,
                        reason: String::new(),
                    },
                },
                SeatEntry {
                    seat: "b",
                    tree: WorktreeBlock {
                        path: "C:/x/platitude-gg/.claude/worktrees/b".to_string(),
                        branch: "worktree-b".to_string(),
                        locked: true,
                        reason: "claude session b (pid 1)".to_string(),
                    },
                },
            ]
        );
    }

    /// Every refusal that meets a claim tells the session the one thing
    /// it can do about it, and the spelling it tells is the catalogue's.
    #[test]
    fn a_claim_moves_only_the_three_ways_the_refusal_names() {
        let text = super::how_claims_move();
        assert!(text.contains("cargo xtask seat release"), "{text}");
        assert!(
            text.contains("PGG_ALLOW_TAKEOVER=1 cargo xtask seat takeover <letter>"),
            "{text}"
        );
        assert!(text.contains("no process is asked"), "{text}");
    }
}
