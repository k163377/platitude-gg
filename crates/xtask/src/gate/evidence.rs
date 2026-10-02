//! What a red step leaves behind for the run after it.
//!
//! A step's log is named by the step's index (`super::step::log_of`), so the
//! next gate in this tree writes over it. A red step's log is copied here
//! under the run that wrote it, where only this module's own sweep takes it
//! away.
//!
//! Cargo stopping on the lock file (a cause the message does not give, the
//! change unjudged) also gets the host's side written beside the log while
//! the tree still stands as it stood: the files a resolve reads, the host's
//! cargo, and the image. The container's side is in the log already
//! (`linux::watched_from_inside`); neither side is the read cargo made.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::subprocess::{Answer, bounded};

/// How many runs' worth of red logs stand here — enough to outlive the
/// re-run that follows a red gate, and the one after that.
const KEEP: usize = 10;

/// How long a question asked about a red step may take. This runs after
/// the step's watched run has ended, on the gate's thread and inside the
/// step's ticket, so an engine that does not answer would hold both
/// unbounded. Sized for a daemon that is not up.
const ASKING_CEILING: Duration = Duration::from_secs(15);

/// The files a resolve reads before it decides the lock is wrong: the
/// three at the root and every member's own manifest — evidence that
/// stopped at the root would miss a member manifest that arrived wrong.
///
/// Read off the tree as `crates/*/Cargo.toml`, where the root Cargo.toml
/// keeps the members; a member kept anywhere else would not be in here.
/// The container's look globs the same shape (`linux::watched_from_inside`).
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
/// it found it: an absent lock is "cannot create", a zero-byte one (valid
/// TOML, an empty resolve) "cannot update", one cut short mid-entry
/// "failed to parse". A lock merely older than the registry still
/// satisfies the manifests and is none of the three.
///
/// The mapping runs one way: a manifest that arrived wrong also gives
/// "cannot update".
///
/// Only cargo's own `error: ` line counts: this file's tests quote the
/// messages, and a red `test xtask` carrying them would otherwise be filed
/// as a cargo that never ran.
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
/// An error is the keeping's own, never a second failure of the step.
pub(crate) fn keep(
    logs: &Path,
    run: &str,
    tree: &Path,
    log: &Path,
    command: &[String],
) -> Result<Option<String>, String> {
    // No log is a step that never wrote one (the tests' faked runs).
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
    // Named after this log: the two sides fail on their own threads, and
    // two reds sharing one scratch file would read each other's answers.
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
/// Both sides print the same digest (`crate::digest`, against the
/// container's `sha256sum`): with digests of different kinds, only byte
/// counts would compare, and those cannot tell two files of one length
/// apart.
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
        let mut inspect = crate::linux::engine::command();
        inspect.args(["image", "inspect", "--format", "json", &tag]);
        let answer = asked("the image inspect", inspect, ASKING_CEILING, asking);
        out.push_str(&format!(
            "image     {tag}\nimage id  {}\n",
            crate::linux::engine::field(&answer, "Id").unwrap_or(answer)
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

/// How much of the digest either side prints — the container's look cuts
/// to the same width, so the two lines stand side by side.
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

/// Removes all but the newest [`KEEP`] runs. A run's name starts with
/// the second it started, so names sort by age.
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

    /// The three refusals, in cargo's own words ([`stopped_on_the_lock`]).
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
        // The verb's own red, naming the file: the step failing.
        assert!(!stopped_on_the_lock(
            "FAIL: the census moved a line for Cargo.lock"
        ));
        // A log that quotes cargo's line (this file's own tests, red) is
        // not cargo's line.
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

    /// The container prints `sha256sum | cut -c1-16` for the same bytes.
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

    /// A question that never answers is `bounded`'s to test
    /// (`subprocess`, `verify::look`).
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
