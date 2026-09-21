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
    /// **Holding the handle is the claim.** The lock under it is the
    /// operating system's, unlocked when this is dropped
    /// ([`crate::locks`]) — and let go anyway when the process ends
    /// without dropping anything, which is what a killed run does.
    _lock: crate::locks::Locked,
}

/// Atomically reserve an explicitly shared path for this process.
/// Claims live outside the target, so a repository under test
/// stays clean.
///
/// **The operating system holds the claim.** A run that is killed
/// leaves its file standing with no lock on it, so there is no dead
/// claim to tell from a live one, and no pid to be wrong about when the
/// machine hands the number out again. What the file says is a note for
/// a person ([`note`]), read by nothing.
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
    // is before the lock, and the standing owner's note stays its
    // owner's. Emptying it is [`note`]'s, under the lock.
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

/// Where the lock files live. **A claim leaves its file standing.**
/// Unlinking a locked file is allowed on both platforms, and it hands
/// one path to two runs: the next asker creates a fresh file under the
/// name and locks that, while the first still holds the one that was
/// unlinked. The sweep takes them, at a day old — which is longer than
/// any claim lives, a run being minutes and its watchdog two — and a
/// file still held is one whose note was written today.
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
/// them. **A person's to read**, and a refusal cannot quote it:
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

/// The directories under the system temp a run leaves something in.
/// Five it claims its own directory in (`claim_dir`'s callers): a
/// verb's pictures and settings, its demo repositories
/// (`demo::claim_root`), a container run's mount
/// (`keepsakes::keepsake_dir`), the roots the tests claim, and the
/// trees the suite stands its files in ([`crate::yard`]) — which their
/// own guards remove, so only a killed run leaves one here. And one it
/// leaves a file in — [`LOCKS`], where a claim is taken and which
/// nothing else ever clears.
const RUN_BASES: [&str; 6] = [
    "pgg-verify",
    "pgg-demo",
    "pgg-linux",
    "pgg-census",
    crate::yard::BASE,
    LOCKS,
];

/// How long a run's directory stands before it is litter. A run is
/// minutes long — its watchdog is two, a cold build ten — and the
/// pictures a person was shown are on the board (`shots`), so a day
/// later what is left under these is nobody's evidence.
const RUN_LITTER_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// The file that takes a directory out of every sweep, for good.
const KEEP: &str = ".pgg-keep";

/// Marks `root` as a tree a person asked for, which is the whole of
/// what the sweep goes by.
///
/// **The mark is what can be asked.** A repository this application is
/// showing is held by nothing — git is a subprocess per operation and
/// no directory is watched — so a handle would answer "nobody's" for
/// the tab that is on screen; a tree built to be looked at later is
/// nobody's by definition; and under `pgg-linux` the only side that
/// could hold one is across the mount.
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

/// Takes away the demo roots **this run** claimed, now that it has
/// passed. Answers how many went.
///
/// **A run that passed has nothing left to look at.** What a person
/// reads off a run is its pictures and its report, which are elsewhere
/// (`keepsakes`, the board); the repository it built is scaffolding. A
/// run that *failed* is the opposite — the tree it stopped on is the
/// scene, and the container it ran in is gone, so the volume is the only
/// place that scene survives ([failed-run forensics]). So this is called
/// on the passing road only, and the sweeps that collect what failures
/// leave behind stay exactly as they were ([`sweep_yesterdays_runs`] on
/// the host, the preparation step's floor in the container).
///
/// **Four things it cannot reach**, and none of them by a guess about
/// age or by reading the directory back:
///
/// * **Another run's roots.** The list is what *this process* claimed
///   (`demo::claimed_roots`), and a claim is a `create_dir` that
///   succeeded — so no name in it was ever anybody else's.
/// * **A template.** Templates are built at a fixed name, not claimed,
///   so they are not in the list at all.
/// * **A tree somebody asked to keep.** [`KEEP`] is checked anyway: a
///   root claimed by `demo-repo` is marked the moment it is built, and
///   the rule reads better where the removing happens.
/// * **A directory something is still writing into.** The app having
///   exited says nothing about the git it spawned: on unix those are
///   re-parented to init the instant it goes, and a directory with a
///   file open in it is removed without complaint. So the question is
///   asked outright — [`crate::reap::others_in_this_group`] — and
///   **nothing is removed unless the answer is nobody**. On Windows the
///   same question is answered by the removal itself, which an open
///   handle refuses.
pub(super) fn give_back_claimed(before: Option<&[u32]>) -> usize {
    give_back(crate::demo::claimed_roots(), arrived_since(before))
}

/// Who is in this run's group now that was not there before the app
/// started — which is the whole of what "the app left something
/// running" can mean here.
///
/// **The group alone is the wrong question.** It holds whatever else
/// this runner was started inside: under a container, the shell the
/// command is wrapped in and its own children (measured — the plain
/// question said two processes were running after every passing run,
/// and the removal never happened). Taking the group before the app
/// and again after leaves only what the app added.
///
/// A number handed out again between the two looks would be read as
/// having been there all along. The two looks are a run apart and
/// Linux hands numbers out in order, so that is not a second this
/// costs a thought; it is written down because it is the one way this
/// answer can be wrong in the unsafe direction.
///
/// `None` either side is "could not be found out", which is not
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

/// [`give_back_claimed`] over what somebody hands in, which is the half
/// a test can drive: the list the caller above reads is this process's
/// own, and a test that emptied it would empty the other tests' too.
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

/// Takes yesterday's run directories away, in the background. Nothing
/// else ever does for the ones a failure left: every run claims a
/// directory, the passing ones give theirs back
/// ([`give_back_claimed`]), and the gate runs hundreds of them a day
/// (measured: sixty thousand of them, six gigabytes, three days after
/// the last sweep by hand). The gate
/// calls this on its way in; the thread is left to itself — a plan is
/// half a second and the temp directory is seconds of reading — and a
/// directory that will not go, or a sweep the process ends first, is the
/// next sweep's. Nothing is said: a gate's verdict is about its tests.
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
/// carrying [`KEEP`] stays whatever its date: age makes a run's
/// directory litter, and a marked tree is somebody's.
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

        // When the operating system lets the lock go is the machine's
        // own affair: the claim is tried under the suite's budget, every
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

    /// **What a passing run gives back, and the one thing it does not.**
    /// The roots are the run's own — a claim is a `create_dir` that
    /// succeeded — so the only question left at the moment of removal is
    /// whether somebody asked for one to be kept.
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

        // Something this run started is still going: nothing is taken.
        assert_eq!(super::give_back(roots.clone(), Some(vec![4242])), 0);
        assert!(scaffolding.exists(), "a root went while a process ran");
        // And an answer nobody could read is not "nobody".
        assert_eq!(super::give_back(roots.clone(), None), 0);
        assert!(scaffolding.exists(), "a root went on an unread answer");

        let gone = super::give_back(roots, Some(Vec::new()));

        assert_eq!(gone, 2, "the count is what actually went");
        assert!(!scaffolding.exists(), "a passing run's root was left");
        assert!(!another.exists(), "a passing run's second root was left");
        assert!(marked.exists(), "a tree somebody asked to keep was taken");
    }

    /// **A reading that failed reaches the removal as a refusal.** The
    /// two answers the real path has to get right are arranged on a
    /// `/proc` of the test's own, because neither can be arranged on
    /// the machine's: a process that vanished between the listing and
    /// the read is not running and is passed over, and a `stat` that
    /// cannot be made sense of is not an empty answer — it takes the
    /// whole reading with it, and nothing is removed.
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

        // Now one that is there and says nothing this can read.
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

    /// **A process that outlives the app is seen.** This is the whole of
    /// what the removal rests on where a removal cannot refuse itself:
    /// unix answers by group, so a child started here — which is in this
    /// runner's group, as a verified run's app and its git are — has to
    /// show up in the list.
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
