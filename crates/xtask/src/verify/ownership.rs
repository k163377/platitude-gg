//! Per-run filesystem ownership for `verify-ui`.

use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub(super) struct ResourceClaim {
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
pub(super) fn claim_resource(
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
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)
        .map_err(|e| {
            format!(
                "{kind} {} is already owned by another verify-ui run: {e}",
                canonical.display()
            )
        })?;
    writeln!(file, "pid={}\npath={identity}", std::process::id())
        .map_err(|e| format!("could not record verify-ui ownership: {e}"))?;
    claimed.insert(key);
    Ok(Some(ResourceClaim { lock }))
}

/// Claim a run-owned directory before any repository, shim, config, or PNG
/// is created in it. `create_dir` is the ownership edge; a timestamp alone
/// only names a collision and lets concurrent runs silently share state.
pub(super) fn fresh_shot_dir(verb: &str) -> Result<PathBuf, String> {
    let base = std::env::temp_dir().join("pg-verify");
    std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let pid = std::process::id();
    for serial in 0..1024_u32 {
        let candidate = base.join(format!("{verb}-{pid}-{nanos}-{serial}"));
        match std::fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "could not claim verify-ui run directory {}: {error}",
                    candidate.display()
                ));
            }
        }
    }
    Err(format!(
        "could not claim a unique verify-ui directory for {verb}"
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
