//! The machine held still for a measurement: the one hold a measurement
//! takes on it, and the wait every verb here that builds makes on that
//! hold.
//!
//! A real-window measurement and a build cannot share a machine. The
//! build shows up in the run's host conditions as load that was not the
//! application, and the run is refused and taken again
//! (`perf::sampler::Limits`); the measurement's window and its
//! vsync-stepped bench slow the build in turn. The CPU gate sees that a
//! build happened only after the run it spoiled, and on a twenty-four
//! thread machine it cannot tell a build from a browser by the counters
//! either. What it cannot do is *order* the two, and that is what this
//! does: `perf` holds the machine still for the length of its runs, every
//! verb here that builds or starts the app waits for a hold to lift
//! before it begins, and a hold waits for the builds already under way to
//! finish. The pre-shell hook refuses a bare `cargo build` or `cargo
//! test` typed while a hold stands, because that one runs outside any
//! verb that could wait (hook/still.rs).
//!
//! Both halves are files beside the repository's own `.git`, which every
//! worktree shares: the hold is one file naming its process, the builds
//! under way one file each. A file whose process is gone is litter and is
//! cleared by whoever meets it, the way a dead seat claim is
//! (`seats::claim_is_dead`) — a killed xtask never unwinds, so nothing
//! else would.
//!
//! **A step a verb starts is under its parent's announcement**, and says
//! nothing of its own ([`UNDER`]): a gate's verify-ui step that waited on
//! the hold would wait for a measurement that is waiting for the gate.
//! The container is under it for a different reason — its processes are
//! not this machine's, so a liveness it answered would be wrong.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::subprocess::{git_query, process_exists};

/// Set on every step a verb here starts (`check::run_step`), so the step
/// neither announces a build its parent already announced nor waits on a
/// hold its parent is being waited for by.
pub(crate) const UNDER: &str = "PG_STILL_UNDER";

/// The hold, beside `.git`: one file naming the measuring process.
const HOLD: &str = "pg-still";

/// The builds under way, beside `.git`: one file per process.
const BUSY: &str = "pg-busy";

/// How long a build waits for a hold to lift. A measurement is minutes of
/// runs, retries included; a hold this old is a run that stopped
/// answering, and the message says which process to look at.
const HOLD_CEILING: Duration = Duration::from_secs(60 * 60);

/// How long a hold waits for the builds already under way. A gate is
/// minutes; a container image built for the first time is more.
const BUSY_CEILING: Duration = Duration::from_secs(30 * 60);

/// How often either side looks again. Short under test, where the waits
/// are measured in the hundreds of milliseconds.
const POLL: Duration = if cfg!(test) {
    Duration::from_millis(50)
} else {
    Duration::from_secs(5)
};

/// A hold on the machine, lifted when dropped. Empty where there was
/// nothing to hold against: under a parent's announcement, in the
/// container, or outside any repository.
#[derive(Debug)]
pub(crate) struct Hold(Option<PathBuf>);

impl Drop for Hold {
    fn drop(&mut self) {
        if let Some(path) = &self.0 {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// A build announced, withdrawn when dropped. Empty for the same reasons
/// a [`Hold`] is.
#[derive(Debug)]
pub(crate) struct Busy(Option<PathBuf>);

impl Drop for Busy {
    fn drop(&mut self) {
        if let Some(path) = &self.0 {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Holds the machine still for `what`, once the builds already under way
/// have finished. Refused while another hold stands: two measurements
/// spoil each other as surely as a build spoils one.
pub(crate) fn hold(tree: &Path, what: &str) -> Result<Hold, String> {
    match common_dir(tree) {
        Some(common) => hold_in(&common, what),
        None => Ok(Hold(None)),
    }
}

/// Announces a build for `what`, once any hold has lifted.
pub(crate) fn busy(tree: &Path, what: &str) -> Result<Busy, String> {
    match common_dir(tree) {
        Some(common) => busy_in(&common, what),
        None => Ok(Busy(None)),
    }
}

/// This workspace, and the build in it announced: the two lines every
/// verb that builds here opens with, as one.
pub(crate) fn announced(what: &str) -> Result<(PathBuf, Busy), String> {
    let root = crate::tree::workspace_root();
    let busy = busy(&root, what)?;
    Ok((root, busy))
}

/// `cargo xtask still`: what stands right now, for a session deciding
/// whether to wait.
pub fn run(args: &[String]) -> Result<(), String> {
    if !args.is_empty() {
        return Err(format!("still takes no arguments (got {args:?})"));
    }
    let common = common_dir(&crate::tree::workspace_root())
        .ok_or("not inside a repository — there is nothing to hold still")?;
    match read_note(&common.join(HOLD)) {
        Some(note) if note.alive() => println!(
            "held still by {} (pid {}, for {})",
            note.what,
            note.pid,
            note.age()
        ),
        Some(note) => println!(
            "a hold left by {} (pid {}), whose process is gone — litter the next verb clears",
            note.what, note.pid
        ),
        None => println!("nothing holds the machine still"),
    }
    let under_way = live_notes(&common.join(BUSY));
    if under_way.is_empty() {
        println!("no build under way (of the ones cargo xtask verbs announce)");
    }
    for note in under_way {
        println!(
            "under way: {} (pid {}, for {})",
            note.what,
            note.pid,
            note.age()
        );
    }
    Ok(())
}

/// Why the pre-shell hook holds `command`, run from `cwd`, or nothing:
/// a bare cargo build typed while a hold stands.
pub(crate) fn objection(cwd: &str, command: &str) -> Option<String> {
    if !builds_outside_a_verb(command) {
        return None;
    }
    let common = common_dir(Path::new(cwd))?;
    let note = read_note(&common.join(HOLD)).filter(Note::alive)?;
    Some(format!(
        "A measurement is holding the machine still ({}, pid {}, for {}): a build beside it \
         is counted as load that was not the application, and the run is refused and taken \
         again — so this cargo line is held the way every `cargo xtask` verb holds itself. \
         Use the verb (`cargo xtask gate` / `check` / `verify-ui` wait for the hold by \
         themselves), or come back once `cargo xtask still` says the hold is gone.",
        note.what,
        note.pid,
        note.age()
    ))
}

/// Whether `command` runs cargo in a way that builds, outside any `cargo
/// xtask` verb. Judged per shell segment, so `cargo xtask structure &&
/// cargo build` is still a bare build.
fn builds_outside_a_verb(command: &str) -> bool {
    const BUILDS: [&str; 12] = [
        "build", "b", "test", "t", "check", "c", "clippy", "run", "r", "bench", "doc", "install",
    ];
    command.split([';', '|', '&']).any(|segment| {
        let tokens: Vec<&str> = segment
            .split_whitespace()
            .map(|token| token.trim_matches(['"', '\'']))
            .collect();
        !tokens.contains(&"xtask")
            && tokens
                .windows(2)
                .any(|pair| pair[0] == "cargo" && BUILDS.contains(&pair[1]))
    })
}

/// The repository's shared `.git`, which every worktree of it resolves to
/// the same — and so the one place a hold is seen from every seat.
fn common_dir(tree: &Path) -> Option<PathBuf> {
    if under() {
        return None;
    }
    let here = tree.to_string_lossy().replace('\\', "/");
    git_query(
        &here,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
    .map(PathBuf::from)
}

/// Whether this process is a step of a verb that already announced
/// itself, or the container's.
fn under() -> bool {
    std::env::var_os(UNDER).is_some() || std::env::var_os(crate::linux::IN_CONTAINER).is_some()
}

fn hold_in(common: &Path, what: &str) -> Result<Hold, String> {
    let path = common.join(HOLD);
    let note = Note::now(what);
    loop {
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                file.write_all(note.text().as_bytes())
                    .map_err(|e| format!("could not write the hold at {}: {e}", path.display()))?;
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                match read_note(&path) {
                    Some(other) if other.alive() => {
                        return Err(format!(
                            "another measurement holds the machine still: {} (pid {}, for {}) — \
                             one at a time, or the two spoil each other. `cargo xtask still` \
                             says when it is gone.",
                            other.what,
                            other.pid,
                            other.age()
                        ));
                    }
                    // Litter, or a hold half-written by a process that is
                    // gone: cleared, and the claim tried again.
                    _ => {
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
            Err(error) => {
                return Err(format!(
                    "could not take the hold at {}: {error}",
                    path.display()
                ));
            }
        }
    }
    // Dropped on the way out of a wait that failed, so a hold that never
    // got its quiet does not stand in everybody's way.
    let hold = Hold(Some(path));
    wait_for_builds(&common.join(BUSY), what)?;
    Ok(hold)
}

/// Waits until no build announced beside `.git` is still running.
fn wait_for_builds(busy: &Path, what: &str) -> Result<(), String> {
    let started = Instant::now();
    let mut said: Vec<u32> = Vec::new();
    loop {
        let under_way = live_notes(busy);
        if under_way.is_empty() {
            return Ok(());
        }
        let pids: Vec<u32> = under_way.iter().map(|note| note.pid).collect();
        if pids != said {
            println!(
                "  {what} holds the machine still, and waits for {} build(s) already under way: {}",
                under_way.len(),
                under_way
                    .iter()
                    .map(|note| format!("{} (pid {}, for {})", note.what, note.pid, note.age()))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            said = pids;
        }
        if started.elapsed() >= BUSY_CEILING {
            return Err(format!(
                "the builds under way did not finish within {} minutes — the hold is lifted; \
                 `cargo xtask still` names them",
                BUSY_CEILING.as_secs() / 60
            ));
        }
        std::thread::sleep(POLL);
    }
}

fn busy_in(common: &Path, what: &str) -> Result<Busy, String> {
    let hold = common.join(HOLD);
    let dir = common.join(BUSY);
    let started = Instant::now();
    let mut said = false;
    loop {
        wait_for_hold(&hold, what, started, &mut said)?;
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("could not make {}: {e}", dir.display()))?;
        let mine = dir.join(std::process::id().to_string());
        // Whole when it appears: a reader that met a half-written note
        // would clear it as litter.
        let staged = dir.join(format!("{}.staged", std::process::id()));
        std::fs::write(&staged, Note::now(what).text())
            .and_then(|()| std::fs::rename(&staged, &mine))
            .map_err(|e| format!("could not announce the build at {}: {e}", mine.display()))?;
        // A hold that came between the wait and the announcement wins:
        // the measurement is the one that cannot share.
        match read_note(&hold) {
            Some(other) if other.alive() => {
                let _ = std::fs::remove_file(&mine);
            }
            _ => return Ok(Busy(Some(mine))),
        }
    }
}

/// Waits until no live hold stands at `hold`, clearing a dead one.
fn wait_for_hold(hold: &Path, what: &str, started: Instant, said: &mut bool) -> Result<(), String> {
    loop {
        match read_note(hold) {
            Some(other) if other.alive() => {
                if !*said {
                    println!(
                        "  a measurement holds the machine still ({}, pid {}, for {}) — {what} \
                         waits for it to end",
                        other.what,
                        other.pid,
                        other.age()
                    );
                    *said = true;
                }
                if started.elapsed() >= HOLD_CEILING {
                    return Err(format!(
                        "the measurement did not end within {} minutes, and pid {} is still \
                         running — look at it before building beside it",
                        HOLD_CEILING.as_secs() / 60,
                        other.pid
                    ));
                }
                std::thread::sleep(POLL);
            }
            Some(_) => {
                let _ = std::fs::remove_file(hold);
                return Ok(());
            }
            None => return Ok(()),
        }
    }
}

/// The builds under way, with the litter of dead ones cleared as it is
/// met.
fn live_notes(busy: &Path) -> Vec<Note> {
    let Ok(entries) = std::fs::read_dir(busy) else {
        return Vec::new();
    };
    let mut live: Vec<Note> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_none())
        .filter_map(|path| match read_note(&path) {
            Some(note) if note.alive() => Some(note),
            _ => {
                let _ = std::fs::remove_file(&path);
                None
            }
        })
        .collect();
    live.sort_by_key(|note| note.pid);
    live
}

fn read_note(path: &Path) -> Option<Note> {
    Note::parse(&std::fs::read_to_string(path).ok()?)
}

/// One note, as a file records it: the process, when it began, and what
/// it is doing.
struct Note {
    pid: u32,
    since: u64,
    what: String,
}

impl Note {
    fn now(what: &str) -> Self {
        Self {
            pid: std::process::id(),
            since: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |since| since.as_secs()),
            what: what.to_string(),
        }
    }

    fn text(&self) -> String {
        format!(
            "pid {}\nsince {}\nwhat {}\n",
            self.pid, self.since, self.what
        )
    }

    fn parse(text: &str) -> Option<Self> {
        let field = |key: &str| {
            text.lines()
                .find_map(|line| line.strip_prefix(key))
                .map(str::trim)
        };
        Some(Self {
            pid: field("pid ")?.parse().ok()?,
            since: field("since ")?.parse().ok()?,
            what: field("what ")?.to_string(),
        })
    }

    fn alive(&self) -> bool {
        process_exists(self.pid)
    }

    fn age(&self) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        crate::seats::format_age(Some(Duration::from_secs(now.saturating_sub(self.since))))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use super::{BUSY, HOLD, Note, builds_outside_a_verb, busy_in, hold_in};

    /// A `.git`-shaped directory of this test's own.
    fn common(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pg-still-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a directory to hold in");
        dir
    }

    /// A pid that has certainly exited: our own child, reaped.
    fn dead_pid() -> u32 {
        let mut probe = if cfg!(windows) {
            let mut command = std::process::Command::new("cmd");
            command.args(["/C", "exit 0"]);
            command
        } else {
            std::process::Command::new("true")
        };
        let mut child = probe.spawn().expect("spawn a short-lived child");
        let pid = child.id();
        child.wait().expect("reap the child");
        pid
    }

    #[test]
    fn a_hold_waits_for_the_builds_under_way() {
        let dir = common("hold-waits");
        let build = busy_in(&dir, "gate").expect("a build announced");
        let held_in = dir.clone();
        let held = std::thread::spawn(move || hold_in(&held_in, "perf").map(|_| ()));
        std::thread::sleep(Duration::from_millis(300));
        assert!(
            !held.is_finished(),
            "the hold must wait for the build under way"
        );
        drop(build);
        held.join()
            .expect("the hold's thread")
            .expect("the hold, once the build is gone");
        assert!(!dir.join(HOLD).exists(), "a hold is lifted with its guard");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_build_waits_for_a_hold_to_lift() {
        let dir = common("build-waits");
        let hold = hold_in(&dir, "perf").expect("the hold");
        let building_in = dir.clone();
        let build = std::thread::spawn(move || busy_in(&building_in, "check").map(|_| ()));
        std::thread::sleep(Duration::from_millis(300));
        assert!(!build.is_finished(), "the build must wait for the hold");
        drop(hold);
        build
            .join()
            .expect("the build's thread")
            .expect("the build, once the hold lifted");
        let left = std::fs::read_dir(dir.join(BUSY))
            .map(|entries| entries.count())
            .unwrap_or(0);
        assert_eq!(left, 0, "an announcement is withdrawn with its guard");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A killed xtask never unwinds, so a hold or an announcement can be
    /// left behind: whoever meets one whose process is gone clears it.
    #[test]
    fn a_note_whose_process_is_gone_is_litter() {
        let dir = common("litter");
        let dead = Note {
            pid: dead_pid(),
            since: 0,
            what: "perf".into(),
        };
        std::fs::write(dir.join(HOLD), dead.text()).expect("a dead hold");
        let build = busy_in(&dir, "check").expect("a dead hold holds nothing");
        assert!(!dir.join(HOLD).exists());
        // Withdrawn before the hold below, which would otherwise wait for
        // this very test.
        drop(build);
        std::fs::create_dir_all(dir.join(BUSY)).expect("the busy directory");
        std::fs::write(dir.join(BUSY).join("1"), dead.text()).expect("a dead announcement");
        let _hold = hold_in(&dir, "perf").expect("a dead build is not waited for");
        assert!(!dir.join(BUSY).join("1").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_second_hold_is_refused_while_the_first_stands() {
        let dir = common("two-holds");
        let _first = hold_in(&dir, "perf").expect("the first hold");
        let refused = hold_in(&dir, "perf again").expect_err("two measurements at once");
        assert!(refused.contains("another measurement"), "{refused}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_note_reads_back_as_it_was_written() {
        let note = Note {
            pid: 7,
            since: 11,
            what: "cargo xtask perf".into(),
        };
        let back = Note::parse(&note.text()).expect("a whole note parses");
        assert_eq!(
            (back.pid, back.since, back.what.as_str()),
            (7, 11, "cargo xtask perf")
        );
        assert!(Note::parse("something else").is_none());
    }

    /// The hook holds the cargo that builds outside any verb, and only
    /// that: the verbs wait for the hold by themselves.
    #[test]
    fn a_bare_cargo_build_is_what_the_hook_holds() {
        for line in [
            "cargo build --release",
            "cd x && cargo test -p platitude-core",
            "PG_X=1 cargo clippy --workspace -- -D warnings",
            "cargo xtask structure && cargo build",
            "cargo run -p platitude-app",
        ] {
            assert!(builds_outside_a_verb(line), "{line}");
        }
        for line in [
            "cargo xtask gate --host-only",
            "cargo run -p xtask -- structure",
            "cargo fmt --all -- --check",
            "git status",
            "cargo tree",
        ] {
            assert!(!builds_outside_a_verb(line), "{line}");
        }
    }
}
