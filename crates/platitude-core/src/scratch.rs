//! Short-lived files handed to git by path (patches, commit messages), and
//! folders git writes into (its trace of a command).
//!
//! They live under `<git_dir>/platitude/` (same filesystem, owner-private,
//! recognisable if left behind) and go by path because the process layer
//! keeps stdin closed.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const DIR_NAME: &str = "platitude";

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A name of this process's own (its pid and count) under the scratch
/// folder, made if it is not there yet.
fn own_name(git_dir: &Path, tag: &str) -> std::io::Result<PathBuf> {
    let dir = git_dir.join(DIR_NAME);
    std::fs::create_dir_all(&dir)?;
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    Ok(dir.join(format!("{tag}-{}-{unique}", std::process::id())))
}

/// A file removed when the handle drops.
#[derive(Debug)]
pub struct ScratchFile {
    path: PathBuf,
}

impl ScratchFile {
    /// Writes `contents` to a fresh file named after `tag`.
    pub fn create(git_dir: &Path, tag: &str, contents: &[u8]) -> std::io::Result<Self> {
        let path = own_name(git_dir, tag)?;
        let mut file = std::fs::File::create(&path)?;
        file.write_all(contents)?;
        file.sync_all()?;
        Ok(Self { path })
    }

    /// A fresh name in the same place with nothing there yet, for a file
    /// git writes itself (a temporary index, `GIT_INDEX_FILE`) — gone with
    /// the handle if git made it.
    ///
    /// The name is this process's own (its pid and count), so whatever is
    /// there already — the file or git's `.lock` beside it — was left by a
    /// process gone before this one took its pid, and goes: git would read a
    /// stale index as where to start.
    pub fn fresh(git_dir: &Path, tag: &str) -> std::io::Result<Self> {
        let path = own_name(git_dir, tag)?;
        let mut lock = path.clone().into_os_string();
        lock.push(".lock");
        for left in [path.as_path(), Path::new(&lock)] {
            match std::fs::remove_file(left) {
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error),
                _ => {}
            }
        }
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Leaves the file behind for a reader that outlives the handle (a
    /// stopped rebase's remaining `exec` lines read their message files on a
    /// later `--continue`). [`Self::sweep`] collects what nobody came back
    /// for.
    pub fn keep(self) -> PathBuf {
        let path = self.path.clone();
        std::mem::forget(self);
        path
    }

    /// Removes every leftover file carrying `tag`. Call only when no reader
    /// of a [`Self::keep`] can be standing.
    pub fn sweep(git_dir: &Path, tag: &str) {
        let Ok(entries) = std::fs::read_dir(git_dir.join(DIR_NAME)) else {
            return;
        };
        let prefix = format!("{tag}-");
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(&prefix)
                && let Err(error) = std::fs::remove_file(entry.path())
            {
                tracing::debug!(path = %entry.path().display(), %error, "scratch leftover not removed");
            }
        }
    }
}

impl Drop for ScratchFile {
    fn drop(&mut self) {
        match std::fs::remove_file(&self.path) {
            // A fresh name git never wrote to (`Self::fresh`).
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                tracing::debug!(path = %self.path.display(), %error, "scratch file not removed")
            }
            Ok(()) => {}
        }
    }
}

/// A folder in the same place, for files git writes there itself (its
/// trace of a command, one file per process, `process::trace`) — removed
/// with everything in it when the handle drops.
#[derive(Debug)]
pub struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    /// A fresh, empty folder named after `tag`. Its name is this process's
    /// own, as [`ScratchFile::fresh`]'s is, so what is there already was
    /// left by a process gone before this one took its pid, and goes.
    pub fn fresh(git_dir: &Path, tag: &str) -> std::io::Result<Self> {
        let path = own_name(git_dir, tag)?;
        match std::fs::remove_dir_all(&path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error),
            _ => {}
        }
        std::fs::create_dir(&path)?;
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Folders this process could not remove — on Windows, a git that outlived
/// its command (a daemon it started) still holding a file in it — tried
/// again each time another goes.
static NOT_REMOVED: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let mut left = NOT_REMOVED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        left.push(self.path.clone());
        left.retain(|path| match std::fs::remove_dir_all(path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                tracing::debug!(path = %path.display(), %error, "scratch folder not removed");
                true
            }
            _ => false,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_and_removes_the_file() {
        let dir = crate::temp::dir();
        let path = {
            let scratch = ScratchFile::create(dir.path(), "patch", b"hello").expect("create");
            let path = scratch.path().to_path_buf();
            assert_eq!(std::fs::read(&path).expect("read"), b"hello");
            path
        };
        assert!(!path.exists(), "dropped handle removes the file");
    }

    /// What a process gone before this one left under the same name — the
    /// file and its lock — is not there when the name is handed out.
    #[test]
    fn a_fresh_name_has_nothing_left_on_it() {
        let dir = crate::temp::dir();
        let next = COUNTER.load(Ordering::Relaxed);
        let left = dir.path().join(DIR_NAME);
        std::fs::create_dir_all(&left).expect("dir");
        for at in next..next + 64 {
            let name = format!("index-{}-{at}", std::process::id());
            std::fs::write(left.join(&name), b"stale").expect("leftover");
            std::fs::write(left.join(format!("{name}.lock")), b"stale").expect("lock");
        }
        let fresh = ScratchFile::fresh(dir.path(), "index").expect("fresh");
        assert!(!fresh.path().exists());
        let mut lock = fresh.path().as_os_str().to_owned();
        lock.push(".lock");
        assert!(!Path::new(&lock).exists());
    }

    #[test]
    fn names_do_not_collide() {
        let dir = crate::temp::dir();
        let a = ScratchFile::create(dir.path(), "msg", b"a").expect("create");
        let b = ScratchFile::create(dir.path(), "msg", b"b").expect("create");
        assert_ne!(a.path(), b.path());
    }

    /// The folder starts empty — what a process gone before this one left
    /// under its name goes — and goes with what git wrote into it.
    #[test]
    fn a_scratch_folder_starts_empty_and_goes_whole() {
        let dir = crate::temp::dir();
        let next = COUNTER.load(Ordering::Relaxed);
        let left = dir.path().join(DIR_NAME);
        for at in next..next + 64 {
            let stale = left.join(format!("TRACE-{}-{at}", std::process::id()));
            std::fs::create_dir_all(&stale).expect("leftover");
            std::fs::write(stale.join("event"), b"stale").expect("leftover's file");
        }
        let path = {
            let fresh = ScratchDir::fresh(dir.path(), "TRACE").expect("fresh");
            assert_eq!(
                std::fs::read_dir(fresh.path()).expect("read").count(),
                0,
                "nothing left on it"
            );
            std::fs::write(fresh.path().join("written"), b"by git").expect("write");
            fresh.path().to_path_buf()
        };
        assert!(!path.exists(), "dropped handle removes the folder");
    }

    /// A file held open without leave to delete it — as a git that outlived
    /// its command holds its trace — keeps its folder; the next folder to go
    /// takes it along.
    #[cfg(windows)]
    #[test]
    fn a_folder_held_open_goes_once_another_one_does() {
        use std::os::windows::fs::OpenOptionsExt;
        const READ_AND_WRITE_ONLY: u32 = 1 | 2;
        let dir = crate::temp::dir();
        let held = ScratchDir::fresh(dir.path(), "TRACE").expect("fresh");
        let path = held.path().to_path_buf();
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .share_mode(READ_AND_WRITE_ONLY)
            .open(path.join("event"))
            .expect("open");
        drop(held);
        assert!(path.exists(), "a file held open keeps it");
        drop(file);
        drop(ScratchDir::fresh(dir.path(), "TRACE").expect("another"));
        assert!(!path.exists(), "gone with the next one");
    }
}
