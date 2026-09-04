//! The worktree seat roster: the survey (`cargo xtask seats`) the
//! session-start greeting shares, and the claim that says whose a seat is.
//!
//! The greeting reports where the seats stood when the session began, and
//! that snapshot goes stale: a seat it called free was measured minutes
//! later holding another session's 13 uncommitted files.
//! This module is the one place a seat is measured, so the command and
//! the greeting cannot drift apart — and the command is the live answer
//! to read before entering a seat (CLAUDE.md ビルド・テスト).

use std::time::{Duration, SystemTime};

/// The reusable worktree seats. Sessions rotate through these six instead
/// of minting a name per topic — a topical worktree is never reused, so
/// every one paid a cold target/ build and kept the gigabytes afterwards
/// (CLAUDE.md ビルド・テスト).
pub(crate) const SEATS: [&str; 6] = ["a", "b", "c", "d", "e", "f"];

/// The mark a session's seat claim carries in `git worktree lock`'s
/// reason, followed by the session id and the Claude process the claim was
/// written from. The entry hooks and the post-write re-claim write it;
/// `land` and the session-end hook release it. A lock without this mark is
/// a person's, and nothing automatic touches it.
pub(crate) const SEAT_CLAIM: &str = "claude-seat";

/// The directory every worktree of this repository sits under.
const WORKTREES: &str = "/.claude/worktrees/";

/// This session, as a claim records it.
///
/// Two marks, because neither alone answers both questions a seat asks.
/// The session id says which conversation holds it, and is what the
/// session-end release matches on. The pid is the one mark that can be
/// put a question to: a claim whose Claude process is gone is litter,
/// and without asking, a still seat can only be guessed at from how long
/// it has been still — a guess that unlocks a live session's seat out
/// from under it.
pub(crate) struct Identity {
    pub session: String,
    pub pid: Option<u32>,
}

impl Identity {
    /// This session's marks: the session id a hook payload carries, or
    /// the environment's for a command run outside one, and the Claude
    /// process both run under. A claim missing either mark still works,
    /// with the question that mark answers left unanswerable.
    pub(crate) fn current(session: Option<&str>) -> Self {
        let session = session
            .map(str::to_string)
            .filter(|session| !session.is_empty())
            .or_else(|| std::env::var("CLAUDE_CODE_SESSION_ID").ok())
            .unwrap_or_default();
        let pid = std::env::var("CLAUDE_PID")
            .ok()
            .and_then(|pid| pid.trim().parse().ok());
        Self { session, pid }
    }

    /// What this session writes into a lock's reason.
    pub(crate) fn reason(&self) -> String {
        match self.pid {
            Some(pid) => format!("{SEAT_CLAIM} {} pid {pid}", self.session),
            None => format!("{SEAT_CLAIM} {}", self.session),
        }
    }

    /// This session as a person reads it beside a lock's reason.
    pub(crate) fn mark(&self) -> String {
        match self.pid {
            Some(pid) => format!("session {} (pid {pid})", self.session),
            None => format!("session {}", self.session),
        }
    }
}

/// What a claim's own marks settle for whoever meets it in somebody
/// else's seat. A session cannot tell a real collision from a stale lock
/// by when the lock was written — seat e was shared for eight minutes
/// because a session read a matching mtime as proof the claim was its
/// own — so the answer is the claim's process, asked.
pub(crate) fn claim_liveness(reason: &str) -> &'static str {
    match holder(reason).and_then(|holder| holder.pid) {
        Some(pid) if crate::subprocess::process_exists(pid) => {
            "That process is running, so the other session is live and both of you are in one tree."
        }
        Some(_) => "That process is gone, so the claim is litter a session left behind.",
        None => {
            "The claim names no process, so it predates the mark that would answer — \
             `cargo xtask seats` says whether the seat is still being worked."
        }
    }
}

/// Whether a lock is a claim nobody is behind any more. Only a claim that
/// names its process can answer; anything else is left standing.
pub(crate) fn claim_is_dead(reason: &str) -> bool {
    holder(reason)
        .and_then(|holder| holder.pid)
        .is_some_and(|pid| !crate::subprocess::process_exists(pid))
}

/// Who a claim names, read back out of a lock's reason. None when the
/// lock carries no claim of ours — a person's lock, which nothing
/// automatic touches.
fn holder(reason: &str) -> Option<Identity> {
    let rest = reason.strip_prefix(SEAT_CLAIM)?.trim_start();
    let (session, pid) = match rest.split_once(" pid ") {
        Some((session, pid)) => (session, pid.trim().parse().ok()),
        None => (rest, None),
    };
    Some(Identity {
        session: session.trim().to_string(),
        pid,
    })
}

/// Where a seat's lock stands relative to this session. Pure but for the
/// liveness probe, so the tests can ask.
pub(crate) enum Standing {
    /// No lock at all.
    Free,
    /// This session's claim.
    Ours,
    /// A claim whose Claude process is gone: litter the roster may clear.
    Stale(String),
    /// Somebody else's live claim, or a lock a person wrote by hand.
    Foreign(String),
}

/// Either mark matching is proof enough that the claim is this session's,
/// and neither matching is not a reason to assume it: seat e was shared
/// by two sessions for eight minutes because one of them read a lock it
/// could not account for as its own anyway (2026-09-02).
pub(crate) fn standing(reason: Option<String>, me: &Identity) -> Standing {
    let Some(reason) = reason else {
        return Standing::Free;
    };
    let Some(holder) = holder(&reason) else {
        return Standing::Foreign(reason);
    };
    if (me.pid.is_some() && holder.pid == me.pid)
        || (!me.session.is_empty() && holder.session == me.session)
    {
        return Standing::Ours;
    }
    match holder.pid {
        Some(pid) if !crate::subprocess::process_exists(pid) => Standing::Stale(reason),
        _ => Standing::Foreign(reason),
    }
}

/// Takes `seat_path` for this session when it is there to take: a free
/// seat is locked, a claim whose process is gone is lifted and locked
/// again, and a live claim somebody else holds is left where it is.
/// Answers where the seat stood once this was done — only `Ours` means
/// the session may work there.
pub(crate) fn take_seat(cwd: &str, seat_path: &str, me: &Identity) -> Standing {
    match standing(lock_reason(seat_path), me) {
        Standing::Ours => Standing::Ours,
        Standing::Foreign(reason) => Standing::Foreign(reason),
        // Two sessions can meet one dead claim in the same moment, and
        // this unlock is not the thing that decides between them: the
        // lock below is, because git refuses the second one.
        Standing::Stale(_) => {
            unlock_seat(cwd, seat_path);
            lock_or_read(cwd, seat_path, me)
        }
        Standing::Free => lock_or_read(cwd, seat_path, me),
    }
}

/// One atomic claim, and what the seat looked like afterwards.
/// `git worktree lock` refuses a second lock, so the loser of a race is
/// told here rather than after settling in.
fn lock_or_read(cwd: &str, seat_path: &str, me: &Identity) -> Standing {
    let mut command = std::process::Command::new("git");
    command
        .arg("-C")
        .arg(cwd)
        // The refusal below is read out of git's own message, and a
        // translated one would read as no refusal at all. Pin the locale
        // so the claim keeps its teeth.
        .env("LC_ALL", "C")
        .args(["worktree", "lock", "--reason", &me.reason(), seat_path]);
    if let Ok(output) = crate::subprocess::run_captured(&mut command)
        && !output.status.success()
        && let Some(rest) = String::from_utf8_lossy(&output.stderr)
            .split("already locked")
            .nth(1)
    {
        let reason = rest
            .split_once("reason:")
            .map(|(_, reason)| reason.trim().to_string())
            .unwrap_or_default();
        return standing(Some(reason), me);
    }
    // A lock git reported nothing about is not a claim yet: read the seat
    // back, so that only a reason naming this session counts as one.
    standing(lock_reason(seat_path), me)
}

/// Lifts whatever lock a seat carries. Callers check first whose it is.
pub(crate) fn unlock_seat(cwd: &str, seat_path: &str) -> bool {
    crate::subprocess::git_query(cwd, &["worktree", "unlock", seat_path]).is_some()
}

/// The reason on this worktree's own lock, if it is locked at all.
pub(crate) fn lock_reason(cwd: &str) -> Option<String> {
    let git_dir =
        crate::subprocess::git_query(cwd, &["rev-parse", "--path-format=absolute", "--git-dir"])?;
    let reason = std::fs::read_to_string(format!("{git_dir}/locked")).ok()?;
    Some(reason.trim().to_string())
}

/// One seat of the roster, surveyed.
pub(crate) struct Seat {
    pub name: &'static str,
    /// None while the seat's worktree has not been created.
    pub state: Option<SeatState>,
}

/// What a created seat holds right now.
pub(crate) struct SeatState {
    /// Branch name, empty while HEAD is detached — a rebase in flight
    /// detaches it, so an active session can read as branchless.
    pub branch: String,
    /// Whether `git worktree lock` holds it — a session's claim (the
    /// entry hooks write one; manual locks land here too).
    pub locked: bool,
    /// The lock's reason, empty when unlocked or given none.
    pub lock_reason: String,
    /// Whether that lock is a claim whose Claude process is gone. Such a
    /// seat is free: the roster takes it back on its own, so nobody has
    /// to judge a long-still seat and nobody unlocks a live one by hand.
    pub claim_dead: bool,
    /// Commits main does not have (`main..HEAD`); None when git could not
    /// answer. Ranges run on HEAD, not the branch name, so a detached
    /// seat still counts.
    pub ahead: Option<u32>,
    /// The reverse (`HEAD..main`); zero of each means HEAD is main's tip.
    pub behind: Option<u32>,
    /// Lines of `status --porcelain`: every uncommitted change, untracked
    /// files included. None when git could not answer.
    pub dirty: Option<usize>,
    /// Time since the seat's own index was last written. Most git run in
    /// the seat refreshes it, so a fresh age means hands on the seat
    /// recently, whatever the counted columns say.
    pub index_age: Option<Duration>,
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
    /// What the session is told. An instruction and not a menu: this path
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
/// Nobody names a letter, because naming one means choosing it from a
/// survey, and a survey is a snapshot two sessions can agree on while
/// both are wrong about it. Only the lock ever decided which of them got
/// the seat, so the choosing happens here, behind that lock: letters are
/// tried until one is claimed, and the letter comes back as an answer
/// rather than going in as a request (CLAUDE.md ビルド・テスト).
pub fn take(args: &[String]) -> Result<(), String> {
    if !args.is_empty() {
        return Err(format!("seat takes no arguments (got {args:?})"));
    }
    let root = crate::tree::workspace_root();
    let assigned = assign(&root.to_string_lossy(), &Identity::current(None))?;
    println!("{}", assigned.report());
    Ok(())
}

/// The seat this session holds, taking one if it holds none.
///
/// Every refusal on the way is a seat somebody else is in, so the loop
/// walks the roster rather than stopping at the first no. What it hands
/// back is a tree that is ready to be worked in: claimed, empty, and at
/// main's tip.
pub(crate) fn assign(cwd: &str, me: &Identity) -> Result<Assigned, String> {
    let (primary, trees) = primary_checkout(cwd)?;
    let entries = seat_entries_of(trees);
    if let Some(held) = held_seat(&entries, me) {
        return Ok(held);
    }
    for name in spread_order() {
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
         seats are not added past f (CLAUDE.md ビルド・テスト)"
            .into(),
    )
}

/// The seat this session already holds, if it holds one. A session works
/// one seat at a time, and asking twice must give the same answer rather
/// than a second seat.
fn held_seat(entries: &[SeatEntry], me: &Identity) -> Option<Assigned> {
    entries
        .iter()
        .find(|entry| matches!(standing(lock_reason(&entry.tree.path), me), Standing::Ours))
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
    if !matches!(take_seat(primary, path, me), Standing::Ours) {
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
    // its commits; a branch still carrying work is skipped, not reset.
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
fn dirty_lines(dir: &str) -> Option<usize> {
    crate::subprocess::git_query(dir, &["--no-optional-locks", "status", "--porcelain"])
        .map(|status| status.lines().filter(|line| !line.is_empty()).count())
}

/// `cargo xtask seats`: one line per seat, and how to read them.
pub fn run(args: &[String]) -> Result<(), String> {
    if !args.is_empty() {
        return Err(format!("seats takes no arguments (got {args:?})"));
    }
    let root = crate::tree::workspace_root();
    let seats = survey(&root.to_string_lossy())
        .ok_or("git worktree list failed — is git on PATH and this a repository?")?;
    print!("{}", render(&seats));
    Ok(())
}

/// Every roster seat, measured now. None when `git worktree list` itself
/// fails (not a repository). The survey only reads: status runs under
/// --no-optional-locks, because a plain status opportunistically rewrites
/// the index it refreshed, and that write would stamp the very index
/// mtimes this survey reports with the survey's own run.
pub(crate) fn survey(cwd: &str) -> Option<Vec<Seat>> {
    let listing = crate::subprocess::git_query(cwd, &["worktree", "list", "--porcelain"])?;
    let entries = seat_entries(&listing);
    let now = SystemTime::now();
    // One thread per seat: the greeting takes this survey on every session
    // start, and six seats of sequential subprocess batches are the
    // difference between a beat and a second.
    Some(std::thread::scope(|scope| {
        let handles = SEATS.map(|name| {
            let entry = entries.iter().find(|entry| entry.seat == name);
            scope.spawn(move || entry.map(|entry| seat_state(entry, now)))
        });
        SEATS
            .into_iter()
            .zip(handles)
            .map(|(name, handle)| Seat {
                name,
                state: handle.join().unwrap_or(None),
            })
            .collect()
    }))
}

/// Measures one created seat. Each figure is None when its git call
/// fails, and the callers print those as unknowns rather than guess.
fn seat_state(entry: &SeatEntry, now: SystemTime) -> SeatState {
    let dir = entry.tree.path.as_str();
    SeatState {
        branch: entry.tree.branch.clone(),
        locked: entry.tree.locked,
        lock_reason: entry.tree.reason.clone(),
        claim_dead: entry.tree.locked && claim_is_dead(&entry.tree.reason),
        ahead: commits_in(dir, "main..HEAD"),
        behind: commits_in(dir, "HEAD..main"),
        dirty: dirty_lines(dir),
        index_age: index_age(dir, now),
    }
}

/// How long since the seat's own index was written. The path has to be
/// asked for: a worktree's admin directory is named after the directory
/// the tree was first created as, not after the seat — one seat here
/// sits on .git/worktrees/skillcare — so .git/worktrees/<seat>/index is
/// a guess that misses (measured).
fn index_age(dir: &str, now: SystemTime) -> Option<Duration> {
    let index = crate::subprocess::git_query(
        dir,
        &["rev-parse", "--path-format=absolute", "--git-path", "index"],
    )?;
    let written = std::fs::metadata(index).ok()?.modified().ok()?;
    Some(now.duration_since(written).unwrap_or(Duration::ZERO))
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

/// Whether two paths name one tree. Windows spells a path in whatever
/// case the writer used, so the comparison there is case-blind.
pub(crate) fn same_tree(left: &str, right: &str) -> bool {
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

/// The reading, one line, the way CLAUDE.md ビルド・テスト has it.
const GUIDE: &str = "reading: dirty>0 or ahead>0 = in use; dirty=0 and ahead=0 = free, and \
    `cargo xtask seat` is what takes one — it claims the seat and puts it at main's tip \
    itself, so nothing here is a letter to pick; a locked seat is held by the session its \
    claim names, unless the note says that session has ended.";

/// The table: a header, one line per seat, and the reading. Pure so the
/// tests can hand it seats git never made.
fn render(seats: &[Seat]) -> String {
    let mut rows = vec![[
        "seat".to_string(),
        "branch".to_string(),
        "at-main".to_string(),
        "ahead".to_string(),
        "dirty".to_string(),
        "index-age".to_string(),
    ]];
    let mut notes = vec![String::new()];
    for seat in seats {
        let (row, note) = seat_row(seat);
        rows.push(row);
        notes.push(note);
    }
    let widths: [usize; 6] =
        std::array::from_fn(|column| rows.iter().map(|row| row[column].len()).max().unwrap_or(0));
    let mut out = String::new();
    for (row, note) in rows.iter().zip(&notes) {
        let mut line = String::new();
        for (cell, width) in row.iter().zip(widths) {
            line.push_str(&format!("{cell:<width$}  "));
        }
        out.push_str(line.trim_end());
        if !note.is_empty() {
            out.push_str("  ");
            out.push_str(note);
        }
        out.push('\n');
    }
    out.push_str(GUIDE);
    out.push('\n');
    out
}

/// One seat's cells, and the note appended past the columns ("locked").
fn seat_row(seat: &Seat) -> ([String; 6], String) {
    let name = seat.name.to_string();
    let Some(state) = &seat.state else {
        return (
            [
                name,
                "(not created)".to_string(),
                "-".to_string(),
                "-".to_string(),
                "-".to_string(),
                "-".to_string(),
            ],
            String::new(),
        );
    };
    let branch = if state.branch.is_empty() {
        "(detached)".to_string()
    } else {
        state.branch.clone()
    };
    let at_main = match (state.ahead, state.behind) {
        (Some(0), Some(0)) => "yes",
        (Some(_), Some(_)) => "no",
        _ => "?",
    };
    let count = |value: Option<u32>| value.map_or("?".to_string(), |value| value.to_string());
    let dirty = state
        .dirty
        .map_or("?".to_string(), |value| value.to_string());
    let held = if state.claim_dead {
        "claim left by a session that ended"
    } else {
        "locked"
    };
    let note = match (state.locked, state.lock_reason.is_empty()) {
        (false, _) => String::new(),
        (true, true) => held.to_string(),
        (true, false) => format!("{held} ({})", state.lock_reason),
    };
    (
        [
            name,
            branch,
            at_main.to_string(),
            count(state.ahead),
            dirty,
            format_age(state.index_age),
        ],
        note.to_string(),
    )
}

/// An age as the shortest round figure that still ranks seats: seconds
/// under a minute, then minutes, hours, days.
pub(crate) fn format_age(age: Option<Duration>) -> String {
    let Some(age) = age else {
        return "?".to_string();
    };
    let seconds = age.as_secs();
    match seconds {
        0..60 => format!("{seconds}s"),
        60..3600 => format!("{}m", seconds / 60),
        3600..86400 => format!("{}h", seconds / 3600),
        _ => format!("{}d", seconds / 86400),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Identity, Seat, SeatEntry, SeatState, Standing, WorktreeBlock, format_age, render,
        seat_entries, standing,
    };
    use std::time::Duration;

    /// A session's marks, as a claim would record them.
    fn me(session: &str, pid: Option<u32>) -> Identity {
        Identity {
            session: session.to_string(),
            pid,
        }
    }

    /// A pid that has certainly exited: our own child, reaped.
    fn dead_pid() -> u32 {
        let mut probe = if cfg!(windows) {
            let mut command = std::process::Command::new("cmd");
            command.args(["/C", "exit 0"]);
            command
        } else {
            std::process::Command::new("true")
        };
        let mut child = probe.spawn().expect("spawn a short-lived child");
        let pid = child.id();
        child.wait().expect("reap the child");
        pid
    }

    #[test]
    fn stands_a_lock_relative_to_the_session() {
        let mine = me("s1", Some(std::process::id()));
        assert!(matches!(standing(None, &mine), Standing::Free));
        assert!(matches!(
            standing(
                Some(format!("claude-seat s9 pid {}", std::process::id())),
                &mine
            ),
            Standing::Ours,
        ));
        // The session id alone still answers for a claim written before
        // the pid was recorded, and for one this process did not write.
        assert!(matches!(
            standing(Some("claude-seat s1".into()), &mine),
            Standing::Ours
        ));
        // Another session, and its process is running: this test's own,
        // which is the one pid it can be sure of. `mine` carries no pid
        // of its own, so nothing but the ids is left to compare.
        assert!(matches!(
            standing(
                Some(format!("claude-seat s2 pid {}", std::process::id())),
                &me("s1", None)
            ),
            Standing::Foreign(_)
        ));
        assert!(matches!(
            standing(Some("parked by hand".into()), &mine),
            Standing::Foreign(_)
        ));
    }

    #[test]
    fn a_claim_whose_process_is_gone_is_litter() {
        let mine = me("s1", Some(std::process::id()));
        assert!(matches!(
            standing(Some(format!("claude-seat s2 pid {}", dead_pid())), &mine),
            Standing::Stale(_)
        ));
        // A claim from before the pid was recorded cannot be asked, so it
        // stays somebody's until its session says otherwise.
        assert!(matches!(
            standing(Some("claude-seat s2".into()), &mine),
            Standing::Foreign(_)
        ));
        // Nor may a session with no marks of its own read a claim as one
        // it wrote: that assumption is what put two sessions in seat e.
        assert!(matches!(
            standing(Some("claude-seat s2".into()), &me("", None)),
            Standing::Foreign(_)
        ));
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

    #[test]
    fn rounds_ages_to_the_rank_that_orders_seats() {
        assert_eq!(format_age(None), "?");
        assert_eq!(format_age(Some(Duration::from_secs(59))), "59s");
        assert_eq!(format_age(Some(Duration::from_secs(60))), "1m");
        assert_eq!(format_age(Some(Duration::from_secs(3 * 3600))), "3h");
        assert_eq!(format_age(Some(Duration::from_secs(9 * 86400))), "9d");
    }

    #[test]
    fn renders_every_kind_of_seat_on_its_own_line() {
        let seats = vec![
            Seat {
                name: "a",
                state: Some(SeatState {
                    branch: "worktree-a".to_string(),
                    locked: false,
                    lock_reason: String::new(),
                    claim_dead: false,
                    ahead: Some(1),
                    behind: Some(0),
                    dirty: Some(13),
                    index_age: Some(Duration::from_secs(16 * 60)),
                }),
            },
            Seat {
                name: "b",
                state: Some(SeatState {
                    branch: String::new(),
                    locked: true,
                    lock_reason: "claude-seat abc123".to_string(),
                    claim_dead: false,
                    ahead: Some(0),
                    behind: Some(0),
                    dirty: Some(0),
                    index_age: None,
                }),
            },
            Seat {
                name: "c",
                state: Some(SeatState {
                    branch: "worktree-c".to_string(),
                    locked: false,
                    lock_reason: String::new(),
                    claim_dead: false,
                    ahead: None,
                    behind: None,
                    dirty: None,
                    index_age: None,
                }),
            },
            Seat {
                name: "d",
                state: None,
            },
        ];
        let table = render(&seats);
        let lines: Vec<&str> = table.lines().collect();
        assert_eq!(lines.len(), 6, "{table}");
        assert!(lines[0].starts_with("seat  branch"), "{table}");
        assert!(
            lines[1].contains("worktree-a")
                && lines[1].contains("no")
                && lines[1].contains("13")
                && lines[1].contains("16m"),
            "{table}"
        );
        assert!(
            lines[2].contains("(detached)")
                && lines[2].contains("yes")
                && lines[2].ends_with("locked (claude-seat abc123)"),
            "{table}"
        );
        assert!(lines[3].contains('?'), "{table}");
        assert!(lines[4].contains("(not created)"), "{table}");
        assert!(lines[5].starts_with("reading:"), "{table}");
    }
}
