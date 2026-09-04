//! The machine held still for a measurement: the one hold a measurement
//! takes on it, and the wait every build here makes on that hold.
//!
//! A real-window measurement and a build cannot share a machine. The
//! build shows up in the run's host conditions as load that was not the
//! application, and the run is refused and taken again
//! (`perf::sampler::Limits`); the measurement's window and its
//! vsync-stepped bench slow the build in turn. The CPU gate sees that a
//! build happened only after the run it spoiled, and on a twenty-four
//! thread machine it cannot tell a build from a browser by the counters
//! either. What it cannot do is *order* the two, and that is what this
//! does: `perf` holds the machine still for the length of its runs; a
//! build announces itself and waits for a hold to lift before it begins;
//! a hold waits for the builds already announced to finish.
//!
//! **Announced where the compiling happens**, not per verb: every app
//! build goes through `tree::app_exe` / `tree::shipped_exe` and every
//! cargo step of `check` and `gate` through `check::run_step`, so a verb
//! that builds is announced without naming itself — `perf`'s own build
//! included. The verbs that are heavy without compiling say so
//! themselves (`verify-ui`, the `linux` container, the corpus). The
//! pre-shell hook holds a bare `cargo build` a session types while a
//! hold stands, because that one runs outside any verb that could wait
//! (hook/still.rs).
//!
//! **Liveness is a file lock, not a pid.** The hold and each
//! announcement are a note beside the repository's own `.git` — which
//! every worktree shares — and a lock file beside the note, held open by
//! the process for as long as the note stands. A `try_lock` that fails is
//! a live holder; one that succeeds is nobody, and the note beside it is
//! litter cleared by whoever meets it. A killed xtask never unwinds, but
//! the operating system releases its locks, so nothing is ever waited for
//! that is not there — and no pid is asked about, so a pid handed to
//! somebody else cannot stand for a measurement that ended.
//!
//! **A step a verb starts is under its parent's announcement**, and says
//! nothing of its own ([`UNDER`], set by [`step`] on every child a verb
//! runs): a gate's verify-ui step that waited on the hold would wait for
//! a measurement that is waiting for the gate. The container's processes
//! are under it for the same reason, and because they are not this
//! machine's.
//!
//! **What ended when** is left as one more note ([`BUILT`]): the last
//! announcement to end stamps its process and the time, which is how a
//! measurement knows whether a build ran between two invocations
//! (`perf::warmth`).

use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use crate::note::{field, now_secs};
use crate::subprocess::common_git_dir;

/// Set on every child a verb here starts ([`step`]), so the child neither
/// announces a build its parent already announced nor waits on a hold
/// its parent is being waited for by.
pub(crate) const UNDER: &str = "PG_STILL_UNDER";

/// The hold's note, beside `.git`; its lock is the same name with
/// [`LOCK`] for an extension.
const HOLD: &str = "pg-still";

/// The announcements, beside `.git`: one note per announcing thread, and
/// a lock beside each.
const BUSY: &str = "pg-busy";

/// The stamp the last announcement to end leaves: its process and when.
const BUILT: &str = "pg-built";

/// The extension of the lock file beside a note.
const LOCK: &str = "lock";

/// How long a build waits for a hold to lift. A measurement is minutes of
/// runs, retries included; a hold this old is a run that stopped
/// answering, and the message names its process.
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

/// How often either side looks again. Short under test, where the waits
/// are measured in the hundreds of milliseconds.
const POLL: Duration = if cfg!(test) {
    Duration::from_millis(50)
} else {
    Duration::from_secs(5)
};

/// How many times a hold tries its lock before calling the holder
/// another measurement. A waiter probing the lock holds it for the
/// microseconds between its `try_lock` and its `drop`, and a hold that
/// met that instant is not a hold that met a measurement.
const HOLD_TRIES: u32 = 5;

/// A note and its lock, held for as long as the note stands. Dropping it
/// removes the note; the lock goes with the handle.
#[derive(Debug)]
struct Held {
    note: PathBuf,
    _lock: File,
}

impl Drop for Held {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_file(&self.note)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            println!(
                "  note: could not take the note at {} down ({error}) — the next verb clears \
                 it, its lock is gone",
                self.note.display()
            );
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

/// A build announced, withdrawn when dropped — and stamped as the last to
/// end ([`BUILT`]). Empty for the reasons a [`Hold`] is, and for a build
/// this thread already announced: the verb's announcement covers the
/// compile inside it.
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
        let stamp = format!("pid {}\nat {}\n", std::process::id(), now_secs());
        if let Err(error) = std::fs::write(&self.stamp, stamp) {
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
    /// rather than per process: the gate runs its two sides in two
    /// threads, and each is its own announcement.
    static ANNOUNCING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// One note per announcement, however many a process makes.
static ANNOUNCEMENTS: AtomicUsize = AtomicUsize::new(0);

/// Holds the machine still for `what`, once the builds already announced
/// have finished. Refused while another hold stands: two measurements
/// spoil each other as surely as a build spoils one.
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

/// This workspace, and the build in it announced: the two lines a verb
/// that is heavy without compiling opens with, as one.
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
/// build that ran between two measurements, which is what evicts the
/// cache the second one would otherwise trust (`perf::warmth`). This
/// process's own ends do not count: a measurement's own build is what
/// made the exe it measures.
pub(crate) fn build_ended_since(tree: &Path, secs: u64) -> bool {
    let Some(common) = common_dir(tree) else {
        return false;
    };
    let Ok(text) = std::fs::read_to_string(common.join(BUILT)) else {
        return false;
    };
    let pid: u32 = field(&text, "pid ")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let at: u64 = field(&text, "at ")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    pid != std::process::id() && at > secs
}

/// The hold standing over the repository `cwd` is in, as a line for the
/// hook's refusal, or nothing.
pub(crate) fn standing(cwd: &str) -> Option<String> {
    let common = common_dir(Path::new(cwd))?;
    held(&common.join(HOLD))
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
    match held(&common.join(HOLD))? {
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

/// The repository's shared `.git`, which every worktree of it resolves to
/// the same — and so the one place a hold is seen from every seat. None
/// under a parent's announcement, where there is nothing to say.
fn common_dir(tree: &Path) -> Option<PathBuf> {
    if std::env::var_os(UNDER).is_some() {
        return None;
    }
    common_git_dir(&crate::seats::slashed(tree)).map(PathBuf::from)
}

fn hold_in(common: &Path, what: &str, polled: &dyn Fn()) -> Result<Hold, String> {
    let note = common.join(HOLD);
    let lock = open_lock(&lock_of(&note))?;
    let mut tries = 0;
    loop {
        match lock.try_lock() {
            Ok(()) => break,
            Err(TryLockError::WouldBlock) => {
                tries += 1;
                if tries >= HOLD_TRIES {
                    let other = read_note(&note).unwrap_or_else(Note::unreadable);
                    return Err(format!(
                        "another measurement holds the machine still: {} — one at a time, or \
                         the two spoil each other. `cargo xtask still` says when it is gone.",
                        other.line()
                    ));
                }
                std::thread::sleep(Duration::from_millis(100));
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
    // Dropped on the way out of a wait that failed, so a hold that never
    // got its quiet does not stand in everybody's way.
    let hold = Hold {
        _held: Some(Held { note, _lock: lock }),
    };
    wait_for_builds(&common.join(BUSY), what, polled)?;
    Ok(hold)
}

/// Waits until no announced build is still running.
fn wait_for_builds(busy: &Path, what: &str, polled: &dyn Fn()) -> Result<(), String> {
    let started = Instant::now();
    let mut said: Vec<String> = Vec::new();
    loop {
        let under_way = live_notes(busy);
        if under_way.is_empty() {
            return Ok(());
        }
        let lines: Vec<String> = under_way.iter().map(Note::line).collect();
        if lines != said {
            println!(
                "  {what} holds the machine still, and waits for {} build(s) already under way: {}",
                lines.len(),
                lines.join(", ")
            );
            said = lines;
        }
        if started.elapsed() >= BUSY_CEILING {
            return Err(format!(
                "the builds under way did not finish within {} minutes — the hold is lifted; \
                 `cargo xtask still` names them",
                BUSY_CEILING.as_secs() / 60
            ));
        }
        polled();
        std::thread::sleep(POLL);
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
    let started = Instant::now();
    let mut said = false;
    loop {
        wait_for_hold(&hold, what, started, &mut said, polled)?;
        std::fs::create_dir_all(&busy)
            .map_err(|e| format!("could not make {}: {e}", busy.display()))?;
        let note = busy.join(format!(
            "{}-{}",
            std::process::id(),
            ANNOUNCEMENTS.fetch_add(1, Ordering::SeqCst)
        ));
        let lock = open_lock(&lock_of(&note))?;
        if let Err(error) = lock.try_lock() {
            return Err(format!(
                "could not lock the announcement at {}: {error}",
                note.display()
            ));
        }
        std::fs::write(&note, Note::now(what).text())
            .map_err(|e| format!("could not announce the build at {}: {e}", note.display()))?;
        let mine = Held { note, _lock: lock };
        // A hold that came between the wait and the announcement wins:
        // the measurement is the one that cannot share.
        if held(&hold)?.is_none() {
            return Ok(Announced {
                _held: mine,
                stamp: common.join(BUILT),
            });
        }
        drop(mine);
    }
}

/// Waits until no live hold stands at `hold`, clearing a dead one.
fn wait_for_hold(
    hold: &Path,
    what: &str,
    started: Instant,
    said: &mut bool,
    polled: &dyn Fn(),
) -> Result<(), String> {
    loop {
        let Some(other) = held(hold)? else {
            return Ok(());
        };
        if !*said {
            println!(
                "  a measurement holds the machine still ({}) — {what} waits for it to end",
                other.line()
            );
            *said = true;
        }
        if started.elapsed() >= HOLD_CEILING {
            return Err(format!(
                "the measurement did not end within {} minutes ({}) — look at it before \
                 building beside it",
                HOLD_CEILING.as_secs() / 60,
                other.line()
            ));
        }
        polled();
        std::thread::sleep(POLL);
    }
}

/// Who holds the lock beside `note`, or nobody. A note whose lock nobody
/// holds is litter, and is taken down here — while this probe holds the
/// lock, so a holder arriving this instant writes its note after the
/// removal and not before.
fn held(note: &Path) -> Result<Option<Note>, String> {
    if !note.exists() {
        return Ok(None);
    }
    let lock = open_lock(&lock_of(note))?;
    match lock.try_lock() {
        Ok(()) => {
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
        Err(TryLockError::WouldBlock) => Ok(Some(read_note(note).unwrap_or_else(Note::unreadable))),
        Err(TryLockError::Error(error)) => Err(format!(
            "could not probe the lock at {}: {error}",
            note.display()
        )),
    }
}

/// The builds under way, litter cleared as it is met.
fn live_notes(busy: &Path) -> Vec<Note> {
    let Ok(entries) = std::fs::read_dir(busy) else {
        return Vec::new();
    };
    let mut live: Vec<Note> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_none())
        .filter_map(|path| held(&path).ok().flatten())
        .collect();
    live.sort_by_key(|note| (note.pid, note.since));
    live
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
struct Note {
    pid: u32,
    since: u64,
    what: String,
}

impl Note {
    fn now(what: &str) -> Self {
        Self {
            pid: std::process::id(),
            since: now_secs(),
            what: what.to_string(),
        }
    }

    /// A lock somebody holds beside a note not yet written, or not
    /// written the way this reads it.
    fn unreadable() -> Self {
        Self {
            pid: 0,
            since: now_secs(),
            what: "a measurement whose note is not written yet".to_string(),
        }
    }

    fn text(&self) -> String {
        format!(
            "pid {}\nsince {}\nwhat {}\n",
            self.pid, self.since, self.what
        )
    }

    fn parse(text: &str) -> Option<Self> {
        Some(Self {
            pid: field(text, "pid ")?.parse().ok()?,
            since: field(text, "since ")?.parse().ok()?,
            what: field(text, "what ")?.to_string(),
        })
    }

    /// The note as a person reads it: what, whose, for how long.
    fn line(&self) -> String {
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
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use super::{BUILT, BUSY, HOLD, Note, busy_in, hold_in, lock_of};

    /// A `.git`-shaped directory of this test's own.
    fn common(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pg-still-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a directory to hold in");
        dir
    }

    /// A counter the waits under test bump on every poll, so a wait is
    /// proved by the poll that happened rather than by a clock.
    fn polls() -> (Arc<AtomicUsize>, impl Fn()) {
        let count = Arc::new(AtomicUsize::new(0));
        let bump = Arc::clone(&count);
        (count, move || {
            bump.fetch_add(1, Ordering::SeqCst);
        })
    }

    fn until_polled(count: &AtomicUsize) {
        while count.load(Ordering::SeqCst) == 0 {
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// The announcements standing in `dir`: the notes, not their locks.
    fn announcements(dir: &std::path::Path) -> usize {
        std::fs::read_dir(dir.join(BUSY))
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter(|entry| entry.path().extension().is_none())
                    .count()
            })
            .unwrap_or(0)
    }

    #[test]
    fn a_hold_waits_for_the_builds_under_way() {
        let dir = common("hold-waits");
        let build = busy_in(&dir, "gate", &|| {}).expect("a build announced");
        let (count, polled) = polls();
        let held_in = dir.clone();
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
            dir.join(BUILT).exists(),
            "the build that ended stamped its end"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_build_waits_for_a_hold_to_lift() {
        let dir = common("build-waits");
        let hold = hold_in(&dir, "perf", &|| {}).expect("the hold");
        let (count, polled) = polls();
        let building_in = dir.clone();
        let build = std::thread::spawn(move || busy_in(&building_in, "check", &polled).map(|_| ()));
        until_polled(&count);
        assert!(!build.is_finished());
        drop(hold);
        build
            .join()
            .expect("the build's thread")
            .expect("the build, once the hold lifted");
        assert_eq!(
            announcements(&dir),
            0,
            "an announcement is withdrawn with its guard"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A killed xtask never unwinds, so a note can be left behind — but
    /// its lock is released with its process, and a note nobody holds the
    /// lock beside is cleared by whoever meets it.
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
        let _hold = hold_in(&dir, "perf", &|| {}).expect("a dead build is not waited for");
        assert!(!dir.join(BUSY).join("1-0").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_second_hold_is_refused_while_the_first_stands() {
        let dir = common("two-holds");
        let _first = hold_in(&dir, "perf", &|| {}).expect("the first hold");
        let refused = hold_in(&dir, "perf again", &|| {}).expect_err("two measurements at once");
        assert!(refused.contains("another measurement"), "{refused}");
        assert!(refused.contains("perf (pid"), "{refused}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A verb's announcement covers the compiles inside it: the same
    /// thread announcing again gets nothing to withdraw, and only the
    /// outer withdrawal takes the note down.
    #[test]
    fn a_build_inside_an_announced_verb_does_not_announce_again() {
        let dir = common("nested");
        let outer = busy_in(&dir, "verify-ui", &|| {}).expect("the verb's announcement");
        let inner = busy_in(&dir, "cargo build", &|| {}).expect("the compile inside it");
        assert!(inner._announced.is_none(), "nothing of its own");
        assert_eq!(announcements(&dir), 1);
        drop(inner);
        assert_eq!(announcements(&dir), 1, "the inner drop withdraws nothing");
        drop(outer);
        assert_eq!(announcements(&dir), 0);
        let _ = std::fs::remove_dir_all(&dir);
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
            lock_of(&PathBuf::from("C:/x/.git/pg-still")),
            PathBuf::from("C:/x/.git/pg-still.lock")
        );
        assert_eq!(
            lock_of(&PathBuf::from("C:/x/.git/pg-busy/12-3")),
            PathBuf::from("C:/x/.git/pg-busy/12-3.lock")
        );
    }
}
