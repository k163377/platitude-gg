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
//! **The ceiling is a diagnosis and nothing else** (.claude/rules/core.md
//! §非同期・並行テスト). What the thread finds is never a pass, and what it
//! writes names the limit it met: a station that stood still for the
//! grace is a wedge; a station reached after the ceiling that had not
//! stood the grace when the last look came is out of time, and whether
//! that was a slow step or a wedge that began late is not known from
//! here — the parent's reaping was seconds away, and what to read then
//! is the station and the load. Every limit is set by the one clock
//! ([`CLOCK`]) the report is read off, so the parent's grace
//! (`xtask::verify::child`) stays behind the whole of the wait, oversleeps
//! included, by its five seconds less what the process took to start the
//! clock.
//!
//! **A run that answers pays nothing for this.** [`at`] is one load of
//! [`CLOCK`], which finds nothing in every process nobody is driving —
//! the shipped build included, whose knobs are the idle record
//! ([`super::knobs`]) — the last word is the one QML already reports
//! through ([`super::report`]), and the thread wakes past a ceiling no
//! passing run reaches.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Added to the run's own ceiling: how long one station may stand past
/// the ceiling before it is a wedge, and so where the first look is
/// taken — the earliest a station reached at the ceiling can have stood
/// it. One number on purpose: the grace and the wedge are one judgement.
/// Well inside the grace the harness allows beyond the same ceiling
/// (`xtask::verify::child`), so the process names its own death rather
/// than being reaped without a word. The QML watchdog's clock starts
/// later than this one's — at `begin()`, once the QML is loaded — so a
/// run whose QML took longer than this to load is ended here, in the
/// event loop, before its own watchdog fires; the account's `the loop
/// last turned` is what tells that loop from one that stopped.
const PAST_THE_CEILING: Duration = Duration::from_secs(10);

/// How much longer a station reached after the ceiling is given to stand
/// the grace. Shorter than the grace, so a station reached late is out
/// of time before it can be called a wedge: the parent's reaping is the
/// outermost bound, and this stays inside it with the pace to spare.
const ONE_MORE_LOOK: Duration = Duration::from_secs(5);

/// How long between looks at the station. A station is announced by
/// nothing this thread can block on — [`at`] is one relaxed store — so it
/// is looked at; a look is one load, and the pace is only how late a
/// wedge is noticed.
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
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
    /// Every station, at the index of its number: what [`Station::of`]
    /// decodes by and what the tests walk. A station added to the enum
    /// is added here too — [`Station::name`] stops compiling until the
    /// enum is walked, and this is the list beside it.
    const ALL: [Station; 9] = [
        Self::Starting,
        Self::EventLoop,
        Self::LeftEventLoop,
        Self::SettingsFlush,
        Self::TabsClosing,
        Self::WritesJoining,
        Self::RuntimeStopping,
        Self::RunDirClearing,
        Self::HubDown,
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
        }
    }

    /// Total, because the only writer is [`at`] and the only reader is the
    /// thread below: a number neither of them wrote is the start of the
    /// run, which is where a process that has passed no station is.
    fn of(raw: u8) -> Self {
        Self::ALL
            .get(usize::from(raw))
            .copied()
            .unwrap_or(Self::Starting)
    }
}

/// A station and when it was reached, kept as one number: the station in
/// the top byte, milliseconds from [`CLOCK`] under it. One store from the
/// step and one load from the look, so a look never pairs a station with
/// the time another was reached at. Zero is the start of the run, at the
/// start of the run.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Stood {
    station: Station,
    reached: Duration,
}

const STATION_SHIFT: u32 = 56;
const REACHED_MASK: u64 = (1 << STATION_SHIFT) - 1;

impl Stood {
    fn pack(self) -> u64 {
        // Clamped to the mask once, in 64 bits: a time past it would bleed
        // into the station's byte.
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

/// The station and when it was reached ([`Stood`]).
static STOOD: AtomicU64 = AtomicU64::new(0);
/// The last thing QML reported and when, which is the last moment the
/// event loop is known to have turned.
static HEARD: Mutex<Option<(String, Duration)>> = Mutex::new(None);
/// The clock every limit here is set by and every number in the report
/// is read off. Started by [`watch`], before the thread is, and by
/// nothing else: a process that was not handed a ceiling has no clock,
/// which is what tells [`at`] and [`heard`] there is nothing to record.
static CLOCK: OnceLock<Instant> = OnceLock::new();

/// Records where the process has got to. Called from the steps
/// themselves, so it must stay this cheap: one load, one store.
pub(crate) fn at(station: Station) {
    let Some(clock) = CLOCK.get() else {
        return;
    };
    let reached = clock.elapsed();
    STOOD.store(Stood { station, reached }.pack(), Ordering::Relaxed);
}

/// Takes the last word off the channel QML reports through
/// ([`super::report`]). What it is worth is the timestamp: the report was
/// made from a slot, so the loop turned then.
///
/// Behind the feature because its one caller is: a build with no harness
/// has no channel to hear anything on.
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
    // The one clock here, a ceiling's: the limits are set by it and the
    // report is read off it, so a look and its account agree. Handed to
    // the thread by value, so the thread's own bound never rests on the
    // record.
    // waits(ceiling): the clock the diagnosis below is read off — it names a wedge, never a pass
    let clock = *CLOCK.get_or_init(Instant::now);
    let shot_dir = knobs.shot_dir.clone();
    let ceiling = Duration::from_millis(ceiling);
    // Detached: nothing joins it, and a process that ends on time takes it
    // with it.
    let spawned = std::thread::Builder::new()
        .name("pg-deadline".into())
        .spawn(move || {
            let ended = hold_out(clock, ceiling);
            end_it(clock, &shot_dir, &ended);
        });
    if let Err(error) = spawned {
        tracing::warn!(%error, "no deadline thread: a wedged run would only be reaped");
    }
}

/// Which limit a look met.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Limit {
    /// One station for the whole of [`PAST_THE_CEILING`]: a wedge.
    StoodStill,
    /// The last look came while the station had stood less than the
    /// grace: it was reached after the ceiling, and whether it would have
    /// moved again is not known — a slow step under load and a wedge that
    /// began late read the same from here.
    OutOfTime,
}

/// What one look makes of the process.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Look {
    Ended(Limit),
    /// Another look, this much later — never later than the last look.
    Again(Duration),
}

/// What ended the wait: the limit, and the station it was decided on.
/// The report names this station and not the record again, so it says
/// what was seen and not what a step wrote since.
struct Ended {
    limit: Limit,
    stood: Stood,
}

/// Sleeps out the ceiling and the grace past it, then looks at the
/// station until one has stood still for the grace or the last look has
/// come, and says which.
///
/// A wedge is a station that stands still: the ending steps past the
/// event loop are each bounded, so one still arriving at new ones is
/// working, not stuck. Without the looks an ending still reaching new
/// stations under load would be ended here as a wedge rather than by the
/// harness a few seconds later — the same run, ended earlier and
/// misnamed, which is a verdict this side has no business making. The
/// looks are bounded by [`ONE_MORE_LOOK`] off the same clock as the
/// sleep, so the harness's own reaping is always still behind them
/// (`xtask::verify::child`).
fn hold_out(clock: Instant, ceiling: Duration) -> Ended {
    let first_look = ceiling + PAST_THE_CEILING;
    let last_look = first_look + ONE_MORE_LOOK;
    // A ceiling, and only a diagnosis: the run's own ceiling and the grace
    // past it, which names a run that has not ended — one that has takes
    // this thread with it.
    // waits(ceiling): slept out rather than waited on, because a wedged process announces nothing —
    // and what this reaches past the ceiling is only ever a failure
    std::thread::sleep(first_look.saturating_sub(clock.elapsed()));
    loop {
        let now = clock.elapsed();
        let stood = Stood::unpack(STOOD.load(Ordering::Relaxed));
        match judge(now, now.saturating_sub(stood.reached), last_look) {
            Look::Ended(limit) => return Ended { limit, stood },
            // Paced, not judged: a station is announced by nothing this
            // thread can block on, so it is looked at again — never later
            // than the last look, and the look is what decides.
            // waits(paced): the loop ends on what a look found ([`judge`]), never on a count of these
            Look::Again(pace) => std::thread::sleep(pace),
        }
    }
}

/// The decision, apart from the clock: a station that has stood for the
/// grace is a wedge; the last look come, a station short of the grace is
/// out of time; otherwise another look, no later than the last one.
fn judge(now: Duration, standing: Duration, last_look: Duration) -> Look {
    if standing >= PAST_THE_CEILING {
        Look::Ended(Limit::StoodStill)
    } else if now >= last_look {
        Look::Ended(Limit::OutOfTime)
    } else {
        Look::Again(LOOK_AGAIN.min(last_look - now))
    }
}

/// Says where the process stood and ends it.
///
/// **The parent's reaping still stands behind this.** `exit` runs the
/// process's own teardown, and a thread wedged holding what that needs
/// can hold this too; the harness's kill guard is what covers that
/// (`xtask::verify::child`).
fn end_it(clock: Instant, shot_dir: &str, ended: &Ended) {
    let report = report(clock, shot_dir, ended);
    tracing::error!(target: "bench", "{report}");
    if !shot_dir.is_empty() {
        let path = Path::new(shot_dir).join(REPORT_FILE);
        if let Err(error) = std::fs::write(&path, format!("{report}\n")) {
            tracing::error!(%error, "the wedge report could not be written beside the pictures");
        }
    }
    std::process::exit(WEDGED);
}

/// The whole of what the process can still say about itself. One moment
/// for the whole line, read as it is written rather than at the look: the
/// loop may have reported since, and every number is against the same
/// clock.
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

/// The report's one line: the limit that was met and where, how far into
/// the run, and what the parent cannot see for itself.
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

/// What QML last reported, and how long ago — the last moment the event
/// loop is known to have turned.
///
/// **Never waits for the record.** A wedge holding the lock would take
/// the report with it, and the report is the only thing left.
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
        judge, secs,
    };

    /// The list is what the number decodes by, so every station's place
    /// in it is its own number.
    #[test]
    fn every_station_answers_to_its_own_number() {
        for station in Station::ALL {
            assert!(Station::of(station as u8) == station, "{}", station.name());
        }
    }

    /// A process that passed no station is at the start of the run, which
    /// is where it is: the report reads the number whatever it holds.
    #[test]
    fn a_number_nobody_wrote_reads_as_the_start() {
        assert!(Station::of(200) == Station::Starting);
    }

    /// One number carries both, so a look reads a station with the time
    /// it was reached at and never with another's.
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

    /// A time past the mask stays in the time's bits and never reaches
    /// the station's byte.
    #[test]
    fn a_time_past_the_mask_is_clamped_under_the_station() {
        let stood = Stood {
            station: Station::HubDown,
            reached: Duration::MAX,
        };
        assert_eq!(Stood::unpack(stood.pack()).station, Station::HubDown);
    }

    /// The first look is taken at the ceiling plus the grace, so a station
    /// reached before the ceiling has stood for the whole grace by then
    /// and is a wedge on the spot — the start of the run included, for a
    /// process that passed no station at all.
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

    /// A station reached after the ceiling is looked at again at the pace
    /// — and never later than the last look, so the parent's grace stays
    /// behind the whole of the wait however the sleeps fall.
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

    /// The last look come, a station short of the grace is out of time
    /// and said to be — not a wedge, which it may or may not have become —
    /// unless it had by then stood the grace, which is the wedge it is. An
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

    /// The account opens with the limit it met — a wedge and a station
    /// out of time read differently — then says how far into the run,
    /// and what only the process could see. Every number is against the
    /// one moment it is written at.
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
