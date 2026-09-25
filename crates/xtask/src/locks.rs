//! The operating system's lock on a file, let go of by unlocking it and
//! closing it after — hold every one in a [`Locked`]
//! (.claude/rules-refs/core.md「lock は unlock で離す」).

use std::fs::File;

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
