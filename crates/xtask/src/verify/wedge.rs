//! What a run that stopped answering is asked, once there is nothing left
//! to ask it.
//!
//! A run reaped at the parent's ceiling used to leave one word —
//! `TIMED OUT` — and the next one to happen started from there again
//! (internal-docs/P3-確認事項.md §check ハング調査で残った観察). The
//! process's own account of where it stood is the app's
//! (`harness::deadline` writes [`REPORT_FILE`] beside the pictures); this
//! is the rest of it, which only the parent can see: how long the app had
//! been silent and what it last said, what pictures reached the disk, and
//! how full the machine was.
//!
//! **Read at the ceiling and nowhere else.** Every line below costs a
//! directory listing and a probe of a handful of lock files, and a run
//! that answers never reaches any of it.

use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};

/// What a process its own deadline thread ended exits with, and the file
/// it leaves beside the pictures. Spelled again rather than shared:
/// xtask depends on std alone (CLAUDE.md 技術スタック), so the app's
/// `harness::deadline::WEDGED` and `REPORT_FILE` are the originals. A
/// drift shows up as a run reporting a bare `exit 97` with no account
/// beside it.
const WEDGED_EXIT: i32 = 97;
const REPORT_FILE: &str = "wedge.txt";

/// The directory the machine's verb lanes stand in, beside the
/// repository's `.git` (`crate::lanes`).
const LANES: &str = "pg-lanes";

/// Clears any account left in `shot_dir` by whoever had it last.
///
/// **Before the app starts, every run.** A named `--shot-dir` is allowed
/// to outlive the run that made it (`verify::run`), so without this a
/// run reaped at the ceiling would be handed the *previous* run's
/// account — a different pid and a different station, read as its own.
pub(super) fn clear_any_account(shot_dir: &Path) {
    if let Err(error) = std::fs::remove_file(shot_dir.join(REPORT_FILE))
        && error.kind() != std::io::ErrorKind::NotFound
    {
        println!(
            "  note: {} could not be cleared ({error}) — a wedge here would read as the last \
             run's",
            shot_dir.join(REPORT_FILE).display()
        );
    }
}

/// Whether the run ended at a ceiling rather than at anything it did:
/// reaped by the parent, or ended by the deadline thread the app carries
/// for the case the parent's kill cannot explain.
///
/// The app's own watchdog is not one of these. It fires from the event
/// loop, so a run it ended has said what it was doing and left a report
/// of its own (`super::outcome`).
pub(super) fn at_a_ceiling(ran: &super::child::Ran) -> bool {
    ran.timed_out || ran.status.and_then(|s| s.code()) == Some(WEDGED_EXIT)
}

/// Everything the parent can still say about a run that stopped
/// answering, as the lines to print under the verdict.
pub(super) fn account(shot_dir: &Path, ran: &super::child::Ran, shots: &[PathBuf]) -> Vec<String> {
    let mut lines = vec![match ran.timed_out {
        true => format!(
            "  the run was reaped at the ceiling after {:.1}s — the app was still standing and \
             had not ended itself",
            ran.elapsed.as_secs_f32()
        ),
        false => format!(
            "  the app ended itself at its own deadline after {:.1}s",
            ran.elapsed.as_secs_f32()
        ),
    }];
    lines.push(match self_account(shot_dir) {
        Some(said) => format!("  the app's own account: {said}"),
        None => format!(
            "  the app left no {REPORT_FILE}: it never reached its own deadline, so what ended \
             it is outside the process"
        ),
    });
    // Only for a run the parent reaped. A process that ended itself spoke
    // in the act of dying, so the seconds before that are its own
    // account's to give — which they are, and against the right moment.
    if ran.timed_out {
        lines.push(silence(ran));
    }
    // What the reaping took with the app — the git it was waiting on,
    // counted — or what could not be looked up and may still be running.
    if let Some(under) = &ran.reaped {
        lines.push(format!("  under the app: {under}"));
    }
    lines.push(format!(
        "  pictures on disk: {}",
        match shots.is_empty() {
            true => "none".to_string(),
            false => shots
                .iter()
                .filter_map(|shot| shot.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(" "),
        }
    ));
    lines.push(format!("  {}", lanes_line()));
    lines
}

/// What the app wrote down about itself, if it got that far.
fn self_account(shot_dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(shot_dir.join(REPORT_FILE)).ok()?;
    let said = text.trim();
    (!said.is_empty()).then(|| said.replace('\n', " / "))
}

/// How long the app had been silent, and what it last said. **The last
/// line, not the last report**: what a run says on its way past a wedge
/// is as often a Qt warning as a report of its own.
fn silence(ran: &super::child::Ran) -> String {
    let last = ran
        .err_lines
        .last()
        .or_else(|| ran.out_lines.last())
        .map_or_else(|| "-".to_string(), |line| format!("`{line}`"));
    match ran.quiet_for {
        Some(quiet) => format!(
            "  silent for the last {:.1}s of it; the last line was {last}",
            quiet.as_secs_f32()
        ),
        None => "  it never said anything at all: the silence is the whole run".to_string(),
    }
}

/// How full the machine was, in the lanes every gate on it shares. A run
/// that stopped answering while the machine was full reads differently
/// from one that stopped answering alone (`crate::lanes`).
fn lanes_line() -> String {
    let Some(lanes) = lanes_dir() else {
        return "lanes: not read (no repository here to find them beside)".to_string();
    };
    match held_in(&lanes) {
        Ok(counted) => format!("lanes: {}", counted.line()),
        Err(why) => format!("lanes: not read ({why})"),
    }
}

fn lanes_dir() -> Option<PathBuf> {
    let root = crate::tree::workspace_root();
    crate::subprocess::common_git_dir(&root.display().to_string())
        .map(|common| Path::new(&common).join(LANES))
}

/// What the probe made of the lanes: how many of each side somebody
/// holds, the marks a landing leaves while its verbs wait, and how many
/// files it could not answer for at all.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Counted {
    /// Ordered, so two reports of the same machine read the same.
    sides: std::collections::BTreeMap<String, (usize, usize)>,
    landings: usize,
    unprobed: usize,
}

impl Counted {
    fn line(&self) -> String {
        let mut said: Vec<String> = self
            .sides
            .iter()
            .map(|(side, (held, seen))| format!("{side} {held}/{seen} held"))
            .collect();
        if self.landings > 0 {
            said.push(format!("{} landing mark(s) standing", self.landings));
        }
        // Said rather than folded into the counts. A container's runner may
        // have no road to the lanes at all — a seat's `.git` is a file
        // naming a directory outside the mount, so the lock files are not
        // there to open — and a silent `0/8` would read as an idle
        // machine, which is the one answer this line must never give
        // wrongly.
        if self.unprobed > 0 {
            said.push(format!("{} could not be probed from here", self.unprobed));
        }
        match said.is_empty() {
            true => "none have ever been taken here".to_string(),
            false => said.join(", "),
        }
    }
}

/// Counts the lock files of each side, and the marks beside them.
///
/// **Takes nothing away.** A lane that probes free here is one nobody is
/// in, and a landing's mark nobody holds is a landing's that is gone —
/// clearing either is the waiting side's business (`crate::lanes`), and
/// doing it from a report would let a dead run's paperwork move a live
/// one's queue.
fn held_in(lanes: &Path) -> Result<Counted, String> {
    let entries = match std::fs::read_dir(lanes) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Counted::default());
        }
        Err(error) => return Err(format!("{}: {error}", lanes.display())),
    };
    let mut counted = Counted::default();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(stem) = name.strip_suffix(".lock") else {
            continue;
        };
        let Some((side, rest)) = stem.split_once('-') else {
            continue;
        };
        let Some(held) = is_held(&entry.path()) else {
            counted.unprobed += 1;
            continue;
        };
        if rest.starts_with("landing-") {
            counted.landings += usize::from(held);
            continue;
        }
        let counts = counted.sides.entry(side.to_string()).or_default();
        counts.0 += usize::from(held);
        counts.1 += 1;
    }
    Ok(counted)
}

/// Whether somebody holds the lock at `path`, and `None` where the probe
/// could not answer. **The two are not the same**: a file this cannot
/// open is not a lane nobody is in, and counting it as free would report
/// a busy machine idle.
fn is_held(path: &Path) -> Option<bool> {
    let file = File::options().read(true).write(true).open(path).ok()?;
    match file.try_lock() {
        Ok(()) => Some(false),
        Err(TryLockError::WouldBlock) => Some(true),
        Err(TryLockError::Error(_)) => None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use super::{Counted, account, at_a_ceiling, clear_any_account, held_in};

    /// A lanes directory of this test's own.
    fn lanes(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pg-wedge-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a directory to hold lanes in");
        dir
    }

    fn lock(dir: &std::path::Path, name: &str) -> std::fs::File {
        let file = std::fs::File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(dir.join(name))
            .expect("a lock file");
        file.try_lock().expect("a lock nobody else has");
        file
    }

    fn ran(timed_out: bool, code: Option<i32>) -> super::super::child::Ran {
        super::super::child::Ran {
            out_lines: Vec::new(),
            err_lines: vec!["screenshot saved=true".into()],
            status: code.map(exit_status),
            timed_out,
            elapsed: Duration::from_secs(140),
            quiet_for: Some(Duration::from_secs(138)),
            reaped: None,
        }
    }

    #[cfg(windows)]
    fn exit_status(code: i32) -> std::process::ExitStatus {
        use std::os::windows::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(code.unsigned_abs())
    }

    #[cfg(unix)]
    fn exit_status(code: i32) -> std::process::ExitStatus {
        use std::os::unix::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(code << 8)
    }

    /// The two ceilings the parent can see: its own kill, and the app
    /// ending itself at a deadline the event loop could not reach.
    #[test]
    fn both_ceilings_are_read_as_one() {
        assert!(at_a_ceiling(&ran(true, None)));
        assert!(at_a_ceiling(&ran(false, Some(97))));
        assert!(!at_a_ceiling(&ran(false, Some(0))));
        assert!(!at_a_ceiling(&ran(false, Some(1))));
    }

    /// Held and free are told apart by the lock, not by the file being
    /// there: every lane leaves its file behind for the next gate.
    #[test]
    fn the_lanes_are_counted_by_who_holds_them() {
        let dir = lanes("counted");
        let _host = lock(&dir, "host-0.lock");
        drop(lock(&dir, "host-1.lock"));
        drop(lock(&dir, "linux-0.lock"));
        let _mark = lock(&dir, "host-landing-4242.lock");

        let held = held_in(&dir).expect("a readable lanes directory");

        assert_eq!(
            held.line(),
            "host 1/2 held, linux 0/1 held, 1 landing mark(s) standing"
        );
    }

    /// A machine no gate has run on yet has no directory, and that is an
    /// answer rather than a failure to read one.
    #[test]
    fn lanes_nobody_has_taken_are_not_an_error() {
        let dir = lanes("untaken").join("never-made");
        assert_eq!(held_in(&dir), Ok(Counted::default()));
        assert_eq!(Counted::default().line(), "none have ever been taken here");
    }

    /// A container's runner may have no road to the lanes — a seat's
    /// `.git` is a file naming a directory outside the mount — so its lock
    /// files cannot be opened at all. **Silence there would read as an
    /// idle machine** — the one answer this line must never give wrongly.
    #[test]
    fn lanes_that_could_not_be_probed_are_said_rather_than_called_free() {
        let counted = Counted {
            unprobed: 8,
            ..Counted::default()
        };
        assert_eq!(counted.line(), "8 could not be probed from here");
        assert!(!counted.line().contains("0/8"), "{}", counted.line());
    }

    /// The account is what the next occurrence is read from, so a run
    /// that left no report of its own still says which of the two
    /// ceilings ended it, how long it had been quiet, and what reached
    /// the disk.
    #[test]
    fn a_run_that_left_no_report_still_accounts_for_itself() {
        let dir = lanes("no-report");
        let shots = vec![dir.join("app.png"), dir.join("overlay.png")];

        let said = account(&dir, &ran(true, None), &shots).join("\n");

        assert!(said.contains("reaped at the ceiling"), "{said}");
        assert!(said.contains("left no wedge.txt"), "{said}");
        assert!(said.contains("silent for the last 138.0s"), "{said}");
        assert!(said.contains("app.png overlay.png"), "{said}");
    }

    /// What the app wrote down about itself is the first thing to read:
    /// it is the only half that knows which side of the event loop the
    /// process was on.
    #[test]
    fn the_apps_own_account_is_carried_through() {
        let dir = lanes("own-account");
        std::fs::write(
            dir.join("wedge.txt"),
            "wedged in `joining the writes in flight` for 10.0s\n",
        )
        .expect("a report to read back");

        let said = account(&dir, &ran(false, Some(97)), &[]).join("\n");

        assert!(said.contains("the app ended itself"), "{said}");
        assert!(said.contains("joining the writes in flight"), "{said}");
        assert!(said.contains("pictures on disk: none"), "{said}");
    }

    /// A named `--shot-dir` outlives the run that made it, so an account
    /// left in one belongs to whoever had it last until this run starts.
    /// Reading a previous run's pid and station as this run's is worse
    /// than having no account at all.
    #[test]
    fn an_account_left_by_the_last_run_is_gone_before_this_one_starts() {
        let dir = lanes("stale");
        std::fs::write(
            dir.join("wedge.txt"),
            "wedged in `the event loop` (pid 1)\n",
        )
        .expect("a report from the run before");

        clear_any_account(&dir);

        let said = account(&dir, &ran(true, None), &[]).join("\n");
        assert!(said.contains("left no wedge.txt"), "{said}");
        assert!(!said.contains("pid 1"), "{said}");
    }

    /// Clearing what is not there is what every fresh run does.
    #[test]
    fn clearing_an_account_nobody_left_is_quiet() {
        clear_any_account(&lanes("nothing-to-clear"));
    }
}
