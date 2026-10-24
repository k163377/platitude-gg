//! The line of landings: who waits for main, in the order the user typed
//! `/land` (internal-docs/反映前テストの機械化.md §land の列).
//!
//! An entry is a session's `.permits/<session>.line`, written by the
//! prompt hook beside the permit and taken away with it. It stands in the
//! line while its permit is open, it has not left, and no other
//! conversation's claim is on its seat. The head lands, the next prepares,
//! and everyone behind waits without a turn of its own: `wait` is a
//! process that ends when the place changes, so a waiting session spends
//! nothing. Every write of an entry, and every reading that writes one
//! (dating the head, letting it go), holds the directory's lock: an entry
//! is load-modify-store, and two processes at it lose one's change.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::hook::permit::COMMAND;
use crate::locks::Locked;
use crate::seats::{Held, Identity, Standing as Claim, WorktreeBlock, roster_letter, standing};
use crate::subprocess::git_query;

/// The entry's kind beside the permit's (`hook::permit::kept_file`).
const KIND: &str = "line";

/// The permits directory's lock file, shared by name and never removed.
const LOCK: &str = "lock";

/// Field separator in the entry file; no value carries a tab.
const SEP: char = '\t';

/// How often a waiting process looks at the line.
const LOOK: Duration = Duration::from_secs(5);

/// How long a wait goes on through a line it cannot read — git not
/// answering — before it ends with that as its answer.
const UNREADABLE: Duration = Duration::from_secs(60);

/// What the head's landing may change that stops the next seat's
/// preparation unread: the rules every landing is held to, and the skills
/// a landing runs by.
const STOPS_PREPARATION: [&str; 5] = [
    "CLAUDE.md",
    ".claude/skills/gate/SKILL.md",
    ".claude/skills/land/SKILL.md",
    ".claude/skills/review-points/SKILL.md",
    ".claude/skills/verify-ui/SKILL.md",
];

/// Under which the head's other changes are read before the next seat
/// decides whether to prepare.
const READ_BEFORE_PREPARING: [&str; 2] = [".claude/", "internal-docs/"];

/// One session's entry as its file remembers it. Times are milliseconds
/// since the epoch: two `/land`s in one second keep their order.
#[derive(Debug, PartialEq, Clone)]
struct Entry {
    /// When the user typed `/land`.
    queued_at: u64,
    /// Where the session sat when it typed it.
    cwd: String,
    /// The desktop app's session, when the environment named one.
    host: String,
    /// From when a reader first found this entry at the head; zero
    /// before.
    head_since: u64,
    /// Why the entry left the line, empty while it stands.
    left: String,
    /// When it left; zero while it stands.
    left_at: u64,
}

impl Entry {
    /// The identity of `session` as its entry knows it: the id, and the
    /// desktop app's session where the entry names one.
    fn identity(&self, session: &str) -> Identity {
        Identity {
            session: session.to_string(),
            pid: None,
            host: (!self.host.is_empty()).then(|| self.host.clone()),
        }
    }
}

/// A session entering the line, written by the prompt hook when the
/// user's message is `/land`, beside the permit it just wrote. `cwd` is
/// where the session sat, `at` the message's time in milliseconds. The
/// conversation's entries under earlier session ids go: this message is
/// theirs too.
pub(crate) fn enter(permit: &Path, cwd: &str, host: Option<&str>, at: u64) {
    let Some(permits) = permit.parent() else {
        return;
    };
    let Ok(_held) = locked(permits) else {
        return;
    };
    store(
        &beside(permit),
        &Entry {
            queued_at: at,
            cwd: cwd.replace('\\', "/"),
            host: host.unwrap_or_default().to_string(),
            head_since: 0,
            left: String::new(),
            left_at: 0,
        },
    );
    retire_earlier_ids(permits, permit, host);
}

/// The entry taken away, by the prompt hook when the user's message is
/// anything else: the permit closed, and the place with it — under every
/// session id the conversation had.
pub(crate) fn clear(permit: &Path, host: Option<&str>) {
    let Some(permits) = permit.parent() else {
        return;
    };
    let Ok(_held) = locked(permits) else {
        return;
    };
    let _gone = std::fs::remove_file(beside(permit));
    retire_earlier_ids(permits, permit, host);
}

/// The entries of the same desktop-app session under other session ids
/// — the conversation went on under a new id (an interrupted prompt
/// edited, `/clear`) and the old id's prompt hook never runs again — are
/// taken away by the id that speaks now.
fn retire_earlier_ids(permits: &Path, permit: &Path, host: Option<&str>) {
    let Some(host) = host.filter(|host| !host.is_empty()) else {
        return;
    };
    let mine = beside(permit);
    for (_, path, entry) in entries(permits) {
        if entry.host == host && path != mine {
            let _gone = std::fs::remove_file(path);
        }
    }
}

/// The entry's file beside a permit's: `<session>.land` → `<session>.line`
/// (a session id carries no dot — `hook::permit::kept_file`).
fn beside(permit: &Path) -> PathBuf {
    permit.with_extension(KIND)
}

/// The directory's lock, held while entries are written.
fn locked(permits: &Path) -> Result<Locked, String> {
    std::fs::create_dir_all(permits)
        .map_err(|e| format!("could not make {}: {e}", permits.display()))?;
    let path = permits.join(LOCK);
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

/// The session leaving the line for `why`: a landing that stopped,
/// nothing to land, a seat released, a question for the user. Says
/// whether the entry stood in the line until now.
pub(crate) fn leave(dir: &str, session: &str, why: &str) -> bool {
    let Some(path) = file(dir, session) else {
        return false;
    };
    let Some(permits) = path.parent() else {
        return false;
    };
    let Ok(_held) = locked(permits) else {
        return false;
    };
    let Some(mut entry) = load(&path) else {
        return false;
    };
    if !entry.left.is_empty() {
        return false;
    }
    entry.left = one_line(why);
    entry.left_at = now();
    store(&path, &entry);
    true
}

/// Where a session's entry stands, as the Stop hook tells it; None when
/// the session has none.
pub(crate) fn standing_for(dir: &str, session: &str) -> Option<String> {
    let entry = load(&file(dir, session)?)?;
    if !entry.left.is_empty() {
        return Some(format!(
            "line: this seat left the line of landings ({}, {}) — the user's next {COMMAND} puts \
             it back at the end.",
            entry.left,
            ago(entry.left_at),
        ));
    }
    let me = entry.identity(session);
    let line = read(dir).ok()?;
    Some(match line.place_of(&me) {
        Some(place) => format!("line: {}.", line.place_said(place)),
        None => "line: this seat is not in the line of landings — its /land was typed outside a \
                 worktree of its own, or the seat's claim moved."
            .to_string(),
    })
}

/// The landing verb's gate: a session whose entry stands behind another's
/// is refused, and one that left is sent back to the user.
pub(crate) fn must_be_head(dir: &str, me: &Identity) -> Result<(), String> {
    if me.session.is_empty() {
        return Ok(());
    }
    let Some(entry) = file(dir, &me.session).and_then(|path| load(&path)) else {
        // A permit from before the line, or a session the hook never saw.
        return Ok(());
    };
    if !entry.left.is_empty() {
        return Err(format!(
            "this seat left the line of landings ({}, {}): the user's next {COMMAND} puts it back \
             at the end, and only then does it land",
            entry.left,
            ago(entry.left_at),
        ));
    }
    let line = read(dir)?;
    match line.place_of(me) {
        Some(0) => Ok(()),
        Some(place) => Err(format!(
            "{} — `{}` says when it is this seat's turn:\n{}",
            line.place_said(place),
            super::WAIT_HEAD.line(),
            line.say()
        )),
        None => Err(format!(
            "this session's {COMMAND} was typed in {}, which is in no worktree with a branch of \
             its own, or that tree's claim is another conversation's now — the line of landings \
             does not hold it",
            entry.cwd
        )),
    }
}

/// `cargo xtask land line | wait next | wait head | leave <why>`.
pub(crate) fn run(args: &[String]) -> Result<(), String> {
    let here = crate::seats::slashed(&crate::tree::workspace_root());
    let me = Identity::current(None);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["line"] => {
            let line = read(&here)?;
            println!("{}", line.say());
            println!("{}", line.mine(&me));
            Ok(())
        }
        ["wait", "next"] => wait(&here, &me, 1),
        ["wait", "head"] => wait(&here, &me, 0),
        ["leave", why @ ..] if !why.is_empty() => {
            if me.session.is_empty() {
                return Err("no session identity — the line is per session".into());
            }
            if leave(&here, &me.session, &why.join(" ")) {
                println!(
                    "this seat left the line of landings; the user's next {COMMAND} puts it back \
                     at the end"
                );
                Ok(())
            } else {
                Err("this seat stands in no line to leave".into())
            }
        }
        other => Err(format!(
            "land takes `line`, `wait next`, `wait head` or `leave <why>` here (got {other:?})"
        )),
    }
}

/// Waits, looking every [`LOOK`], until this session's place is `until`
/// or nearer the head, then says where it stands and what that stage
/// does. Ends with an error the moment the session is not in the line; a
/// line that cannot be read is looked at again, for [`UNREADABLE`] at
/// most.
fn wait(here: &str, me: &Identity, until: usize) -> Result<(), String> {
    if me.session.is_empty() {
        return Err(
            "no session identity — the line is per session (CLAUDE_CODE_SESSION_ID)".into(),
        );
    }
    let mut unreadable_since: Option<std::time::Instant> = None;
    loop {
        match read(here) {
            Ok(line) => {
                unreadable_since = None;
                let place = line.place_of(me).ok_or_else(|| left_or_gone(here, me))?;
                if place <= until {
                    println!("{}", line.mine(me));
                    return Ok(());
                }
            }
            Err(why) => {
                // waits(ceiling): a line that cannot be read — git not
                // answering — is looked at again for `UNREADABLE`, then the
                // wait ends with that as its answer
                let since = *unreadable_since.get_or_insert_with(std::time::Instant::now);
                if since.elapsed() >= UNREADABLE {
                    return Err(format!(
                        "the line could not be read for {}s — {why}. Run `{}` again once git \
                         answers",
                        UNREADABLE.as_secs(),
                        if until == 0 {
                            super::WAIT_HEAD.line()
                        } else {
                            super::WAIT_NEXT.line()
                        }
                    ));
                }
            }
        }
        // waits(paced): a look at the line every `LOOK` — the place is read
        // off files no event announces, and the loop ends when this seat
        // reaches `until` or is not in the line
        std::thread::sleep(LOOK);
    }
}

/// Why a session that waited is not in the line.
fn left_or_gone(here: &str, me: &Identity) -> String {
    match file(here, &me.session).and_then(|path| load(&path)) {
        Some(entry) if !entry.left.is_empty() => format!(
            "this seat left the line of landings ({}, {}) — the user's next {COMMAND} puts it \
             back at the end",
            entry.left,
            ago(entry.left_at),
        ),
        Some(_)
            if crate::hook::permit::asked(here, &me.session).is_some_and(|asked| asked.waits()) =>
        {
            format!(
                "this session is not in the line: its landing waits for the user's approval of \
                 the pictures its seat put up after their {COMMAND} — their next {COMMAND} lands \
                 the branch"
            )
        }
        Some(_) => format!(
            "this session is not in the line: its permit closed (the user's later message), its \
             {COMMAND} was typed outside a worktree, or the seat's claim is another \
             conversation's"
        ),
        None => format!(
            "this session has no place in the line: no {COMMAND} of the user's stands open for it"
        ),
    }
}

/// One session standing in the line: its entry, and the seat and branch
/// the entry's cwd put it in.
#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Place {
    session: String,
    /// The roster letter, or the tree's own directory name off the roster.
    seat: String,
    branch: String,
    path: PathBuf,
    entry: Entry,
}

impl Place {
    fn identity(&self) -> Identity {
        self.entry.identity(&self.session)
    }
}

/// The line as it stands, oldest `/land` first.
pub(crate) struct Line {
    places: Vec<Place>,
    primary: String,
}

impl Line {
    fn place_of(&self, me: &Identity) -> Option<usize> {
        self.places.iter().position(|place| place.identity().is(me))
    }

    fn head(&self) -> Option<&Place> {
        self.places.first()
    }

    /// Where a place stands, in words.
    fn place_said(&self, place: usize) -> String {
        match place {
            0 => "this seat is the head of the line of landings".to_string(),
            1 => format!(
                "this seat is next in the line of landings, behind seat {}'s landing ({})",
                self.places[0].seat, self.places[0].branch
            ),
            _ => format!(
                "this seat is {} in the line of landings, {} landing(s) ahead of it",
                ordinal(place + 1),
                place
            ),
        }
    }

    /// The whole line, a place a line.
    pub(crate) fn say(&self) -> String {
        if self.places.is_empty() {
            return "the line of landings is empty".to_string();
        }
        let mut out = String::from("the line of landings, oldest /land first:\n");
        for (at, place) in self.places.iter().enumerate() {
            let stage = match at {
                0 => "landing",
                1 => "preparing",
                _ => "waiting",
            };
            out.push_str(&format!(
                "  {:<9} seat {} ({}), /land {}{}\n",
                stage,
                place.seat,
                place.branch,
                ago(place.entry.queued_at),
                if at == 0 && place.entry.head_since > 0 {
                    format!(", at the head for {}", elapsed(place.entry.head_since))
                } else {
                    String::new()
                }
            ));
        }
        out.trim_end().to_string()
    }

    /// This session's place and what its stage does — the line `wait`
    /// ends with.
    fn mine(&self, me: &Identity) -> String {
        let Some(place) = self.place_of(me) else {
            return "this session is not in the line of landings".to_string();
        };
        let mut out = self.place_said(place);
        match place {
            0 => {
                let main = git_query(&self.primary, &["rev-parse", "--short", "main"])
                    .unwrap_or_else(|| "?".to_string());
                out.push_str(&format!(
                    ": main is at {main}. Rebase onto it, then review, fix, confirm, reorganise \
                     the commits, docs, and `{}` (the land skill's head stage).",
                    super::LAND.line()
                ));
            }
            1 => {
                if let Some(head) = self.head() {
                    out.push_str(". ");
                    out.push_str(&said_of_the_head(
                        rule_changes(&self.primary, &head.branch).as_ref(),
                        &head.seat,
                        &head.branch,
                    ));
                }
            }
            _ => out.push_str(&format!(
                " — nothing to do until `{}` ends.",
                super::WAIT_NEXT.line()
            )),
        }
        out
    }
}

/// The line read off the entries and the seats' claims, under the
/// directory's lock. The head's entry is dated at the head on first
/// sight, and let go when the machine's holder found nothing happening
/// for [`crate::awake::QUIET`] since then (`awake::last_quiet_release`).
pub(crate) fn read(dir: &str) -> Result<Line, String> {
    let (primary, trees) = crate::seats::primary_checkout(dir)?;
    let permits = Path::new(&primary).join(".permits");
    let _held = locked(&permits)?;
    let mut places = Vec::new();
    for (session, path, entry) in entries(&permits) {
        if !entry.left.is_empty() || !crate::hook::permit::open_at(&path.with_extension("land")) {
            continue;
        }
        let Some(tree) = tree_of(&trees, &entry.cwd) else {
            continue;
        };
        let seat = roster_letter(&tree.path)
            .map(str::to_string)
            .unwrap_or_else(|| {
                tree.path
                    .rsplit('/')
                    .next()
                    .unwrap_or(&tree.path)
                    .to_string()
            });
        let me = entry.identity(&session);
        // A seat nobody claims contradicts nothing; another conversation's
        // claim says the session that typed /land there is gone from it.
        let reason = tree.locked.then(|| tree.reason.clone());
        if !matches!(
            standing(reason, &me, Held::BySession),
            Claim::Ours | Claim::Free
        ) {
            continue;
        }
        places.push(Place {
            session,
            seat,
            branch: tree.branch.clone(),
            path,
            entry,
        });
    }
    places.sort_by(|a, b| (a.entry.queued_at, &a.session).cmp(&(b.entry.queued_at, &b.session)));
    let released = crate::awake::last_quiet_release(Path::new(&primary));
    settle_the_head(&mut places, released);
    Ok(Line { places, primary })
}

/// The listed worktree an entry's cwd sits in: the nearest, since a seat
/// lies under the primary checkout (`.claude/worktrees/`). The primary
/// itself holds main and is no place to land from — a `/land` typed there
/// has no place.
fn tree_of<'a>(trees: &'a [WorktreeBlock], cwd: &str) -> Option<&'a WorktreeBlock> {
    let cwd = cwd.replace('\\', "/");
    trees
        .iter()
        .skip(1)
        .filter(|tree| under(&cwd, &tree.path))
        .max_by_key(|tree| tree.path.len())
}

/// Whether `path` is `root` or lies under it, as the system reads case.
fn under(path: &str, root: &str) -> bool {
    let root = root.trim_end_matches('/');
    let (path, root) = if cfg!(windows) {
        (path.to_ascii_lowercase(), root.to_ascii_lowercase())
    } else {
        (path.to_string(), root.to_string())
    };
    path == root
        || path
            .strip_prefix(&root)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Dates the head at the head, and lets a head go that the holder found
/// quiet for [`crate::awake::QUIET`] after that date — the hang the user
/// named, judged by the machine's own quiet, not by the line. Under the
/// directory's lock (`read`).
fn settle_the_head(places: &mut Vec<Place>, released: Option<u64>) {
    while let Some(head) = places.first_mut() {
        if head.entry.head_since == 0 {
            head.entry.head_since = now();
            store(&head.path, &head.entry);
            break;
        }
        if !quiet_let_go(head.entry.head_since, released) {
            break;
        }
        head.entry.left = format!(
            "nothing happened on the machine for {}h after this seat reached the head",
            crate::awake::QUIET.as_secs() / 3600
        );
        head.entry.left_at = now();
        store(&head.path, &head.entry);
        places.remove(0);
    }
}

/// Whether a quiet release at `released` finds a head dated `head_since`
/// quiet for the whole of the holder's quiet (both in milliseconds).
fn quiet_let_go(head_since: u64, released: Option<u64>) -> bool {
    let quiet = u64::try_from(crate::awake::QUIET.as_millis()).unwrap_or(u64::MAX);
    released.is_some_and(|at| at >= head_since.saturating_add(quiet))
}

/// What a landing changes that bears on the next seat's preparation.
#[derive(Debug, PartialEq, Default)]
struct RuleChanges {
    stop: Vec<String>,
    read: Vec<String>,
}

/// What the next seat is told about the head's landing: the rule changes
/// sorted ([`RuleChanges`]), or that they could not be read.
fn said_of_the_head(changes: Option<&RuleChanges>, seat: &str, branch: &str) -> String {
    let Some(changes) = changes else {
        return format!(
            "Seat {seat}'s changes could not be read (`git diff main...{branch}` answered \
             nothing): read them yourself before preparing anything."
        );
    };
    if !changes.stop.is_empty() {
        return format!(
            "Seat {seat}'s landing changes {} — the rules every landing is held to: prepare \
             nothing, and `{}` says when main holds it.",
            changes.stop.join(", "),
            super::WAIT_HEAD.line()
        );
    }
    if !changes.read.is_empty() {
        return format!(
            "Seat {seat}'s landing changes {} — read those diffs (`git diff main...{branch} -- \
             <path>`) and decide: a rule or design change that applies to this seat's work \
             means prepare nothing; otherwise read this seat's own code and fix only plain \
             defects.",
            changes.read.join(", ")
        );
    }
    format!(
        "Seat {seat}'s landing changes no rule or design document: read this seat's own code \
         and fix only plain defects, then `{}`.",
        super::WAIT_HEAD.line()
    )
}

/// The rule and design documents `branch` changes against main, sorted
/// into those that stop the next seat's preparation and those it reads;
/// None when git did not answer.
fn rule_changes(primary: &str, branch: &str) -> Option<RuleChanges> {
    // `-z`: names as they are — without it git quotes a name beyond ASCII
    // (`core.quotePath`), and a Japanese document's path matches nothing.
    let changed = git_query(
        primary,
        &["diff", "--name-only", "-z", &format!("main...{branch}")],
    )?;
    Some(sort_rule_changes(changed.split('\0')))
}

fn sort_rule_changes<'a>(paths: impl Iterator<Item = &'a str>) -> RuleChanges {
    let mut changes = RuleChanges::default();
    for path in paths.map(str::trim).filter(|path| !path.is_empty()) {
        if STOPS_PREPARATION.contains(&path) {
            changes.stop.push(path.to_string());
        } else if READ_BEFORE_PREPARING
            .iter()
            .any(|under| path.starts_with(under))
        {
            changes.read.push(path.to_string());
        }
    }
    changes
}

/// Every entry file in the permits directory: the session, the path and
/// what it holds.
fn entries(permits: &Path) -> Vec<(String, PathBuf, Entry)> {
    let Ok(dir) = std::fs::read_dir(permits) else {
        return Vec::new();
    };
    let suffix = format!(".{KIND}");
    dir.flatten()
        .filter_map(|found| {
            let name = found.file_name().to_string_lossy().to_string();
            let session = name.strip_suffix(&suffix)?.to_string();
            let path = found.path();
            let entry = load(&path)?;
            Some((session, path, entry))
        })
        .collect()
}

fn file(dir: &str, session: &str) -> Option<PathBuf> {
    crate::hook::permit::kept_file(dir, session, KIND)
}

fn load(path: &Path) -> Option<Entry> {
    let text = std::fs::read_to_string(path).ok()?;
    parse(text.lines().next()?)
}

fn parse(line: &str) -> Option<Entry> {
    let mut fields = line.split(SEP);
    let queued_at = fields.next()?.parse().ok()?;
    let cwd = fields.next()?.to_string();
    let host = fields.next().unwrap_or_default().to_string();
    let head_since = fields.next().and_then(|n| n.parse().ok()).unwrap_or(0);
    let left = fields.next().unwrap_or_default().to_string();
    let left_at = fields.next().and_then(|n| n.parse().ok()).unwrap_or(0);
    Some(Entry {
        queued_at,
        cwd,
        host,
        head_since,
        left,
        left_at,
    })
}

/// Writes the entry whole (`permit::write_whole`): a reader sees the old
/// line or the new. One that cannot be written reads as absent, and an
/// absent entry governs nothing.
fn store(path: &Path, entry: &Entry) {
    let line = format!(
        "{}{SEP}{}{SEP}{}{SEP}{}{SEP}{}{SEP}{}\n",
        entry.queued_at, entry.cwd, entry.host, entry.head_since, entry.left, entry.left_at
    );
    if let Err(_unheard) = crate::hook::permit::write_whole(path, &line) {
        // Nobody to tell from inside a hook.
    }
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn ordinal(n: usize) -> String {
    let suffix = match (n % 10, n % 100) {
        (1, 11) | (2, 12) | (3, 13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

/// Milliseconds since the epoch.
pub(crate) fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| u64::try_from(since.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

fn ago(millis: u64) -> String {
    crate::hook::permit::ago(millis / 1000)
}

fn elapsed(millis: u64) -> String {
    crate::hook::permit::elapsed(millis / 1000)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{
        Entry, Line, Place, RuleChanges, load, ordinal, parse, quiet_let_go, said_of_the_head,
        settle_the_head, sort_rule_changes, store, tree_of, under,
    };
    use crate::seats::Identity;

    fn entry(queued_at: u64) -> Entry {
        Entry {
            queued_at,
            cwd: "C:/x/.claude/worktrees/b".into(),
            host: "host-1".into(),
            head_since: 0,
            left: String::new(),
            left_at: 0,
        }
    }

    fn place(session: &str, host: &str, seat: &str, queued_at: u64, path: &Path) -> Place {
        let mut entry = entry(queued_at);
        entry.host = host.into();
        Place {
            session: session.into(),
            seat: seat.into(),
            branch: format!("worktree-{seat}"),
            path: path.to_path_buf(),
            entry,
        }
    }

    fn me(session: &str, host: Option<&str>) -> Identity {
        Identity {
            session: session.into(),
            pid: None,
            host: host.map(str::to_string),
        }
    }

    #[test]
    fn an_entry_reads_back_and_a_short_one_stands_in_the_line() {
        let dir = crate::yard::Yard::new("line");
        let path = dir.join("s.line");
        let mut written = entry(1_700_000_000_000);
        written.head_since = 1_700_000_100_000;
        written.left = "gate failed: qmltest".into();
        written.left_at = 1_700_000_200_000;
        store(&path, &written);
        assert_eq!(load(&path), Some(written));
        let short = parse("1700000000000\tC:/x/.claude/worktrees/b").expect("entry");
        assert_eq!(short.queued_at, 1_700_000_000_000);
        assert!(short.left.is_empty() && short.head_since == 0 && short.host.is_empty());
        assert!(parse("soon\tC:/x").is_none());
    }

    #[test]
    fn a_place_is_found_by_session_or_by_host_and_said_by_its_stage() {
        let nowhere = PathBuf::from("nowhere");
        let line = Line {
            places: vec![
                place("s1", "h1", "c", 10, &nowhere),
                place("s2", "h2", "b", 20, &nowhere),
                place("s3", "", "e", 30, &nowhere),
            ],
            primary: String::new(),
        };
        assert_eq!(line.place_of(&me("s1", None)), Some(0));
        assert_eq!(
            line.place_of(&me("s2-later", Some("h2"))),
            Some(1),
            "the conversation under a later session id keeps its place"
        );
        assert_eq!(line.place_of(&me("s3", None)), Some(2));
        assert_eq!(line.place_of(&me("", Some("h9"))), None);
        assert_eq!(
            line.place_of(&me("", None)),
            None,
            "no identity matches an entry with no host"
        );
        assert!(line.place_said(0).contains("head of the line"));
        assert!(
            line.place_said(1).contains("next") && line.place_said(1).contains("seat c"),
            "{}",
            line.place_said(1)
        );
        assert!(line.place_said(2).contains("3rd"), "{}", line.place_said(2));
        let said = line.say();
        assert!(
            said.contains("landing   seat c") && said.contains("preparing seat b"),
            "{said}"
        );
        assert!(said.contains("waiting   seat e"), "{said}");
    }

    #[test]
    fn the_head_is_dated_on_first_sight_and_let_go_only_after_a_whole_quiet() {
        let dir = crate::yard::Yard::new("line-head");
        let path = dir.join("s1.line");
        store(&path, &entry(100));
        let mut places = vec![place("s1", "", "c", 100, &path)];
        settle_the_head(&mut places, None);
        assert_eq!(places.len(), 1);
        let dated = load(&path).expect("entry");
        assert!(dated.head_since > 0, "the head is dated at the head");
        assert_eq!(places[0].entry.head_since, dated.head_since);
        let quiet = u64::try_from(crate::awake::QUIET.as_millis()).expect("millis");
        // A release inside the quiet after the date is no verdict on it.
        assert!(!quiet_let_go(dated.head_since, Some(dated.head_since + 1)));
        assert!(!quiet_let_go(dated.head_since, None));
        assert!(quiet_let_go(
            dated.head_since,
            Some(dated.head_since + quiet)
        ));
        let mut stale = dated.clone();
        stale.head_since = 1_000;
        store(&path, &stale);
        let behind = dir.join("s2.line");
        store(&behind, &entry(200));
        let mut stale_place = place("s1", "", "c", 100, &path);
        stale_place.entry = stale;
        let mut places = vec![stale_place, place("s2", "", "b", 200, &behind)];
        settle_the_head(&mut places, Some(1_000 + quiet));
        assert_eq!(
            places.len(),
            1,
            "the quiet head is let go, the next is dated"
        );
        assert_eq!(places[0].session, "s2");
        let gone = load(&path).expect("entry");
        assert!(gone.left.contains("nothing happened"), "{}", gone.left);
        assert!(load(&behind).expect("entry").head_since > 0);
    }

    #[test]
    fn rule_changes_sort_into_stops_reads_and_nothing() {
        let changes = sort_rule_changes(
            "CLAUDE.md\ncrates/xtask/src/land.rs\n.claude/rules-refs/app-ui.md\n\
             internal-docs/デザイン規約.md\n.claude/skills/verify-ui/verbs.md\n\
             .claude/skills/land/SKILL.md\n"
                .lines(),
        );
        assert_eq!(
            changes,
            RuleChanges {
                stop: vec!["CLAUDE.md".into(), ".claude/skills/land/SKILL.md".into()],
                read: vec![
                    ".claude/rules-refs/app-ui.md".into(),
                    "internal-docs/デザイン規約.md".into(),
                    ".claude/skills/verify-ui/verbs.md".into(),
                ],
            }
        );
        assert!(said_of_the_head(Some(&changes), "c", "worktree-c").contains("prepare nothing"));
        let read_only = sort_rule_changes(".claude/rules/core.md\n".lines());
        assert!(said_of_the_head(Some(&read_only), "c", "worktree-c").contains("read those diffs"));
        let none = sort_rule_changes("crates/platitude-core/src/lib.rs\n".lines());
        assert_eq!(none, RuleChanges::default());
        assert!(
            said_of_the_head(Some(&none), "c", "worktree-c").contains("no rule or design document")
        );
        assert!(
            said_of_the_head(None, "c", "worktree-c").contains("could not be read"),
            "git not answering is not 'no changes'"
        );
    }

    #[test]
    fn ordinals_are_spelled_as_a_person_counts() {
        assert_eq!(ordinal(1), "1st");
        assert_eq!(ordinal(2), "2nd");
        assert_eq!(ordinal(3), "3rd");
        assert_eq!(ordinal(4), "4th");
        assert_eq!(ordinal(11), "11th");
        assert_eq!(ordinal(22), "22nd");
    }

    #[test]
    fn an_entry_sits_in_the_nearest_listed_tree_and_never_in_the_primary() {
        let block = |path: &str| crate::seats::WorktreeBlock {
            path: path.into(),
            branch: String::new(),
            locked: false,
            reason: String::new(),
        };
        let trees = vec![
            block("C:/x"),
            block("C:/x/.claude/worktrees/b"),
            block("C:/x/seat-a"),
        ];
        let found = |cwd: &str| tree_of(&trees, cwd).map(|tree| tree.path.as_str());
        assert_eq!(
            found("C:/x/.claude/worktrees/b/crates"),
            Some("C:/x/.claude/worktrees/b")
        );
        assert_eq!(found("C:\\x\\seat-a"), Some("C:/x/seat-a"));
        assert_eq!(
            found("C:/x/crates"),
            None,
            "the primary is no place to land from"
        );
        assert_eq!(found("C:/x/seat-ab"), None, "a sibling sharing the prefix");
        assert!(!under("/y/seat-a", "/y/seat"));
        assert!(under("/y/seat", "/y/seat/"));
        if cfg!(windows) {
            assert!(under("c:/X/Seat-A/src", "C:/x/seat-a"));
        }
    }

    fn git(dir: &str, args: &[&str]) {
        assert!(
            crate::subprocess::git_query(dir, args).is_some(),
            "git {args:?} in {dir}"
        );
    }

    /// A roster seat of a throwaway repository, locked with `reason`.
    fn seat(repo: &str, letter: &str, reason: &str) -> String {
        let path = format!("{repo}/.claude/worktrees/{letter}");
        git(
            repo,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                &format!("worktree-{letter}"),
                &path,
            ],
        );
        git(repo, &["worktree", "lock", "--reason", reason, &path]);
        path
    }

    /// A permit as the prompt hook writes one, and the line entry beside
    /// it — `/land` typed in `cwd` at `at`.
    fn asked(repo: &str, session: &str, standing: &str, cwd: &str, host: Option<&str>, at: u64) {
        let permit = Path::new(repo)
            .join(".permits")
            .join(format!("{session}.land"));
        std::fs::create_dir_all(permit.parent().expect("dir")).expect("permits dir");
        std::fs::write(&permit, format!("{standing}\t{}\t/land\n", at / 1000)).expect("permit");
        super::enter(&permit, cwd, host, at);
    }

    /// A throwaway repository on main with one commit, and its path as
    /// git spells it.
    fn repository(name: &str) -> (crate::yard::Yard, String) {
        let yard = crate::yard::Yard::new(name);
        let repo = crate::seats::slashed(&yard);
        git(&repo, &["init", "-q", "-b", "main"]);
        commit(&repo, &["--allow-empty", "-m", "root"]);
        (yard, repo)
    }

    fn commit(repo: &str, args: &[&str]) {
        let mut all = vec!["-c", "user.name=t", "-c", "user.email=t@x", "commit", "-q"];
        all.extend_from_slice(args);
        git(repo, &all);
    }

    #[test]
    fn the_line_is_the_open_permits_whose_seats_claims_are_theirs_oldest_first() {
        let (_yard, repo) = repository("line-read");
        let seat_b = seat(&repo, "b", "claude-seat s1 pid 1 host h1");
        let seat_c = seat(&repo, "c", "claude-seat s2");
        let seat_d = seat(&repo, "d", "claude-seat s5");
        // Seat e is nobody's: a session that typed /land there stands.
        let seat_e = format!("{repo}/.claude/worktrees/e");
        git(
            &repo,
            &["worktree", "add", "-q", "-b", "worktree-e", &seat_e],
        );
        asked(&repo, "s2", "open", &seat_c, None, 200_000);
        asked(&repo, "s1", "open", &seat_b, Some("h1"), 100_000);
        asked(&repo, "s6", "open", &seat_e, None, 300_000);
        // Typed in seat b, whose claim is s1's: no place.
        asked(&repo, "s3", "open", &seat_b, None, 50_000);
        // A closed permit, and one typed in the primary checkout.
        asked(&repo, "s4", "closed", &seat_c, None, 60_000);
        asked(&repo, "s5", "open", &repo, None, 70_000);
        let line = super::read(&repo).expect("line");
        assert_eq!(
            line.places
                .iter()
                .map(|place| (place.session.as_str(), place.seat.as_str()))
                .collect::<Vec<_>>(),
            vec![("s1", "b"), ("s2", "c"), ("s6", "e")]
        );
        assert!(line.places[0].entry.head_since > 0, "the head is dated");
        assert_eq!(line.place_of(&me("s1-later", Some("h1"))), Some(0));
        assert!(super::must_be_head(&repo, &me("s1", Some("h1"))).is_ok());
        let behind = super::must_be_head(&repo, &me("s2", None)).expect_err("next");
        assert!(
            behind.contains("next") && behind.contains("seat b"),
            "{behind}"
        );
        assert!(
            super::must_be_head(&repo, &me("s9", None)).is_ok(),
            "a session the hook never saw is not governed"
        );
        let typed_outside = super::must_be_head(&repo, &me("s5", None)).expect_err("no seat");
        assert!(typed_outside.contains("no worktree"), "{typed_outside}");
        assert!(super::leave(&repo, "s2", "gate failed: qmltest"));
        assert!(!super::leave(&repo, "s2", "again"), "left once");
        let left = super::must_be_head(&repo, &me("s2", None)).expect_err("left");
        assert!(
            left.contains("left the line") && left.contains("qmltest"),
            "{left}"
        );
        assert!(
            super::standing_for(&repo, "s2")
                .expect("standing")
                .contains("left the line")
        );
        assert_eq!(super::read(&repo).expect("line").places.len(), 2);
        let _unused = seat_d;
    }

    #[test]
    fn a_conversations_next_message_under_a_new_id_takes_the_old_ids_place_away() {
        let (_yard, repo) = repository("line-conversation");
        let seat_b = seat(&repo, "b", "claude-seat s1 pid 1 host h1");
        asked(&repo, "s1", "open", &seat_b, Some("h1"), 100_000);
        assert_eq!(super::read(&repo).expect("line").places.len(), 1);
        // The conversation of s1 goes on under a new session id: its next
        // message, /land or not, takes the old id's entry away.
        let later = Path::new(&repo).join(".permits").join("s1-later.land");
        std::fs::write(&later, "closed\t400\t見た\n").expect("permit");
        super::clear(&later, Some("h1"));
        assert!(!Path::new(&repo).join(".permits/s1.line").exists());
        assert_eq!(super::read(&repo).expect("line").places.len(), 0);
        // And a /land under the new id takes a place of its own, at the end.
        asked(&repo, "s1", "open", &seat_b, Some("h1"), 100_000);
        asked(&repo, "s1-later", "open", &seat_b, Some("h1"), 200_000);
        let line = super::read(&repo).expect("line");
        assert_eq!(line.places.len(), 1);
        assert_eq!(line.places[0].session, "s1-later");
    }

    #[test]
    fn a_japanese_documents_path_is_read_as_it_is() {
        let (yard, repo) = repository("line-quotepath");
        // A branch changing a Japanese-named document: its name is read as
        // it is, where git would quote it (`core.quotePath`).
        git(&repo, &["switch", "-q", "-c", "topic"]);
        std::fs::create_dir_all(yard.join("internal-docs")).expect("docs dir");
        std::fs::write(
            yard.join("internal-docs").join("反映前テストの機械化.md"),
            "x\n",
        )
        .expect("doc");
        git(&repo, &["add", "internal-docs"]);
        commit(&repo, &["-m", "docs"]);
        git(&repo, &["switch", "-q", "main"]);
        let changes = super::rule_changes(&repo, "topic").expect("read");
        assert_eq!(
            changes.read,
            vec!["internal-docs/反映前テストの機械化.md".to_string()]
        );
        assert!(changes.stop.is_empty());
        assert!(
            super::rule_changes(&repo, "no-such-branch").is_none(),
            "git not answering is not 'no changes'"
        );
    }
}
