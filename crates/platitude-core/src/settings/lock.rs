//! Sole use of a store, held by the kernel for as long as the process runs.

use std::path::PathBuf;

use super::{LOCK_FILE, Store};

impl Store {
    /// Where the lock sits: beside the state file, which stays on this
    /// machine where the settings may roam. `None` for a store with no
    /// files.
    pub fn lock_path(&self) -> Option<PathBuf> {
        let beside = self
            .state_path
            .as_deref()
            .or(self.settings_path.as_deref())?;
        Some(beside.with_file_name(LOCK_FILE))
    }

    /// Asks for sole use of these files. The answer separates "somebody
    /// else has it" from "the question could not be asked" (a network
    /// share can leave file locking unanswered).
    pub fn claim(&self) -> Claim {
        let Some(path) = self.lock_path() else {
            // Nothing for a second process to overwrite. A run naming
            // `PGG_CONFIG_DIR` holds the real lock (the `solo` verb relies
            // on it).
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

/// Sole use of a store, for as long as this value is alive. The kernel
/// owns it: the process ending releases it too, so a crash leaves nothing
/// stale.
#[derive(Debug)]
pub struct Lock {
    /// Holding the handle open *is* the lock; nothing is read from it.
    file: Option<std::fs::File>,
}

impl Drop for Lock {
    /// Unlocked before the close: a `flock` goes with the open file
    /// description, which a fork copies, so a close alone leaves the lock
    /// standing until a child forked meanwhile reaches its `execve`.
    fn drop(&mut self) {
        if let Some(file) = &self.file {
            // The close after this releases it anyway.
            let _ = file.unlock();
        }
    }
}

/// What came back from [`Store::claim`].
#[derive(Debug)]
pub enum Claim {
    /// Nobody else is using these files. Hold on to it.
    Ours(Lock),
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

    /// What the unlock in [`Lock`]'s `Drop` is for: a child handed the
    /// claim's lock description stands in for one a fork hands over, and
    /// the next asker must have the store at once, not once the child is
    /// gone. Linux only, where `flock(2)` promises the inheritance.
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
        // An automated run that names no directory lands here, and several
        // run at once.
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
