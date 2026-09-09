//! Per-run filesystem ownership: the directory a run is handed, and the
//! claim that says one process has it.
//!
//! Reachable past `verify` because a run in a container is owned from
//! out here — `/out` is a mount, and the directory behind it is claimed
//! on this side (`keepsakes::bridge`).

use std::collections::BTreeSet;
use std::fs::TryLockError;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub(crate) struct ResourceClaim {
    /// Never read: **holding the handle is the claim**. The lock under
    /// it is the operating system's, unlocked when this is dropped
    /// ([`crate::locks`]) — and let go anyway when the process ends
    /// without dropping anything, which is what a killed run does.
    _lock: crate::locks::Locked,
}

/// Atomically reserve an explicitly shared path for this process. Claims
/// live outside the target so a repository does not become dirty merely
/// because it is under test.
///
/// **The operating system holds the claim; the lock file's bytes hold
/// nothing.** A run that is killed leaves its file standing with no lock
/// on it, so there is no dead claim to tell from a live one, and no pid
/// to be wrong about when the machine hands the number out again. What
/// the file says is a note for a person ([`note`]), read by nothing.
pub(crate) fn claim_resource(
    target: &Path,
    kind: &str,
    claimed: &mut BTreeSet<u64>,
) -> Result<Option<ResourceClaim>, String> {
    let canonical = std::fs::canonicalize(target)
        .map_err(|e| format!("could not resolve {kind} {}: {e}", target.display()))?;
    let locks = std::env::temp_dir().join(LOCKS);
    let (lock, key) = lock_of(&locks, &canonical);
    if claimed.contains(&key) {
        return Ok(None);
    }
    std::fs::create_dir_all(&locks).map_err(|e| e.to_string())?;

    // `truncate(false)` is the claim's: a file is emptied at open, which
    // is before the lock, and the standing owner's note is not this
    // run's to erase. Emptying it is [`note`]'s, under the lock.
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock)
        .map_err(|e| format!("could not open the claim {}: {e}", lock.display()))?;
    match file.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => {
            return Err(format!(
                "{kind} {} is already owned by another verify-ui run",
                canonical.display()
            ));
        }
        Err(TryLockError::Error(e)) => {
            return Err(format!(
                "could not claim {kind} {}: {e}",
                canonical.display()
            ));
        }
    }
    note(&mut file, &canonical)
        .map_err(|e| format!("could not record verify-ui ownership: {e}"))?;
    claimed.insert(key);
    Ok(Some(ResourceClaim {
        _lock: crate::locks::Locked::new(file),
    }))
}

/// Where the lock files live. **A claim never deletes the file it
/// locked.** Unlinking a locked file is allowed on both platforms, and
/// it hands one path to two runs: the next asker creates a fresh file
/// under the name and locks that, while the first still holds the one
/// that was unlinked. The sweep takes them instead, at a day old — which
/// is longer than any claim lives, a run being minutes and its watchdog
/// two — and a file still held is one whose note was written today.
const LOCKS: &str = "pgg-verify-locks";

/// The file a claim on `canonical` is taken on, and the key by which a
/// run tells the paths it already holds. One name per resolved path, and
/// on Windows one per spelling of it: two cases of a directory are one
/// resource.
fn lock_of(locks: &Path, canonical: &Path) -> (PathBuf, u64) {
    let mut identity = canonical.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        identity.make_ascii_lowercase();
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    identity.hash(&mut hasher);
    let key = hasher.finish();
    (locks.join(format!("{key:016x}.lock")), key)
}

/// What run made this file, for the person looking at a directory of
/// them. **Nothing reads it back**, and a refusal cannot quote it:
/// Windows refuses a read that overlaps a locked range, so what a claim
/// says is legible only once the claim is let go — which is the moment
/// it stops being about anybody.
fn note(file: &mut std::fs::File, canonical: &Path) -> std::io::Result<()> {
    file.set_len(0)?;
    writeln!(
        file,
        "pid={}\npath={}",
        std::process::id(),
        canonical.display()
    )
}

/// Claim a run-owned directory before any repository, shim, config, or PNG
/// is created in it.
pub(super) fn fresh_shot_dir(verb: &str) -> Result<PathBuf, String> {
    claim_dir(&std::env::temp_dir().join("pgg-verify"), verb)
}

/// The directories under the system temp a run leaves something in. Four
/// it claims its own directory in (`claim_dir`'s callers): a verb's
/// pictures and settings, its demo repositories (`demo::claim_root`), a
/// container run's mount (`keepsakes::keepsake_dir`), and the roots the
/// tests claim. And one it leaves a file in — [`LOCKS`], where a claim
/// is taken and which nothing else ever clears.
const RUN_BASES: [&str; 5] = ["pgg-verify", "pgg-demo", "pgg-linux", "pgg-census", LOCKS];

/// How long a run's directory stands before it is litter. A run is
/// minutes long — its watchdog is two, a cold build ten — and the
/// pictures a person was shown are on the board (`shots`), so a day
/// later what is left under these is nobody's evidence.
const RUN_LITTER_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// The file that takes a directory out of every sweep, for good.
const KEEP: &str = ".pgg-keep";

/// Marks `root` as a tree a person asked for rather than one a run left,
/// which is the whole of what the sweep goes by.
///
/// **Whether the tree is open cannot be asked instead.** A repository
/// this application is showing is held by nothing — git is a subprocess
/// per operation and no directory is watched — so a handle would answer
/// "nobody's" for the tab that is on screen; a tree built to be looked at
/// later is nobody's by definition; and under `pgg-linux` the only side
/// that could hold one is across the mount.
///
/// A mark is forever: a floor of any length is a guess at how long a
/// person keeps a tree, and nothing here can make that guess. Marked
/// trees are the person's to remove.
pub(crate) fn keep(root: &Path) -> Result<(), String> {
    std::fs::write(
        root.join(KEEP),
        "Built by hand (`cargo xtask demo-repo`). No sweep takes a directory\n\
         holding this file. Delete the directory when you are done with it.\n",
    )
    .map_err(|e| format!("could not mark {} as kept: {e}", root.display()))
}

/// Takes yesterday's run directories away, in the background. Nothing
/// else ever does: every run claims a directory and leaves it, and the
/// gate runs hundreds of them a day (measured: sixty thousand of them,
/// six gigabytes, three days after the last sweep by hand). The gate
/// calls this on its way in; the thread is not waited for — a plan is
/// half a second and the temp directory is seconds of reading — and a
/// directory that will not go, or a sweep the process ends first, is the
/// next sweep's. Nothing is said: a gate's verdict is not about litter.
pub(crate) fn sweep_yesterdays_runs() {
    std::thread::spawn(|| {
        let temp = std::env::temp_dir();
        let now = SystemTime::now();
        for base in RUN_BASES {
            sweep_older_than(&temp.join(base), now, RUN_LITTER_AGE);
        }
    });
}

/// Removes the entries of `base` last written `age` or longer before
/// `now`, and answers how many went. An entry whose age cannot be read
/// stays: a date nobody can read is no grounds for deleting. An entry
/// carrying [`KEEP`] stays whatever its date: age is what makes a run's
/// directory litter, and a tree somebody asked for is not one.
fn sweep_older_than(base: &Path, now: SystemTime, age: Duration) -> usize {
    let Ok(entries) = std::fs::read_dir(base) else {
        return 0;
    };
    let mut gone = 0;
    for entry in entries.flatten() {
        let old = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|written| now.duration_since(written).ok())
            .is_some_and(|since| since >= age);
        if !old {
            continue;
        }
        let path = entry.path();
        // Only the old are asked, so the runs of a day pay nothing for
        // the question and the marked pay one stat apiece.
        if path.join(KEEP).exists() {
            continue;
        }
        let removed = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        if removed.is_ok() {
            gone += 1;
        }
    }
    gone
}

/// A directory under `base` that this call made and nobody else has.
///
/// **`create_dir` is the ownership edge**; a timestamp alone only names a
/// collision and lets concurrent runs silently share state, because
/// `create_dir_all` answers the same for a directory it made and one that
/// was already standing there. The pid and the serial are in the name for
/// the two ways a clock alone repeats itself: every process on this
/// machine reads the same one, and one process can ask twice inside a
/// single tick of it.
pub(crate) fn claim_dir(base: &Path, stem: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(base).map_err(|e| format!("could not make {}: {e}", base.display()))?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let pid = std::process::id();
    for serial in 0..1024_u32 {
        let candidate = base.join(format!("{stem}-{pid}-{nanos}-{serial}"));
        match std::fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "could not claim the run directory {}: {error}",
                    candidate.display()
                ));
            }
        }
    }
    Err(format!(
        "could not claim a unique directory for {stem} under {}",
        base.display()
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::io::BufRead;
    use std::path::Path;

    /// A day is what makes a run's directory litter, read off the clock
    /// the sweep is handed: the same two entries stand when the sweep
    /// runs now and go when it runs the day after tomorrow.
    #[test]
    fn a_sweep_takes_the_runs_of_a_day_ago_and_leaves_todays() {
        let base = super::claim_dir(&std::env::temp_dir().join("pgg-census"), "sweep")
            .expect("a base of this test's own");
        let run = super::claim_dir(&base, "run").expect("a run directory");
        std::fs::write(run.join("app.png"), b"picture").expect("a picture in it");
        std::fs::write(base.join("stray"), b"a file beside the runs").expect("a stray file");
        // waits(measured): the clock is handed to the sweep under test, which reads no other
        let now = std::time::SystemTime::now();
        let age = super::RUN_LITTER_AGE;
        assert_eq!(super::sweep_older_than(&base, now, age), 0);
        assert!(run.join("app.png").is_file(), "today's run stands");
        let later = now + age + age;
        assert_eq!(super::sweep_older_than(&base, later, age), 2);
        assert!(!run.exists(), "yesterday's run went, picture and all");
        assert!(!base.join("stray").exists());
        std::fs::remove_dir(&base).expect("the base, empty now");
    }

    /// The mark is what the sweep goes by, and it does not expire: the
    /// hand-built tree stands on the day its neighbour's run goes, and on
    /// every day after.
    #[test]
    fn a_sweep_leaves_a_marked_tree_and_takes_the_run_beside_it() {
        let base = super::claim_dir(&std::env::temp_dir().join("pgg-census"), "sweep-keep")
            .expect("a base of this test's own");
        let run = super::claim_dir(&base, "run").expect("a run directory");
        let kept = super::claim_dir(&base, "by-hand").expect("a hand-built directory");
        super::keep(&kept).expect("the mark a hand-built tree carries");
        std::fs::write(kept.join("repo"), b"the tree a person means to open").expect("a work tree");
        let age = super::RUN_LITTER_AGE;
        // waits(measured): the clock is handed to the sweep under test, which reads no other
        let now = std::time::SystemTime::now();
        assert_eq!(super::sweep_older_than(&base, now + age * 2, age), 1);
        assert!(!run.exists(), "the run beside it went");
        assert!(kept.join("repo").is_file(), "the marked tree stands");
        // Nothing is left for a later sweep to find: the mark has no
        // floor to outlive, so a month buys the sweep no more than a day.
        assert_eq!(super::sweep_older_than(&base, now + age * 30, age), 0);
        assert!(kept.join("repo").is_file(), "and stands a month on");
        std::fs::remove_dir_all(&base).expect("the base and the tree it kept");
    }

    #[test]
    fn concurrent_runs_atomically_claim_distinct_directories() {
        let start = std::sync::Arc::new(std::sync::Barrier::new(16));
        let threads: Vec<_> = (0..16)
            .map(|_| {
                let start = start.clone();
                std::thread::spawn(move || {
                    start.wait();
                    super::fresh_shot_dir("parallel-claim").expect("claim a run directory")
                })
            })
            .collect();
        let claims: Vec<_> = threads
            .into_iter()
            .map(|thread| thread.join().expect("claim thread"))
            .collect();

        let unique: BTreeSet<_> = claims.iter().collect();
        assert_eq!(unique.len(), claims.len());
        assert!(claims.iter().all(|path| path.is_dir()));

        for path in claims {
            std::fs::remove_dir(path).expect("remove empty claimed directory");
        }
    }

    /// What turns a run of this test binary into the process that holds
    /// a claim: the path to hold, and the line it says once it has it.
    const HELD_FOR: &str = "PGG_OWNERSHIP_HOLD";
    const HELD: &str = "claim held";

    /// Names the holder to the run that starts it. A filter rather than
    /// `--exact`: the module path is not this test's to know, and the
    /// name is one binary's.
    const HOLDER: &str = "holds_a_claim_until_it_is_killed";

    /// Holds a claim on the path it is handed until it is killed — the
    /// second process of the test below, which starts it as another run
    /// of this same binary. Ignored because nothing else hands it a
    /// path, and without one there is nothing here to run.
    #[test]
    #[ignore = "the second process of a_claim_a_killed_run_held_is_taken_by_the_next"]
    fn holds_a_claim_until_it_is_killed() {
        let Ok(target) = std::env::var(HELD_FOR) else {
            return;
        };
        let mut claimed = BTreeSet::new();
        let held = super::claim_resource(Path::new(&target), "test resource", &mut claimed)
            .expect("the claim this run was started to take")
            .expect("new claim");
        println!("{HELD}");
        // Held for as long as the run that started this one holds its
        // end of the pipe: the read answers when that end closes, so a
        // holder whose run is gone does not stand on — and one that is
        // killed, as the test below kills it, never gets that far.
        let mut word = String::new();
        std::io::stdin()
            .read_line(&mut word)
            .expect("the pipe from the run that started this one");
        drop(held);
    }

    /// What a killed run leaves behind is a lock file with no lock on
    /// it: the operating system lets go of what a process held however
    /// that process ended, so the next run takes the path and has
    /// nothing to clean up first. The claim is held from a process of
    /// its own — a lock refuses across handles, so one thread cannot
    /// play both runs.
    #[test]
    fn a_claim_a_killed_run_held_is_taken_by_the_next() {
        let target = super::fresh_shot_dir("killed-claim").expect("target directory");
        // Its stdin is a pipe this run holds the other end of: what the
        // holder stands on, and what lets it go if this run dies first.
        let mut holder =
            std::process::Command::new(std::env::current_exe().expect("this test binary"))
                .args([HOLDER, "--ignored", "--nocapture"])
                .env(HELD_FOR, &target)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .spawn()
                .expect("a second run of this binary, to hold the claim");
        let mut said =
            std::io::BufReader::new(holder.stdout.take().expect("the holder's own output"));
        let mut line = String::new();
        while line.trim_end() != HELD {
            line.clear();
            let read = said.read_line(&mut line).expect("the holder's own output");
            assert!(read > 0, "the holder ended without taking the claim");
        }

        let mut mine = BTreeSet::new();
        let refused = super::claim_resource(&target, "test resource", &mut mine)
            .expect_err("a claim another process holds is refused");
        assert!(refused.contains("already owned"), "{refused}");

        holder.kill().expect("the holder is ended");
        holder.wait().expect("the holder is reaped");

        // The file the killed run locked stands: nothing but the sweep
        // ever takes one. Asked before this run claims anything, so the
        // file standing here is the dead run's own.
        let canonical = std::fs::canonicalize(&target).expect("the resolved target");
        let (lock, _) = super::lock_of(&std::env::temp_dir().join(super::LOCKS), &canonical);
        assert!(lock.is_file(), "{} went with its owner", lock.display());

        // When the operating system lets the lock go is not this test's
        // to assert: the claim is tried under the suite's budget, every
        // try the real claim, and a lock never let go fails by name with
        // the last refusal in the message.
        let taken = crate::wait::until(
            "the killed run's claim let go",
            || {
                let mut next = BTreeSet::new();
                super::claim_resource(&target, "test resource", &mut next)
            },
            |outcome| matches!(outcome, Ok(Some(_))),
        );

        drop(taken);
        std::fs::remove_dir(target).expect("remove empty target directory");
    }

    #[test]
    fn an_explicit_resource_has_one_owner_at_a_time() {
        let target = super::fresh_shot_dir("resource-claim").expect("target directory");
        let mut first_set = BTreeSet::new();
        let first = super::claim_resource(&target, "test resource", &mut first_set)
            .expect("first claim")
            .expect("new claim");

        let mut second_set = BTreeSet::new();
        let error = super::claim_resource(&target, "test resource", &mut second_set)
            .expect_err("a concurrent owner must be refused");
        assert!(error.contains("already owned"));

        drop(first);
        super::claim_resource(&target, "test resource", &mut second_set)
            .expect("the path can be reused sequentially")
            .expect("new claim after release");
        std::fs::remove_dir(target).expect("remove empty target directory");
    }

    /// One resource, four runs asking in the same moment: the lock is
    /// the resource's only owner, so exactly one of them is handed it
    /// and the rest are told it is somebody's.
    #[test]
    fn one_run_of_several_racing_for_a_resource_takes_it() {
        let target = super::fresh_shot_dir("racing-claim").expect("target directory");
        let racers = 4;
        let start = std::sync::Arc::new(std::sync::Barrier::new(racers));
        let threads: Vec<_> = (0..racers)
            .map(|_| {
                let start = start.clone();
                let target = target.clone();
                std::thread::spawn(move || {
                    let mut claimed = BTreeSet::new();
                    start.wait();
                    super::claim_resource(&target, "test resource", &mut claimed)
                })
            })
            .collect();
        let outcomes: Vec<_> = threads
            .into_iter()
            .map(|thread| thread.join().expect("racing claim"))
            .collect();

        let taken = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, Ok(Some(_))))
            .count();
        assert_eq!(taken, 1, "one run owns the resource: {outcomes:?}");
        let refusals: Vec<_> = outcomes
            .iter()
            .filter_map(|outcome| outcome.as_ref().err())
            .collect();
        assert_eq!(refusals.len(), racers - 1);
        for refusal in refusals {
            assert!(refusal.contains("already owned"), "{refusal}");
        }

        drop(outcomes);
        std::fs::remove_dir(target).expect("remove empty target directory");
    }
}
