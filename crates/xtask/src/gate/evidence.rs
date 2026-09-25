//! What a red step leaves behind for the run after it.
//!
//! A step's log is named by the step's index (`super::step::log_of`), so the
//! next gate in this tree writes over it: the one container run that has
//! ever been stopped by `Cargo.lock` lost its log to the re-run that
//! passed, and what it said is known only from a terminal that happened
//! to still hold it. A red step's log is copied here under the run that
//! wrote it, where nothing but this module's own sweep takes it away.
//!
//! One shape of red gets more than a copy. Cargo stopping on the lock
//! file is a cargo that did not agree with a file on disk, for a
//! reason that is not in the message, and the change stands unjudged.
//! The host's side of that same question is
//! written beside the log while the tree still stands as it stood: what
//! the files a resolve reads are out here (the root's three and every
//! member's manifest), which cargo is out here, and which image was in
//! there. The container's side of it is in the log already
//! (`linux::watched_from_inside`), and **neither side is the read cargo
//! made** — see that function for what a bracket can and cannot settle.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::subprocess::{Answer, bounded};

/// How many runs' worth of red logs stand here. A run keeps only the
/// logs of the steps that failed, so this is small; what it has to
/// outlive is the re-run that follows a red gate, and the one after
/// that.
const KEEP: usize = 10;

/// How long a question asked about a red step may take. **The evidence
/// is held to a ceiling of its own**: this runs after the step's
/// own watched run has ended, on the gate's own thread and inside the
/// step's ticket, so a docker that does not answer would hold both for
/// as long as it liked. A daemon that is up answers in well under a
/// second; what this height is set for is one that is not.
const ASKING_CEILING: Duration = Duration::from_secs(15);

/// The files a resolve reads before it decides the lock is wrong: the
/// three at the root, and **every member's own manifest** — a resolve
/// reads all of them, so evidence that stopped at the root would call a
/// run sound when it was a member manifest that arrived wrong.
///
/// Read off the tree, so a member added to the
/// workspace is not a member this forgets. What it reads is every
/// `crates/*/Cargo.toml`, which is where this workspace keeps its
/// members (root Cargo.toml); **a member kept anywhere else would not be
/// in here**. The container's own look globs the same shape
/// (`linux::watched_from_inside`).
fn read_to_resolve(tree: &Path) -> Vec<String> {
    let mut files: Vec<String> = ["Cargo.lock", "Cargo.toml", ".cargo/config.toml"]
        .map(str::to_string)
        .to_vec();
    let mut members: Vec<String> = std::fs::read_dir(tree.join("crates"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .map(|member| format!("crates/{member}/Cargo.toml"))
        .filter(|manifest| tree.join(manifest).is_file())
        .collect();
    members.sort();
    files.extend(members);
    files
}

/// Cargo's three ways of saying it would not stand on the lock file as
/// it found it: it would have rewritten one, written a first one, or
/// could not read the one that is there.
///
/// **Measured** (2026-09-15, a throwaway crate, the host's cargo): an
/// absent lock is "cannot create", a lock of zero bytes is "cannot
/// update" — an empty file is valid TOML, so it parses as a resolve with
/// nothing in it — and a lock cut short mid-entry is "failed to parse".
/// A lock that is merely older than the registry is none of the three:
/// it still satisfies the manifests, so cargo stands on it.
///
/// **The mapping runs one way.** "cannot update" is what cargo
/// says about any lock its resolve disagrees with, an empty one being
/// only the cheapest way to get there — a manifest that arrived wrong
/// says the same thing. Which is why the evidence beside this is
/// the bytes of every file the resolve reads, on both sides, as
/// they stood.
///
/// **Cargo's own line.** A log that
/// quotes the message is not a cargo that stopped — this file's own
/// tests hold those sentences as data, so a red `test xtask` carrying
/// them in a panic would otherwise be filed as a cargo that never ran.
fn stopped_on_the_lock(log: &str) -> bool {
    log.lines().any(|line| {
        let Some(said) = line.trim_start().strip_prefix("error: ") else {
            return false;
        };
        [
            "cannot update the lock file",
            "cannot create the lock file",
            "failed to parse lock file",
        ]
        .iter()
        .any(|shape| said.starts_with(shape))
    })
}

/// Copies a red step's log under this run, and — when the log is cargo
/// stopping on the lock file — writes the host's side of it alongside.
/// Answers the line the gate should say about this red, if any.
///
/// The error is the keeping's own: a log that could not be copied is
/// worth saying so, and is never a second failure of the step.
pub(crate) fn keep(
    logs: &Path,
    run: &str,
    tree: &Path,
    log: &Path,
    command: &[String],
) -> Result<Option<String>, String> {
    // No log is a step that never wrote one (the tests' faked
    // runs), and there is nothing here to do about
    // it.
    let Ok(wrote) = std::fs::read(log) else {
        return Ok(None);
    };
    let said = String::from_utf8_lossy(&wrote);
    let kept = logs.join("failed");
    let dir = kept.join(run);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let name = log
        .file_name()
        .ok_or_else(|| format!("{} has no file name", log.display()))?;
    std::fs::copy(log, dir.join(name)).map_err(|e| format!("{}: {e}", log.display()))?;
    sweep(&kept);
    if !stopped_on_the_lock(&said) {
        return Ok(None);
    }
    let beside = dir.join(format!("{}.host.txt", name.to_string_lossy()));
    // Named after this log: both sides of a gate fail on their own
    // threads, and two reds sharing one scratch file would read each
    // other's answers.
    let asking = dir.join(format!("{}.asked.txt", name.to_string_lossy()));
    std::fs::write(&beside, host_side(run, tree, &asking, command, &said))
        .map_err(|e| format!("{}: {e}", beside.display()))?;
    Ok(Some(format!(
        "cargo stopped on the lock file — a cargo failure of undetermined cause, the change \
         unjudged. Both sides of it: {}",
        shown(tree, &dir)
    )))
}

/// The host's answer to what the container was asked, taken while the
/// tree still stands as it stood.
///
/// **Both sides are the same digest** (`crate::digest`, against the
/// container's `sha256sum`), cut to the same width: equal byte counts
/// and two digests of different kinds could not tell the same file from
/// a different one of the same length, which is the whole question.
///
/// Everything asked of another program in here is bounded
/// ([`ASKING_CEILING`]).
fn host_side(run: &str, tree: &Path, asking: &Path, command: &[String], said: &str) -> String {
    let mut out = format!("gate run  {run}\ntree      {}\n", tree.display());
    out.push_str(&format!("step      {}\n", command.join(" ")));
    let mut cargo = Command::new("cargo");
    cargo.arg("--version");
    out.push_str(&format!(
        "host cargo {}\n",
        asked("the host's cargo", cargo, ASKING_CEILING, asking)
    ));
    if let Some(tag) = image_in(said) {
        let mut docker = Command::new("docker");
        docker.args(["image", "inspect", "--format", "{{.Id}}", &tag]);
        out.push_str(&format!(
            "image     {tag}\nimage id  {}\n",
            asked("docker image inspect", docker, ASKING_CEILING, asking)
        ));
    }
    if std::fs::remove_file(asking).is_err() {
        out.push_str(&format!("(left {} behind)\n", asking.display()));
    }
    for file in read_to_resolve(tree) {
        out.push_str(&format!(
            "host {file} {}\n",
            as_it_stands(&tree.join(&file))
        ));
    }
    out.push_str(
        "\nthe container's own look at the same files is in the log beside this, on the \
         pgg-probe lines. bytes=0 on the before line says that run was handed an empty file; \
         two intact ends say only that nothing was standing wrong before the command and \
         after it — the read cargo itself made is in between, and is not bracketed.\n",
    );
    out
}

/// What another program said, or why it did not — within
/// `ceiling`, and never a second failure of the step.
fn asked(what: &str, command: Command, ceiling: Duration, scratch: &Path) -> String {
    match bounded(what, command, ceiling, Some(scratch)) {
        Answer::Ended { status, stdout } if status.success() => stdout.trim().to_string(),
        Answer::Ended { status, .. } => format!("{what} exited {status}"),
        Answer::OutOfTime { pid, after, ended } => format!(
            "{what} stood past {}s (pid {pid}) and was ended here{}",
            after.as_secs(),
            match ended {
                Ok(()) => String::new(),
                Err(why) => format!(" — or was not: {why}"),
            }
        ),
        Answer::Unstarted(why) => why,
    }
}

/// One file as this side has it: what a cargo out here would have read.
/// The digest is cut to the width the container's look prints, so the
/// two lines stand side by side.
fn as_it_stands(path: &Path) -> String {
    let Ok(bytes) = std::fs::read(path) else {
        return "absent".to_string();
    };
    let sha = crate::digest::sha256_hex(&bytes);
    format!(
        "bytes={} sha={}",
        bytes.len(),
        &sha[..SHA_SHOWN.min(sha.len())]
    )
}

/// How much of the digest either side prints. Sixteen hex characters is
/// what tells two files apart here; the whole thing would push the line
/// past the width of everything else in the file.
const SHA_SHOWN: usize = 16;

/// The image the run was in, from the line the launcher wrote when it
/// came back non-zero (`linux::in_container`).
fn image_in(said: &str) -> Option<String> {
    said.lines()
        .filter_map(|line| line.trim().strip_prefix("error: the run in "))
        .filter_map(|rest| rest.split_whitespace().next())
        .next_back()
        .map(str::to_string)
}

/// The path as the person reading the gate's line is standing: inside
/// the tree if it is inside it.
fn shown(tree: &Path, path: &Path) -> String {
    path.strip_prefix(tree)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

/// All but the newest [`KEEP`] runs. The name is the second the run
/// started, which sorts as it counts.
fn sweep(kept: &Path) {
    let Ok(entries) = std::fs::read_dir(kept) else {
        return;
    };
    let mut runs: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    if runs.len() <= KEEP {
        return;
    }
    runs.sort();
    for old in &runs[..runs.len() - KEEP] {
        if std::fs::remove_dir_all(old).is_err() {
            // A directory somebody is reading is not worth a red gate.
            println!("gate: left {} alone", old.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::yard::Yard;

    /// A directory of this test's own, gone when the test is.
    fn ours(name: &str) -> Yard {
        Yard::new(&format!("evidence-{name}"))
    }

    fn log_saying(dir: &Path, name: &str, text: &str) -> PathBuf {
        let logs = dir.join("target").join("gate-logs");
        std::fs::create_dir_all(&logs).expect("logs");
        let log = logs.join(name);
        std::fs::write(&log, text).expect("a log");
        log
    }

    /// The three refusals, in cargo's own words — measured against a
    /// throwaway crate, one state each (see [`stopped_on_the_lock`]).
    #[test]
    fn the_three_ways_cargo_says_it_would_not_stand_on_the_lock() {
        for said in [
            "error: cannot update the lock file /work/Cargo.lock because --locked was passed",
            "error: cannot create the lock file /work/Cargo.lock because --locked was passed",
            "error: failed to parse lock file at: /work/Cargo.lock",
        ] {
            assert!(stopped_on_the_lock(said), "{said:?}");
        }
        assert!(!stopped_on_the_lock(
            "error: test failed, to rerun pass `--lib`"
        ));
        // The verb's own red, which names the file for a different
        // reason: this is the step failing.
        assert!(!stopped_on_the_lock(
            "FAIL: the census moved a line for Cargo.lock"
        ));
        // A log that *quotes* cargo's line is not cargo's line — this
        // file's own tests are such a log, and a red `test xtask`
        // carrying them must stay the step's own failure.
        assert!(!stopped_on_the_lock(
            "test gate::evidence::tests::the_three_ways ... FAILED\n\
             note: \"error: cannot update the lock file /work/Cargo.lock\"\n"
        ));
        // Cargo's line inside a log of other lines is still cargo's.
        assert!(stopped_on_the_lock(
            "screenshots and settings: C:/x\n\
             error: cannot update the lock file /work/Cargo.lock because --locked was passed\n\
             error: the run in pgg-linux:core-x exited 101\n"
        ));
    }

    /// A resolve reads every member's manifest, so the evidence does.
    /// Read off the tree, so a member added to the workspace arrives
    /// here without anybody remembering to add it.
    #[test]
    fn every_members_manifest_is_part_of_what_a_resolve_reads() {
        let dir = ours("members");
        for member in ["platitude-core", "xtask"] {
            std::fs::create_dir_all(dir.join("crates").join(member)).expect("a member");
            std::fs::write(
                dir.join("crates").join(member).join("Cargo.toml"),
                "[package]\n",
            )
            .expect("its manifest");
        }
        // A directory under crates/ that is not a member of anything.
        std::fs::create_dir_all(dir.join("crates").join("not-a-crate")).expect("a stray");
        assert_eq!(
            read_to_resolve(&dir),
            vec![
                "Cargo.lock".to_string(),
                "Cargo.toml".to_string(),
                ".cargo/config.toml".to_string(),
                "crates/platitude-core/Cargo.toml".to_string(),
                "crates/xtask/Cargo.toml".to_string(),
            ]
        );
        // A tree with no crates/ at all is the three at the root.
        let bare = ours("bare");
        assert_eq!(read_to_resolve(&bare).len(), 3);
    }

    /// The same digest on both sides, cut to the same width: the
    /// container prints `sha256sum | cut -c1-16` for the same bytes.
    #[test]
    fn the_digest_is_the_one_the_container_prints() {
        let dir = ours("digest");
        let file = dir.join("Cargo.lock");
        std::fs::write(&file, "version = 4\n").expect("a lock");
        assert_eq!(
            as_it_stands(&file),
            format!(
                "bytes=12 sha={}",
                &crate::digest::sha256_hex(b"version = 4\n")[..16]
            )
        );
        assert_eq!(as_it_stands(&dir.join("nothing-here")), "absent");
    }

    /// A question that cannot be asked is an answer. What
    /// a question that never answers does is `bounded`'s own
    /// (`subprocess`, `verify::look`'s tests): the ceiling is passed in
    /// here so the call sites say what they are willing to wait.
    #[test]
    fn a_question_that_cannot_be_asked_answers_anyway() {
        let dir = ours("asking");
        let why = asked(
            "a missing program",
            Command::new("pgg-no-such-program-at-all"),
            Duration::from_secs(1),
            &dir.join("asked.txt"),
        );
        assert!(why.contains("could not be started"), "{why}");
    }

    #[test]
    fn the_image_comes_off_the_launchers_last_word() {
        let said = "screenshots and settings: C:/x\n\
                    error: the run in pgg-linux:app-d3f3924bdfb2ec3c exited 101\n";
        assert_eq!(
            image_in(said).as_deref(),
            Some("pgg-linux:app-d3f3924bdfb2ec3c")
        );
        assert_eq!(image_in("nothing of the sort\n"), None);
    }

    /// The whole point: the log of a red step outlives the re-run that
    /// would have written over it.
    #[test]
    fn a_red_steps_log_is_kept_where_the_next_run_does_not_reach() {
        let dir = ours("kept");
        let logs = dir.join("target").join("gate-logs");
        let log = log_saying(&dir, "linux-238.log", "error: test failed\n");
        let note = keep(&logs, "1789397704-55500", &dir, &log, &["cargo".into()]).expect("kept");
        assert_eq!(note, None, "an ordinary red says nothing new");
        let copy = logs
            .join("failed")
            .join("1789397704-55500")
            .join("linux-238.log");
        assert!(copy.is_file(), "{} is not there", copy.display());
        // The re-run writes over the log in place; the copy stands.
        std::fs::write(&log, "PASS\n").expect("the re-run's log");
        assert_eq!(
            std::fs::read_to_string(&copy).expect("the copy"),
            "error: test failed\n"
        );
    }

    #[test]
    fn cargo_stopping_on_the_lock_is_named_and_answered_from_this_side() {
        let dir = ours("lock");
        let logs = dir.join("target").join("gate-logs");
        std::fs::write(dir.join("Cargo.lock"), "version = 4\n").expect("a lock");
        let log = log_saying(
            &dir,
            "linux-238.log",
            "pgg-probe before Cargo.lock bytes=0 sha=e3b0c44298fc1c14\n\
             error: cannot update the lock file /work/Cargo.lock because --locked was passed\n",
        );
        let note = keep(
            &logs,
            "1789397704-55500",
            &dir,
            &log,
            &["cargo".into(), "test".into()],
        )
        .expect("kept")
        .expect("a line to say");
        assert!(note.contains("undetermined"), "{note}");
        let beside = logs
            .join("failed")
            .join("1789397704-55500")
            .join("linux-238.log.host.txt");
        let wrote = std::fs::read_to_string(&beside).expect("the host's side");
        assert!(wrote.contains("host Cargo.lock bytes=12"), "{wrote}");
        assert!(wrote.contains("host Cargo.toml absent"), "{wrote}");
        assert!(wrote.contains("step      cargo test"), "{wrote}");
    }

    #[test]
    fn a_step_that_wrote_no_log_leaves_nothing_behind() {
        let dir = ours("nolog");
        let logs = dir.join("target").join("gate-logs");
        let note = keep(
            &logs,
            "1789397704-55500",
            &dir,
            &logs.join("linux-99.log"),
            &[],
        )
        .expect("no log is fine");
        assert_eq!(note, None);
        assert!(!logs.join("failed").exists());
    }

    #[test]
    fn only_the_newest_runs_stand() {
        let dir = ours("sweep");
        let kept = dir.join("failed");
        for run in 0..KEEP + 3 {
            std::fs::create_dir_all(kept.join(format!("17893977{run:02}-1"))).expect("a run");
        }
        sweep(&kept);
        let mut left: Vec<String> = std::fs::read_dir(&kept)
            .expect("the kept runs")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(left.len(), KEEP);
        assert_eq!(left[0], format!("17893977{:02}-1", 3));
    }
}
