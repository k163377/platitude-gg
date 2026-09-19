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

use std::time::SystemTime;

mod claim;
mod commands;
mod survey;

pub(crate) use claim::{
    Held, Identity, SEAT_CLAIM, Standing, claim_is_dead, claim_liveness, lock_reason, standing,
    take_seat, unlock_seat,
};
pub(crate) use commands::COMMANDS;
pub(crate) use survey::{Seat, format_age, run, survey};
// Built only by the greeting's tests: the survey is the one thing
// that measures a seat, and everything else reads what it measured.
#[cfg(test)]
pub(crate) use survey::SeatState;

/// The reusable worktree seats. Sessions rotate through these six —
/// a topical worktree is never reused, so
/// every one paid a cold target/ build and kept the gigabytes afterwards
/// (CLAUDE.md ビルド・テスト).
pub(crate) const SEATS: [&str; 6] = ["a", "b", "c", "d", "e", "f"];

/// The directory every worktree of this repository sits under.
const WORKTREES: &str = "/.claude/worktrees/";

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
    let root = crate::tree::workspace_root().to_string_lossy().to_string();
    let me = Identity::current(None);
    match args.first().map(String::as_str) {
        None => {
            println!("{}", assign(&root, &me)?.report());
            Ok(())
        }
        Some("release") if args.len() == 1 => release(&root, &me),
        _ => Err(format!(
            "seat takes no arguments, or `release` to hand this session's seat \
             back (got {args:?})"
        )),
    }
}

/// `cargo xtask seat release`: the seat this session holds goes back to
/// the roster.
///
/// For handing a seat back without landing it — work abandoned, or a
/// stretch that ends in a branch somebody else will merge. A landing
/// hands its own seat back already (`land::release_claim`), and the
/// SessionEnd the machine's sleep hands out lifts nothing, because that
/// event reaches every open conversation (CLAUDE.md ビルド・テスト).
/// What the seat still carries is named on the way out: the roster hands
/// out no seat with work in it, so a release leaves that work for a
/// reader to land or drop.
fn release(root: &str, me: &Identity) -> Result<(), String> {
    me.require_session()?;
    let (primary, trees) = primary_checkout(root)?;
    let Some(held) = held_seat(&seat_entries_of(trees), me) else {
        return Err("this session holds no seat, so there is none to release".into());
    };
    if !unlock_seat(&primary, &held.path) {
        return Err(format!(
            "the claim on seat {} did not release — `git worktree unlock {}` by hand",
            held.seat, held.path
        ));
    }
    println!("seat {} released{}", held.seat, left_behind(&held.path));
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
         uncommitted file(s), so the roster will not hand it to anybody until \
         those land or go"
    )
}

/// The seat this session holds, taking one if it holds none.
///
/// Every refusal on the way is a seat somebody else is in, so the loop
/// walks the whole roster. What it hands
/// back is a tree that is ready to be worked in: claimed, empty, and at
/// main's tip.
pub(crate) fn assign(cwd: &str, me: &Identity) -> Result<Assigned, String> {
    me.require_session()?;
    let (primary, trees) = primary_checkout(cwd)?;
    let entries = seat_entries_of(trees);
    if let Some(held) = held_seat(&entries, me) {
        return Ok(held);
    }
    // The tree the session is standing in comes first when it is free to
    // take: a landed seat is empty at main's tip with its claim handed
    // back (`land`), and a session that goes on working there wants that
    // tree — its warm target/. A tree somebody else holds is passed over
    // like any other; the claim below is still what decides, and the
    // listing is only where the letters come from.
    let standing_in = worktree_root(cwd).and_then(|root| {
        entries
            .iter()
            .find(|entry| same_tree(&entry.tree.path, &root))
    });
    if let Some(entry) = standing_in
        && let Some(taken) = claim_existing(&primary, &entry.tree.path, entry.seat, me)
    {
        return Ok(taken);
    }
    // A tree that refused the claim above (ahead, or dirty) is skipped
    // on its turn: the answer would be the same, at the cost of a
    // second lock and unlock on it.
    let asked = standing_in.map(|entry| entry.seat);
    for name in spread_order()
        .into_iter()
        .filter(|name| Some(*name) != asked)
    {
        let taken = match entries.iter().find(|entry| entry.seat == name) {
            Some(entry) => claim_existing(&primary, &entry.tree.path, name, me),
            None => create_seat(&primary, name, me),
        };
        if let Some(taken) = taken {
            return Ok(taken);
        }
    }
    Err(
        "every seat a-f is held, carrying unmerged commits, or holding \
         uncommitted work — no seat is free to hand out. Tell the user; \
         the roster ends at f (CLAUDE.md ビルド・テスト)"
            .into(),
    )
}

/// The seat this session already holds, if it holds one. A session works
/// one seat at a time, and asking twice gives the same
/// answer.
fn held_seat(entries: &[SeatEntry], me: &Identity) -> Option<Assigned> {
    entries
        .iter()
        .find(|entry| {
            matches!(
                standing(lock_reason(&entry.tree.path), me, Held::BySession),
                Standing::Ours
            )
        })
        .map(|entry| Assigned {
            seat: entry.seat,
            path: entry.tree.path.clone(),
            note: String::new(),
            held: true,
        })
}

/// Claims one seat that already exists. Everything is judged again after
/// the lock takes: what the listing said was a snapshot, and the state
/// that decides whether this seat can be worked is the state behind the
/// claim. A seat that turns out to hold somebody's work is handed back.
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
    let ahead = commits_in(path, "main..HEAD");
    let dirty = dirty_lines(path);
    // Commits main does not have are a merge waiting to happen, and
    // uncommitted files are somebody's afternoon: neither is this
    // session's to start on top of.
    if ahead != Some(0) || dirty != Some(0) {
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

/// Puts a claimed seat on its own letter's branch at main's tip, which is
/// where a stretch of work begins (CLAUDE.md ビルド・テスト). Safe only
/// because the caller proved the seat merged and clean behind its lock.
fn start_at_main(path: &str, seat: &str) -> String {
    // Whatever this letter still has on the shot board belongs to the
    // work that just ended here, and a stretch of work beginning is the
    // one moment the board can be sure of that (shots/sweep.rs).
    crate::shots::seat_reused(seat);
    let branch = format!("worktree-{seat}");
    let on_branch = crate::subprocess::git_query(path, &["rev-parse", "--abbrev-ref", "HEAD"])
        .is_some_and(|head| head == branch);
    if on_branch && commits_in(path, "HEAD..main") == Some(0) {
        return String::new();
    }
    match crate::subprocess::git_query(path, &["switch", "-C", &branch, "main"]) {
        Some(_) => format!(", started at main's tip on {branch}"),
        None => format!(
            ", but it could not be put at main's tip — run `git switch -C {branch} main` there \
             before working"
        ),
    }
}

/// Creates a letter the roster never made, locked in the same step that
/// makes it: `worktree add --lock` leaves no moment between the tree
/// existing and being claimed for a second session to arrive in.
fn create_seat(primary: &str, seat: &'static str, me: &Identity) -> Option<Assigned> {
    let branch = format!("worktree-{seat}");
    // A letter with no tree can still own a branch, left behind when its
    // worktree was removed. Reusing that name is only safe once main has
    // its commits; a branch still carrying work is skipped.
    if commits_in(primary, &format!("main..{branch}")).is_some_and(|ahead| ahead > 0) {
        return None;
    }
    let path = format!("{primary}{WORKTREES}{seat}");
    crate::subprocess::git_query(
        primary,
        &[
            "worktree",
            "add",
            "--lock",
            "--reason",
            // Only a session ever creates a letter; the rig has its own.
            &me.reason(Held::BySession),
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
            let seat = worktree_root(&block.path).and_then(|root| {
                let name = root.rsplit('/').next()?.to_string();
                SEATS.iter().find(|seat| **seat == name).copied()
            })?;
            Some(SeatEntry { seat, tree: block })
        })
        .collect()
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
    use super::{SeatEntry, WorktreeBlock, seat_entries};

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
}
