//! Short-lived files handed to git by path (patches, commit messages).
//!
//! They live under `<git_dir>/platitude/` rather than the system temp
//! directory: same filesystem as the repository, owner-private like the
//! rest of the git dir, and self-evident when one is ever left behind.
//! Nothing is passed to git on stdin — the process layer keeps stdin closed
//! so a misbehaving command can never wait on it.

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

impl ScratchFile {
    /// Writes `contents` to a fresh file named after `tag`.
    pub fn create(git_dir: &Path, tag: &str, contents: &[u8]) -> std::io::Result<Self> {
        let dir = git_dir.join(DIR_NAME);
        std::fs::create_dir_all(&dir)?;
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = dir.join(format!("{tag}-{}-{unique}", std::process::id()));
        let mut file = std::fs::File::create(&path)?;
        file.write_all(contents)?;
        file.sync_all()?;
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchFile {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_file(&self.path) {
            tracing::debug!(path = %self.path.display(), %error, "scratch file not removed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_and_removes_the_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = {
            let scratch = ScratchFile::create(dir.path(), "patch", b"hello").expect("create");
            let path = scratch.path().to_path_buf();
            assert_eq!(std::fs::read(&path).expect("read"), b"hello");
            path
        };
        assert!(!path.exists(), "dropped handle removes the file");
    }

    #[test]
    fn names_do_not_collide() {
        let dir = tempfile::tempdir().expect("tempdir");
        let a = ScratchFile::create(dir.path(), "msg", b"a").expect("create");
        let b = ScratchFile::create(dir.path(), "msg", b"b").expect("create");
        assert_ne!(a.path(), b.path());
    }
}
