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
    /// Holding the handle is the claim ([`crate::locks`]).
    _lock: crate::locks::Locked,
}

/// Atomically reserve an explicitly shared path for this process.
/// Claims live outside the target, so a repository under test
/// stays clean.
///
/// The OS lock is the claim, so a killed run leaves no dead claim to
/// tell from a live one and no pid to trust.
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

    // `truncate(false)`: truncating at open comes before the lock and
    // would empty the standing owner's note. [`note`] empties it under
    // the lock.
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

/// Where the lock files live. A claim leaves its file standing:
/// unlinking a locked file (allowed on both platforms) would hand one
/// path to two runs — the next asker locks a fresh file under the name
/// while the first still holds the unlinked one. The sweep takes them at
/// a day old, longer than any claim lives.
const LOCKS: &str = "pgg-verify-locks";

/// The lock file for `canonical`, and the key a run tells the paths it
/// holds by. On Windows the key folds case: two spellings of a directory
/// are one resource.
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

/// What run made this file, for a person. A refusal cannot quote it:
/// Windows refuses a read that overlaps a locked range, so the note is
/// legible only once the claim is let go.
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

/// The bases under the system temp a run leaves something in: the
/// directories `claim_dir` makes — pictures and settings, demo
/// repositories (`demo::claim_root`), a container run's mount
/// (`keepsakes::keepsake_dir`), test roots, and [`crate::yard`]'s trees —
/// and [`LOCKS`], which nothing else clears.
const RUN_BASES: [&str; 6] = [
    "pgg-verify",
    "pgg-demo",
    "pgg-linux",
    "pgg-census",
    crate::yard::BASE,
    LOCKS,
];

/// How long a run's directory stands before it is litter: far longer
/// than any run, and the pictures a person was shown are on the board
/// (`shots`).
const RUN_LITTER_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// The file that takes a directory out of every sweep, for good.
const KEEP: &str = ".pgg-keep";

/// Marks `root` as a tree a person asked for; the sweeps go by this
/// mark alone. A mark rather than a held handle: a repository the app is
/// showing is held by nothing (git is a subprocess per operation), so a
/// handle would call the open tab nobody's. The mark never expires —
/// marked trees are the person's to remove.
pub(crate) fn keep(root: &Path) -> Result<(), String> {
    std::fs::write(
        root.join(KEEP),
        "Built by hand (`cargo xtask demo-repo`). No sweep takes a directory\n\
         holding this file. Delete the directory when you are done with it.\n",
    )
    .map_err(|e| format!("could not mark {} as kept: {e}", root.display()))
}

/// Takes away the demo roots **this run** claimed, now that it has
/// passed, and answers how many went. Passing road only: a failed run's
/// tree is the scene, and in a container the volume is the only place it
/// survives. What failures leave is the sweeps' ([`sweep_yesterdays_runs`]
/// on the host, the preparation step in the container).
///
/// What it never takes:
///
/// * **Another run's roots** — the list is this process's successful
///   `create_dir`s (`demo::claimed_roots`).
/// * **A template** — built at a fixed name, never claimed.
/// * **A tree marked [`KEEP`].**
/// * **A directory something is still writing into.** The app's git
///   children outlive it (on unix re-parented to init, and an open file
///   does not stop the removal), so nothing is removed unless
///   [`crate::reap::others_in_this_group`] answers nobody. On Windows an
///   open handle refuses the removal itself.
pub(super) fn give_back_claimed(before: Option<&[u32]>) -> usize {
    give_back(crate::demo::claimed_roots(), arrived_since(before))
}

/// Who is in this run's group now that was not there before the app
/// started. The group alone is the wrong question: in a container it
/// also holds the wrapping shell and its children, and the removal would
/// never happen.
///
/// A pid handed out again between the two looks reads as "was there" —
/// the one unsafe-direction error, unlikely since Linux hands pids out
/// in order. `None` either side is "could not be found out", not
/// "nobody".
fn arrived_since(before: Option<&[u32]>) -> Option<Vec<u32>> {
    let before = before?;
    Some(
        crate::reap::others_in_this_group()?
            .into_iter()
            .filter(|pid| !before.contains(pid))
            .collect(),
    )
}

/// [`give_back_claimed`] over roots handed in, so a test can drive it
/// without emptying the process-wide list the other tests share.
fn give_back(roots: Vec<PathBuf>, others: Option<Vec<u32>>) -> usize {
    match others {
        Some(others) if others.is_empty() => {}
        Some(others) => {
            println!(
                "demo repositories kept: {} process(es) this run started are still going",
                others.len()
            );
            return 0;
        }
        None => {
            println!("demo repositories kept: what is still running could not be read");
            return 0;
        }
    }
    let mut gone = 0;
    for root in roots {
        if root.join(KEEP).exists() {
            continue;
        }
        if std::fs::remove_dir_all(&root).is_ok() {
            gone += 1;
        }
    }
    gone
}

/// Takes yesterday's run directories away on a background thread — the
/// only thing that clears what failed runs leave (passing ones give
/// theirs back, [`give_back_claimed`]). Callers do not wait: a directory
/// that will not go, or a sweep the process ends first, is the next
/// sweep's. Silent: a gate's verdict is about its tests.
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
/// stays — a date nobody can read is no grounds for deleting — and so
/// does one carrying [`KEEP`], whatever its date.
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
/// `create_dir` is the ownership edge: `create_dir_all` answers the same
/// for a directory that was already standing, so a timestamp name alone
/// lets concurrent runs silently share one. The pid and serial cover two
/// processes, or one process twice, within one clock tick.
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

    /// The age is read off the clock the sweep is handed, so the same
    /// entries stand now and go two days on.
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

    /// The mark does not expire: the marked tree stands the day its
    /// neighbour goes, and a month on.
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

    /// What turns a run of this test binary into the claim holder: the
    /// path to hold, and the line it says once it holds it.
    const HELD_FOR: &str = "PGG_OWNERSHIP_HOLD";
    const HELD: &str = "claim held";

    /// Passed as a filter rather than with `--exact`: the module path is
    /// not this test's to know.
    const HOLDER: &str = "holds_a_claim_until_it_is_killed";

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
        // Held until the starting run's end of the pipe closes, so a
        // holder whose run died does not stand on.
        let mut word = String::new();
        std::io::stdin()
            .read_line(&mut word)
            .expect("the pipe from the run that started this one");
        drop(held);
    }

    /// A killed run leaves a lock file with no lock on it: the OS lets go
    /// however the process ended, so the next run takes the path with
    /// nothing to clean up. The holder is a second process because the
    /// subject is a process being killed.
    #[test]
    fn a_claim_a_killed_run_held_is_taken_by_the_next() {
        let target = super::fresh_shot_dir("killed-claim").expect("target directory");
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

        // Asked before this run claims anything, so the file standing
        // here is the dead run's own.
        let canonical = std::fs::canonicalize(&target).expect("the resolved target");
        let (lock, _) = super::lock_of(&std::env::temp_dir().join(super::LOCKS), &canonical);
        assert!(lock.is_file(), "{} went with its owner", lock.display());

        // When the OS lets the lock go is the machine's affair, so the
        // real claim is retried under the suite's budget.
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

    /// The roots handed in are the run's own, so what is left to ask at
    /// removal is who is still running and whether a root is marked.
    #[test]
    fn a_passing_run_gives_back_its_roots_and_spares_a_marked_one() {
        let base = crate::yard::Yard::new("giveback");
        let made = |leaf: &str| {
            let dir = base.join(leaf);
            std::fs::create_dir_all(&dir).expect("a root");
            std::fs::write(dir.join("a.txt"), "x").expect("something in it");
            dir
        };
        let scaffolding = made("basic-1-2-0");
        let another = made("stashes-1-3-0");
        let marked = made("kept-1-4-0");
        super::keep(&marked).expect("the mark");

        let roots = vec![scaffolding.clone(), another.clone(), marked.clone()];

        assert_eq!(super::give_back(roots.clone(), Some(vec![4242])), 0);
        assert!(scaffolding.exists(), "a root went while a process ran");
        assert_eq!(super::give_back(roots.clone(), None), 0);
        assert!(scaffolding.exists(), "a root went on an unread answer");

        let gone = super::give_back(roots, Some(Vec::new()));

        assert_eq!(gone, 2, "the count is what actually went");
        assert!(!scaffolding.exists(), "a passing run's root was left");
        assert!(!another.exists(), "a passing run's second root was left");
        assert!(marked.exists(), "a tree somebody asked to keep was taken");
    }

    /// On a `/proc` of the test's own, since the machine's cannot arrange
    /// either case: a process gone between listing and read is passed
    /// over, and an unreadable `stat` fails the whole reading, so nothing
    /// is removed.
    #[test]
    #[cfg(target_os = "linux")]
    fn a_reading_that_failed_keeps_the_roots() {
        let base = crate::yard::Yard::new("unreadable");
        let root = base.join("basic-1-2-0");
        std::fs::create_dir_all(&root).expect("a root to keep");

        let fake = base.join("proc");
        let me = std::process::id();
        let stat_for = |pid: u32, line: &str| {
            let dir = fake.join(pid.to_string());
            std::fs::create_dir_all(&dir).expect("a process directory");
            std::fs::write(dir.join("stat"), line).expect("its stat");
        };
        stat_for(me, &format!("{me} (xtask) R 1 {me} 0 0\n"));
        // In the listing and gone by the read: no `stat` at all.
        std::fs::create_dir_all(fake.join("4242")).expect("a process that went");

        let seen = crate::reap::others_in_group_under(&fake).expect("a readable answer");
        assert!(
            seen.is_empty(),
            "a process that had already gone was counted as running: {seen:?}"
        );
        assert_eq!(
            super::give_back(vec![root.clone()], Some(seen)),
            1,
            "nothing was in the way and the root stayed"
        );

        std::fs::create_dir_all(&root).expect("the root again");
        stat_for(4243, "nonsense with no bracket\n");
        assert!(
            crate::reap::others_in_group_under(&fake).is_none(),
            "a stat nobody could read was taken for an empty answer"
        );
        let kept = super::give_back(
            vec![root.clone()],
            crate::reap::others_in_group_under(&fake),
        );
        assert_eq!(kept, 0, "a root went on a reading that had failed");
        assert!(root.exists(), "a root went on a reading that had failed");
    }

    /// What the removal rests on where it cannot refuse itself (unix): a
    /// child in this runner's group, as a run's app and its git are, has
    /// to be in the list.
    #[test]
    #[cfg(target_os = "linux")]
    fn a_child_that_is_still_running_is_in_the_answer() {
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("a child that outlives the look");
        let running = crate::reap::others_in_this_group().expect("the listing");
        let seen = running.contains(&child.id());
        let _ = child.kill();
        let _ = child.wait();
        assert!(
            seen,
            "a child of this run was not in {running:?} — a removal would have gone ahead"
        );
    }
}
