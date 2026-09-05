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
//! A tree holds one gate at a time for the same reason: a second gate in
//! the same tree runs the same steps over the same build directory, and
//! the two wait on each other's cargo for the whole of it. The first
//! keeps the tree; the second is refused with the first's pid.
//!
//! Liveness is the lock, as in `still`: a lock nobody holds is free
//! whatever note stands beside it, so a gate killed mid-run leaves
//! nothing to clean up and nothing anybody waits for.

use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::still::Note;

/// The lanes, beside `.git`: one lock file per lane and side.
const LANES: &str = "pg-lanes";

/// How often a verb looks again for a free lane. Short under test, where
/// the waits are measured in the tens of milliseconds.
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

/// The lanes of one side on this machine: where they stand and how many
/// there are. Every gate names the same count, so however many gates run,
/// that many verbs of the side run between them.
pub(crate) struct Lanes<'a> {
    pub(crate) common: &'a Path,
    pub(crate) side: &'a str,
    pub(crate) count: usize,
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

impl Lanes<'_> {
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
        loop {
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
            if started.elapsed() >= LANE_CEILING {
                return Err(format!(
                    "no {} lane freed in {} minutes — the verbs holding them are hung, or the \
                     gates holding them are; `cargo xtask still` names what is under way",
                    self.side,
                    LANE_CEILING.as_secs() / 60
                ));
            }
            polled();
            std::thread::sleep(POLL);
            looked_again = true;
        }
    }
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
    match lock.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => {
            let other = std::fs::read_to_string(note)
                .ok()
                .and_then(|text| Note::parse(&text))
                .unwrap_or_else(Note::unreadable);
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
        Err(TryLockError::Error(error)) => {
            return Err(format!(
                "could not probe the gate lock at {}: {error}",
                note.display()
            ));
        }
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
        let host = Lanes {
            common: &dir,
            side: "host",
            count: 2,
        };
        let first = host.take().expect("the first lane");
        let _second = host.take().expect("the second lane");
        let linux = Lanes {
            common: &dir,
            side: "linux",
            count: 2,
        };
        let _elsewhere = linux.take().expect("the other side's lane is free");
        let (count, polled) = polls();
        let waiting_in = dir.clone();
        let third = std::thread::spawn(move || {
            let host = Lanes {
                common: &waiting_in,
                side: "host",
                count: 2,
            };
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
