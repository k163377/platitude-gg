//! The machine held still for a measurement: the one hold a measurement
//! takes on it, and the wait every build here makes on that hold
//! (internal-docs/反映前テストの機械化.md §計測との直列化).
//!
//! A real-window measurement and a build spoil each other, and the CPU
//! gate (`perf::sampler::Limits`) sees a build only after the run it
//! spoiled, so the two are ordered here: `perf` holds the machine still
//! for its runs; a build announces itself and waits for a hold to lift
//! before it begins; a hold waits for the builds already announced.
//!
//! Builds are announced where the compiling happens (`app_build::app_exe` /
//! `app_build::shipped_exe`, `check::run_step`), so a verb that builds need not
//! name itself; verbs heavy without compiling announce themselves. A bare
//! `cargo build` a session types runs outside any verb, so the pre-shell
//! hook holds it (hook/still.rs).
//!
//! Liveness is a file lock beside each note (beside the shared `.git`),
//! held for as long as the note stands: a failing `try_lock` is a live
//! holder, a succeeding one makes the note litter for whoever meets it.
//! The OS releases a killed xtask's locks and no pid is asked about, so a
//! reused pid cannot stand for an ended measurement. Locks are let go of
//! by unlocking ([`crate::locks`]).
//!
//! A step a verb starts is under its parent's announcement ([`UNDER`], set
//! by [`step`]; the container's processes too): a gate's verify-ui step
//! that waited on the hold would wait for a measurement waiting for the
//! gate.
//!
//! Each process stamps when its announcement ended ([`BUILT`]), so a
//! measurement can tell whether another process's build ran between two
//! invocations (`perf::warmth`); one stamp per process keeps its own end
//! from covering somebody else's.

use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use crate::command::{self, Permission, Where};
use crate::locks::Locked;
use crate::note::{field, now_secs};
use crate::subprocess::common_git_dir;
use crate::wait::{Budget, LOOK_AGAIN, TRY_AGAIN, Wait};

pub(crate) static STILL: command::Command = command::Command {
    id: "still.standing",
    call: "still",
    purpose: "whether a measurement holds the machine still, and which builds are under way",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&STILL];

/// Set on every child a verb here starts ([`step`]), so the child neither
/// announces a build its parent already announced nor waits on a hold
/// its parent is being waited for by.
pub(crate) const UNDER: &str = "PGG_STILL_UNDER";

/// The hold's note, beside `.git`.
const HOLD: &str = "pgg-still";

/// The announcements, beside `.git`: one note per announcing thread, and
/// a lock beside each.
const BUSY: &str = "pgg-busy";

/// The stamps, beside `.git`: one per process, the time its last
/// announcement ended, swept after [`STAMP_FOR`] as they are read.
const BUILT: &str = "pgg-built";

const LOCK: &str = "lock";

/// How long a build waits for a hold to lift: a measurement is minutes of
/// runs, so a hold this old is a run that stopped answering. A ceiling
/// only — a hold shows no progress while it stands.
const HOLD_CEILING: Duration = if cfg!(test) {
    Duration::from_secs(20)
} else {
    Duration::from_secs(60 * 60)
};

/// How long a hold waits for the builds already under way. A gate is
/// minutes; a container image built for the first time is more.
const BUSY_CEILING: Duration = if cfg!(test) {
    Duration::from_secs(20)
} else {
    Duration::from_secs(30 * 60)
};

/// How long a name a reader is sweeping is given to come free. A sweep
/// holds the lock for the microseconds of two calls ([`held`],
/// [`announcing`]); a name held past this is a real holder. Generous,
/// because too short refuses a build for the leavings of a dead run.
pub(crate) const SWEEP: Duration = Duration::from_millis(500);

/// How long a stamp is kept: longer than any warm window it could answer
/// (`perf::warmth`), shorter than a pid's turn to come round again.
const STAMP_FOR: u64 = 24 * 60 * 60;

/// Whose name a note's lock file stands at, which decides whether the
/// file comes down with the note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Name {
    /// One every process here opens: the hold. It stays — removed, a
    /// waiter that opened it before would lock a file no longer at the
    /// name while the next holder locks a new one there, two locks that
    /// hold nothing against each other.
    Shared,
    /// One nothing else writes: an announcement. It goes with the note,
    /// and a killed process's is cleared by the next reader
    /// ([`live_notes`]).
    Own,
}

/// A note and its lock, held for as long as the note stands. Dropping it
/// removes the note, and the lock file where the name is [`Name::Own`].
#[derive(Debug)]
struct Held {
    note: PathBuf,
    name: Name,
    /// Last, so the names come down before the lock does: dropped after
    /// the `Drop` below has run.
    _lock: Locked,
}

impl Drop for Held {
    fn drop(&mut self) {
        clear(&self.note);
        if self.name == Name::Own {
            clear(&lock_of(&self.note));
        }
    }
}

/// A hold on the machine, lifted when dropped. Empty where there was
/// nothing to hold against: under a parent's announcement, or outside any
/// repository.
#[derive(Debug)]
pub(crate) struct Hold {
    _held: Option<Held>,
}

/// A build announced, withdrawn when dropped — and stamped as ended
/// ([`BUILT`]). Empty for the reasons a [`Hold`] is, and for a build this
/// thread already announced.
#[derive(Debug)]
pub(crate) struct Busy {
    _announced: Option<Announced>,
}

#[derive(Debug)]
struct Announced {
    _held: Held,
    stamp: PathBuf,
}

impl Drop for Announced {
    fn drop(&mut self) {
        // A plain file where the stamps' directory goes is cleared, so
        // the directory can stand there.
        if let Some(stamps) = self.stamp.parent()
            && stamps.is_file()
        {
            clear(stamps);
        }
        let written = self
            .stamp
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&self.stamp, format!("at {}\n", now_secs())));
        if let Err(error) = written {
            println!(
                "  note: could not stamp the build's end at {} ({error})",
                self.stamp.display()
            );
        }
        ANNOUNCING.with(|announcing| announcing.set(false));
        // The note comes down after this, with `_held`: a hold that sees
        // the note gone sees the stamp too.
    }
}

thread_local! {
    /// Whether this thread has an announcement standing, so a build
    /// inside an announced verb does not announce again. Per thread
    /// because the gate's two sides are two threads, each its own
    /// announcement.
    static ANNOUNCING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// One note per announcement, however many a process makes.
static ANNOUNCEMENTS: AtomicUsize = AtomicUsize::new(0);

/// Holds the machine still for `what`, once the builds already announced
/// have finished. Refused while another hold stands: two measurements
/// spoil each other too.
pub(crate) fn hold(tree: &Path, what: &str) -> Result<Hold, String> {
    match common_dir(tree) {
        Some(common) => hold_in(&common, what, &|| {}),
        None => Ok(Hold { _held: None }),
    }
}

/// Announces a build for `what`, once any hold has lifted.
pub(crate) fn busy(tree: &Path, what: &str) -> Result<Busy, String> {
    match common_dir(tree) {
        Some(common) => busy_in(&common, what, &|| {}),
        None => Ok(Busy { _announced: None }),
    }
}

/// Waits until no measurement holds the machine still, and announces
/// nothing: what a unit does before it takes a ticket from the machine's
/// budget (internal-docs/反映前テストの機械化.md §機械の予算と優先キュー).
/// The announcement still happens inside the step (`busy`), which waits
/// out a hold that arrives in between.
///
/// Silent under a parent's announcement, as `busy` is. Takes the `.git`
/// the caller already knows, since it is asked once per unit
/// (`budget::Pool::admit_once_the_machine_is_free`).
pub(crate) fn until_free_in(common: &Path, what: &str) -> Result<(), String> {
    until_free_in_unless(common, what, &|| false).map(|_free| ())
}

/// The same, given up the moment `stop` says so: `Ok(false)` is a wait
/// that ended with the machine still held (`budget::Pool::admit_unless`).
pub(crate) fn until_free_in_unless(
    common: &Path,
    what: &str,
    stop: &dyn Fn() -> bool,
) -> Result<bool, String> {
    if std::env::var_os(UNDER).is_some() {
        return Ok(true);
    }
    let mut wait = Wait::new(what, Budget::whole(HOLD_CEILING), LOOK_AGAIN);
    let mut said = false;
    wait_for_hold(&common.join(HOLD), what, &mut wait, &mut said, &|| {}, stop)
}

/// This workspace, and the build in it announced: how a verb heavy
/// without compiling opens.
pub(crate) fn announced(what: &str) -> Result<(PathBuf, Busy), String> {
    let root = crate::tree::workspace_root();
    let busy = busy(&root, what)?;
    Ok((root, busy))
}

/// Marks `command` as a step of this verb: under its announcement.
pub(crate) fn step(command: &mut Command) {
    command.env(UNDER, "1");
}

/// Whether an announcement of another process ended after `secs` — a
/// build between two measurements evicts the cache the second would
/// trust (`perf::warmth`). This process's own ends do not count: its
/// build made the exe it measures.
pub(crate) fn build_ended_since(tree: &Path, secs: u64) -> bool {
    common_dir(tree).is_some_and(|common| stamps_ended_since(&common.join(BUILT), secs))
}

/// [`build_ended_since`] over `stamps`, sweeping the ones a day old.
fn stamps_ended_since(stamps: &Path, secs: u64) -> bool {
    let Ok(entries) = std::fs::read_dir(stamps) else {
        return false;
    };
    let now = now_secs();
    let mut ended = false;
    for path in entries.filter_map(Result::ok).map(|entry| entry.path()) {
        let pid: u32 = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.parse().ok())
            .unwrap_or(0);
        let at: u64 = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| field(&text, "at ")?.parse().ok())
            .unwrap_or(0);
        if at + STAMP_FOR < now {
            clear(&path);
            continue;
        }
        if pid != std::process::id() && at > secs {
            ended = true;
        }
    }
    ended
}

/// The hold standing over the repository `cwd` is in, as a line for the
/// hook's refusal, or nothing.
pub(crate) fn standing(cwd: &str) -> Option<String> {
    let common = common_dir(Path::new(cwd))?;
    held(&common.join(HOLD), "a measurement")
        .ok()
        .flatten()
        .map(|note| note.line())
}

/// `cargo xtask still`: what stands right now, for a session deciding
/// whether to wait.
pub fn run(args: &[String]) -> Result<(), String> {
    if !args.is_empty() {
        return Err(format!("still takes no arguments (got {args:?})"));
    }
    let common = common_dir(&crate::tree::workspace_root())
        .ok_or("not inside a repository — there is nothing to hold still")?;
    match held(&common.join(HOLD), "a measurement")? {
        Some(note) => println!("held still by {}", note.line()),
        None => println!("nothing holds the machine still"),
    }
    let under_way = live_notes(&common.join(BUSY));
    if under_way.is_empty() {
        println!("no build under way (of the ones cargo xtask verbs announce)");
    }
    for note in under_way {
        println!("under way: {}", note.line());
    }
    Ok(())
}

/// The repository's shared `.git` — the one place every seat sees a hold.
/// None under a parent's announcement.
fn common_dir(tree: &Path) -> Option<PathBuf> {
    if std::env::var_os(UNDER).is_some() {
        return None;
    }
    common_git_dir(&crate::seats::slashed(tree)).map(PathBuf::from)
}

fn hold_in(common: &Path, what: &str, polled: &dyn Fn()) -> Result<Hold, String> {
    let note = common.join(HOLD);
    let lock = open_lock(&lock_of(&note))?;
    // Tried for a sweep's span ([`held`]): a lock held past it is a
    // measurement's.
    let mut tries = Wait::new(what, Budget::whole(SWEEP), TRY_AGAIN);
    loop {
        match lock.try_lock() {
            Ok(()) => break,
            Err(TryLockError::WouldBlock) => {
                if tries.look_again("the hold's lock").is_err() {
                    let other =
                        read_note(&note).unwrap_or_else(|| Note::unreadable("a measurement"));
                    return Err(format!(
                        "another measurement holds the machine still: {} — one at a time, or \
                         the two spoil each other. `cargo xtask still` says when it is gone.",
                        other.line()
                    ));
                }
            }
            Err(TryLockError::Error(error)) => {
                return Err(format!(
                    "could not lock the hold at {}: {error}",
                    note.display()
                ));
            }
        }
    }
    std::fs::write(&note, Note::now(what).text())
        .map_err(|e| format!("could not write the hold at {}: {e}", note.display()))?;
    // Made before the wait, so a wait that fails lifts it.
    let hold = Hold {
        _held: Some(Held {
            note,
            name: Name::Shared,
            _lock: Locked::new(lock),
        }),
    };
    wait_for_builds(&common.join(BUSY), what, polled)?;
    Ok(hold)
}

fn wait_for_builds(busy: &Path, what: &str, polled: &dyn Fn()) -> Result<(), String> {
    let mut wait = Wait::new(
        format!("{what} holding the machine still"),
        Budget::whole(BUSY_CEILING),
        LOOK_AGAIN,
    );
    // Said when the set of builds changes — by which builds they are, not
    // by their lines, which carry an age that moves on its own.
    let mut said: Vec<(u32, u64)> = Vec::new();
    loop {
        let under_way = live_notes(busy);
        if under_way.is_empty() {
            return Ok(());
        }
        let lines: Vec<String> = under_way.iter().map(Note::line).collect();
        let builds: Vec<(u32, u64)> = under_way
            .iter()
            .map(|note| (note.pid, note.since))
            .collect();
        if builds != said {
            println!(
                "  {what} holds the machine still, and waits for {} build(s) already under way: {}",
                lines.len(),
                lines.join(", ")
            );
            said = builds;
        }
        wait.saw(lines.join(", "));
        polled();
        wait.look_again("the end of the builds under way")
            .map_err(|expired| {
                format!("{expired} — the hold is lifted; `cargo xtask still` names them")
            })?;
    }
}

fn busy_in(common: &Path, what: &str, polled: &dyn Fn()) -> Result<Busy, String> {
    if ANNOUNCING.with(|announcing| announcing.replace(true)) {
        return Ok(Busy { _announced: None });
    }
    let announced = announce(common, what, polled);
    if announced.is_err() {
        ANNOUNCING.with(|announcing| announcing.set(false));
    }
    announced.map(|announced| Busy {
        _announced: Some(announced),
    })
}

fn announce(common: &Path, what: &str, polled: &dyn Fn()) -> Result<Announced, String> {
    let hold = common.join(HOLD);
    let busy = common.join(BUSY);
    // One wait across the loop: a hold that came between the wait and
    // the announcement is waited out on the same budget.
    let mut wait = Wait::new(what, Budget::whole(HOLD_CEILING), LOOK_AGAIN);
    let mut said = false;
    loop {
        wait_for_hold(&hold, what, &mut wait, &mut said, polled, &|| false)?;
        std::fs::create_dir_all(&busy)
            .map_err(|e| format!("could not make {}: {e}", busy.display()))?;
        let note = busy.join(format!(
            "{}-{}",
            std::process::id(),
            ANNOUNCEMENTS.fetch_add(1, Ordering::SeqCst)
        ));
        let lock = lock_beside(&note)?;
        std::fs::write(&note, Note::now(what).text())
            .map_err(|e| format!("could not announce the build at {}: {e}", note.display()))?;
        let mine = Held {
            note,
            name: Name::Own,
            _lock: lock,
        };
        // A hold that came between the wait and the announcement wins.
        if held(&hold, "a measurement")?.is_none() {
            return Ok(Announced {
                _held: mine,
                stamp: common.join(BUILT).join(std::process::id().to_string()),
            });
        }
        drop(mine);
    }
}

/// The lock beside `note`, held and still at its own name. A lock file
/// here can be a dead run's that a reader is sweeping under its lock
/// ([`announcing`]), so a file found held, or held after the name went,
/// is opened again (rules-refs/core.md「lock ファイルは握ったまま消し」).
fn lock_beside(note: &Path) -> Result<Locked, String> {
    lock_beside_polled(note, &|| {})
}

fn lock_beside_polled(note: &Path, polled: &dyn Fn()) -> Result<Locked, String> {
    let path = lock_of(note);
    let mut tries = Wait::new(
        format!("the announcement at {}", note.display()),
        Budget::whole(SWEEP),
        TRY_AGAIN,
    );
    loop {
        let lock = open_lock(&path)?;
        match lock.try_lock() {
            Ok(()) => {
                let lock = Locked::new(lock);
                if path.exists() {
                    return Ok(lock);
                }
                // The name went while this held the file: opened again.
                tries.saw("the lock granted on a file no longer at the name");
            }
            Err(TryLockError::WouldBlock) => tries.saw("the lock held"),
            Err(TryLockError::Error(error)) => {
                return Err(format!(
                    "could not lock the announcement at {}: {error}",
                    note.display()
                ));
            }
        }
        polled();
        tries
            .look_again("the lock at its own name")
            .map_err(|expired| {
                format!(
                    "could not announce the build: {expired} — a reader is sweeping a dead run's \
                 leavings there, and is not letting go"
                )
            })?;
    }
}

/// Waits until no live hold stands at `hold`, clearing a dead one — or
/// until `stop` says so, which answers `false`.
fn wait_for_hold(
    hold: &Path,
    what: &str,
    wait: &mut Wait,
    said: &mut bool,
    polled: &dyn Fn(),
    stop: &dyn Fn() -> bool,
) -> Result<bool, String> {
    loop {
        let Some(other) = held(hold, "a measurement")? else {
            return Ok(true);
        };
        if stop() {
            return Ok(false);
        }
        if !*said {
            println!(
                "  a measurement holds the machine still ({}) — {what} waits for it to end",
                other.line()
            );
            *said = true;
        }
        wait.saw(other.line());
        polled();
        wait.look_again("the end of the measurement holding the machine still")
            .map_err(|expired| format!("{expired} — look at it before building beside it"))?;
    }
}

/// Who holds the lock beside `note`, or nobody. A note nobody holds is
/// litter, taken down while this holds the lock, so a holder arriving
/// now writes its note after the removal. `what` names the writer while
/// the lock is held and the note not yet written.
fn held(note: &Path, what: &str) -> Result<Option<Note>, String> {
    if !note.exists() {
        return Ok(None);
    }
    let lock = open_lock(&lock_of(note))?;
    match lock.try_lock() {
        Ok(()) => {
            // Unlocked, not just closed, at the end of this arm, so
            // others wait out these two calls ([`SWEEP`]) and not a fork.
            let _lock = Locked::new(lock);
            if let Err(error) = std::fs::remove_file(note)
                && error.kind() != std::io::ErrorKind::NotFound
            {
                return Err(format!(
                    "could not clear the note at {} that nothing holds: {error}",
                    note.display()
                ));
            }
            Ok(None)
        }
        Err(TryLockError::WouldBlock) => Ok(Some(
            read_note(note).unwrap_or_else(|| Note::unreadable(what)),
        )),
        Err(TryLockError::Error(error)) => Err(format!(
            "could not probe the lock at {}: {error}",
            note.display()
        )),
    }
}

/// The builds under way, litter cleared as it is met — a note nobody
/// holds, and a lock file whose note is gone.
fn live_notes(busy: &Path) -> Vec<Note> {
    let Ok(entries) = std::fs::read_dir(busy) else {
        return Vec::new();
    };
    let mut live = Vec::new();
    for path in entries.filter_map(Result::ok).map(|entry| entry.path()) {
        if path.extension().is_some() {
            if !path.with_extension("").exists()
                && let Some(note) = announcing(&path)
            {
                live.push(note);
            }
            continue;
        }
        if let Ok(Some(note)) = held(&path, "a build") {
            live.push(note);
        }
    }
    live.sort_by_key(|note| (note.pid, note.since));
    live
}

/// A lock file whose note is not there: a build announcing itself this
/// instant ([`announce`] locks before it writes), or a killed process's
/// leavings. One somebody holds is a build under way — missing it would
/// let a hold stand beside it. One nobody holds is litter, taken down
/// while this holds the lock, as [`held`] does
/// (rules-refs/core.md「lock ファイルは握ったまま消し」).
fn announcing(lock: &Path) -> Option<Note> {
    let file = File::options().read(true).write(true).open(lock).ok()?;
    match file.try_lock() {
        Ok(()) => {
            let _file = Locked::new(file);
            clear(lock);
            None
        }
        Err(TryLockError::WouldBlock) => Some(Note::unreadable("a build")),
        // Left for a reader that can probe it.
        Err(TryLockError::Error(_)) => None,
    }
}

/// Removes a file that may already be gone, and says so when it would
/// not go for any other reason.
fn clear(path: &Path) {
    if let Err(error) = std::fs::remove_file(path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        println!(
            "  note: could not clear {} ({error}) — the next verb tries again",
            path.display()
        );
    }
}

fn open_lock(path: &Path) -> Result<File, String> {
    File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| format!("could not open the lock at {}: {e}", path.display()))
}

fn lock_of(note: &Path) -> PathBuf {
    note.with_extension(LOCK)
}

fn read_note(path: &Path) -> Option<Note> {
    Note::parse(&std::fs::read_to_string(path).ok()?)
}

/// One note, as a file records it: the process, when it began, and what
/// it is doing.
pub(crate) struct Note {
    pid: u32,
    since: u64,
    what: String,
}

impl Note {
    pub(crate) fn now(what: &str) -> Self {
        Self {
            pid: std::process::id(),
            since: now_secs(),
            what: what.to_string(),
        }
    }

    /// A lock somebody holds beside a note not yet written, or not
    /// written the way this reads it. `what` is the asker's word for
    /// whoever writes this kind of note.
    pub(crate) fn unreadable(what: &str) -> Self {
        Self {
            pid: 0,
            since: now_secs(),
            what: format!("{what} whose note is not written yet"),
        }
    }

    /// Whose note this is — never whether its holder is alive, which is
    /// the lock's answer (`linux::runner::owned_by_the_gate`).
    pub(crate) fn pid(&self) -> u32 {
        self.pid
    }

    pub(crate) fn text(&self) -> String {
        format!(
            "pid {}\nsince {}\nwhat {}\n",
            self.pid, self.since, self.what
        )
    }

    pub(crate) fn parse(text: &str) -> Option<Self> {
        Some(Self {
            pid: field(text, "pid ")?.parse().ok()?,
            since: field(text, "since ")?.parse().ok()?,
            what: field(text, "what ")?.to_string(),
        })
    }

    /// The note as a person reads it: what, whose, for how long.
    pub(crate) fn line(&self) -> String {
        let age = Duration::from_secs(now_secs().saturating_sub(self.since));
        format!(
            "{} (pid {}, for {})",
            self.what,
            self.pid,
            crate::seats::format_age(Some(age))
        )
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Mutex;

    use super::{
        BUILT, BUSY, HOLD, Note, STAMP_FOR, busy_in, hold_in, live_notes, lock_beside_polled,
        lock_of, open_lock, stamps_ended_since,
    };
    use crate::yard::Yard;

    /// A `.git`-shaped directory of this test's own, gone when the test is.
    fn common(name: &str) -> Yard {
        Yard::new(&format!("still-{name}"))
    }

    /// Every look the wait under test takes is sent on the channel.
    fn polls() -> (std::sync::mpsc::Receiver<()>, impl Fn()) {
        let (said, looks) = std::sync::mpsc::channel();
        (looks, move || {
            let _ = said.send(());
        })
    }

    /// Waits for the first look: a wait that has looked once found the
    /// other side up.
    fn until_polled(looks: &std::sync::mpsc::Receiver<()>) {
        crate::wait::heard("the wait under test", "a look", looks);
    }

    /// The files standing under the announcements: notes, and lock files.
    fn standing(dir: &std::path::Path) -> (usize, usize) {
        std::fs::read_dir(dir.join(BUSY))
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .fold((0, 0), |(notes, locks), entry| {
                        if entry.path().extension().is_some() {
                            (notes, locks + 1)
                        } else {
                            (notes + 1, locks)
                        }
                    })
            })
            .unwrap_or((0, 0))
    }

    #[test]
    fn a_hold_waits_for_the_builds_under_way() {
        let dir = common("hold-waits");
        let build = busy_in(&dir, "gate", &|| {}).expect("a build announced");
        let (count, polled) = polls();
        let held_in = dir.to_path_buf();
        let held = std::thread::spawn(move || hold_in(&held_in, "perf", &polled).map(|_| ()));
        // The hold polled while the build stood: it was waiting for it.
        until_polled(&count);
        assert!(!held.is_finished());
        drop(build);
        held.join()
            .expect("the hold's thread")
            .expect("the hold, once the build is gone");
        assert!(!dir.join(HOLD).exists(), "a hold is lifted with its guard");
        assert!(
            dir.join(BUILT)
                .join(std::process::id().to_string())
                .exists(),
            "the build that ended stamped its end under its pid"
        );
    }

    #[test]
    fn a_build_waits_for_a_hold_to_lift() {
        let dir = common("build-waits");
        let hold = hold_in(&dir, "perf", &|| {}).expect("the hold");
        let (count, polled) = polls();
        let building_in = dir.to_path_buf();
        let build = std::thread::spawn(move || busy_in(&building_in, "check", &polled).map(|_| ()));
        until_polled(&count);
        assert!(!build.is_finished());
        drop(hold);
        build
            .join()
            .expect("the build's thread")
            .expect("the build, once the hold lifted");
        assert_eq!(
            standing(&dir),
            (0, 0),
            "an announcement is withdrawn with its guard, lock file included"
        );
    }

    /// Binds `Name::Shared`: a hold's lock file removed with the hold
    /// would let two measurements run at once.
    #[test]
    fn the_lock_a_hold_frees_is_the_one_the_next_hold_is_refused_by() {
        let dir = common("one-hold");
        let hold = hold_in(&dir, "perf", &|| {}).expect("the hold");
        // A waiter that opened the lock while the hold still stood.
        let waiter = open_lock(&lock_of(&dir.join(HOLD))).expect("the lock beside the hold");
        drop(hold);
        waiter
            .try_lock()
            .expect("the lock the lifted hold let go of");
        let refused = hold_in(&dir, "another perf", &|| {}).expect_err("a second measurement");
        assert!(refused.contains("another measurement"), "{refused}");
        drop(waiter);
    }

    /// What the unlock is for: a child handed the hold's lock description
    /// (as a fork would be) must not keep the lock past the hold — a
    /// close would. Linux only, where `flock(2)` promises the inheritance;
    /// the gate's lock has the same net (`lanes`, `gate::stamps`).
    #[test]
    #[cfg(target_os = "linux")]
    fn a_lock_let_go_of_is_free_though_a_forked_child_holds_the_description() {
        use std::process::{Command, Stdio};

        let dir = common("carried");
        let hold = hold_in(&dir, "perf", &|| {}).expect("the hold");
        let waiter = open_lock(&lock_of(&dir.join(HOLD))).expect("the lock beside the hold");
        // Handed the description as its stdin, the child holds a copy of
        // it for as long as it lives — past the drop below.
        let mut carrier = Command::new("sleep")
            .arg("5")
            .stdin(Stdio::from(
                hold._held
                    .as_ref()
                    .expect("the hold's guard")
                    ._lock
                    .handle()
                    .try_clone()
                    .expect("a second handle on the description"),
            ))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("a child handed the lock's description");
        drop(hold);
        waiter
            .try_lock()
            .expect("the lock the lifted hold unlocked");
        assert!(
            carrier.try_wait().expect("ask after the child").is_none(),
            "the child let the description go before the lock was asked for"
        );
        carrier.kill().expect("the child that carried it");
        carrier.wait().expect("the child that carried it");
        drop(waiter);
    }

    /// A killed xtask leaves its note but not its lock: such a note, and a
    /// lock file whose note is gone, are cleared by whoever meets them.
    #[test]
    fn a_note_nobody_holds_the_lock_beside_is_litter() {
        let dir = common("litter");
        let dead = Note {
            pid: 1,
            since: 0,
            what: "perf".into(),
        };
        std::fs::write(dir.join(HOLD), dead.text()).expect("a dead hold");
        let build = busy_in(&dir, "check", &|| {}).expect("a dead hold holds nothing");
        assert!(!dir.join(HOLD).exists());
        drop(build);
        std::fs::create_dir_all(dir.join(BUSY)).expect("the busy directory");
        std::fs::write(dir.join(BUSY).join("1-0"), dead.text()).expect("a dead announcement");
        std::fs::write(dir.join(BUSY).join("1-9.lock"), "").expect("an orphaned lock file");
        let _hold = hold_in(&dir, "perf", &|| {}).expect("a dead build is not waited for");
        assert!(!dir.join(BUSY).join("1-0").exists());
        assert!(!dir.join(BUSY).join("1-9.lock").exists());
    }

    #[test]
    fn an_announcement_not_yet_readable_is_named_a_build() {
        let dir = common("half-written");
        std::fs::create_dir_all(dir.join(BUSY)).expect("the busy directory");
        let note = dir.join(BUSY).join("1-0");
        std::fs::write(&note, "half a note").expect("a note mid-write");
        let held = open_lock(&lock_of(&note)).expect("the lock beside it");
        held.try_lock().expect("held, as its announcer holds it");
        let under_way = live_notes(&dir.join(BUSY));
        assert_eq!(under_way.len(), 1, "the locked announcement was dropped");
        assert!(
            under_way[0]
                .line()
                .starts_with("a build whose note is not written yet"),
            "{}",
            under_way[0].line()
        );
        drop(held);
    }

    /// The lock is taken and the note not yet written. Taking that lock
    /// file down would leave the note written next with no lock beside
    /// it, cleared by the next reader as a dead process's.
    #[test]
    fn a_lock_file_held_beside_no_note_is_a_build_announcing_itself() {
        let dir = common("announcing");
        std::fs::create_dir_all(dir.join(BUSY)).expect("the busy directory");
        let lock = dir.join(BUSY).join("1-0.lock");
        let held = open_lock(&lock).expect("the lock an announcer takes before its note");
        held.try_lock().expect("held, as its announcer holds it");
        let under_way = live_notes(&dir.join(BUSY));
        assert_eq!(under_way.len(), 1, "the announcement was not counted");
        assert!(
            under_way[0]
                .line()
                .starts_with("a build whose note is not written yet"),
            "{}",
            under_way[0].line()
        );
        assert!(
            lock.exists(),
            "the lock file was taken from under the announcer"
        );
        drop(held);
    }

    /// A lock file at an announcement's name can be a dead run's (a pid
    /// comes round, and every run's first announcement is numbered the
    /// same), held by the reader sweeping it. The announcement waits that
    /// out rather than refusing a build for a gone run's leavings.
    #[test]
    fn an_announcement_waits_out_the_reader_sweeping_its_name() {
        let dir = common("swept-name");
        std::fs::create_dir_all(dir.join(BUSY)).expect("the busy directory");
        let note = dir.join(BUSY).join("1-0");
        let sweeping = open_lock(&lock_of(&note)).expect("a dead run's lock file");
        sweeping
            .try_lock()
            .expect("held, as the reader sweeping it holds it");
        let sweeping = Mutex::new(Some(sweeping));
        let lock = lock_beside_polled(&note, &|| {
            // Swept as `announcing` does, on the first look again, so the
            // try after it is what is under test.
            if let Some(held) = sweeping
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .take()
            {
                let _ = std::fs::remove_file(lock_of(&note));
                drop(held);
            }
        })
        .expect("the lock at the announcement's own name");
        assert!(
            lock_of(&note).exists(),
            "the announcement holds a lock that is not at its name"
        );
        drop(lock);
    }

    #[test]
    fn a_second_hold_is_refused_while_the_first_stands() {
        let dir = common("two-holds");
        let _first = hold_in(&dir, "perf", &|| {}).expect("the first hold");
        let refused = hold_in(&dir, "perf again", &|| {}).expect_err("two measurements at once");
        assert!(refused.contains("another measurement"), "{refused}");
        assert!(refused.contains("perf (pid"), "{refused}");
    }

    #[test]
    fn a_build_inside_an_announced_verb_does_not_announce_again() {
        let dir = common("nested");
        let outer = busy_in(&dir, "verify-ui", &|| {}).expect("the verb's announcement");
        let inner = busy_in(&dir, "cargo build", &|| {}).expect("the compile inside it");
        assert!(inner._announced.is_none(), "nothing of its own");
        assert_eq!(standing(&dir).0, 1);
        drop(inner);
        assert_eq!(standing(&dir).0, 1, "the inner drop withdraws nothing");
        drop(outer);
        assert_eq!(standing(&dir), (0, 0));
    }

    #[test]
    fn only_another_processes_build_counts_as_ended_since() {
        let dir = common("stamps");
        let stamps = dir.join(BUILT);
        std::fs::create_dir_all(&stamps).expect("the stamp directory");
        let now = crate::note::now_secs();
        let write = |pid: u32, at: u64| {
            std::fs::write(stamps.join(pid.to_string()), format!("at {at}\n")).expect("a stamp");
        };
        write(std::process::id(), now);
        assert!(
            !stamps_ended_since(&stamps, now - 60),
            "this process's own build is not another's"
        );
        write(1, now - 30);
        assert!(stamps_ended_since(&stamps, now - 60));
        assert!(!stamps_ended_since(&stamps, now));
        write(2, now - STAMP_FOR - 1);
        assert!(
            !stamps_ended_since(&stamps, now),
            "a stamp a day old is swept"
        );
        assert!(!stamps.join("2").exists());
    }

    #[test]
    fn a_file_where_the_stamps_directory_goes_is_cleared() {
        let dir = common("stamp-file");
        std::fs::write(dir.join(BUILT), "pid 1\nat 0\n").expect("a file in the directory's place");
        drop(busy_in(&dir, "gate", &|| {}).expect("a build announced"));
        assert!(
            dir.join(BUILT)
                .join(std::process::id().to_string())
                .is_file(),
            "the stamp is under the directory"
        );
    }

    #[test]
    fn a_note_reads_back_as_it_was_written() {
        let note = Note {
            pid: 7,
            since: 11,
            what: "cargo xtask perf".into(),
        };
        let back = Note::parse(&note.text()).expect("a whole note parses");
        assert_eq!(
            (back.pid, back.since, back.what.as_str()),
            (7, 11, "cargo xtask perf")
        );
        assert!(Note::parse("something else").is_none());
        assert_eq!(
            lock_of(&PathBuf::from("C:/x/.git/pgg-still")),
            PathBuf::from("C:/x/.git/pgg-still.lock")
        );
        assert_eq!(
            lock_of(&PathBuf::from("C:/x/.git/pgg-busy/12-3")),
            PathBuf::from("C:/x/.git/pgg-busy/12-3.lock")
        );
    }
}
