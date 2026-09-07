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
//! writes that pid the instant it has the lock and takes it down under
//! the lock at the end, so a lock held with nothing to read beside it is
//! a gate at one edge or the other, looked at again before it is named
//! (`still::SWEEP`).
//!
//! Liveness is the lock, as in `still`: a lock nobody holds is free
//! whatever note stands beside it, so a gate killed mid-run leaves
//! nothing to clean up and nothing anybody waits for.

use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use crate::locks::Locked;
use crate::still::{Note, SWEEP};
use crate::wait::{Budget, LOOK_AGAIN, TRY_AGAIN, Wait};

/// The lanes, beside `.git`: one lock file per lane and side.
const LANES: &str = "pg-lanes";

/// The mark a landing's verb holds beside the lanes while it waits for
/// one: `<side>-landing-<pid>.lock`, one per landing and side.
const LANDING: &str = "landing";

/// How long a verb waits for a lane before the gate calls the holders
/// hung: no verb holds one longer than its own ceilings — the app's
/// watchdog, the step's silence ceiling (`check::run_step`). A ceiling
/// and nothing finer: a lane held changes in nothing until it frees
/// (`crate::wait`).
const LANE_CEILING: Duration = if cfg!(test) {
    Duration::from_secs(20)
} else {
    Duration::from_secs(30 * 60)
};

/// The extension of a lock file beside a note.
const LOCK: &str = "lock";

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
    /// wait unmarked. None between the last waiter letting go and the
    /// next one putting the mark up.
    waiting: Mutex<(usize, Option<Locked>)>,
}

/// A lane held for as long as this stands, and how long it took to get:
/// the wait for somebody else's verb to finish, zero when a lane was
/// free at the first look — the microseconds of probing the lock files
/// are not a wait.
#[derive(Debug)]
pub(crate) struct Lane {
    _lock: Locked,
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
        let mut wait = Wait::new(
            format!("the {} lanes", self.side),
            Budget::whole(LANE_CEILING),
            LOOK_AGAIN,
        );
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
                                _lock: Locked::new(lock),
                                waited: if wait.looks() > 0 {
                                    wait.elapsed()
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
            if self.landing && waiting.is_none() {
                waiting = Some(self.wait_marked(&lanes)?);
            }
            wait.saw(if aside {
                "a landing's verb waiting ahead"
            } else {
                "every lane held"
            });
            polled();
            wait.look_again("a free lane").map_err(|expired| {
                format!(
                    "{expired} — the verbs holding them are hung, or the gates holding them \
                     are; `cargo xtask still` names what is under way"
                )
            })?;
        }
    }

    /// Counts one more of this landing's verbs as waiting, and puts the
    /// mark up when it is the first.
    fn wait_marked(&self, lanes: &Path) -> Result<Waiting<'_>, String> {
        let mut waiting = self
            .waiting
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if waiting.0 == 0 {
            waiting.1 = Some(mark(&self.mark_path(lanes))?);
        }
        waiting.0 += 1;
        Ok(Waiting { lanes: self })
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
        if waiting.0 == 0
            && let Some(mark) = waiting.1.take()
        {
            // Taken down under its own lock, the way a gate's verb takes
            // a dead landing's down: the name never stands unlocked.
            let _ = std::fs::remove_file(self.lanes.mark_path(&self.lanes.common.join(LANES)));
            drop(mark);
        }
    }
}

/// Puts a landing's mark up at `path`: the file, locked, at its name.
/// A gate's verb that finds the file unlocked takes it for a dead
/// landing's and removes it — under the lock ([`a_landing_waits`]), so
/// the two answers a landing can get in that instant are the lock held
/// by the sweep, and the lock granted on a file the sweep has already
/// taken from the name (opened here before the sweep, locked here after
/// it). Neither is a mark: a mark is up once it is locked *and* still
/// there, and is put up again otherwise. The name is this process's own,
/// so a sweep is the one holder it can meet, and a name still held past
/// the sweep's span is refused as an announcement is
/// (`still::lock_beside`) — not waited out unmarked, which would put the
/// landing's verbs behind every gate's with nothing to say why.
fn mark(path: &Path) -> Result<Locked, String> {
    mark_polled(path, &|| {})
}

fn mark_polled(path: &Path, polled: &dyn Fn()) -> Result<Locked, String> {
    let mut tries = Wait::new(
        format!("the landing's mark at {}", path.display()),
        Budget::whole(SWEEP),
        TRY_AGAIN,
    );
    loop {
        let file = open_lock(path)?;
        match file.try_lock() {
            Ok(()) => {
                let file = Locked::new(file);
                if std::fs::metadata(path).is_ok() {
                    return Ok(file);
                }
                // The name went while this held the file: let go of and
                // opened again, the sweep being microseconds long.
                tries.saw("the lock granted on a file no longer at the name");
            }
            // The sweep holds the lock it is removing under.
            Err(TryLockError::WouldBlock) => tries.saw("the lock held"),
            Err(TryLockError::Error(error)) => {
                return Err(format!(
                    "could not put up the landing's mark at {}: {error}",
                    path.display()
                ));
            }
        }
        polled();
        tries
            .look_again("the lock at its own name")
            .map_err(|expired| {
                format!(
                    "could not put up the landing's mark: {expired} — a gate's verb is sweeping a \
                 dead landing's mark there, and is not letting go"
                )
            })?;
    }
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
                // Taken down while this holds the lock, as `still::held`
                // takes a note down: a landing putting its mark up at
                // this name this instant is between its own open and its
                // own lock, and a file removed from under it there is one
                // it goes on holding under no name — every other gate
                // reading an unmarked wait, and the landing's verbs
                // behind theirs for the whole of it. It meets the lock
                // instead, and puts its mark up once this is done
                // ([`mark`]).
                let lock = Locked::new(lock);
                let _ = std::fs::remove_file(&path);
                drop(lock);
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
/// down with it, and the lock after the note; a process that never
/// unwinds leaves both to the operating system.
///
/// The lock file stays where it is, and nothing collects there: it stands
/// at one name per tree (`gate::running_note`), not one per run. Removing
/// a lock file at a name every gate opens is what would stop it being one
/// lock — a second gate that opened it first would go on holding a file
/// no longer at that name while a third made a new one there
/// (`still::Name`).
#[derive(Debug)]
pub(crate) struct Sole {
    note: PathBuf,
    /// Last, so the note comes down before the lock does: dropped after
    /// the `Drop` below has run.
    _lock: Locked,
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
    // A gate writes its note the instant it has the lock and takes it
    // down under the lock at the end, so a held lock with nothing
    // readable beside it is a gate at one of those two edges: looked at
    // again for the span of a sweep, and named only once that is spent.
    let mut unnamed = Wait::new("the gate", Budget::whole(SWEEP), TRY_AGAIN);
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
                if unnamed.look_again("the note beside the held lock").is_err() {
                    break Some(Note::unreadable("a gate"));
                }
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
        _lock: Locked::new(lock),
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
    use std::sync::Mutex;
    use std::time::Duration;

    use super::{Lanes, mark_polled, open_lock, sole};

    /// A `.git`-shaped directory of this test's own.
    fn common(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pg-lanes-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a directory to hold lanes in");
        dir
    }

    /// The waits under test say every look they take on a channel, so a
    /// look is proved by the word of it rather than by a clock.
    fn polls() -> (std::sync::mpsc::Receiver<()>, impl Fn()) {
        let (said, looks) = std::sync::mpsc::channel();
        (looks, move || {
            let _ = said.send(());
        })
    }

    /// Waits for the first look: a wait that has looked once is a wait
    /// that found the lane held. Under the suite's budget, so a wait that
    /// never looks is named rather than left to the harness's kill.
    fn until_polled(looks: &std::sync::mpsc::Receiver<()>) {
        crate::wait::heard("the wait under test", "a look", looks);
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
            crate::wait::heard("the test", "the word to let the lane go", &released);
            drop(lane);
        });
        // Both have looked and found the lane held; the landing's mark is up.
        until_polled(&gate_polls);
        until_polled(&landing_polls);
        drop(first);
        crate::wait::heard("the landing's verb", "the lane that freed", &taken);
        // The gate's verb goes on looking while the landing's holds it:
        // a look taken after the lane changed hands, the earlier ones
        // drained first.
        while gate_polls.try_recv().is_ok() {}
        crate::wait::heard(
            "the gate's verb",
            "another look while the landing's held the lane",
            &gate_polls,
        );
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

    /// The mark's own file, held for the instant a gate's verb takes a
    /// dead landing's down under its lock. The landing puts its mark up
    /// once that instant has passed: one that gave up there would wait
    /// unmarked, its verbs behind every gate's for the whole of the wait.
    #[test]
    fn a_mark_is_put_up_once_the_file_is_let_go_of() {
        let dir = common("mark-held");
        let lanes = dir.join(super::LANES);
        std::fs::create_dir_all(&lanes).expect("the lanes");
        let path = lanes.join("host-landing-1.lock");
        let sweeping = open_lock(&path).expect("the file a sweep removes under");
        sweeping.try_lock().expect("held, as the sweep holds it");
        let sweeping = Mutex::new(Some(sweeping));
        let mark = mark_polled(&path, &|| {
            // Let go of on the first look again, so what is under test is
            // the try after it rather than a clock.
            drop(
                sweeping
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .take(),
            );
        })
        .expect("the mark, once the sweep let the file go");
        assert!(path.exists(), "the mark stands at its own name");
        drop(mark);
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

    /// What the unlock is for. A child handed the lock's open file
    /// description outright stands in for one a fork hands over: the
    /// gate here lets the lock go and takes its note down while that
    /// description is still held by somebody who writes no note. The
    /// unlock reaches the description rather than this process's handle
    /// on it, so the next gate has the tree at once — a close would
    /// have left it refused until the child was gone.
    ///
    /// Linux, where `flock(2)` promises the inheritance and where a
    /// carried lock is seen at all; the gate suite nets the same release
    /// from outside (`gate::stamps`).
    #[test]
    #[cfg(target_os = "linux")]
    fn a_lock_let_go_of_is_free_though_a_forked_child_holds_the_description() {
        use std::process::{Command, Stdio};

        let dir = common("carried");
        let note = dir.join("target").join("gate-running");
        let first = sole(&note, "gate --all").expect("the first gate");
        let mut carrier = Command::new("sleep")
            .arg("5")
            .stdin(Stdio::from(
                first
                    ._lock
                    .handle()
                    .try_clone()
                    .expect("a second handle on the description"),
            ))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("a child handed the lock's description");
        drop(first);
        assert!(!note.exists(), "the note comes down with the gate");
        let _again = sole(&note, "gate").expect("the tree, the lock having been unlocked");
        assert!(
            carrier.try_wait().expect("ask after the child").is_none(),
            "the child let the description go before the tree was asked for"
        );
        carrier.kill().expect("the child that carried it");
        carrier.wait().expect("the child that carried it");
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

    /// The refusal is worded by whoever asks. A gate that spent the span
    /// of a sweep (`still::SWEEP`) on a lock nobody wrote a note beside
    /// names a gate — not the measurement `still` holds the machine for.
    #[test]
    fn a_lock_nobody_named_is_refused_in_the_gate_s_own_words() {
        let dir = common("unnamed");
        let note = dir.join("target").join("gate-running");
        std::fs::create_dir_all(dir.join("target")).expect("target");
        let held = open_lock(&note.with_extension(super::LOCK))
            .expect("a lock beside a note nobody writes");
        held.try_lock()
            .expect("held from here, as a carried lock is");
        let refused = sole(&note, "gate").expect_err("a lock held with nothing beside it");
        assert!(refused.contains("already running"), "{refused}");
        assert!(
            refused.contains("a gate whose note is not written yet"),
            "{refused}"
        );
        drop(held);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
