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
//! **[`TRAIL_FILE`] is the one of these that does not need the app to
//! still be able to answer.** The report is written once, at the end, by
//! a thread inside the process; the trail is written as each step begins.
//! A run killed from outside, or stopped past its own `exiting` where the
//! exit has already ended every other thread, leaves the second and not
//! the first — which is why a missing report says only that the write was
//! never reached, and never where the process stood.
//!
//! **Read at a red that may be a process that stopped, and nowhere
//! else** — the ceiling, and the one red that is not a ceiling at all
//! ([`trail`], [`lanes_line`]). Every line below costs a directory
//! listing and a probe of a handful of lock files, and a run that answers
//! never reaches any of it.

use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// What a process its own deadline thread ended exits with, and the file
/// it leaves beside the pictures. Spelled again rather than shared:
/// xtask depends on std alone (CLAUDE.md 技術スタック), so the app's
/// `harness::deadline::WEDGED` and `REPORT_FILE` are the originals. A
/// drift shows up as a run reporting a bare `exit 97` with no account
/// beside it.
const WEDGED_EXIT: i32 = 97;
const REPORT_FILE: &str = "wedge.txt";
const TRAIL_FILE: &str = "stations.txt";

/// The ledger the machine's budget stands in, beside the repository's
/// `.git` (`crate::budget`).
const LEDGER: &str = "pgg-budget";

/// Clears any account left in `shot_dir` by whoever had it last — the
/// report and the trail both.
///
/// **Before the app starts, every run.** A named `--shot-dir` is allowed
/// to outlive the run that made it (`verify::run`), so without this a
/// run reaped at the ceiling would be handed the *previous* run's
/// account — a different pid and a different station, read as its own.
/// The app empties the trail again as it comes up, which covers a run
/// this could not clear; this covers the run that never comes up at all.
pub(super) fn clear_any_account(shot_dir: &Path) {
    for file in [REPORT_FILE, TRAIL_FILE] {
        if let Err(error) = std::fs::remove_file(shot_dir.join(file))
            && error.kind() != std::io::ErrorKind::NotFound
        {
            println!(
                "  note: {} could not be cleared ({error}) — a wedge here would read as the last \
                 run's",
                shot_dir.join(file).display()
            );
        }
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
    lines.extend(trail(shot_dir));
    lines.extend(stopped_in(shot_dir));
    let own = self_account(shot_dir);
    lines.push(match &own {
        Some(said) => format!("  the app's own account: {said}"),
        // **Only that the write was never reached.** The thread logs
        // before it saves, and there are ways to stop a process that
        // never reach either: a kill from outside, and a wedge past
        // `exiting`, where the exit has already ended every other thread.
        // Where it stood is the trail's to say, above.
        None => format!(
            "  the app left no {REPORT_FILE}: it did not reach the write at the end of its own \
             deadline, which says nothing more than that — the trail above is where it stood"
        ),
    });
    // For both ceilings. A process that ended itself named the station it
    // stood in, but only a wedge's account is finished by that: a station
    // reached late is `out of time` without saying whether it was a slow
    // step or a wedge that began too late to stand the grace, and the
    // seconds of silence before it are the only side that can tell them
    // apart.
    lines.push(silence(ran, own.as_deref()));
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

/// The stations the run reached, as the one line to print under any red
/// where a process may have stopped.
///
/// **The half that does not need the process to still be answering.**
/// Every mark here was on the disk before the step it names began, so a
/// run stopped in a way that leaves no report of its own still says how
/// far it got — which is the whole of what a `TIMED OUT` used to be
/// missing.
///
/// The seconds are the app's own clock, started in `main`, and the run's
/// are the parent's, started at the spawn; the two differ by however long
/// the process took to get going. They cannot be subtracted to measure
/// how long the last station lasted.
pub(super) fn trail(shot_dir: &Path) -> Vec<String> {
    let marks = match read_trail(shot_dir) {
        Ok(marks) => marks,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return vec![format!(
                "  the run left no {TRAIL_FILE}: no stations are recorded; its progress is unknown"
            )];
        }
        Err(error) => {
            return vec![format!(
                "  could not read {TRAIL_FILE}: {error}; its progress is unknown"
            )];
        }
    };
    if marks.is_empty() {
        return vec![format!(
            "  the run's {TRAIL_FILE} is empty or has no complete records; its progress is unknown"
        )];
    }
    vec![format!(
        "  the stations it reached: {}",
        marks
            .iter()
            .map(|(at, station)| format!("{station} {at:.1}s"))
            .collect::<Vec<_>>()
            .join(" > ")
    )]
}

/// The last recorded step at a ceiling. A failed append can hide later
/// progress, so this is a location in the record, not proof of a wedge.
fn stopped_in(shot_dir: &Path) -> Option<String> {
    let (last_at, last) = read_trail(shot_dir).ok()?.pop()?;
    Some(format!(
        "  it got as far as `{last}` {last_at:.1}s into its own run; this is the last recorded \
         station, not a measurement of time spent there"
    ))
}

/// The trail as pairs of seconds and station, in the order they were
/// reached. Read errors are kept apart from an empty file.
///
/// **A line that does not parse is dropped, never guessed at.** The file
/// is appended to a line at a time by a process that can be stopped
/// between the write and the newline, so the tail of it is the one place
/// a torn record can appear.
fn read_trail(shot_dir: &Path) -> std::io::Result<Vec<(f32, String)>> {
    let text = std::fs::read_to_string(shot_dir.join(TRAIL_FILE))?;
    Ok(text
        .split_inclusive('\n')
        .filter_map(|line| {
            let line = line.strip_suffix('\n')?;
            let (at, station) = line.split_once(' ')?;
            let at: f32 = at.strip_suffix('s')?.parse().ok()?;
            (at.is_finite() && at >= 0.0 && !station.is_empty()).then(|| (at, station.to_owned()))
        })
        .collect())
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
///
/// **Counted to the account, wherever there is one to count to.** The
/// app's account goes to stderr as well as to [`REPORT_FILE`], and Qt's
/// teardown writes after it, so the seconds at the end of a run that
/// ended itself are the pause between two dying words: a tenth of a
/// second for a process that had by then said nothing for eleven.
fn silence(ran: &super::child::Ran, own: Option<&str>) -> String {
    let Some(quiet) = ran.quiet_for else {
        return "  it never said anything at all: the silence is the whole run".to_string();
    };
    if let Some((before_it, line)) = own.and_then(|said| quiet_before_the_account(ran, said)) {
        return format!(
            "  silent for the {:.1}s before it wrote that account; the line before it was {line}",
            before_it.as_secs_f32()
        );
    }
    let last = ran
        .err_lines
        .last()
        .or_else(|| ran.out_lines.last())
        .map_or_else(|| "-".to_string(), |line| format!("`{}`", clipped(line)));
    format!(
        "  silent for the last {:.1}s of it; the last line was {last}",
        quiet.as_secs_f32()
    )
}

/// The silence that ran up to the account, and what the app had said
/// last before it.
///
/// `None` where the account is not among the lines this run left — a
/// report that never reached stderr, a run whose stderr the parent could
/// not read, or a ceiling the app never reached at all — and the end of
/// the run is what the silence is counted to instead.
fn quiet_before_the_account(ran: &super::child::Ran, said: &str) -> Option<(Duration, String)> {
    // The report is one line. A longer one is joined with ` / ` by
    // [`self_account`], which no line of the app's carries, so the piece
    // before the first join is what a line can be found by.
    let written = said.split(" / ").next()?;
    let wrote = ran
        .err_lines
        .iter()
        .rposition(|line| line.contains(written))?;
    let wrote_at = *ran.err_at.get(wrote)?;
    // Whichever stream spoke last before it. The app talks on stderr —
    // tracing and Qt both — but nothing here may take the other half for
    // empty.
    let before = ran
        .err_at
        .iter()
        .zip(&ran.err_lines)
        .take(wrote)
        .chain(
            ran.out_at
                .iter()
                .zip(&ran.out_lines)
                .filter(|(at, _)| **at < wrote_at),
        )
        .max_by_key(|(at, _)| **at);
    Some(match before {
        Some((at, line)) => (wrote_at.saturating_sub(*at), format!("`{}`", clipped(line))),
        None => (wrote_at, "-".to_string()),
    })
}

/// How much of a line the account quotes. The app clips its own quote to
/// the same width (`harness::deadline`), and for the same reason: a run
/// whose last word was the census names two hundred components, and four
/// lines that answer the run are worth more than the whole of one of
/// them.
const KEEP: usize = 160;

/// Cuts a line to [`KEEP`], on a character boundary. The mark is ASCII:
/// a stream this could not spell reaches here as replacement characters
/// already ([`crate::app_out`]), and the mark must not be one of them.
fn clipped(line: &str) -> String {
    match line.char_indices().nth(KEEP) {
        Some((at, _)) => format!("{}...", &line[..at]),
        None => line.to_owned(),
    }
}

/// How full the machine was, in the budget every gate on it draws on. A
/// run that stopped answering while the machine was full reads
/// differently from one that stopped answering alone (`crate::budget`).
///
/// Read by the ceiling above and by the one red that is not a ceiling at
/// all: a run the app's own watchdog ended turned its loop the whole
/// time and simply never reached the verb's completion, and how much of
/// the machine was running beside it is the difference between a verb
/// that is wrong and a verb that was starved (`super::outcome`).
pub(super) fn lanes_line() -> String {
    let Some(ledger) = ledger_dir() else {
        return "lanes: not read (no repository here to find them beside)".to_string();
    };
    match held_in(&ledger) {
        Ok(counted) => format!("lanes: {}", counted.line()),
        Err(why) => format!("lanes: not read ({why})"),
    }
}

fn ledger_dir() -> Option<PathBuf> {
    let root = crate::tree::workspace_root();
    crate::subprocess::common_git_dir(&root.display().to_string())
        .map(|common| Path::new(&common).join(LEDGER))
}

/// What the probe made of the ledger: how much of the machine somebody
/// is holding and on how many units, how many units are queued behind
/// them, how many landings are in line, and how many tickets it could
/// not answer for at all.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Counted {
    /// The weights the running units hold, and how many they are.
    held: (u32, usize),
    waiting: usize,
    landings: usize,
    unprobed: usize,
}

impl Counted {
    fn line(&self) -> String {
        let mut said: Vec<String> = Vec::new();
        if self.held.1 > 0 {
            said.push(format!(
                "{} weight held by {} unit(s)",
                self.held.0, self.held.1
            ));
        }
        if self.waiting > 0 {
            said.push(format!("{} unit(s) waiting", self.waiting));
        }
        if self.landings > 0 {
            said.push(format!("{} landing(s) in line", self.landings));
        }
        // Said rather than folded into the counts. A container's runner may
        // have no road to the ledger at all — a seat's `.git` is a file
        // naming a directory outside the mount, so the ticket files are
        // not there to open — and a silent "nothing held" would read as
        // an idle machine, which is the one answer this line must never
        // give wrongly.
        if self.unprobed > 0 {
            said.push(format!("{} could not be probed from here", self.unprobed));
        }
        match said.is_empty() {
            true => "nothing is running on it".to_string(),
            false => said.join(", "),
        }
    }
}

/// Counts the tickets somebody is holding, and what they say.
///
/// **Takes nothing away.** A ticket that probes free here belongs to a
/// process that is gone, and clearing it is the waiting side's business
/// (`crate::budget`): doing it from a report would let a dead run's
/// paperwork move a live one's queue. It is not counted either — a
/// ticket nobody holds is holding nothing.
fn held_in(ledger: &Path) -> Result<Counted, String> {
    let entries = match std::fs::read_dir(ledger) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Counted::default());
        }
        Err(error) => return Err(format!("{}: {error}", ledger.display())),
    };
    let mut counted = Counted::default();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(stem) = name.strip_suffix(".lock").filter(|s| s.starts_with("t-")) else {
            continue;
        };
        let Some(held) = is_held(&entry.path()) else {
            counted.unprobed += 1;
            continue;
        };
        if !held {
            continue;
        }
        // The ticket beside the lock says what its holder is doing. One
        // that cannot be read is a unit on the machine all the same —
        // said as unprobed rather than dropped.
        let Some((weight, running, turn)) =
            crate::budget::probe(&ledger.join(stem)).or_else(|| {
                counted.unprobed += 1;
                None
            })
        else {
            continue;
        };
        if turn {
            counted.landings += 1;
        } else if running {
            counted.held.0 += weight;
            counted.held.1 += 1;
        } else {
            counted.waiting += 1;
        }
    }
    Ok(counted)
}

/// Whether somebody holds the lock at `path`, and `None` where the probe
/// could not answer. **The two are not the same**: a file this cannot
/// open is not a ticket nobody holds, and counting it as free would
/// report a busy machine idle.
fn is_held(path: &Path) -> Option<bool> {
    let file = File::options().read(true).write(true).open(path).ok()?;
    match file.try_lock() {
        // Let go of by unlocking, so a probe of an idle lane cannot hand
        // it to a forked child and read it back busy ([`crate::locks`]).
        Ok(()) => {
            drop(crate::locks::Locked::new(file));
            Some(false)
        }
        Err(TryLockError::WouldBlock) => Some(true),
        Err(TryLockError::Error(_)) => None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use super::{
        Counted, TRAIL_FILE, account, at_a_ceiling, clear_any_account, held_in, stopped_in, trail,
    };

    /// A lanes directory of this test's own.
    fn lanes(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pgg-wedge-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a directory to hold lanes in");
        dir
    }

    /// A lock this test holds until it drops it — through [`Locked`], so
    /// that letting go is an unlock rather than a close: a close leaves
    /// the lock standing on every open file description a neighbouring
    /// test's fork carried away, and a ticket nobody holds would then
    /// probe as held (`crate::locks`). Seen on Linux, where `flock`
    /// follows the description.
    fn lock(dir: &std::path::Path, name: &str) -> crate::locks::Locked {
        let file = std::fs::File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(dir.join(name))
            .expect("a lock file");
        file.try_lock().expect("a lock nobody else has");
        crate::locks::Locked::new(file)
    }

    fn ran(timed_out: bool, code: Option<i32>) -> super::super::child::Ran {
        super::super::child::Ran {
            out_lines: Vec::new(),
            err_lines: vec!["screenshot saved=true".into()],
            out_at: Vec::new(),
            err_at: vec![Duration::from_secs(2)],
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

    /// A ticket somebody holds is on the machine; one nobody holds
    /// belongs to a process that is gone, and is neither counted nor
    /// cleared from here.
    #[test]
    fn the_machine_is_counted_off_the_tickets_somebody_holds() {
        let dir = lanes("counted");
        let ticket = |name: &str, weight: u32, running: u8, turn: u8| {
            std::fs::write(
                dir.join(name),
                format!(
                    "seq 1\npid 7\nweight {weight}\nbudget 24\nrank normal\nturn {turn}\n\
                     running {running}\nsince 0\nseat a\nside host\nwhat a unit\n"
                ),
            )
            .expect("a ticket");
        };
        ticket("t-0", 4, 1, 0);
        let _running = lock(&dir, "t-0.lock");
        ticket("t-1", 1, 0, 0);
        let _waiting = lock(&dir, "t-1.lock");
        ticket("t-2", 0, 1, 1);
        let _landing = lock(&dir, "t-2.lock");
        // A gate that was killed: its ticket says four are running and
        // nobody holds the lock beside it.
        ticket("t-3", 4, 1, 0);
        drop(lock(&dir, "t-3.lock"));

        let held = held_in(&dir).expect("a readable ledger");

        assert_eq!(
            held.line(),
            "4 weight held by 1 unit(s), 1 unit(s) waiting, 1 landing(s) in line"
        );
        assert!(dir.join("t-3").exists(), "a report cleared the ledger");
    }

    /// A machine no gate has run on yet has no directory, and that is an
    /// answer rather than a failure to read one.
    #[test]
    fn a_ledger_nobody_has_written_is_not_an_error() {
        let dir = lanes("untaken").join("never-made");
        assert_eq!(held_in(&dir), Ok(Counted::default()));
        assert_eq!(Counted::default().line(), "nothing is running on it");
    }

    /// A container's runner may have no road to the ledger — a seat's
    /// `.git` is a file naming a directory outside the mount — so its
    /// ticket files cannot be opened at all. **Silence there would read
    /// as an idle machine** — the one answer this line must never give
    /// wrongly.
    #[test]
    fn tickets_that_could_not_be_probed_are_said_rather_than_called_free() {
        let counted = Counted {
            unprobed: 8,
            ..Counted::default()
        };
        assert_eq!(counted.line(), "8 could not be probed from here");
        assert!(
            !counted.line().contains("nothing is running"),
            "{}",
            counted.line()
        );
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

    /// A run that ended itself out of time leaves an account that does
    /// not say whether the station was a slow step or a wedge that began
    /// late, so the silence the parent watched is carried through for
    /// that ceiling too — **counted to the account and not past it**.
    /// The account is a line on stderr as much as a file, and Qt writes
    /// more on the way down, so the end of such a run is the pause
    /// between two dying words rather than the silence that says where
    /// it stood.
    #[test]
    fn the_silence_of_a_run_that_ended_itself_is_counted_to_its_account() {
        let dir = lanes("own-silence");
        let report = "out of time in `joining the writes in flight` after 3.1s: reached past the \
                      ceiling, stood less than the grace";
        std::fs::write(dir.join("wedge.txt"), format!("{report}\n"))
            .expect("a report to read back");
        let mut ran = ran(false, Some(97));
        ran.err_lines = vec![
            "INFO bench: auto_act ran=app-menu".into(),
            format!("ERROR bench: {report}"),
            "QObject::~QObject: Timers cannot be stopped from another thread".into(),
        ];
        ran.err_at = [1_000, 12_000, 12_050].map(Duration::from_millis).to_vec();
        ran.quiet_for = Some(Duration::from_millis(100));

        let said = account(&dir, &ran, &[]).join("\n");

        assert!(said.contains("silent for the 11.0s"), "{said}");
        assert!(
            said.contains("`INFO bench: auto_act ran=app-menu`"),
            "{said}"
        );
        assert!(!said.contains("0.1s"), "{said}");
    }

    /// The line is quoted, not carried. A run whose last word was the
    /// census names two hundred components, and the four lines that
    /// answer the run have to stay readable under it.
    #[test]
    fn the_line_the_account_is_counted_to_is_quoted_at_a_width() {
        let dir = lanes("own-silence-long");
        let report = "wedged in `left the event loop` for 11.0s";
        std::fs::write(dir.join("wedge.txt"), format!("{report}\n"))
            .expect("a report to read back");
        let mut ran = ran(false, Some(97));
        ran.err_lines = vec![
            format!("INFO bench: census={}", "AppMenuItem,".repeat(200)),
            format!("ERROR bench: {report}"),
        ];
        ran.err_at = [1_000, 12_000].map(Duration::from_millis).to_vec();

        let said = account(&dir, &ran, &[]).join("\n");

        let quoted = said
            .lines()
            .find(|line| line.contains("silent for the 11.0s"))
            .expect("the silence line");
        assert!(quoted.chars().count() < 260, "{quoted}");
        assert!(quoted.ends_with("...`"), "{quoted}");
    }

    /// The account reaches the file and the stream by two roads, and a
    /// run can leave one without the other — a window whose stderr
    /// nobody was holding open takes that road to `OutputDebugStringW`
    /// (`platitude_gg::logsink`). The end of the run is what the silence
    /// is counted to then, which is all there is to count to.
    #[test]
    fn an_account_that_never_reached_the_stream_is_counted_to_the_end() {
        let dir = lanes("own-account-unheard");
        std::fs::write(
            dir.join("wedge.txt"),
            "wedged in `the event loop` for 10.0s\n",
        )
        .expect("a report to read back");

        let said = account(&dir, &ran(false, Some(97)), &[]).join("\n");

        assert!(said.contains("silent for the last 138.0s"), "{said}");
    }

    /// The trail is the record that does not need the process: a run
    /// stopped where nothing inside it can report still says which step
    /// it was in, and how much of the run was spent there.
    #[test]
    fn a_run_that_left_no_report_is_still_placed_by_its_trail() {
        let dir = lanes("trail");
        std::fs::write(
            dir.join("stations.txt"),
            "0.0s starting\n0.3s event-loop\n1.9s left-event-loop\n2.1s exiting\n",
        )
        .expect("a trail to read back");

        let said = account(&dir, &ran(true, None), &[]).join("\n");

        assert!(said.contains("starting 0.0s > event-loop 0.3s"), "{said}");
        assert!(said.contains("exiting 2.1s"), "{said}");
        assert!(said.contains("it got as far as `exiting` 2.1s"), "{said}");
        assert!(!said.contains("137.9s"), "different clock origins: {said}");
        // And the missing report is read for what it is: the write was
        // never reached, which on this shape it never can be.
        assert!(said.contains("did not reach the write"), "{said}");
        assert!(
            !said.contains("never reached its own deadline"),
            "the absence of a report does not place the deadline thread: {said}"
        );
    }

    /// A trail is appended to a line at a time, so the one place a torn
    /// record can appear is its tail — dropped rather than read as a
    /// station, which is the only reading that could name the wrong step.
    #[test]
    fn a_half_written_last_line_is_dropped_rather_than_guessed_at() {
        let dir = lanes("trail-torn");
        std::fs::write(
            dir.join("stations.txt"),
            "0.0s starting\n0.3s event-loop\n0.9s exit",
        )
        .expect("a trail cut off mid-line");

        let said = trail(&dir).join("\n");

        assert!(said.contains("starting 0.0s > event-loop 0.3s"), "{said}");
        assert!(!said.contains("0.9"), "{said}");
    }

    /// Missing and empty records are different observations, neither of
    /// which establishes whether the process reached a station.
    #[test]
    fn a_trail_nobody_left_and_an_empty_one_read_differently() {
        let dir = lanes("trail-absent");
        assert!(trail(&dir).join("\n").contains("left no stations.txt"));
        std::fs::write(dir.join("stations.txt"), "").expect("an emptied trail");
        assert!(trail(&dir).join("\n").contains("is empty"));
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
        std::fs::write(dir.join("stations.txt"), "0.0s starting\n9.9s hub-down\n")
            .expect("a trail from the run before");

        clear_any_account(&dir);

        let said = account(&dir, &ran(true, None), &[]).join("\n");
        assert!(said.contains("left no wedge.txt"), "{said}");
        assert!(!said.contains("pid 1"), "{said}");
        assert!(said.contains("left no stations.txt"), "{said}");
        assert!(!said.contains("hub-down"), "{said}");
    }

    /// Clearing what is not there is what every fresh run does.
    #[test]
    fn clearing_an_account_nobody_left_is_quiet() {
        clear_any_account(&lanes("nothing-to-clear"));
    }

    #[test]
    fn missing_or_unreadable_stations_do_not_place_the_process() {
        let dir = lanes("trail-unknown");
        let absent = trail(&dir).join("\n");
        assert!(!absent.contains("did not reach"), "{absent}");
        std::fs::create_dir(dir.join(TRAIL_FILE)).expect("an unreadable trail path");
        let unreadable = trail(&dir).join("\n");
        assert!(unreadable.contains("could not read"), "{unreadable}");
    }

    #[test]
    fn startup_delay_is_not_charged_to_the_last_station() {
        let dir = lanes("trail-clock-origin");
        std::fs::write(dir.join(TRAIL_FILE), "0.0s starting\n2.0s exiting\n")
            .expect("the child's clock starts after process startup");
        let mut delayed = ran(true, None);
        delayed.elapsed = Duration::from_secs(24);
        let said = account(&dir, &delayed, &[]).join("\n");
        assert!(
            !said.contains("22.0s"),
            "startup delay was charged to exiting: {said}"
        );
        assert!(stopped_in(&dir).unwrap().contains("last recorded station"));
    }
}
