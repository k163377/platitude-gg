//! The operating system's lock on a file, and the one way it is let go
//! of here.
//!
//! Every lock in this runner is a `File` the process holds — `flock` on
//! Linux, `LockFileEx` on Windows — and what differs between the two
//! ways of releasing one is who else is still holding it afterwards.
//! `flock` goes with the open file description, and a fork copies
//! every description a process has, so a lock released by *closing*
//! the file stands until the last child forked over that instant has
//! reached its `execve`: 200 releases in 200,000 were carried that
//! way against a neighbouring thread spawning throughout, where an
//! explicit unlock was 0 in 200,000. `LOCK_UN` reaches the
//! description itself, whoever holds a copy of it; a close only lets
//! go of this process's own reference to it.
//!
//! Windows carries nothing either way — the handles opened here are not
//! inheritable, and `LockFileEx` does not follow one into a child — so
//! the rule is one rule for both platforms: **a lock is let go of by
//! unlocking it, and closed after.** Hold every one in a [`Locked`],
//! which does that, and nothing downstream has to wait a carried lock
//! out.

use std::fs::File;

/// A file whose lock this process holds, released when this is dropped.
///
/// A name that comes down with the lock comes down *before* this does:
/// removing it while the lock is still held is what keeps the next
/// holder from opening a name this one is about to take away
/// (.claude/rules-refs/core.md). So whatever holds both clears the name
/// in its own `Drop`, which runs before its fields are dropped.
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
        // A lock that will not come off is one the close after this
        // releases anyway; there is nobody to tell and nothing to do.
        let _ = self.0.unlock();
    }
}
