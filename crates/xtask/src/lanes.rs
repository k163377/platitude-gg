//! What runs at once across the machine: the verb lanes every gate on it
//! shares, and the one gate a tree holds.
//!
//! A gate runs its verify-ui verbs several at a time (`gate::verbs`), and
//! the count that fits this machine — a third of its threads — is one
//! gate's worth. Seats gate at the same time, and each one's count adds
//! up: with three of them the machine runs three gates' worth of apps,
//! containers and the git they spawn, every verb slows down, and the
//! watchdogs start calling slow verbs dead. So the count is the
//! machine's, not the gate's: a lane is a file lock beside the
//! repository's own `.git`, which every worktree shares, and a verb holds
//! one for as long as it runs. Two gates at once run one machine's count
//! of verbs between them, not two.
//!
//! A landing's verbs go ahead of every other gate's. `land` is the one
//! thing that moves main, and in one line with the seats' own gates its
//! verbs stand for minutes behind verbs whose branches rebase over what
//! lands anyway (the minutes are in internal-docs/反映前テストの機械化.md
//! §群の並走と動詞の並列). So while a landing's verb is waiting for a
//! lane it holds a mark beside them, and a gate's verb that sees the
//! mark leaves the next lane to free alone. Two landings share as gates
//! do; a landing that stopped waiting, or died waiting, holds no mark,
//! and the lanes are everybody's again.
//!
//! A tree holds one gate at a time for the same reason: a second gate in
//! the same tree runs the same steps over the same build directory, and
//! the two wait on each other's cargo for the whole of it. The first
//! keeps the tree; the second is refused with the first's pid. A gate
//! writes that pid the instant it has the lock, so a lock held with
//! nothing to read beside it names nobody, and is waited out rather than
//! answered ([`UNCLAIMED`]).
//!
//! Liveness is the lock, as in `still`: a lock nobody holds is free
//! whatever note stands beside it, so a gate killed mid-run leaves
//! nothing to clean up and nothing anybody waits for.

use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::still::Note;

/// The lanes, beside `.git`: one lock file per lane and side.
const LANES: &str = "pg-lanes";

/// The mark a landing's verb holds beside the lanes while it waits for
/// one: `<side>-landing-<pid>.lock`, one per landing and side.
const LANDING: &str = "landing";

/// How often a wait here looks again: a verb for a free lane, a gate for
/// a tree. Short under test, where the waits are measured in the tens of
/// milliseconds.
const POLL: Duration = if cfg!(test) {
    Duration::from_millis(20)
} else {
    Duration::from_millis(250)
};

/// How long a verb waits for a lane before the gate calls the holders
/// hung: no verb holds one longer than its own ceilings — the app's
/// watchdog, the step's silence ceiling (`check::run_step`).
const LANE_CEILING: Duration = if cfg!(test) {
    Duration::from_secs(20)
} else {
    Duration::from_secs(30 * 60)
};

/// The extension of a lock file beside a note.
const LOCK: &str = "lock";

/// How long [`sole`] waits on a held lock nobody has written a note
/// beside. `flock` goes with the open file description, and a fork copies
/// every one, so a child forked over the gate's lock carries it until its
/// `execve` — after the gate here has let the lock go and taken its note
/// down. The window is that child's scheduling, and reaches the next gate
/// on a loaded container. Past this, a lock nobody names is a lock all
/// the same, and the note is what it is.
const UNCLAIMED: Duration = Duration::from_secs(5);

/// The lanes of one side on this machine: where they stand and how many
/// there are. Every gate names the same count, so however many gates run,
/// that many verbs of the side run between them.
pub(crate) struct Lanes<'a> {
    common: &'a Path,
    side: &'a str,
    count: usize,
    /// A landing's verbs, handed the next lane to free before any other
    /// gate's verb that waits for one.
    landing: bool,
    /// How many of this gate's verbs are waiting for a lane, and the mark
    /// held for as long as any is — a landing's alone; a gate's verbs
    /// wait unmarked.
    waiting: Mutex<(usize, Option<File>)>,
}

/// A lane held for as long as this stands, and how long it took to get:
/// the wait for somebody else's verb to finish, zero when a lane was
/// free at the first look — the microseconds of probing the lock files
/// are not a wait.
#[derive(Debug)]
pub(crate) struct Lane {
    _lock: File,
    pub(crate) waited: Duration,
}

impl<'a> Lanes<'a> {
    /// The lanes of `side` beside `common` (the repository's `.git`),
    /// `count` of them, for a gate's verbs or a landing's.
    pub(crate) fn new(common: &'a Path, side: &'a str, count: usize, landing: bool) -> Self {
        Self {
            common,
            side,
            count,
            landing,
            waiting: Mutex::new((0, None)),
        }
    }

    /// Takes one of the lanes, waiting for one to free.
    pub(crate) fn take(&self) -> Result<Lane, String> {
        self.take_polled(&|| {})
    }

    fn take_polled(&self, polled: &dyn Fn()) -> Result<Lane, String> {
        let lanes = self.common.join(LANES);
        std::fs::create_dir_all(&lanes)
            .map_err(|e| format!("could not make {}: {e}", lanes.display()))?;
        let started = Instant::now();
        let mut looked_again = false;
        // A landing's verb that has looked once and found nothing is
        // counted as waiting until it has a lane, this being the count.
        let mut waiting: Option<Waiting<'_>> = None;
        loop {
            // A gate's verb leaves the lanes alone while a landing's is
            // waiting: the next one to free is the landing's.
            let aside = !self.landing && a_landing_waits(&lanes, self.side)?;
            if !aside {
                for lane in 0..self.count.max(1) {
                    let lock = open_lock(&lanes.join(format!("{}-{lane}.{LOCK}", self.side)))?;
                    match lock.try_lock() {
                        Ok(()) => {
                            return Ok(Lane {
                                _lock: lock,
                                waited: if looked_again {
                                    started.elapsed()
                                } else {
                                    Duration::ZERO
                                },
                            });
                        }
                        Err(TryLockError::WouldBlock) => {}
                        Err(TryLockError::Error(error)) => {
                            return Err(format!(
                                "could not probe the {} lane {lane}: {error}",
                                self.side
                            ));
                        }
                    }
                }
            }
            if started.elapsed() >= LANE_CEILING {
                return Err(format!(
                    "no {} lane freed in {} minutes — the verbs holding them are hung, or the \
                     gates holding them are; `cargo xtask still` names what is under way",
                    self.side,
                    LANE_CEILING.as_secs() / 60
                ));
            }
            if self.landing && waiting.is_none() {
                waiting = Some(self.wait_marked(&lanes));
            }
            polled();
            std::thread::sleep(POLL);
            looked_again = true;
        }
    }

    /// Counts one more of this landing's verbs as waiting, and puts the
    /// mark up when it is the first. A mark that could not be put up is
    /// a wait like a gate's, not an error: the lane comes all the same.
    fn wait_marked(&self, lanes: &Path) -> Waiting<'_> {
        let mut waiting = self
            .waiting
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if waiting.0 == 0 {
            waiting.1 = mark(&self.mark_path(lanes));
        }
        waiting.0 += 1;
        Waiting { lanes: self }
    }

    fn mark_path(&self, lanes: &Path) -> PathBuf {
        lanes.join(format!(
            "{}-{LANDING}-{}.{LOCK}",
            self.side,
            std::process::id()
        ))
    }
}

/// One waiting verb of a landing, counted for as long as this stands. The
/// last one to stop waiting takes the mark down with it.
struct Waiting<'a> {
    lanes: &'a Lanes<'a>,
}

impl Drop for Waiting<'_> {
    fn drop(&mut self) {
        let mut waiting = self
            .lanes
            .waiting
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        waiting.0 = waiting.0.saturating_sub(1);
        if waiting.0 == 0 && waiting.1.take().is_some() {
            let _ = std::fs::remove_file(self.lanes.mark_path(&self.lanes.common.join(LANES)));
        }
    }
}

/// Puts a landing's mark up at `path`: the file, locked. A gate's verb
/// that finds the file unlocked takes it for a dead landing's and removes
/// it, and can do so between this open and this lock — so a mark is only
/// up once it is locked *and* still there, and is put up again otherwise.
fn mark(path: &Path) -> Option<File> {
    for _ in 0..8 {
        let file = open_lock(path).ok()?;
        match file.try_lock() {
            Ok(()) if std::fs::metadata(path).is_ok() => return Some(file),
            Ok(()) => drop(file),
            Err(_) => return None,
        }
    }
    None
}

/// Whether a landing's verb is waiting for a lane of `side`: a mark of
/// the side's that somebody holds. One nobody holds was a landing's that
/// is gone, and comes down here.
fn a_landing_waits(lanes: &Path, side: &str) -> Result<bool, String> {
    let prefix = format!("{side}-{LANDING}-");
    let suffix = format!(".{LOCK}");
    let entries = std::fs::read_dir(lanes).map_err(|e| format!("{}: {e}", lanes.display()))?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with(&prefix) || !name.ends_with(&suffix) {
            continue;
        }
        let path = entry.path();
        let Ok(lock) = File::options().read(true).write(true).open(&path) else {
            continue;
        };
        match lock.try_lock() {
            Ok(()) => {
                drop(lock);
                let _ = std::fs::remove_file(&path);
            }
            Err(TryLockError::WouldBlock) => return Ok(true),
            Err(TryLockError::Error(error)) => {
                return Err(format!(
                    "could not probe the landing's mark {}: {error}",
                    path.display()
                ));
            }
        }
    }
    Ok(false)
}

/// The one gate of a tree, for as long as this stands. The note comes
/// down with it; the lock goes with the handle, and with the process.
#[derive(Debug)]
pub(crate) struct Sole {
    note: PathBuf,
    _lock: File,
}

impl Drop for Sole {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_file(&self.note)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            println!(
                "  note: could not clear {} ({error}) — the next gate here reads it as litter",
                self.note.display()
            );
        }
    }
}

/// Holds `note` as the one gate of its tree, `what` written in it for the
/// next asker — or says who holds it, and that it has to be waited for.
pub(crate) fn sole(note: &Path, what: &str) -> Result<Sole, String> {
    if let Some(parent) = note.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("could not make {}: {e}", parent.display()))?;
    }
    let lock = open_lock(&note.with_extension(LOCK))?;
    let asked = Instant::now();
    // A gate writes its note the instant it has the lock, so a held lock
    // with nothing readable beside it is not a gate holding the tree —
    // it is the carried window, waited out here.
    let other = loop {
        match lock.try_lock() {
            Ok(()) => break None,
            Err(TryLockError::WouldBlock) => {
                if let Some(other) = std::fs::read_to_string(note)
                    .ok()
                    .and_then(|text| Note::parse(&text))
                {
                    break Some(other);
                }
                if asked.elapsed() >= UNCLAIMED {
                    break Some(Note::unreadable());
                }
                std::thread::sleep(POLL);
            }
            Err(TryLockError::Error(error)) => {
                return Err(format!(
                    "could not probe the gate lock at {}: {error}",
                    note.display()
                ));
            }
        }
    };
    if let Some(other) = other {
        return Err(format!(
            "a gate is already running in this tree: {} — one at a time: a second gate here \
             would run the same steps over the same build directory, and the two would wait \
             on each other's cargo. Wait for it — its verdict lands on the output of the \
             command that started it (a tool call that timed out is still running in the \
             background; read that output file). Only a process that is gone frees the \
             tree by itself.",
            other.line()
        ));
    }
    std::fs::write(note, Note::now(what).text())
        .map_err(|e| format!("could not write {}: {e}", note.display()))?;
    Ok(Sole {
        note: note.to_path_buf(),
        _lock: lock,
    })
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use super::{Lanes, sole};

    /// A `.git`-shaped directory of this test's own.
    fn common(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pg-lanes-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a directory to hold lanes in");
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
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Two lanes, three verbs: the third waits until one of the first two
    /// is done, and the lanes of the other side are nobody's business.
    #[test]
    fn a_third_verb_waits_for_one_of_two_lanes_to_free() {
        let dir = common("two-lanes");
        let host = Lanes::new(&dir, "host", 2, false);
        let first = host.take().expect("the first lane");
        let _second = host.take().expect("the second lane");
        let linux = Lanes::new(&dir, "linux", 2, false);
        let _elsewhere = linux.take().expect("the other side's lane is free");
        let (count, polled) = polls();
        let waiting_in = dir.clone();
        let third = std::thread::spawn(move || {
            let host = Lanes::new(&waiting_in, "host", 2, false);
            host.take_polled(&polled).map(|lane| lane.waited)
        });
        until_polled(&count);
        assert!(!third.is_finished(), "a third verb ran on two lanes");
        drop(first);
        let waited = third
            .join()
            .expect("the third's thread")
            .expect("the third lane, once one freed");
        assert!(waited > Duration::ZERO, "the wait is reported");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// One lane, held, and two verbs waiting for it — a gate's and a
    /// landing's: the landing's is handed it when it frees, and the
    /// gate's only once the landing's is done with it.
    #[test]
    fn a_landing_s_verb_is_handed_the_lane_a_gate_s_verb_was_waiting_for() {
        let dir = common("landing-first");
        let first = Lanes::new(&dir, "host", 1, false).take().expect("the lane");
        let (gate_polls, gate_polled) = polls();
        let (landing_polls, landing_polled) = polls();
        let waiting_in = dir.clone();
        let behind = std::thread::spawn(move || {
            let gate = Lanes::new(&waiting_in, "host", 1, false);
            gate.take_polled(&gate_polled).map(|lane| lane.waited)
        });
        let (handed, taken) = std::sync::mpsc::channel();
        let (release, released) = std::sync::mpsc::channel::<()>();
        let waiting_in = dir.clone();
        let ahead = std::thread::spawn(move || {
            let landing = Lanes::new(&waiting_in, "host", 1, true);
            let lane = landing
                .take_polled(&landing_polled)
                .expect("the landing's lane");
            handed.send(()).expect("say the lane was handed over");
            released.recv().expect("the word to let it go");
            drop(lane);
        });
        // Both have looked and found the lane held; the landing's mark is up.
        until_polled(&gate_polls);
        until_polled(&landing_polls);
        drop(first);
        taken
            .recv()
            .expect("the landing's verb was handed the lane that freed");
        // The gate's verb goes on looking while the landing's holds it.
        let looks = gate_polls.load(Ordering::SeqCst);
        while gate_polls.load(Ordering::SeqCst) == looks {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            !behind.is_finished(),
            "a gate's verb was handed the lane ahead of the landing's"
        );
        release.send(()).expect("let the landing's verb go");
        ahead.join().expect("the landing's thread");
        let waited = behind
            .join()
            .expect("the gate's thread")
            .expect("the gate's lane, once the landing's verb was done");
        assert!(waited > Duration::ZERO, "the wait is reported");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A landing's mark nobody holds is a dead landing's: a gate's verb
    /// takes a lane at the first look, and the mark comes down.
    #[test]
    fn a_landing_s_mark_nobody_holds_is_litter() {
        let dir = common("litter-mark");
        let lanes = dir.join(super::LANES);
        std::fs::create_dir_all(&lanes).expect("the lanes");
        let mark = lanes.join("host-landing-1.lock");
        std::fs::write(&mark, b"").expect("a dead landing's mark");
        let lane = Lanes::new(&dir, "host", 1, false)
            .take()
            .expect("a lane, the mark being nobody's");
        assert_eq!(lane.waited, Duration::ZERO, "the mark cost a wait");
        assert!(!mark.exists(), "the dead landing's mark stands");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_tree_holds_one_gate_and_names_it_to_the_second() {
        let dir = common("one-gate");
        let note = dir.join("target").join("gate-running");
        let first = sole(&note, "gate --all").expect("the first gate");
        let refused = sole(&note, "gate").expect_err("a second gate in the same tree");
        assert!(refused.contains("already running"), "{refused}");
        assert!(refused.contains("gate --all (pid"), "{refused}");
        drop(first);
        assert!(!note.exists(), "the note comes down with the gate");
        let _again = sole(&note, "gate").expect("the tree, once the first gate is done");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The window [`super::UNCLAIMED`] is for, held open on purpose. A
    /// child handed the lock's open file description outright stands in
    /// for one a fork hands over: the gate here lets the lock go and
    /// takes its note down, and the description is still held by
    /// somebody who writes no note. The gate that asks waits the child
    /// out rather than naming a holder it cannot read.
    ///
    /// Linux, where `flock(2)` promises the inheritance and where the
    /// carried lock is seen; the gate suite nets the same window from
    /// outside (`gate::stamps`).
    #[test]
    #[cfg(target_os = "linux")]
    fn a_lock_a_forked_child_carries_is_waited_out() {
        use std::process::{Command, Stdio};

        let dir = common("carried");
        let note = dir.join("target").join("gate-running");
        let first = sole(&note, "gate --all").expect("the first gate");
        let mut carrier = Command::new("sleep")
            .arg("1")
            .stdin(Stdio::from(
                first
                    ._lock
                    .try_clone()
                    .expect("a second handle on the description"),
            ))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("a child handed the lock's description");
        drop(first);
        assert!(!note.exists(), "the note comes down with the gate");
        let _again = sole(&note, "gate").expect("the tree, once the carried lock is gone");
        assert!(
            carrier.try_wait().expect("ask after the child").is_some(),
            "the gate had the tree while the child still carried the lock"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A note left by a gate that was killed names a process that holds
    /// no lock: the tree is free, and the note is written over.
    #[test]
    fn a_note_nobody_holds_the_lock_beside_is_litter() {
        let dir = common("litter");
        let note = dir.join("target").join("gate-running");
        std::fs::create_dir_all(dir.join("target")).expect("target");
        std::fs::write(&note, "pid 1\nsince 0\nwhat gate\n").expect("a dead gate's note");
        let _mine = sole(&note, "gate --host-only").expect("a dead gate holds nothing");
        let text = std::fs::read_to_string(&note).expect("the note");
        assert!(text.contains("what gate --host-only"), "{text}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
