//! Seat assignment, release, and takeover.

use super::worktrees::{WORKTREES, commits_in, dirty_lines, has_a_directory, seat_entries_of};
use super::{
    Held, Identity, SEATS, SeatEntry, Standing, WorktreeBlock, claim, commands, entry,
    is_a_seat_claim, lock_reason, offer, primary_checkout, same_tree, seat_in_repository, slashed,
    standing, survey, take_seat, transcripts, unlock_seat, whose, worktree_root,
};

/// How a seat claim moves, for every refusal that meets one — read by a
/// session that can do nothing about it but tell the user.
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
    /// What the session is told: this path is the one EnterWorktree
    /// argument that will be let through, the claim behind it being this
    /// session's. A seat it held already is said to be kept: a session
    /// that comes back to its seat after a resume reads a second ask as a
    /// letter it lost, and tells the user it took the seat again.
    pub(crate) fn report(&self, me: &Identity) -> String {
        let standing = if self.held {
            "was already this session's"
        } else {
            "is this session's now"
        };
        let entry = if !has_a_directory(&self.path) {
            treeless(self.seat, &self.path)
        } else if self.already_in(me) {
            "this session is already working in it — nothing to enter".to_string()
        } else {
            format!("enter it with EnterWorktree path={}", self.path)
        };
        let kept = match self.held {
            true => format!(
                "\nits claim never came off, so nothing was taken again: name it to the user \
                 as the seat the work goes on in (「席 {} で続ける」), never as released or \
                 taken back",
                self.seat
            ),
            false => String::new(),
        };
        format!("seat {} {standing}{}\n{entry}{kept}", self.seat, self.note)
    }

    /// Whether the session already works in this seat (EnterWorktree would
    /// refuse the tree it is run from). The recorded entry settles it
    /// (`entry::entered`); the shell is the weaker witness, since
    /// `cd <checkout> && cargo xtask seat` stands in the checkout.
    fn already_in(&self, me: &Identity) -> bool {
        let entered = seat_in_repository(&self.path).is_some_and(|(root, seat)| {
            entry::entered(std::path::Path::new(&root), &me.session, seat)
        });
        entered || inside(&self.path)
    }
}

/// Whether this shell already stands in `path` — for a session whose
/// entry was never recorded (entered by a path the hook did not see).
fn inside(path: &str) -> bool {
    let Ok(at) = std::env::current_dir() else {
        return false;
    };
    let here = at.to_string_lossy().replace('\\', "/");
    let (here, path) = if cfg!(windows) {
        (here.to_ascii_lowercase(), path.to_ascii_lowercase())
    } else {
        (here, path.to_string())
    };
    here == path || here.starts_with(&format!("{path}/"))
}

/// `cargo xtask seat`: the roster hands this session a seat. It takes no
/// letter: one picked from a survey is a snapshot two sessions can both
/// be wrong about, so the choosing happens here, behind the lock.
pub fn take(args: &[String]) -> Result<(), String> {
    let (root, args) = rooted(args)?;
    let me = Identity::current(None);
    match args {
        [] => {
            println!("{}", assign(&root, &me)?.report(&me));
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

/// Where this call is made from: `--dir <path>` (the suite's sandboxes),
/// else the working directory. That says which roster is asked and where
/// the session stands (`assign`) — not the tree this binary was built in
/// (`tree::workspace_root`), which differs once `cargo xtask` runs from
/// outside a seat.
pub(super) fn rooted(args: &[String]) -> Result<(String, &[String]), String> {
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

/// `cargo xtask seat release [<letter>]`: hands a seat back without
/// landing it (a landing already does, `land::release_claim`). SessionEnd
/// lifts nothing: the machine's sleep sends it to every open conversation.
/// What the seat still carries is named on the way out — the roster hands
/// it to nobody else, and asking from that tree takes it back
/// (`claim_where_it_stands`). The letter is needed only when the session
/// holds more than one and stands in neither.
pub(super) fn release(root: &str, me: &Identity, letter: Option<&str>) -> Result<(), String> {
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
    // A seat handed back lands nothing: its place in the line of landings
    // ends with the claim.
    crate::land::line::leave(&primary, &me.session, "the seat was released");
    println!(
        "seat {} released{}",
        held.seat,
        left_behind(&held.tree.path)
    );
    Ok(())
}

/// What a released seat still carries. A figure git could not give is
/// reported as unknown, never as a silent zero.
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
/// whoever holds it, on the user's word (the pre-shell hook admits the
/// verb only behind its escape flag, hook/seat.rs). Nothing about the
/// letter is judged, because the user already has: the session inherits
/// it as it stands, nothing is put at main's tip and the board is not
/// swept — the work goes on (`start_at_main` is the other case). A letter
/// whose tree went away grows one back on its own branch; a letter never
/// made is made.
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
    println!("{}", taken.report(me));
    Ok(())
}

/// What a session holding a treeless letter is told in place of a path
/// to enter: the entry door reads the claim from the tree and finds none
/// there, so being sent in sends the session round to the roster again.
pub(crate) fn treeless(seat: &str, path: &str) -> String {
    format!(
        "its tree is gone ({path}) — the work stays on worktree-{seat}, and only `{}` grows \
         the tree back, on the user's word: tell the user",
        commands::TAKEOVER.line().replace("<letter>", seat)
    )
}

/// Clears git's record of a tree whose directory is gone, so a tree can
/// be made at that path again. Unlocked first: git keeps a locked tree's
/// record however long it is missing.
fn clear_missing_registration(primary: &str, path: &str) {
    unlock_seat(primary, path);
    let _ = crate::subprocess::git_query(primary, &["worktree", "remove", "--force", path]);
}

/// A letter with a tree, moved to this session. The displaced claim is
/// named: the user asked by letter, not by session, and it costs that
/// session its seat.
fn take_over_tree(
    primary: &str,
    path: &str,
    seat: &'static str,
    me: &Identity,
) -> Result<Assigned, String> {
    // Put back if this session's claim will not go on.
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
            // A person's lock comes off by hand only.
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
        // A takeover that got no letter must not leave it nobody's: put
        // back what was displaced.
        Standing::Free => Err(format!(
            "seat {seat} would not take this session's claim{} — `cargo xtask seats` reads \
             the roster",
            put_back(primary, path, displaced.as_deref())
        )),
    }
}

/// Writes a displaced claim back onto a seat, for a takeover that lifted
/// one and could not put its own on; answers how the letter is left.
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

/// What a seat carries, said wherever a seat is handed over or named to
/// its session as it stands.
pub(crate) fn carrying(path: &str) -> String {
    match (commits_in(path, "main..HEAD"), dirty_lines(path)) {
        (Some(ahead), Some(dirty)) => format!(
            "it carries {ahead} commit(s) main does not have and {dirty} uncommitted file(s)"
        ),
        _ => "git could not say what it carries".to_string(),
    }
}

/// A letter with no tree, on the user's word: the tree comes back on the
/// letter's own branch with its commits, or at main's tip when no branch
/// is left — locked in the same step, as in `create_seat`. `registered`
/// is the block git still lists for a directory that went; whose claim it
/// carries is named on the way out.
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
            // A person's lock stays here too, read from the record.
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

/// The seat this session holds, taking one if it holds none: a letter
/// from the roster comes empty at main's tip, and the tree the session
/// stands in comes as it left it (`claim_where_it_stands`). Asked from
/// the checkout, the letter its transcript last put it in is asked for
/// first.
pub(crate) fn assign(cwd: &str, me: &Identity) -> Result<Assigned, String> {
    me.require_session()?;
    let (primary, trees) = primary_checkout(cwd)?;
    let entries = seat_entries_of(trees);
    let here = worktree_root(cwd);
    if let Some(held) = held_seat(&entries, me, here.as_deref()) {
        return Ok(held);
    }
    // The tree the session stands in comes first, on its own terms: a
    // session going on after its landing wants its warm target/ and its
    // work. Somebody else's claim there still decides.
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
    // A letter that refused the claim above is somebody else's.
    let asked = standing_in.map(|entry| entry.seat);
    let sat_in = here
        .is_none()
        .then(|| transcript_seat(&primary, me))
        .flatten();
    for name in offer::offer_order(&primary, &entries, sat_in)
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

/// The letter of this repository `me`'s conversation last stood in, by
/// its transcript: `cd <checkout> && cargo xtask seat` runs in the
/// checkout while the session sits on in its seat after a landing. Only
/// asked for first, as any letter is (`claim_existing`): a tree carrying
/// work under no claim is not this session's, whose edits claim it back.
fn transcript_seat(primary: &str, me: &Identity) -> Option<&'static str> {
    let cwd = transcripts::stand_of(&me.session)?.cwd.replace('\\', "/");
    let (repository, letter) = seat_in_repository(&cwd)?;
    same_tree(&repository, primary).then_some(letter)
}

/// The seat this session's conversation holds, read and left as it is —
/// for the greeting, whose `cwd` is where the session began: a resume
/// names the starting directory while the session goes on in its seat.
/// Of several, the one its transcript last stood in, else the first: the
/// greeting cannot ask the directory where the session works.
pub(crate) fn conversation_seat(cwd: &str, me: &Identity) -> Option<(&'static str, String)> {
    let (primary, trees) = primary_checkout(cwd).ok()?;
    let mine: Vec<SeatEntry> = seat_entries_of(trees)
        .into_iter()
        .filter(|entry| is_ours(&entry.tree, me))
        .collect();
    let entry = match mine.as_slice() {
        [] => return None,
        [one] => one,
        several => {
            let stood = transcript_seat(&primary, me);
            several
                .iter()
                .find(|entry| Some(entry.seat) == stood)
                .unwrap_or(&several[0])
        }
    };
    Some((entry.seat, entry.tree.path.clone()))
}

/// Whether the claim the listing shows on a letter is this session's.
/// From the listing, not `lock_reason` (which asks from inside the
/// worktree): a letter whose directory went missing would answer "no
/// claim" to its own session, which could then not even release it.
fn is_ours(block: &WorktreeBlock, me: &Identity) -> bool {
    let reason = block.locked.then(|| block.reason.clone());
    matches!(standing(reason, me, Held::BySession), Standing::Ours)
}

/// The seat this session already holds: of several (a takeover on top of
/// its own), the one it stands in, else the first. A claim its app
/// session wrote under an earlier session id is this session's too
/// (`claim::Identity::host`), renamed for it here and said so, with what
/// the tree carries: the session that remembers no claim learns where its
/// work is — or, after a `/clear`, that the tree is not empty.
fn held_seat(entries: &[SeatEntry], me: &Identity, here: Option<&str>) -> Option<Assigned> {
    let mine: Vec<&SeatEntry> = entries
        .iter()
        .filter(|entry| is_ours(&entry.tree, me))
        .collect();
    let entry = mine
        .iter()
        .find(|entry| here.is_some_and(|here| same_tree(&entry.tree.path, here)))
        .or_else(|| mine.first())?;
    let path = &entry.tree.path;
    if has_a_directory(path) {
        // Renamed for this session where the claim names an earlier id, or
        // no host (`claim::take_seat`).
        take_seat(path, path, me, Held::BySession);
    }
    let note = match claim::carried_over(&entry.tree.reason, me) {
        true => format!(
            ", claimed as {} — an earlier session id of this app session (a prompt edited \
             after an interrupt, or a /clear); {}",
            whose(&entry.tree.reason).replacen("held by ", "", 1),
            carrying(path)
        ),
        false => String::new(),
    };
    Some(Assigned {
        seat: entry.seat,
        path: path.clone(),
        note,
        held: true,
    })
}

/// Claims one existing seat, judged again behind the lock (the listing
/// was a snapshot). Only an unclaimed seat is ever locked here
/// (`claim::standing` lifts no seat claim), so one found holding work is
/// handed back unclaimed.
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

/// Claims the seat the session is standing in, whatever it carries: the
/// roster's emptiness test guards somebody else's work, and this tree's
/// is the session's own (its claim comes off at every land and release).
/// The work stays as it is; only an empty seat starts at main's tip.
///
/// The other doors hold the same rule — entering
/// (`hook::seat::reclaims_on_entry`), editing (`hook::seat::reclaim`),
/// starting there (`hook::greeting`). A picture is no door
/// (`may_picture`). A rule of its own here would hand the session a
/// different letter while its work sits in one the roster can hand to
/// nobody.
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

/// Whether beginning a fresh stretch here throws nothing away — asked
/// before every `start_at_main`. The letter's own branch is asked apart
/// from HEAD because it is the name `start_at_main` resets: a seat
/// detached at main's tip over a branch with commits reads clean by its
/// tree alone. A figure git would not give is not a zero.
fn nothing_to_lose(path: &str, seat: &str) -> bool {
    commits_in(path, "main..HEAD") == Some(0)
        && dirty_lines(path) == Some(0)
        && commits_in(path, &format!("main..worktree-{seat}")) == Some(0)
}

/// Puts a claimed seat on its own letter's branch at main's tip. Safe
/// only because the caller proved the seat merged and clean behind its
/// lock (`nothing_to_lose`).
fn start_at_main(path: &str, seat: &str) -> String {
    // This letter's runs on the shot board belong to the work that just
    // ended here (shots/sweep.rs). Said aloud: the session beginning here
    // often took them.
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

/// Creates a letter the roster never made, locked in the same step:
/// `worktree add --lock` leaves no moment between the tree existing and
/// being claimed for a second session to arrive in. `registered` is as in
/// `grow_back`.
fn create_seat(
    primary: &str,
    seat: &'static str,
    me: &Identity,
    registered: Option<&WorktreeBlock>,
) -> Option<Assigned> {
    let branch = format!("worktree-{seat}");
    // `-B` below resets a branch a removed tree left behind: one still
    // carrying work is skipped, and a takeover grows it back (`grow_back`).
    if survey::stranded_work(primary, seat).is_some() {
        return None;
    }
    if let Some(registered) = registered {
        // A claim on a letter whose directory is gone is still its
        // session's, and clearing the record would lift it. Read from the
        // listing: `lock_reason` reads the tree that is gone.
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
    // Pictures still under this letter's name argued for work that went
    // with the removed tree (shots/sweep.rs).
    crate::shots::seat_reused(seat);
    Some(Assigned {
        seat,
        path,
        note: ", newly created — the roster had not made this letter yet".to_string(),
        held: false,
    })
}

#[cfg(test)]
mod tests {
    /// The spelling a refusal tells is the catalogue's.
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
