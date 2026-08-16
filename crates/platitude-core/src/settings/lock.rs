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
            return Claim::Ours(Lock { _file: None });
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
            Ok(()) => Claim::Ours(Lock { _file: Some(file) }),
            Err(std::fs::TryLockError::WouldBlock) => Claim::Taken,
            Err(std::fs::TryLockError::Error(error)) => Claim::Unknown(error),
        }
    }
}

/// Sole use of a store, for as long as this value is alive.
///
/// The kernel owns it: dropping the handle releases it, and so does the
/// process ending, however it ends. There is no stale file to clean up
/// after a crash, and no identifier written anywhere that could outlive
/// the process that wrote it.
#[derive(Debug)]
pub struct Lock {
    /// Never read. Holding the handle open *is* the lock.
    _file: Option<std::fs::File>,
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
        assert!(matches!(store.claim(), Claim::Ours(_)));
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
