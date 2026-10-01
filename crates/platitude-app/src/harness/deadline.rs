//! Where the process stood when it stopped answering.
//!
//! The run's own ceiling is a QML `Timer` (`auto/AutoShotDriver.qml`): it
//! fires only while the event loop turns, never in the teardown `main`
//! runs after the event loop returns, and a wedge in either is otherwise
//! reaped as a bare `TIMED OUT` (internal-docs/ハング調査.md). So a thread
//! of its own sleeps out the same deadline and, if the process is still
//! standing, writes down where it stood and ends it.
//!
//! The stations also go to disk as they are reached ([`TRAIL_FILE`]): a
//! kill from outside, and on Windows anything past [`Station::Exiting`],
//! stop the run before that thread can report.
//!
//! The ceiling is a diagnosis only (.claude/rules/core.md
//! §非同期・並行テスト). Every limit is read off the one [`CLOCK`], so the
//! parent's grace (`xtask::verify::child`) stays behind the whole wait,
//! oversleeps included.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Added to the run's own ceiling: how long one station may stand before
/// it is a wedge, and so where the first look is taken ([`first_look`]).
/// One number on purpose: the grace and the wedge are one judgement.
/// Kept inside the parent's grace past the same ceiling
/// (`xtask::verify::child`), so the process names its own death. The QML
/// watchdog's clock starts later (at `begin()`), so a run whose QML loads
/// slower than this is ended here, in the event loop; the account's
/// `the loop last turned` tells that loop from one that stopped.
const PAST_THE_CEILING: Duration = Duration::from_secs(10);

/// How much longer a station reached after the ceiling is given to stand
/// the grace. With [`PAST_THE_CEILING`] it must stay inside the parent's
/// grace, with the pace to spare.
const ONE_MORE_LOOK: Duration = Duration::from_secs(5);

/// How long between looks at the station ([`at`] is one relaxed store,
/// nothing to block on); only how late a wedge is noticed.
const LOOK_AGAIN: Duration = Duration::from_millis(250);

/// What a process ended here exits with.
pub(crate) const WEDGED: i32 = 97;

/// What the report is left in, beside the pictures (`xtask::keepsakes`
/// carries it out of the container).
pub(crate) const REPORT_FILE: &str = "wedge.txt";

/// The trail, beside the pictures: one `<seconds> <slug>` line per
/// station, appended as it is reached and emptied by [`watch`].
///
/// On disk before each step begins, so the parent can read it whatever
/// ended the run — [`REPORT_FILE`] needs a process still able to write.
/// Read at a ceiling only (`xtask::verify::wedge`).
const TRAIL_FILE: &str = "stations.txt";

/// The places a run passes through that a wedge can be in: a coarse,
/// hand-kept stack of the few steps that block.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub(crate) enum Station {
    /// Before the event loop: the runtime, the settings, the engine.
    Starting = 0,
    /// Loading the window's QML, then inside the event loop (`exec` in
    /// `main`). A process still here at the deadline never
    /// turned its loop far enough to fire the QML watchdog.
    EventLoop = 1,
    /// The event loop has returned and the teardown has not begun.
    LeftEventLoop = 2,
    SettingsFlush = 3,
    TabsClosing = 4,
    WritesJoining = 5,
    RuntimeStopping = 6,
    RunDirClearing = 7,
    /// The hub is down; Qt is still standing.
    HubDown = 8,
    /// Dropping the QML engine and the Qt application while Qt's own
    /// threads still run, so the exit finds nothing of Qt's left to take
    /// down (`main`).
    QtTearingDown = 9,
    /// Inside `std::process::exit`: on Windows `ExitProcess` ends every
    /// other thread — the deadline thread among them — before the loaded
    /// libraries' detach, so a hang there is named only by the trail.
    Exiting = 10,
}

impl Station {
    /// Every station, at the index of its number ([`Station::of`] decodes
    /// by it). A new station is added here by hand — only the matches
    /// below are checked by the compiler.
    const ALL: [Station; 11] = [
        Self::Starting,
        Self::EventLoop,
        Self::LeftEventLoop,
        Self::SettingsFlush,
        Self::TabsClosing,
        Self::WritesJoining,
        Self::RuntimeStopping,
        Self::RunDirClearing,
        Self::HubDown,
        Self::QtTearingDown,
        Self::Exiting,
    ];

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
            Self::QtTearingDown => "tearing Qt down",
            Self::Exiting => "exiting",
        }
    }

    /// The word a station is named by outside this file: in the trail and
    /// in `--fault-hang` (`xtask::verify::faults`), one vocabulary so a
    /// check reads back the name it asked for. [`Station::name`] is the
    /// report's prose.
    pub(super) fn slug(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::EventLoop => "event-loop",
            Self::LeftEventLoop => "left-event-loop",
            Self::SettingsFlush => "settings-flush",
            Self::TabsClosing => "tabs-closing",
            Self::WritesJoining => "writes-joining",
            Self::RuntimeStopping => "runtime-stopping",
            Self::RunDirClearing => "run-dir-clearing",
            Self::HubDown => "hub-down",
            Self::QtTearingDown => "qt-tearing-down",
            Self::Exiting => "exiting",
        }
    }

    /// Total: only [`at`] writes the number, so one it never wrote is the
    /// start of the run.
    fn of(raw: u8) -> Self {
        Self::ALL
            .get(usize::from(raw))
            .copied()
            .unwrap_or(Self::Starting)
    }
}

/// A station and when it was reached, kept as one number (the station in
/// the top byte, milliseconds from [`CLOCK`] under it) so a look never
/// pairs a station with another's time. Zero is the start of the run.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Stood {
    station: Station,
    reached: Duration,
}

const STATION_SHIFT: u32 = 56;
const REACHED_MASK: u64 = (1 << STATION_SHIFT) - 1;

impl Stood {
    fn pack(self) -> u64 {
        // Clamped: a time past the mask would bleed into the station's byte.
        let millis = self
            .reached
            .as_secs()
            .saturating_mul(1000)
            .saturating_add(u64::from(self.reached.subsec_millis()))
            .min(REACHED_MASK);
        (self.station as u64) << STATION_SHIFT | millis
    }

    fn unpack(raw: u64) -> Self {
        Self {
            station: Station::of(u8::try_from(raw >> STATION_SHIFT).unwrap_or(0)),
            reached: Duration::from_millis(raw & REACHED_MASK),
        }
    }
}

/// A packed [`Stood`].
static STOOD: AtomicU64 = AtomicU64::new(0);
/// The last thing QML reported and when: the last moment the event loop
/// is known to have turned.
static HEARD: Mutex<Option<(String, Duration)>> = Mutex::new(None);
/// Set only by [`watch`], for a run handed a ceiling; unset, it tells
/// [`at`] and [`heard`] there is nothing to record.
static CLOCK: OnceLock<Instant> = OnceLock::new();
/// Where the trail is kept; unset without a ceiling and a shot directory.
static TRAIL: OnceLock<PathBuf> = OnceLock::new();

/// Records where the process has got to. In a process with no ceiling
/// this is one load of [`CLOCK`].
pub(crate) fn at(station: Station) {
    let Some(clock) = CLOCK.get() else {
        return;
    };
    let reached = clock.elapsed();
    STOOD.store(Stood { station, reached }.pack(), Ordering::Relaxed);
    leave_a_mark(station, reached);
    // Held saves let go before a hold at the same station, which never
    // returns.
    super::faults::release_saves_at(station);
    hold_here(station);
}

/// Appends one station to the trail, opening and closing the file around
/// each line so a wedged process holds nothing open in the pictures'
/// directory.
fn leave_a_mark(station: Station, reached: Duration) {
    let Some(path) = TRAIL.get() else {
        return;
    };
    if let Err(error) = append(path, &format!("{} {}\n", secs(reached), station.slug())) {
        tracing::warn!(%error, station = station.slug(), "a station did not reach the trail");
    }
}

fn append(path: &Path, line: &str) -> std::io::Result<()> {
    use std::io::Write as _;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(line.as_bytes())
}

/// Holds the process at one station for good, where a run asked for it
/// (`PGG_FAULT_HANG`, `xtask::verify::faults`). After the mark: the trail
/// naming the held station is what the fault checks.
#[cfg(feature = "automation")]
fn hold_here(station: Station) {
    /// Only how often the held thread wakes; the hold ends at a ceiling
    /// outside it.
    const NAP: Duration = Duration::from_secs(1);

    if super::knobs().fault_hang != station.slug() {
        return;
    }
    tracing::error!(target: "bench", "fault: held at `{}` for good", station.slug());
    loop {
        // waits(ceiling): the fault is the wedge under test, and a ceiling is the only way out of it
        std::thread::sleep(NAP);
    }
}

#[cfg(not(feature = "automation"))]
fn hold_here(_station: Station) {}

/// Takes the last word off the channel QML reports through
/// ([`super::report`]). What it is worth is the timestamp: the report was
/// made from a slot, so the loop turned then.
#[cfg(feature = "automation")]
pub(super) fn heard(message: &str) {
    let Some(clock) = CLOCK.get() else {
        return;
    };
    if let Ok(mut last) = HEARD.lock() {
        *last = Some((message.to_owned(), clock.elapsed()));
    }
}

/// Starts the thread that outlives a wedge, for a run that was handed a
/// ceiling; every other process starts nothing.
pub(crate) fn watch() {
    let knobs = super::knobs();
    let Ok(ceiling) = u64::try_from(knobs.watchdog_ms) else {
        return;
    };
    if ceiling == 0 {
        return;
    }
    // waits(ceiling): the clock the diagnosis below is read off — it names a wedge, never a pass
    let clock = *CLOCK.get_or_init(Instant::now);
    let shot_dir = knobs.shot_dir.clone();
    let ceiling = Duration::from_millis(ceiling);
    // Emptied: a named `--shot-dir` outlives its run, and the last run's
    // stations would read as this one's.
    if !shot_dir.is_empty() {
        let path = Path::new(&shot_dir).join(TRAIL_FILE);
        match std::fs::write(&path, "") {
            Ok(()) => {
                TRAIL.get_or_init(|| path);
            }
            Err(error) => {
                tracing::warn!(%error, "no trail: a run that stops here would leave no stations");
            }
        }
    }
    if knobs.fault_no_deadline {
        // The shape of a wedge past `exiting`, made to order: no report,
        // only the trail (`xtask::verify::faults`).
        tracing::warn!(target: "bench", "fault: no deadline thread; only the trail can say where this run stood");
    } else {
        // Detached: a process that ends on time takes it with it.
        let spawned = std::thread::Builder::new()
            .name("pgg-deadline".into())
            .spawn(move || {
                let ended = hold_out(clock, ceiling);
                end_it(clock, &shot_dir, &ended);
            });
        if let Err(error) = spawned {
            tracing::warn!(%error, "no deadline thread: a wedged run would only be reaped");
        }
    }
    // Last: a run held at the first station already has its ceiling and
    // its trail.
    at(Station::Starting);
}

/// Which limit a look met.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Limit {
    /// One station for the whole of [`PAST_THE_CEILING`]: a wedge.
    StoodStill,
    /// The last look came before the station had stood the grace: a slow
    /// step under load and a wedge that began late read the same from here.
    OutOfTime,
}

/// What one look makes of the process.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Look {
    Ended(Limit),
    /// Another look, this much later — never later than the last look.
    Again(Duration),
}

/// What ended the wait, and the station it was decided on — the report
/// names this one, not whatever a step wrote since.
struct Ended {
    limit: Limit,
    stood: Stood,
}

/// Sleeps out to the first look ([`find_first_look`]), then looks until a
/// station has stood still for the grace or the last look has come.
///
/// A wedge is a station that stands still: the teardown steps are each
/// bounded, so a run still reaching new stations under load is working,
/// and ending it at the first look would misname it a wedge.
fn hold_out(clock: Instant, ceiling: Duration) -> Ended {
    let first_look = find_first_look(clock, ceiling);
    let last_look = first_look + ONE_MORE_LOOK;
    // waits(ceiling): slept out, because a wedged process announces nothing — and what this
    // reaches past the ceiling is only ever a failure
    std::thread::sleep(first_look.saturating_sub(clock.elapsed()));
    loop {
        let now = clock.elapsed();
        let stood = Stood::unpack(STOOD.load(Ordering::Relaxed));
        match judge(now, now.saturating_sub(stood.reached), last_look) {
            Look::Ended(limit) => return Ended { limit, stood },
            // waits(paced): the loop ends on what a look found ([`judge`]), and on that alone
            Look::Again(pace) => std::thread::sleep(pace),
        }
    }
}

/// Where the first look is taken: the grace past the run's own ceiling —
/// or, for a run ordered to hold at a station, the grace past the moment
/// it got there, where that comes first (rules-refs/core.md
/// 「天井の起点を因果の駅に置く」).
fn first_look(ceiling: Duration, held_since: Option<Duration>) -> Duration {
    let past_the_ceiling = ceiling + PAST_THE_CEILING;
    held_since.map_or(past_the_ceiling, |since| {
        (since + PAST_THE_CEILING).min(past_the_ceiling)
    })
}

/// A run ordered to hold at a station is looked at on the pace until it
/// stands there; any other goes straight to the ceiling's look.
fn find_first_look(clock: Instant, ceiling: Duration) -> Duration {
    let past_the_ceiling = first_look(ceiling, None);
    let Some(ordered) = ordered_hold() else {
        return past_the_ceiling;
    };
    loop {
        let stood = Stood::unpack(STOOD.load(Ordering::Relaxed));
        if stood.station == ordered {
            return first_look(ceiling, Some(stood.reached));
        }
        let now = clock.elapsed();
        if now >= past_the_ceiling {
            return past_the_ceiling;
        }
        // waits(paced): the station is announced by nothing this thread can block on, and the ceiling's own look
        // bounds the wait
        std::thread::sleep(LOOK_AGAIN.min(past_the_ceiling - now));
    }
}

/// The station a run was ordered to hold at (`PGG_FAULT_HANG`), if any.
#[cfg(feature = "automation")]
fn ordered_hold() -> Option<Station> {
    let slug = super::knobs().fault_hang.as_str();
    Station::ALL
        .into_iter()
        .find(|station| station.slug() == slug)
}

#[cfg(not(feature = "automation"))]
fn ordered_hold() -> Option<Station> {
    None
}

/// One look's decision, apart from the clock.
fn judge(now: Duration, standing: Duration, last_look: Duration) -> Look {
    if standing >= PAST_THE_CEILING {
        Look::Ended(Limit::StoodStill)
    } else if now >= last_look {
        Look::Ended(Limit::OutOfTime)
    } else {
        Look::Again(LOOK_AGAIN.min(last_look - now))
    }
}

/// Says where the process stood and ends it with no teardown
/// ([`terminate`]). The parent's reaping (`xtask::verify::child`) still
/// covers a process the kernel would not end.
fn end_it(clock: Instant, shot_dir: &str, ended: &Ended) {
    let report = report(clock, shot_dir, ended);
    tracing::error!(target: "bench", "{report}");
    if !shot_dir.is_empty() {
        let path = Path::new(shot_dir).join(REPORT_FILE);
        if let Err(error) = std::fs::write(&path, format!("{report}\n")) {
            tracing::error!(%error, "the wedge report could not be written beside the pictures");
        }
    }
    terminate(WEDGED);
}

/// Ends the process with `code` and no teardown at all.
///
/// Not `exit`: the wedged thread still holds whatever it wedged on, and a
/// detach handler (Windows `ExitProcess`) or atexit handler (unix) that
/// waits on it hangs for good, so the code never reaches the parent. The
/// report is already unbuffered on stderr and disk (`crate::logsink`,
/// `std::fs::write`), so nothing this skips is owed.
#[cfg(windows)]
#[expect(
    unsafe_code,
    reason = "TerminateProcess has no safe binding, and one signature does not earn a \
              dependency (winframe::win32 says the same)"
)]
fn terminate(code: i32) -> ! {
    // SAFETY: both calls are plain integer arguments to functions that
    // take no memory; the handle is the pseudo-handle for this process,
    // which needs no closing.
    unsafe {
        TerminateProcess(GetCurrentProcess(), code.unsigned_abs());
    }
    // Only where the kernel refused; the parent reaps a hang here.
    std::process::exit(code)
}

// SAFETY: the signatures are transcribed from the Win32 headers
// (processthreadsapi.h); the arguments are a handle and an integer.
#[cfg(windows)]
#[expect(unsafe_code, reason = "as above")]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> *mut std::ffi::c_void;
    fn TerminateProcess(process: *mut std::ffi::c_void, code: u32) -> i32;
}

/// The same off the C library: `_exit` is the exit that runs nothing.
#[cfg(unix)]
#[expect(
    unsafe_code,
    reason = "_exit has no safe binding, and one signature does not earn a dependency"
)]
fn terminate(code: i32) -> ! {
    // SAFETY: one integer argument to a function that does not return.
    unsafe { _exit(code) }
}

// SAFETY: the signature is transcribed from unistd.h; the argument is an
// integer and the call does not return.
#[cfg(unix)]
#[expect(unsafe_code, reason = "as above")]
unsafe extern "C" {
    fn _exit(code: i32) -> !;
}

/// One moment for the whole line: every number in it is against the
/// same `now`.
fn report(clock: Instant, shot_dir: &str, ended: &Ended) -> String {
    let now = clock.elapsed();
    account(
        ended,
        now,
        std::process::id(),
        &last_word(now),
        &pictures(shot_dir),
    )
}

fn account(ended: &Ended, now: Duration, pid: u32, last_word: &str, pictures: &str) -> String {
    let station = ended.stood.station.name();
    let standing = secs(now.saturating_sub(ended.stood.reached));
    let opening = match ended.limit {
        Limit::StoodStill => format!("wedged in `{station}` for {standing}"),
        Limit::OutOfTime => format!(
            "out of time in `{station}` after {standing}: reached past the ceiling, stood less \
             than the grace"
        ),
    };
    format!(
        "{opening}, {} into the run (pid {pid}); {last_word}; pictures: {pictures}",
        secs(now)
    )
}

/// What QML last reported, and how long ago. Read only if the lock is
/// free: a wedge holding it must not take the report with it.
fn last_word(now: Duration) -> String {
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

/// How much of a report is carried: enough to say which report it was
/// (the census is kilobytes on one line).
const KEEP: usize = 160;

/// Cuts a report to [`KEEP`], on a character boundary. The mark is ASCII:
/// Windows Qt logs in the local code page (verify-ui skill).
fn clipped(message: &str) -> String {
    match message.char_indices().nth(KEEP) {
        Some((at, _)) => format!("{}...", &message[..at]),
        None => message.to_owned(),
    }
}

/// The pictures on disk when the process stopped, named one by one: a
/// run wedged between the two grabs has one of a pair.
fn pictures(shot_dir: &str) -> String {
    if shot_dir.is_empty() {
        return "-".into();
    }
    let mut names: Vec<String> = std::fs::read_dir(shot_dir)
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

fn secs(duration: Duration) -> String {
    format!("{:.1}s", duration.as_secs_f64())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        Ended, LOOK_AGAIN, Limit, Look, ONE_MORE_LOOK, PAST_THE_CEILING, Station, Stood, account,
        first_look, judge, secs,
    };

    #[test]
    fn every_station_answers_to_its_own_number() {
        for station in Station::ALL {
            assert!(Station::of(station as u8) == station, "{}", station.name());
        }
    }

    #[test]
    fn a_number_nobody_wrote_reads_as_the_start() {
        assert!(Station::of(200) == Station::Starting);
    }

    /// The trail and `--fault-hang` name a station by its word, and the
    /// trail line is `<seconds> <slug>`: a shared word or a space would
    /// misplace a wedge.
    #[test]
    fn every_station_has_one_word_of_its_own() {
        let mut said: Vec<&str> = Station::ALL.iter().map(|s| s.slug()).collect();
        said.sort_unstable();
        let spelled = said.len();
        said.dedup();
        assert_eq!(said.len(), spelled, "two stations answer to one word");
        for station in Station::ALL {
            let slug = station.slug();
            assert!(!slug.is_empty() && !slug.contains(' '), "{slug}");
        }
    }

    /// A hang past the exit is named only by the trail, under this word
    /// (`xtask::verify::faults` asks for it).
    #[test]
    fn the_exit_is_a_station_of_its_own() {
        assert_eq!(Station::Exiting.slug(), "exiting");
        assert_eq!(Station::ALL.last().copied(), Some(Station::Exiting));
    }

    #[test]
    fn a_station_and_when_it_was_reached_travel_as_one_number() {
        let stood = Stood {
            station: Station::WritesJoining,
            reached: Duration::from_millis(130_250),
        };
        assert_eq!(Stood::unpack(stood.pack()), stood);
        let start = Stood {
            station: Station::Starting,
            reached: Duration::ZERO,
        };
        assert_eq!(start.pack(), 0, "what a record nobody wrote holds");
        assert_eq!(Stood::unpack(0), start);
    }

    #[test]
    fn a_time_past_the_mask_is_clamped_under_the_station() {
        let stood = Stood {
            station: Station::HubDown,
            reached: Duration::MAX,
        };
        assert_eq!(Stood::unpack(stood.pack()).station, Station::HubDown);
    }

    /// Never later than the ceiling's own look, which still bounds a hold
    /// never reached.
    #[test]
    fn a_held_run_is_first_looked_at_from_the_hold() {
        let ceiling = Duration::from_secs(60);
        assert_eq!(first_look(ceiling, None), ceiling + PAST_THE_CEILING);
        let held = Duration::from_millis(4_300);
        assert_eq!(first_look(ceiling, Some(held)), held + PAST_THE_CEILING);
        let short = Duration::from_secs(4);
        assert_eq!(
            first_look(short, Some(Duration::from_secs(20))),
            short + PAST_THE_CEILING
        );
    }

    /// The start of the run included, for a process that passed no station.
    #[test]
    fn a_station_standing_the_whole_grace_is_a_wedge_at_the_first_look() {
        let first_look = Duration::from_secs(130);
        let last_look = first_look + ONE_MORE_LOOK;
        assert_eq!(
            judge(first_look, PAST_THE_CEILING, last_look),
            Look::Ended(Limit::StoodStill)
        );
        assert_eq!(
            judge(first_look, first_look, last_look),
            Look::Ended(Limit::StoodStill)
        );
    }

    #[test]
    fn a_station_short_of_the_grace_is_looked_at_again_no_later_than_the_last_look() {
        let first_look = Duration::from_secs(130);
        let last_look = first_look + ONE_MORE_LOOK;
        let short = Duration::from_secs(1);
        assert_eq!(judge(first_look, short, last_look), Look::Again(LOOK_AGAIN));
        let near = last_look - Duration::from_millis(100);
        assert_eq!(
            judge(near, short, last_look),
            Look::Again(Duration::from_millis(100))
        );
    }

    /// Unless it has stood the grace by then, which is a wedge. An
    /// oversleep past the last look is still the last look.
    #[test]
    fn the_last_look_ends_a_station_short_of_the_grace_as_out_of_time() {
        let last_look = Duration::from_secs(135);
        let short = Duration::from_secs(3);
        assert_eq!(
            judge(last_look, short, last_look),
            Look::Ended(Limit::OutOfTime)
        );
        assert_eq!(
            judge(last_look + Duration::from_millis(40), short, last_look),
            Look::Ended(Limit::OutOfTime)
        );
        assert_eq!(
            judge(last_look, PAST_THE_CEILING, last_look),
            Look::Ended(Limit::StoodStill)
        );
    }

    fn ended(limit: Limit, reached: Duration) -> Ended {
        Ended {
            limit,
            stood: Stood {
                station: Station::WritesJoining,
                reached,
            },
        }
    }

    #[test]
    fn the_account_names_the_limit_it_met() {
        let now = Duration::from_millis(135_200);
        let wedged = account(
            &ended(Limit::StoodStill, Duration::from_millis(125_000)),
            now,
            4242,
            "the loop never reported anything",
            "app.png",
        );
        assert_eq!(
            wedged,
            "wedged in `joining the writes in flight` for 10.2s, 135.2s into the run (pid 4242); \
             the loop never reported anything; pictures: app.png"
        );
        let out_of_time = account(
            &ended(Limit::OutOfTime, Duration::from_millis(132_100)),
            now,
            4242,
            "the loop never reported anything",
            "-",
        );
        assert_eq!(
            out_of_time,
            "out of time in `joining the writes in flight` after 3.1s: reached past the ceiling, \
             stood less than the grace, 135.2s into the run (pid 4242); the loop never reported \
             anything; pictures: -"
        );
    }

    #[test]
    fn a_report_says_seconds_to_one_place() {
        assert_eq!(secs(Duration::ZERO), "0.0s");
        assert_eq!(secs(Duration::from_millis(140_250)), "140.2s");
    }

    #[test]
    fn a_report_too_long_to_carry_is_cut() {
        let census = format!("census={}", "AppCard,".repeat(500));
        let cut = super::clipped(&census);
        assert!(cut.starts_with("census=AppCard,"), "{cut}");
        assert_eq!(cut.chars().count(), super::KEEP + 3);
    }

    /// A report naming a branch or a path can carry any character.
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
