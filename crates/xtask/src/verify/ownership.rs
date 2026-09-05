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
use std::time::{SystemTime, UNIX_EPOCH};

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

/// Whether the process that wrote `lock` still exists. Unreadable or
/// half-written locks answer "alive": refusing is the safe side, and the
/// writer may be between create and write.
fn holder_alive(lock: &Path) -> bool {
    let Some(pid) = std::fs::read_to_string(lock).ok().and_then(|text| {
        text.lines()
            .find_map(|line| line.strip_prefix("pid=")?.trim().parse::<u32>().ok())
    }) else {
        return true;
    };
    crate::subprocess::process_exists(pid)
}

/// Claim a run-owned directory before any repository, shim, config, or PNG
/// is created in it.
pub(super) fn fresh_shot_dir(verb: &str) -> Result<PathBuf, String> {
    claim_dir(&std::env::temp_dir().join("pg-verify"), verb)
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

    #[test]
    fn a_lock_whose_writer_is_gone_is_reclaimed() {
        let target = super::fresh_shot_dir("stale-claim").expect("target directory");
        // A pid that has certainly exited: our own child, reaped.
        let mut probe = if cfg!(windows) {
            let mut c = std::process::Command::new("cmd");
            c.args(["/C", "exit 0"]);
            c
        } else {
            std::process::Command::new("true")
        };
        let child = probe.spawn().expect("spawn a short-lived child");
        let dead_pid = child.id();
        let mut child = child;
        child.wait().expect("reap the child");

        // First claim writes the lock, then the file is doctored to name
        // the dead pid — the state a killed run leaves behind.
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
