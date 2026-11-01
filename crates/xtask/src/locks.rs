//! The operating system's lock on a file, let go of by unlocking it and
//! closing it after — hold every one in a [`Locked`]
//! (.claude/rules-refs/core.md「lock は unlock で離す」).
//!
//! A `flock` belongs to the open file description, and a fork copies every
//! one: a lock let go of by closing alone stays held by whatever child a
//! neighbouring thread forked meanwhile, until its `execve`. Windows has no
//! such window. Unlocked, the next answer needs one read: what is left to
//! wait out is a sweeper removing the name under its lock, microseconds
//! (ceiling `still::SWEEP`, spacing `wait::TRY_AGAIN`). Nets (Linux only):
//! `a_lock_frees_the_tree_when_unlocked_and_not_when_merely_closed` and
//! `a_lock_let_go_of_is_free_though_a_forked_child_holds_the_description`.
//! A test that builds the closed-only form on purpose puts that half last:
//! first, it leaves the lock file to a neighbour's forked child, and the
//! test's second `try_lock` on it meets `WouldBlock`.

use std::fs::File;
use std::path::Path;

/// A file whose lock this process holds, released when this is dropped.
///
/// A name that comes down with the lock is removed while it is still
/// held: whatever holds both clears the name in its own `Drop`, which
/// runs before its fields are dropped (.claude/rules-refs/core.md
/// 「lock ファイルは握ったまま消し」).
#[derive(Debug)]
pub(crate) struct Locked(File);

impl Locked {
    /// `file`, whose lock this process has just been granted.
    pub(crate) fn new(file: File) -> Self {
        Self(file)
    }

    /// The handle under the lock, for a net that hands a child a copy of
    /// its open file description.
    #[cfg(all(test, target_os = "linux"))]
    pub(crate) fn handle(&self) -> &File {
        &self.0
    }
}

impl Drop for Locked {
    fn drop(&mut self) {
        // A lock that will not come off is released by the close after.
        let _ = self.0.unlock();
    }
}

/// The lock file at `path`, opened to be locked: made where it is not,
/// never truncated.
pub(crate) fn open(path: &Path) -> Result<File, String> {
    File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| format!("could not open the lock at {}: {e}", path.display()))
}

/// The lock at `path`, waited for and held.
pub(crate) fn hold(path: &Path) -> Result<Locked, String> {
    let file = open(path)?;
    file.lock()
        .map_err(|e| format!("could not lock {}: {e}", path.display()))?;
    Ok(Locked::new(file))
}
