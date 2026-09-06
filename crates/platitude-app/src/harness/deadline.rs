//! Where the process stood when it stopped answering.
//!
//! The run's own ceiling is a QML `Timer` (`auto/AutoShotDriver.qml`), so
//! it can only fire while the Qt event loop is turning — and only until
//! the loop is left. `QApp::run` returning is the end of it, and `main`
//! goes on afterwards to flush the settings, join the writes still in
//! flight and stop the runtime, none of which the timer can reach. A
//! process wedged in either place says nothing for the whole of the
//! harness's wait and is reaped as one line, `TIMED OUT`, with no way to
//! tell the two apart (internal-docs/P3-確認事項.md §check ハング調査で
//! 残った観察).
//!
//! So the ceiling here is not on the loop. A thread of its own sleeps out
//! the same deadline and, if the process is still standing, writes down
//! where it stood and ends it.
//!
//! **A run that answers pays nothing for this.** [`at`] is a relaxed load
//! that is false in every process nobody is driving — the shipped build
//! included, whose knobs are the idle record ([`super::knobs`]) — the
//! last word is the one QML already reports through ([`super::report`]),
//! and the thread wakes past a ceiling no passing run reaches.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Added to the run's own ceiling: what a process gets to finish an
/// ending that has already gone past the QML watchdog. Well inside the
/// grace the harness allows beyond the same ceiling
/// (`xtask::verify::child`), so the process names its own death rather
/// than being reaped without a word.
const PAST_THE_CEILING: Duration = Duration::from_secs(10);

/// How much longer a process that is still reaching new stations is given
/// past that, and how often it is looked at. Both inside the harness's
/// own grace, so the reaping stays the outermost bound.
const ONE_MORE_LOOK: Duration = Duration::from_secs(5);
const LOOK_AGAIN: Duration = Duration::from_millis(250);

/// What a process ended here exits with. A number of its own, so the
/// harness can say what happened rather than only that it was not zero.
pub(crate) const WEDGED: i32 = 97;

/// What the report is left in, beside the pictures. Read by the harness
/// and carried out of the container with them (`xtask::keepsakes`).
pub(crate) const REPORT_FILE: &str = "wedge.txt";

/// The places a run passes through that a wedge can be in. Coarse on
/// purpose: this is the stack the process cannot be asked for once it has
/// stopped answering, kept by hand at the few steps that block.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Station {
    /// Before the event loop: the runtime, the settings, the engine.
    Starting = 0,
    /// Inside `QApp::run`. A process still here at the deadline never
    /// turned its loop far enough to fire the QML watchdog.
    EventLoop = 1,
    /// `QApp::run` has returned and the teardown has not begun.
    LeftEventLoop = 2,
    SettingsFlush = 3,
    TabsClosing = 4,
    WritesJoining = 5,
    RuntimeStopping = 6,
    RunDirClearing = 7,
    /// The hub is down and only the exit is left.
    HubDown = 8,
}

impl Station {
    fn name(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::EventLoop => "the event loop",
            Self::LeftEventLoop => "left the event loop",
            Self::SettingsFlush => "flushing the settings",
            Self::TabsClosing => "closing the tabs",
            Self::WritesJoining => "joining the writes in flight",
            Self::RuntimeStopping => "stopping the runtime",
            Self::RunDirClearing => "clearing the run directory",
            Self::HubDown => "the hub is down",
        }
    }

    /// Total, because the only writer is [`at`] and the only reader is the
    /// thread below: a number neither of them wrote is the start of the
    /// run, which is where a process that has passed no station is.
    fn of(raw: u8) -> Self {
        match raw {
            1 => Self::EventLoop,
            2 => Self::LeftEventLoop,
            3 => Self::SettingsFlush,
            4 => Self::TabsClosing,
            5 => Self::WritesJoining,
            6 => Self::RuntimeStopping,
            7 => Self::RunDirClearing,
            8 => Self::HubDown,
            _ => Self::Starting,
        }
    }
}

/// Whether anything is keeping the record. False in every process that
/// was not handed a ceiling, which is what makes [`at`] free.
static WATCHING: AtomicBool = AtomicBool::new(false);
static STATION: AtomicU8 = AtomicU8::new(Station::Starting as u8);
/// When the station was reached, milliseconds from [`CLOCK`].
static REACHED: AtomicU64 = AtomicU64::new(0);
/// The last thing QML reported and when, which is the last moment the
/// event loop is known to have turned.
static HEARD: Mutex<Option<(String, u64)>> = Mutex::new(None);
static CLOCK: OnceLock<Instant> = OnceLock::new();

/// Records where the process has got to. Called from the steps
/// themselves, so it must stay this cheap.
pub(crate) fn at(station: Station) {
    if !WATCHING.load(Ordering::Relaxed) {
        return;
    }
    STATION.store(station as u8, Ordering::Relaxed);
    REACHED.store(elapsed_ms(), Ordering::Relaxed);
}

/// Takes the last word off the channel QML reports through
/// ([`super::report`]). What it is worth is the timestamp: the report was
/// made from a slot, so the loop turned then.
///
/// Behind the feature because its one caller is: a build with no harness
/// has no channel to hear anything on.
#[cfg(feature = "automation")]
pub(super) fn heard(message: &str) {
    if !WATCHING.load(Ordering::Relaxed) {
        return;
    }
    if let Ok(mut last) = HEARD.lock() {
        *last = Some((message.to_owned(), elapsed_ms()));
    }
}

/// Starts the thread that outlives a wedge, for a run that was handed a
/// ceiling. Every other process — the shipped build, a window somebody
/// opened — is told nothing and starts nothing.
pub(crate) fn watch() {
    let knobs = super::knobs();
    let Ok(ceiling) = u64::try_from(knobs.watchdog_ms) else {
        return;
    };
    if ceiling == 0 {
        return;
    }
    let _ = CLOCK.set(Instant::now());
    WATCHING.store(true, Ordering::Relaxed);
    let shot_dir = knobs.shot_dir.clone();
    let deadline = Duration::from_millis(ceiling) + PAST_THE_CEILING;
    // Detached: nothing joins it, and a process that ends on time takes it
    // with it.
    let spawned = std::thread::Builder::new()
        .name("pg-deadline".into())
        .spawn(move || {
            std::thread::sleep(deadline);
            wait_out_a_process_still_moving();
            end_it(&shot_dir);
        });
    if let Err(error) = spawned {
        tracing::warn!(%error, "no deadline thread: a wedged run would only be reaped");
    }
}

/// Holds off a process that is still moving from one station to the
/// next, and comes back the moment it stops.
///
/// A wedge is a station that stands still: the ending steps past the
/// event loop are each bounded, so one still arriving at new ones is
/// working, not stuck. Without this a slow ending under load would be
/// killed here rather than by the harness ten seconds later — the same
/// run, ended earlier, which is a verdict this side has no business
/// making. Bounded to [`ONE_MORE_LOOK`] so the harness's own reaping is
/// always still behind it (`xtask::verify::child`).
fn wait_out_a_process_still_moving() {
    let deadline = Instant::now() + ONE_MORE_LOOK;
    while Instant::now() < deadline {
        let stood = elapsed_ms().saturating_sub(REACHED.load(Ordering::Relaxed));
        if stood >= PAST_THE_CEILING.as_millis().try_into().unwrap_or(u64::MAX) {
            return;
        }
        std::thread::sleep(LOOK_AGAIN);
    }
}

/// Says where the process stood and ends it.
///
/// **The parent's reaping still stands behind this.** `exit` runs the
/// process's own teardown, and a thread wedged holding what that needs
/// can hold this too; the harness's kill guard is what covers that
/// (`xtask::verify::child`).
fn end_it(shot_dir: &str) {
    let report = report();
    tracing::error!(target: "bench", "{report}");
    if !shot_dir.is_empty() {
        let path = Path::new(shot_dir).join(REPORT_FILE);
        if let Err(error) = std::fs::write(&path, format!("{report}\n")) {
            tracing::error!(%error, "the wedge report could not be written beside the pictures");
        }
    }
    std::process::exit(WEDGED);
}

/// The whole of what the process can still say about itself.
fn report() -> String {
    let station = Station::of(STATION.load(Ordering::Relaxed));
    let reached = REACHED.load(Ordering::Relaxed);
    let now = elapsed_ms();
    format!(
        "wedged in `{}` for {}, {} into the run (pid {}); {}; pictures: {}",
        station.name(),
        secs(now.saturating_sub(reached)),
        secs(now),
        std::process::id(),
        last_word(now),
        pictures(),
    )
}

/// What QML last reported, and how long ago — the last moment the event
/// loop is known to have turned.
///
/// **Never waits for the record.** A wedge holding the lock would take
/// the report with it, and the report is the only thing left.
fn last_word(now: u64) -> String {
    match HEARD.try_lock().map(|held| held.clone()) {
        Ok(Some((message, when))) => format!(
            "the loop last turned {} ago, reporting `{}`",
            secs(now.saturating_sub(when)),
            clipped(&message),
        ),
        Ok(None) => "the loop never reported anything".into(),
        Err(_) => "what the loop last reported could not be read".into(),
    }
}

/// How much of a report is worth carrying. The census names every
/// component a run showed — four kilobytes of one line — and what is
/// wanted here is which report it was, not what it said.
const KEEP: usize = 160;

/// Cuts a report to [`KEEP`], on a character boundary. The mark is ASCII:
/// a Windows Qt writes its log lines in the local code page, so a report
/// line with anything else in it arrives as bytes nobody can read
/// (verify-ui skill).
fn clipped(message: &str) -> String {
    match message.char_indices().nth(KEEP) {
        Some((at, _)) => format!("{}...", &message[..at]),
        None => message.to_owned(),
    }
}

/// The pictures on disk when the process stopped, named rather than
/// counted: a run wedged between the two grabs has one of a pair.
fn pictures() -> String {
    let knobs = super::knobs();
    if knobs.shot_dir.is_empty() {
        return "-".into();
    }
    let mut names: Vec<String> = std::fs::read_dir(&knobs.shot_dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .filter(|name| name.ends_with(".png"))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    if names.is_empty() {
        "none".into()
    } else {
        names.join(" ")
    }
}

fn elapsed_ms() -> u64 {
    let millis = CLOCK.get().map_or(0, |clock| clock.elapsed().as_millis());
    u64::try_from(millis).unwrap_or(u64::MAX)
}

fn secs(millis: u64) -> String {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a duration this is worth printing is seconds, not centuries"
    )]
    let seconds = millis as f64 / 1000.0;
    format!("{seconds:.1}s")
}

#[cfg(test)]
mod tests {
    use super::{Station, secs};

    #[test]
    fn every_station_answers_to_its_own_number() {
        for station in [
            Station::Starting,
            Station::EventLoop,
            Station::LeftEventLoop,
            Station::SettingsFlush,
            Station::TabsClosing,
            Station::WritesJoining,
            Station::RuntimeStopping,
            Station::RunDirClearing,
            Station::HubDown,
        ] {
            assert!(Station::of(station as u8) == station, "{}", station.name());
        }
    }

    /// A process that passed no station is at the start of the run, which
    /// is where it is: the report reads the number whatever it holds.
    #[test]
    fn a_number_nobody_wrote_reads_as_the_start() {
        assert!(Station::of(200) == Station::Starting);
    }

    #[test]
    fn a_report_says_seconds_to_one_place() {
        assert_eq!(secs(0), "0.0s");
        assert_eq!(secs(140_250), "140.2s");
    }

    /// The census is one line naming every component a run showed. What
    /// the report wants of it is which report it was.
    #[test]
    fn a_report_too_long_to_carry_is_cut() {
        let census = format!("census={}", "AppCard,".repeat(500));
        let cut = super::clipped(&census);
        assert!(cut.starts_with("census=AppCard,"), "{cut}");
        assert_eq!(cut.chars().count(), super::KEEP + 3);
    }

    /// Cut on a character, never in the middle of one: a report naming a
    /// branch or a path carries whatever the app spelled it with.
    #[test]
    fn a_report_is_cut_between_characters() {
        let said = "報告".repeat(200);
        let cut = super::clipped(&said);
        assert!(cut.ends_with("..."), "{cut}");
        assert!(said.starts_with(cut.trim_end_matches('.')), "{cut}");
    }

    #[test]
    fn a_report_that_fits_is_carried_whole() {
        assert_eq!(
            super::clipped("auto_act complete=ref-list"),
            "auto_act complete=ref-list"
        );
    }
}
