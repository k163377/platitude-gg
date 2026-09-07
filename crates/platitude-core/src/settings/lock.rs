//! Sole use of a store, held by the kernel for as long as the process runs.

use std::path::PathBuf;

use super::{LOCK_FILE, Store};

impl Store {
    /// Where the lock sits: beside the state file, which is the one two
    /// processes overwrite in turn — and which is per-machine, where the
    /// settings may be a roaming profile that follows a person to another
    /// computer. `None` for a store with no files.
    pub fn lock_path(&self) -> Option<PathBuf> {
        let beside = self
            .state_path
            .as_deref()
            .or(self.settings_path.as_deref())?;
        Some(beside.with_file_name(LOCK_FILE))
    }

    /// Asks for sole use of these files.
    ///
    /// The answer separates "somebody else has it" from "the question
    /// could not be asked": a redirected profile or a network share can
    /// leave file locking unanswered, and a lock nobody can take must
    /// never become the reason a window will not open.
    pub fn claim(&self) -> Claim {
        let Some(path) = self.lock_path() else {
            // A store with no files has nothing for a second process to
            // overwrite — the ephemeral store `PG_*` automation without a
            // named `PG_CONFIG_DIR` gets. A run that names one holds the
            // real lock like anyone else (the `solo` verb relies on it).
            return Claim::Ours(Lock { file: None });
        };
        if let Some(dir) = path.parent()
            && let Err(error) = std::fs::create_dir_all(dir)
        {
            return Claim::Unknown(error);
        }
        let file = match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
        {
            Ok(file) => file,
            Err(error) => return Claim::Unknown(error),
        };
        match file.try_lock() {
            Ok(()) => Claim::Ours(Lock { file: Some(file) }),
            Err(std::fs::TryLockError::WouldBlock) => Claim::Taken,
            Err(std::fs::TryLockError::Error(error)) => Claim::Unknown(error),
        }
    }
}

/// Sole use of a store, for as long as this value is alive.
///
/// The kernel owns it: dropping this releases it, and so does the
/// process ending, however it ends. There is no stale file to clean up
/// after a crash, and no identifier written anywhere that could outlive
/// the process that wrote it.
#[derive(Debug)]
pub struct Lock {
    /// Nothing is ever read out of the file. Holding the handle open
    /// *is* the lock, and the `Drop` below reaches for it only to let
    /// that lock go.
    file: Option<std::fs::File>,
}

impl Drop for Lock {
    /// Unlocked rather than merely closed. A `flock` goes with the open
    /// file description, and a fork copies every description a process
    /// has, so a lock let go of by closing the handle stands until the
    /// last child forked over that instant reaches its `execve` — and
    /// the next asker, told the store is taken, would be told it by a
    /// child of its own rather than by a second application. `LOCK_UN`
    /// reaches the description itself, whoever holds a copy of it.
    fn drop(&mut self) {
        if let Some(file) = &self.file {
            // A lock that will not come off is one the close after this
            // releases anyway, and no window opens or fails to open over
            // the answer.
            let _ = file.unlock();
        }
    }
}

/// What came back from [`Store::claim`].
#[derive(Debug)]
pub enum Claim {
    /// Nobody else is using these files. Hold on to it.
    Ours(Lock),
    /// Another process is using them.
    Taken,
    /// The lock could not be asked for. Carry on: a filesystem that will
    /// not answer is not a second application.
    Unknown(std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::testkit::dir_store;
    use crate::settings::{Settings, State};

    #[test]
    fn only_one_process_at_a_time_holds_a_store() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = dir_store(dir.path());
        let first = store.claim();
        assert!(matches!(first, Claim::Ours(_)), "nobody else has it");
        assert!(
            matches!(store.claim(), Claim::Taken),
            "a second asker is turned away"
        );

        drop(first);
        assert!(
            matches!(store.claim(), Claim::Ours(_)),
            "a store let go of is free at once"
        );
    }

    /// What the unlock in [`Lock`]'s `Drop` is for, held open on
    /// purpose. A child handed the claim's lock description outright
    /// stands in for one a fork hands over: the claim lets the lock go
    /// while that description is still held by somebody that is not a
    /// second application. The unlock reaches the description rather
    /// than this process's handle on it, so the next asker has the store
    /// at once — a close would have had it refused by the child until
    /// the child was gone.
    ///
    /// Linux, where `flock(2)` promises the inheritance and where a
    /// carried lock is seen at all; the same release is netted on the
    /// gate's locks in `xtask` (`still`, `lanes`).
    #[test]
    #[cfg(target_os = "linux")]
    fn a_lock_let_go_of_is_free_though_a_forked_child_holds_the_description() {
        use std::process::{Command, Stdio};

        let dir = tempfile::tempdir().expect("tempdir");
        let store = dir_store(dir.path());
        let Claim::Ours(held) = store.claim() else {
            panic!("nobody else has it")
        };
        let mut carrier = Command::new("sleep")
            .arg("5")
            .stdin(Stdio::from(
                held.file
                    .as_ref()
                    .expect("the claim's handle")
                    .try_clone()
                    .expect("a second handle on the description"),
            ))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("a child handed the lock's description");
        drop(held);
        assert!(
            matches!(store.claim(), Claim::Ours(_)),
            "the store the claim unlocked is free"
        );
        assert!(
            carrier.try_wait().expect("ask after the child").is_none(),
            "the child let the description go before the store was asked for"
        );
        carrier.kill().expect("the child that carried it");
        carrier.wait().expect("the child that carried it");
    }

    #[test]
    fn a_store_with_no_files_is_never_taken() {
        // Every automated run lands here, and two of them run at once.
        let store = Store::ephemeral();
        let first = store.claim();
        assert!(store.lock_path().is_none());
        assert!(matches!(first, Claim::Ours(_)));
        assert!(matches!(store.claim(), Claim::Ours(_)));
    }

    #[test]
    fn the_lock_is_not_one_of_the_two_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = dir_store(dir.path());
        let held = store.claim();
        store.save_state(&State::default()).expect("save");
        store.save_settings(&Settings::default()).expect("save");
        assert!(
            matches!(store.claim(), Claim::Taken),
            "a flush must not hand the store to the next process"
        );
        drop(held);
    }
}
