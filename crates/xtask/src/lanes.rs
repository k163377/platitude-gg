//! The one gate a tree holds.
//!
//! A second gate in the same tree runs the same steps over the same
//! build directory, and the two wait on each other's cargo for the whole
//! of it. The first keeps the tree; the second is refused with the
//! first's pid rather than queued — there is nothing for it to do that
//! the first is not already doing. A gate writes that pid the instant it
//! has the lock and takes it down under the lock at the end, so a lock
//! held with nothing to read beside it is a gate at one edge or the
//! other, looked at again before it is named (`still::SWEEP`).
//!
//! What runs at once *across* the machine is the budget (`crate::budget`)
//! — one pool over every unit of both sides and every seat, with a
//! landing at the head of the queue. This lock is the one thing that
//! stayed a lock: it is an exclusion rather than a share, and it refuses
//! instead of waiting, so it never stands between a landing and the
//! machine.
//!
//! Liveness is the lock, as in `still` and `budget`: a lock nobody holds
//! is free whatever note stands beside it, so a gate killed mid-run
//! leaves nothing to clean up and nothing anybody waits for.

use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};

use crate::locks::Locked;
use crate::still::{Note, SWEEP};
use crate::wait::{Budget, TRY_AGAIN, Wait};

/// The extension of a lock file beside a note.
const LOCK: &str = "lock";

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

    use super::{open_lock, sole};

    /// A `.git`-shaped directory of this test's own.
    fn common(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pg-lanes-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a directory to hold a gate's note in");
        dir
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
