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
    /// Nothing is ever read out of the file. Holding the handle open *is*
    /// the lock.
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
    use std::time::{Duration, Instant};

    use super::*;
    use crate::settings::testkit::dir_store;
    use crate::settings::{Settings, State};

    /// How long [`ours_once_free`] waits out a lock this process let go
    /// of. Past this, a lock still held is one somebody means to hold,
    /// and the wait was for nothing.
    const CARRIED: Duration = Duration::from_secs(5);

    /// The store claimed, once whoever else has this process's open file
    /// description has let go of it. A `flock` goes with the description
    /// rather than the handle, and a fork copies every description, so a
    /// lock this process drops is held on past the drop for as long as a
    /// child a neighbouring test spawned in that instant has yet to
    /// `execve` — this suite forks with a thread per core, and on Linux
    /// that window reaches in here. [`Claim::Taken`] is the right answer
    /// to the question `claim` is asked; it is not the answer to the one
    /// being asked here, so the answer that stands is the one taken.
    fn ours_once_free(store: &Store) -> Lock {
        let asked = Instant::now();
        loop {
            match store.claim() {
                Claim::Ours(lock) => return lock,
                Claim::Taken => assert!(
                    asked.elapsed() < CARRIED,
                    "the store is still held {} seconds after this process let it go",
                    CARRIED.as_secs()
                ),
                Claim::Unknown(error) => panic!("the store could not be claimed: {error}"),
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

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
        ours_once_free(&store);
    }

    /// The window [`ours_once_free`] is for, held open on purpose. A
    /// child handed the claim's lock description outright stands in for
    /// one a fork hands over: the claim lets the lock go, and the
    /// description is still held by somebody that is not a second
    /// application. The next asker is refused by that child rather than
    /// by another window, which is why the answer it takes is the one
    /// after the child rather than the first.
    ///
    /// Linux, where `flock(2)` promises the inheritance and where the
    /// carried lock is seen; the same window is netted on the gate's
    /// locks in `xtask` (`still`, `lanes`).
    #[test]
    #[cfg(target_os = "linux")]
    fn a_lock_a_neighbour_s_fork_carries_is_waited_out() {
        use std::process::{Command, Stdio};

        let dir = tempfile::tempdir().expect("tempdir");
        let store = dir_store(dir.path());
        let Claim::Ours(held) = store.claim() else {
            panic!("nobody else has it")
        };
        let mut carrier = Command::new("sleep")
            .arg("1")
            .stdin(Stdio::from(
                held._file
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
            matches!(store.claim(), Claim::Taken),
            "the child carries the lock this process let go of"
        );
        ours_once_free(&store);
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
