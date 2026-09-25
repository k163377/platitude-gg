//! The one gate a tree holds: a second gate in the same tree would run
//! the same steps over the same build directory and wait on the first's
//! cargo, so it is refused with the first's pid. What runs at once across
//! the machine is the budget's (`crate::budget`); this is an exclusion
//! and answers at once.
//!
//! Liveness is the lock, as in `still` and `budget`: a lock nobody holds
//! is free whatever note stands beside it, so a gate killed mid-run
//! leaves nothing to clean up.

use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};

use crate::locks::Locked;
use crate::still::{Note, SWEEP};
use crate::wait::{Budget, TRY_AGAIN, Wait};

/// The extension of a lock file beside a note.
const LOCK: &str = "lock";

/// The one gate of a tree, for as long as this stands. The note comes
/// down with it.
///
/// The lock file is never removed (one name per tree,
/// `gate::run::running_note`): removing a lock file at a name every gate
/// opens stops it being one lock (`still::Name`).
#[derive(Debug)]
pub(crate) struct Sole {
    note: PathBuf,
    /// Last: dropped after the `Drop` below has taken the note down.
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
    // down under the lock at the end, so a held lock with no readable note
    // is a gate at one of those edges: looked at again for a sweep's span
    // before it is named.
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
    use super::{open_lock, sole};
    use crate::yard::Yard;

    /// A `.git`-shaped directory of this test's own, gone when the test is.
    fn common(name: &str) -> Yard {
        Yard::new(&format!("lanes-{name}"))
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
    }

    /// What the unlock is for: a child handed the lock's open file
    /// description (as a fork would) still holds it when the gate ends.
    /// The unlock reaches the description itself, so the next gate has the
    /// tree at once — a close would leave it refused until the child was
    /// gone. Linux only, where `flock(2)` promises the inheritance; the
    /// gate suite nets the same release from outside (`gate::stamps`).
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
    }

    #[test]
    fn a_note_nobody_holds_the_lock_beside_is_litter() {
        let dir = common("litter");
        let note = dir.join("target").join("gate-running");
        std::fs::create_dir_all(dir.join("target")).expect("target");
        std::fs::write(&note, "pid 1\nsince 0\nwhat gate\n").expect("a dead gate's note");
        let _mine = sole(&note, "gate --host-only").expect("a dead gate holds nothing");
        let text = std::fs::read_to_string(&note).expect("the note");
        assert!(text.contains("what gate --host-only"), "{text}");
    }

    /// A lock left without a note for a sweep's span (`still::SWEEP`) is
    /// named as a gate — not as the measurement `still` holds the machine
    /// for.
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
    }
}
