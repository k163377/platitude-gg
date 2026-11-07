//! The one short-lived file this crate keeps of its own: a temporary index
//! (`GIT_INDEX_FILE`), which git reads and writes by path and no stdin can
//! stand in for. A message, a patch and a rebase's plan go in on stdin
//! instead (`process::Input`); a reword's message the helper puts down in
//! git's own rebase folder, which git takes away (`sequencer::run`).
//!
//! It lives under `<git_dir>/platitude/` (same filesystem, owner-private,
//! recognisable if left behind).

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
    /// A fresh name in the same place with nothing there yet, for a
    /// temporary index (`GIT_INDEX_FILE`) git writes or a copy of the
    /// worktree's own is put on (`discards::copy`) — gone with the handle
    /// once anything stands there.
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
}

impl Drop for ScratchFile {
    fn drop(&mut self) {
        if let Err(error) = remove(&self.path) {
            tracing::warn!(path = %self.path.display(), %error, "scratch file left behind");
        }
    }
}

/// Gone already counts as removed: a fresh name git never wrote to.
fn remove(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        removed => removed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// What git wrote under the name goes with the handle.
    #[test]
    fn what_git_wrote_goes_with_the_handle() {
        let dir = crate::temp::dir();
        let path = {
            let fresh = ScratchFile::fresh(dir.path(), "index").expect("fresh");
            std::fs::write(fresh.path(), b"git's index").expect("as git writes it");
            fresh.path().to_path_buf()
        };
        assert!(!path.exists(), "dropped handle removes the file");
    }

    #[test]
    fn names_do_not_collide() {
        let dir = crate::temp::dir();
        let a = ScratchFile::fresh(dir.path(), "index").expect("fresh");
        let b = ScratchFile::fresh(dir.path(), "index").expect("fresh");
        assert_ne!(a.path(), b.path());
    }
}
