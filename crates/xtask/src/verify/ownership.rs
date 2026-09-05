//! Per-run filesystem ownership: the directory a run is handed, and the
//! claim that says one process has it.
//!
//! Reachable past `verify` because a run in a container is owned from
//! out here — `/out` is a mount, and the directory behind it is claimed
//! on this side (`keepsakes::bridge`).

use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub(crate) struct ResourceClaim {
    lock: PathBuf,
}

impl Drop for ResourceClaim {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.lock);
    }
}

/// Atomically reserve an explicitly shared path for this process. Claims
/// live outside the target so a repository does not become dirty merely
/// because it is under test.
pub(crate) fn claim_resource(
    target: &Path,
    kind: &str,
    claimed: &mut BTreeSet<u64>,
) -> Result<Option<ResourceClaim>, String> {
    let canonical = std::fs::canonicalize(target)
        .map_err(|e| format!("could not resolve {kind} {}: {e}", target.display()))?;
    let mut identity = canonical.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        identity.make_ascii_lowercase();
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    identity.hash(&mut hasher);
    let key = hasher.finish();
    if claimed.contains(&key) {
        return Ok(None);
    }

    let locks = std::env::temp_dir().join("pg-verify-locks");
    std::fs::create_dir_all(&locks).map_err(|e| e.to_string())?;
    let lock = locks.join(format!("{key:016x}.lock"));
    let refused = |e: &std::io::Error| {
        format!(
            "{kind} {} is already owned by another verify-ui run: {e}",
            canonical.display()
        )
    };
    let open_new = || {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock)
    };
    let mut file = match open_new() {
        Ok(file) => file,
        // A lock whose writer is gone is litter, not ownership: the temp
        // directory outlives every killed run, and without this one
        // taskkill would refuse the path until a reboot.
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && !holder_alive(&lock) => {
            let _ = std::fs::remove_file(&lock);
            open_new().map_err(|e| refused(&e))?
        }
        Err(e) => return Err(refused(&e)),
    };
    writeln!(file, "pid={}\npath={identity}", std::process::id())
        .map_err(|e| format!("could not record verify-ui ownership: {e}"))?;
    claimed.insert(key);
    Ok(Some(ResourceClaim { lock }))
}

/// Whether the process that wrote `lock` still exists — and is a task
/// runner, which is the only thing that ever writes one: a pid that a
/// killed run left behind is a name the machine gives out again, and a
/// stranger under it must not hold the path (`subprocess::task_runner_exists`).
/// Unreadable or half-written locks answer "alive": refusing is the safe
/// side, and the writer may be between create and write.
fn holder_alive(lock: &Path) -> bool {
    let Some(pid) = std::fs::read_to_string(lock).ok().and_then(|text| {
        text.lines()
            .find_map(|line| line.strip_prefix("pid=")?.trim().parse::<u32>().ok())
    }) else {
        return true;
    };
    crate::subprocess::task_runner_exists(pid)
}

/// Claim a run-owned directory before any repository, shim, config, or PNG
/// is created in it.
pub(super) fn fresh_shot_dir(verb: &str) -> Result<PathBuf, String> {
    claim_dir(&std::env::temp_dir().join("pg-verify"), verb)
}

/// The directories under the system temp that every run of this runner
/// claims its own directory in (`claim_dir`'s callers): a verb's
/// pictures and settings, its demo repositories (`demo::claim_root`), a
/// container run's mount (`keepsakes::keepsake_dir`), and the roots the
/// tests claim.
const RUN_BASES: [&str; 4] = ["pg-verify", "pg-demo", "pg-linux", "pg-census"];

/// How long a run's directory stands before it is litter. A run is
/// minutes long — its watchdog is two, a cold build ten — and the
/// pictures a person was shown are on the board (`shots`), so a day
/// later what is left under these is nobody's evidence.
const RUN_LITTER_AGE: Duration = Duration::from_secs(24 * 60 * 60);

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
/// stays: a date nobody can read is no grounds for deleting.
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

    /// A day is what makes a run's directory litter, read off the clock
    /// the sweep is handed: the same two entries stand when the sweep
    /// runs now and go when it runs the day after tomorrow.
    #[test]
    fn a_sweep_takes_the_runs_of_a_day_ago_and_leaves_todays() {
        let base = super::claim_dir(&std::env::temp_dir().join("pg-census"), "sweep")
            .expect("a base of this test's own");
        let run = super::claim_dir(&base, "run").expect("a run directory");
        std::fs::write(run.join("app.png"), b"picture").expect("a picture in it");
        std::fs::write(base.join("stray"), b"a file beside the runs").expect("a stray file");
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

    /// The state a killed run leaves behind: a lock naming a process that
    /// is gone. The pid is one no process can have rather than a reaped
    /// child's — a reaped number is the kernel's to give out again, and
    /// under a suite forking git on every thread it is somebody else's
    /// before the second claim asks (`subprocess::NO_SUCH_PID`).
    #[test]
    fn a_lock_whose_writer_is_gone_is_reclaimed() {
        let target = super::fresh_shot_dir("stale-claim").expect("target directory");
        let dead_pid = crate::subprocess::NO_SUCH_PID;

        // First claim writes the lock, then the file is doctored to name
        // the dead pid.
        let mut first_set = BTreeSet::new();
        let first = super::claim_resource(&target, "test resource", &mut first_set)
            .expect("first claim")
            .expect("new claim");
        let lock = first.lock.clone();
        std::mem::forget(first);
        std::fs::write(&lock, format!("pid={dead_pid}\npath=doctored\n")).expect("doctor the lock");

        let mut second_set = BTreeSet::new();
        super::claim_resource(&target, "test resource", &mut second_set)
            .expect("a stale lock is reclaimed")
            .expect("new claim over the stale lock");
        std::fs::remove_dir(target).expect("remove empty target directory");
    }

    /// A lock whose pid a stranger inherited is as stale as one whose pid
    /// nobody has: only a task runner ever writes one, so a process of any
    /// other name under the number is the number given out again.
    #[test]
    fn a_lock_whose_pid_a_stranger_inherited_is_reclaimed() {
        let target = super::fresh_shot_dir("inherited-claim").expect("target directory");
        // A live process that is not a task runner: a child of this
        // test's own, held alive for the length of the claim.
        let mut stranger = if cfg!(windows) {
            let mut c = std::process::Command::new("ping");
            c.args(["-n", "30", "127.0.0.1"]);
            c
        } else {
            let mut c = std::process::Command::new("sleep");
            c.arg("30");
            c
        };
        let mut stranger = stranger
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("a process that is not the runner");
        let inherited = stranger.id();

        let mut first_set = BTreeSet::new();
        let first = super::claim_resource(&target, "test resource", &mut first_set)
            .expect("first claim")
            .expect("new claim");
        let lock = first.lock.clone();
        std::mem::forget(first);
        std::fs::write(&lock, format!("pid={inherited}\npath=doctored\n"))
            .expect("doctor the lock");

        let mut second_set = BTreeSet::new();
        let reclaimed = super::claim_resource(&target, "test resource", &mut second_set);
        stranger.kill().expect("the stranger is ended");
        stranger.wait().expect("the stranger is reaped");
        reclaimed
            .expect("a lock held by a stranger is reclaimed")
            .expect("new claim over the inherited lock");
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
}
