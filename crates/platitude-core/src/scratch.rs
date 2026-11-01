//! Short-lived files handed to git by path (patches, commit messages).
//!
//! They live under `<git_dir>/platitude/` (same filesystem, owner-private,
//! recognisable if left behind) and go by path because the process layer
//! keeps stdin closed.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const DIR_NAME: &str = "platitude";

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A file removed when the handle drops.
#[derive(Debug)]
pub struct ScratchFile {
    path: PathBuf,
}

/// A write that failed over a file that could not be removed after it: the
/// write's own error first, then the file and why it stayed.
#[derive(Debug, thiserror::Error)]
#[error("{failed} ({} was left behind: {removal})", path.display())]
pub struct LeftBehind {
    pub failed: std::io::Error,
    pub path: PathBuf,
    pub removal: std::io::Error,
}

impl ScratchFile {
    /// Writes `contents` to a fresh file named after `tag`.
    ///
    /// The handle owns the file from the moment it is made: a write or a
    /// sync that fails takes the file with it and answers the failure as it
    /// was — or, where the file could not be removed, the same kind holding
    /// the failure, the file and why it stayed ([`LeftBehind`]).
    pub fn create(git_dir: &Path, tag: &str, contents: &[u8]) -> std::io::Result<Self> {
        let path = Self::name(git_dir, tag)?;
        let mut file = File::create(&path)?;
        let scratch = Self { path };
        let filled = write(&mut file, contents).and_then(|()| sync(&file));
        // Closed before anything removes it: Windows deletes an open file
        // only once its last handle closes.
        drop(file);
        match filled {
            Ok(()) => Ok(scratch),
            Err(failed) => Err(scratch.discard(failed)),
        }
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
        let path = Self::name(git_dir, tag)?;
        let mut lock = path.clone().into_os_string();
        lock.push(".lock");
        for left in [path.as_path(), Path::new(&lock)] {
            remove(left)?;
        }
        Ok(Self { path })
    }

    /// A name of this process's own (its pid and count) under `git_dir`.
    fn name(git_dir: &Path, tag: &str) -> std::io::Result<PathBuf> {
        let dir = git_dir.join(DIR_NAME);
        std::fs::create_dir_all(&dir)?;
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        Ok(dir.join(format!("{tag}-{}-{unique}", std::process::id())))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Removes the file a failed write left, answering `failed` as it was
    /// where the file went and as [`LeftBehind`] where it stayed.
    fn discard(self, failed: std::io::Error) -> std::io::Error {
        let mut owned = std::mem::ManuallyDrop::new(self);
        let path = std::mem::take(&mut owned.path);
        let Err(removal) = remove(&path) else {
            return failed;
        };
        tracing::warn!(
            path = %path.display(),
            error = %failed,
            %removal,
            "a failed scratch write left its file behind"
        );
        std::io::Error::new(
            failed.kind(),
            LeftBehind {
                failed,
                path,
                removal,
            },
        )
    }
}

impl Drop for ScratchFile {
    fn drop(&mut self) {
        if let Err(error) = remove(&self.path) {
            tracing::warn!(path = %self.path.display(), %error, "scratch file left behind");
        }
    }
}

// The three steps a test makes fail (mry; the mocks exist in debug builds
// only, and on the thread that set them).

#[mry::mry(skip_args(File, u8))]
fn write(file: &mut File, contents: &[u8]) -> std::io::Result<()> {
    file.write_all(contents)
}

#[mry::mry(skip_args(File))]
fn sync(file: &File) -> std::io::Result<()> {
    file.sync_all()
}

/// Gone already counts as removed: a fresh name git never wrote to
/// ([`ScratchFile::fresh`]).
#[mry::mry(skip_args(Path))]
fn remove(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        removed => removed,
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Error, ErrorKind};

    use super::*;

    /// What stands under the scratch folder.
    fn left(git_dir: &Path) -> Vec<String> {
        std::fs::read_dir(git_dir.join(DIR_NAME))
            .map(|entries| {
                entries
                    .flatten()
                    .map(|entry| entry.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default()
    }

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

    /// The file is made before the bytes go in, so a write that fails has
    /// one standing; it goes, and the write's own error is the answer.
    #[test]
    #[mry::lock(write)]
    fn a_write_that_fails_takes_its_file_and_answers_its_own_error() {
        mock_write().returns_with(|| Err(Error::new(ErrorKind::StorageFull, "the disk is full")));
        let dir = crate::temp::dir();
        let err = ScratchFile::create(dir.path(), "patch", b"hello").expect_err("the write fails");
        assert_eq!(err.kind(), ErrorKind::StorageFull);
        assert_eq!(err.to_string(), "the disk is full");
        assert!(
            err.get_ref()
                .and_then(|inner| inner.downcast_ref::<LeftBehind>())
                .is_none(),
            "nothing was left to name"
        );
        assert_eq!(left(dir.path()), Vec::<String>::new());
    }

    /// The bytes are all there when the sync fails: the file goes all the
    /// same.
    #[test]
    #[mry::lock(sync)]
    fn a_sync_that_fails_after_a_whole_write_takes_the_file() {
        mock_sync().returns_with(|| Err(Error::other("the sync failed")));
        let dir = crate::temp::dir();
        let err = ScratchFile::create(dir.path(), "msg", b"subject\n").expect_err("the sync fails");
        assert_eq!(err.kind(), ErrorKind::Other);
        assert_eq!(err.to_string(), "the sync failed");
        assert_eq!(left(dir.path()), Vec::<String>::new());
    }

    /// A file that will not go stays named: the answer is still the sync's
    /// failure, by kind and first words, and says which file stayed and why.
    #[test]
    #[mry::lock(sync, remove)]
    fn a_file_that_will_not_go_is_named_under_the_failure_it_followed() {
        mock_sync().returns_with(|| Err(Error::new(ErrorKind::StorageFull, "the sync failed")));
        mock_remove().returns_with(|| {
            Err(Error::new(
                ErrorKind::PermissionDenied,
                "held open elsewhere",
            ))
        });
        let dir = crate::temp::dir();
        let err = ScratchFile::create(dir.path(), "msg", b"subject\n").expect_err("the sync fails");
        assert_eq!(err.kind(), ErrorKind::StorageFull);
        let stayed = left(dir.path());
        assert_eq!(stayed.len(), 1, "removal was refused: {stayed:?}");
        let said = err.to_string();
        assert!(said.starts_with("the sync failed"), "{said}");
        assert!(said.contains(&stayed[0]), "names the file: {said}");
        assert!(said.contains("held open elsewhere"), "says why: {said}");
        let left_behind = err
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<LeftBehind>())
            .expect("the parts stay apart for a reader");
        assert_eq!(left_behind.failed.kind(), ErrorKind::StorageFull);
        assert_eq!(left_behind.removal.kind(), ErrorKind::PermissionDenied);
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
}
