//! What the measured process weighed, and what the machine around it was
//! doing meanwhile — which says whether the weight is a reading at all: a
//! locked session, a dark screen and a parallel build all look like a slow
//! application.
//!
//! One resident sampler for the whole run: a PowerShell per 100ms sample
//! would perturb what is measured. Linux reads `/proc` directly.

mod host;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use host::WAKE_SECS;
pub(super) use host::{keep_awake, wait_for_quiet};
#[cfg(target_os = "linux")]
use linux::linux_sampler;
#[cfg(windows)]
pub(super) use windows::{CREATE_SUSPENDED, Lines, await_line, end};
#[cfg(windows)]
use windows::{windows_arm, windows_watch};

/// Ticks the window is given to be placed before the screen it is on
/// counts against it. One second at [`super::SAMPLE_MS`].
const SETTLED_TICKS: usize = 10;

/// Ticks in a row with nobody able to look at the screen before the run
/// is refused: a focus change or a consent prompt flashing past must not
/// cost a retake, and a lock lasts far longer.
const BLIND_TICKS: usize = 3;

/// One tick: the process, and the machine it was on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Sample {
    pub(super) working_set: u64,
    pub(super) os_peak_working_set: Option<u64>,
    pub(super) private: u64,
    /// The OS device name of the screen the window was on, empty while
    /// there is no window yet — not Qt's friendly name
    /// (rules-refs/app-ui.md「性能計測の画面対応」).
    pub(super) display: String,
    /// Whether the window in front belonged to the measured process.
    /// Evidence only: a window not in front is still composited, and the
    /// app does not always take the focus off the shell that started it.
    pub(super) foreground: bool,
    /// Whether somebody could have been looking at the screen. False while
    /// locked or on the secure desktop (UAC, Ctrl+Alt+Del, either side of
    /// a lock): the compositor stops presenting, and the scroll bench's
    /// animation freezes.
    ///
    /// Read off *who owns the foreground window* (`LockApp` / `LogonUI`
    /// while locked): `GetForegroundWindow` returns a handle while locked,
    /// and `LockApp` keeps running after the unlock, so neither alone says
    /// anything. No foreground window at all is the secure desktop.
    pub(super) interactive: bool,
    /// Whether the display was off at this tick — `Some(true)` off,
    /// `Some(false)` on or dimmed, `None` before the power broadcast
    /// answered. Read off `GUID_CONSOLE_DISPLAY_STATE`, which the display
    /// timer and the power button both drive.
    pub(super) dark: Option<bool>,
    pub(super) minimized: bool,
    pub(super) windowed: bool,
    /// Whole-machine processor time, in 100ns units, cumulative.
    /// `kernel` includes `idle`, as `GetSystemTimes` reports it.
    pub(super) kernel: u64,
    pub(super) user: u64,
    pub(super) idle: u64,
    /// The measured process's processor time, same units — with its
    /// children's, where a job object took them ([`Sample::job`]): the
    /// app's git counted as somebody else's trips the peak gate
    /// (ci/baseline/git-slots-windows-x64.md §巡回と重なった時).
    pub(super) app: u64,
    /// The process's own time alone, so the evidence can say how much of
    /// `app` was its children.
    pub(super) own: u64,
    /// Whether `app` counts the children: false where the process could
    /// not be put in the job object, and `app` is `own`.
    pub(super) job: bool,
    /// Milliseconds since the last input event of any kind, including the
    /// zero-pixel move [`Awake`](host::Awake) injects every `WAKE_SECS` —
    /// so it says not whether a person was here, only whether the wake
    /// helper still runs (a value past one interval says it does not).
    pub(super) away_ms: u64,
}

impl Sample {
    /// Capacity consumed between two ticks, in 100ns units across every
    /// core: what `busy` and the process's own share are read against.
    fn capacity_since(&self, previous: &Sample) -> u64 {
        (self.kernel.saturating_sub(previous.kernel)) + (self.user.saturating_sub(previous.user))
    }

    fn busy_since(&self, previous: &Sample) -> u64 {
        self.capacity_since(previous)
            .saturating_sub(self.idle.saturating_sub(previous.idle))
    }
}

/// What the machine was doing while the run was measured.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Conditions {
    pub(super) samples: usize,
    /// Ticks that had a window at all, and of those the ones where it was
    /// in front and the ones where it was minimised.
    pub(super) windowed: usize,
    pub(super) foreground: usize,
    pub(super) minimized: usize,
    /// Ticks taken while somebody could have been looking at the screen.
    pub(super) interactive: usize,
    /// The unbroken stretch of ticks with nobody able to look, in
    /// progress, and the longest one — what the gate reads (`BLIND_TICKS`).
    pub(super) blind: usize,
    pub(super) longest_blind: usize,
    /// Ticks taken with the display off, and ticks taken with it on or
    /// dimmed; short of `samples` together means the broadcast had not
    /// answered. Only a D3D run is refused on them (`screen_complaint`).
    pub(super) dark: usize,
    pub(super) lit: usize,
    /// Every screen the window was seen on, in the order first seen.
    pub(super) displays: Vec<String>,
    /// Whole-machine load over the sampled span, and the share of it that
    /// was not the measured process.
    pub(super) busy_percent: f64,
    pub(super) foreign_percent: f64,
    /// The busiest single tick's foreign load — one parallel build shows
    /// up here long before it moves the average.
    pub(super) peak_foreign_percent: f64,
    /// The longest this run went with no input event at all
    /// ([`Sample::away_ms`]).
    pub(super) away_ms: u64,
    /// Whether the process's children were counted as its own
    /// ([`Sample::job`]), which decides what "not this process" means
    /// in the report.
    pub(super) children_counted: bool,
}

impl Conditions {
    fn absorb(&mut self, sample: &Sample, previous: Option<&Sample>) {
        self.samples += 1;
        self.away_ms = self.away_ms.max(sample.away_ms);
        self.children_counted |= sample.job;
        if sample.interactive {
            self.interactive += 1;
            self.blind = 0;
        } else {
            self.blind += 1;
            self.longest_blind = self.longest_blind.max(self.blind);
        }
        match sample.dark {
            Some(true) => self.dark += 1,
            Some(false) => self.lit += 1,
            None => {}
        }
        if sample.windowed {
            self.windowed += 1;
            if sample.foreground {
                self.foreground += 1;
            }
            if sample.minimized {
                self.minimized += 1;
            }
            // Not the first second: the window is mapped where the
            // platform puts it and only then moved onto the screen the run
            // asked for, and that move is not the window wandering.
            if self.windowed > SETTLED_TICKS
                && !sample.display.is_empty()
                && !self.displays.contains(&sample.display)
            {
                self.displays.push(sample.display.clone());
            }
        }
        if let Some(previous) = previous {
            let capacity = sample.capacity_since(previous);
            if capacity > 0 {
                let busy = percent(sample.busy_since(previous), capacity);
                let own = percent(sample.app.saturating_sub(previous.app), capacity);
                self.peak_foreign_percent = self.peak_foreign_percent.max(busy - own);
            }
        }
    }

    /// The averages, taken over the whole span, so each tick weighs what
    /// it lasted and an uneven cadence is read as it fell.
    fn close(&mut self, first: Option<&Sample>, last: Option<&Sample>) {
        let (Some(first), Some(last)) = (first, last) else {
            return;
        };
        let capacity = last.capacity_since(first);
        if capacity == 0 {
            return;
        }
        self.busy_percent = percent(last.busy_since(first), capacity);
        self.foreign_percent =
            self.busy_percent - percent(last.app.saturating_sub(first.app), capacity);
    }

    /// The share of the run the window spent in front, or `None` where
    /// there was never a window to ask about (`--no-open` still has one;
    /// a run that died before mapping it does not).
    pub(super) fn foreground_share(&self) -> Option<f64> {
        (self.windowed > 0).then(|| self.foreground as f64 / self.windowed as f64)
    }

    /// Whether the machine was watched at all: the processor counters are
    /// cumulative, so a rate needs two ticks.
    pub(super) fn watched(&self) -> bool {
        self.samples >= 2
    }

    /// Why this run says nothing about the machine it ran on.
    ///
    /// The run's own fault, not the machine's: a retake would produce the
    /// same nothing (`measure::Spoiled`), and `--allow-noisy`, which opens
    /// the gates on evidence, leaves it standing.
    pub(super) fn unwatched(&self) -> Option<String> {
        (!self.watched()).then(|| {
            format!(
                "the host was never sampled — {} tick(s), where a rate needs two",
                self.samples
            )
        })
    }

    /// Why this run is not a reading of the application, or nothing.
    /// `software`: the run drew with the software scene graph, so the
    /// screen's state is only evidence and no helper injected input.
    ///
    /// Not a warning: publishing a busy machine's run as a slower
    /// application is the failure this exists to stop.
    pub(super) fn complaint(
        &self,
        limits: &Limits,
        pinned: Option<&str>,
        software: bool,
    ) -> Option<String> {
        if !self.watched() {
            // An unwatched run is `Self::unwatched`'s to report.
            return None;
        }
        if limits.quiet_percent.is_infinite() {
            // `--allow-noisy`: every gate below is open, and the report
            // still prints the conditions.
            return None;
        }
        if self.minimized > 0 {
            return Some(format!(
                "the window was minimised for {} of {} sampled ticks",
                self.minimized, self.windowed
            ));
        }
        if self.longest_blind >= BLIND_TICKS {
            return Some(format!(
                "nobody could have been looking at the screen for {} of {} sampled ticks — a \
                 locked session or the secure desktop stops the compositor presenting, and the \
                 frames stop with it",
                self.samples - self.interactive,
                self.samples
            ));
        }
        if let Some(screen) = self.screen_complaint(software) {
            return Some(screen);
        }
        // Nothing injects input under --software, so there the interval
        // says how long the person has been away, not that a helper died.
        if !software && self.away_ms > WAKE_SECS * 2_000 {
            return Some(format!(
                "no input of any kind reached the machine for {:.0}s — the wake helper injects one \
                 every {WAKE_SECS}s, so it has stopped and nothing is holding the display timer \
                 off. A run whose screen went dark is not a reading of the application, and only \
                 the scroll bench would notice on its own",
                self.away_ms as f64 / 1_000.0
            ));
        }
        if self.displays.len() > 1 {
            return Some(format!(
                "the window moved between screens ({}) — one run must stay on one screen, whose \
                 refresh rate the frames are read against",
                self.displays.join(", ")
            ));
        }
        // Pinning a window asks; it does not decide: a screen unplugged,
        // asleep or renamed before the launch leaves the window wherever
        // the platform put it, while the report divides the frame rate by
        // the refresh of the screen that was *asked* for.
        let landed_elsewhere = pinned
            .zip(self.displays.first())
            .is_some_and(|(pinned, landed)| landed.as_str() != pinned);
        if landed_elsewhere {
            return Some(format!(
                "the window was pinned to {} but came up on {} — the frame rate is read as a \
                 share of the pinned screen's refresh, and two screens do not run at one rate",
                pinned.unwrap_or("?"),
                self.displays.join(", ")
            ));
        }
        if self.foreign_percent > limits.foreign_percent {
            return Some(format!(
                "{:.1}% of the machine went to something other than the measured process (allows \
                 {:.1}%)",
                self.foreign_percent, limits.foreign_percent
            ));
        }
        if self.peak_foreign_percent > limits.peak_foreign_percent {
            return Some(format!(
                "one 100ms tick gave {:.1}% of the machine to something else (allows {:.1}%)",
                self.peak_foreign_percent, limits.peak_foreign_percent
            ));
        }
        None
    }

    /// Why the screen was not in the state the renderer needs, or
    /// nothing. For D3D one dark tick is enough: every frame after it went
    /// nowhere.
    fn screen_complaint(&self, software: bool) -> Option<String> {
        if software {
            return None;
        }
        (self.dark > 0).then(|| {
            format!(
                "the display was off for {} of {} sampled ticks — nothing is composited to a \
                 display that is off, so no frame reached the window while it was; keep the \
                 screen awake, or measure with --software, whose frames need no display",
                self.dark, self.samples
            )
        })
    }
}

/// How quiet a machine has to be for its numbers to be the application's.
#[derive(Debug, Clone, Copy)]
pub(super) struct Limits {
    /// The share of the screen's own refresh the scroll bench has to
    /// deliver: a floor under whether anything was presented at all — an
    /// occluded window, a dark screen and a lock all stop the frames
    /// (`perf::frames_delivered`).
    pub(super) frame_share: f64,
    pub(super) foreign_percent: f64,
    pub(super) peak_foreign_percent: f64,
    /// How busy the whole machine may be *before* a run starts, where
    /// there is no measured process to subtract.
    pub(super) quiet_percent: f64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            frame_share: 0.5,
            // A parallel build, not a desktop in use: an editor, a browser
            // and a container runtime sitting open must not trip it, or
            // every run anybody could take is refused (the numbers:
            // rules-refs/app-ui.md「性能計測は機械の状態も測って、駄目な run は落として撮り直す」).
            foreign_percent: 35.0,
            peak_foreign_percent: 75.0,
            quiet_percent: 30.0,
        }
    }
}

impl Limits {
    /// Every gate open (`--allow-noisy`): the numbers still come out, only
    /// the refusal to publish them goes.
    pub(super) const OPEN: Limits = Limits {
        frame_share: 0.0,
        foreign_percent: f64::INFINITY,
        peak_foreign_percent: f64::INFINITY,
        quiet_percent: f64::INFINITY,
    };
}

fn percent(part: u64, whole: u64) -> f64 {
    if whole == 0 {
        return 0.0;
    }
    part as f64 * 100.0 / whole as f64
}

/// What the sampler has seen so far. Shared, so the parent can read the
/// settled value off the same series that produced the peak.
#[derive(Debug, Default)]
pub(super) struct Series {
    pub(super) peak_working_set: u64,
    pub(super) os_peak_working_set: Option<u64>,
    pub(super) peak_private: u64,
    pub(super) last: Option<Sample>,
    first: Option<Sample>,
    pub(super) conditions: Conditions,
    /// Every tick's two memory numbers, on the parent's clock, so a
    /// moment the app names can be read back off the series it was
    /// sampled in (`fonts::FontWalk::weigh`). A few thousand a run.
    pub(super) history: Vec<super::fonts::Tick>,
}

impl Series {
    fn absorb(&mut self, sample: Sample, at_us: u64) {
        self.history.push(super::fonts::Tick {
            at_us,
            working_set: sample.working_set,
            private: sample.private,
        });
        self.peak_working_set = self.peak_working_set.max(sample.working_set);
        self.os_peak_working_set = self.os_peak_working_set.max(sample.os_peak_working_set);
        self.peak_private = self.peak_private.max(sample.private);
        self.conditions.absorb(&sample, self.last.as_ref());
        if self.first.is_none() {
            self.first = Some(sample.clone());
        }
        self.last = Some(sample);
        self.conditions
            .close(self.first.as_ref(), self.last.as_ref());
    }
}

pub(super) struct Sampler {
    pub(super) series: Arc<Mutex<Series>>,
    handle: std::thread::JoinHandle<Result<(), String>>,
}

impl Sampler {
    /// The series as it stands, copied out so the sampler is never held
    /// up by a reader.
    pub(super) fn read(&self) -> Series {
        Self::snapshot(&self.series)
    }

    fn snapshot(series: &Arc<Mutex<Series>>) -> Series {
        let held = series.lock().unwrap_or_else(|e| e.into_inner());
        Series {
            peak_working_set: held.peak_working_set,
            os_peak_working_set: held.os_peak_working_set,
            peak_private: held.peak_private,
            last: held.last.clone(),
            first: held.first.clone(),
            conditions: held.conditions.clone(),
            history: held.history.clone(),
        }
    }

    pub(super) fn finish(self) -> Result<Series, String> {
        self.handle
            .join()
            .map_err(|_| "memory sampler panicked".to_string())??;
        Ok(Self::snapshot(&self.series))
    }
}

/// The sampler with its script compiled and its job object made, waiting
/// to be told which process to watch.
///
/// Armed *before* the app starts, so the C# compile stays out of the
/// timed window. The process it is then told about is started suspended
/// ([`CREATE_SUSPENDED`]) and joins the job object before its first
/// instruction, so no child of it can exist outside the job.
pub(super) struct Armed {
    csv: std::fs::File,
    /// How long the sampler keeps watching past being told its process.
    window: std::time::Duration,
    #[cfg(windows)]
    child: std::process::Child,
    #[cfg(windows)]
    lines: Lines,
}

/// Arms the sampler for a run of at most `window`. With `software` the
/// display is neither held on nor the window raised (`windows_script`).
pub(super) fn arm(
    window: std::time::Duration,
    csv: std::fs::File,
    software: bool,
) -> Result<Armed, String> {
    #[cfg(windows)]
    {
        let (child, lines) = windows_arm(window.as_secs() + 5, software)?;
        Ok(Armed {
            csv,
            window,
            child,
            lines,
        })
    }
    #[cfg(not(windows))]
    {
        let _ = software;
        Ok(Armed { csv, window })
    }
}

impl Armed {
    /// Names `pid` to the sampler and samples it until it is reaped or
    /// the window passes. Answers the sampler and the instant the process
    /// was running — on Windows the moment the script resumed it, once it
    /// was in the job object.
    pub(super) fn watch(self, pid: u32) -> Result<(Sampler, Instant), String> {
        #[cfg(windows)]
        let (child, lines) = {
            let mut child = self.child;
            {
                let mut stdin = child.stdin.take().ok_or("memory sampler stdin missing")?;
                writeln!(stdin, "{pid}")
                    .and_then(|()| stdin.flush())
                    .map_err(|e| format!("could not name the process to the sampler: {e}"))?;
            }
            match await_line(self.lines, "resumed") {
                Ok(lines) => (child, lines),
                Err(said) => {
                    end(&mut child, "the sampler");
                    return Err(format!("the sampler did not resume the process — {said}"));
                }
            }
        };
        // waits(measured): the instant the samples are written against
        // (`parent_elapsed_us`), and the run's own clock handed on to `measure` —
        // read for nothing here
        let started = Instant::now();
        let window = self.window;
        let series = Arc::new(Mutex::new(Series::default()));
        let shared = Arc::clone(&series);
        let handle = std::thread::spawn(move || {
            let mut csv = self.csv;
            writeln!(
                csv,
                "parent_elapsed_us,working_set_bytes,private_bytes,display_name,windowed,\
                 foreground,interactive,minimized,kernel_100ns,user_100ns,idle_100ns,\
                 process_100ns,own_100ns,children_counted,display_off,os_peak_working_set_bytes"
            )
            .map_err(|e| e.to_string())?;
            let mut record = |sample: Sample| -> Result<(), String> {
                let at_us = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
                writeln!(
                    csv,
                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                    at_us,
                    sample.working_set,
                    sample.private,
                    if sample.display.is_empty() {
                        "-"
                    } else {
                        &sample.display
                    },
                    u8::from(sample.windowed),
                    u8::from(sample.foreground),
                    u8::from(sample.interactive),
                    u8::from(sample.minimized),
                    sample.kernel,
                    sample.user,
                    sample.idle,
                    sample.app,
                    sample.own,
                    u8::from(sample.job),
                    match sample.dark {
                        Some(true) => "1",
                        Some(false) => "0",
                        None => "-",
                    },
                    sample
                        .os_peak_working_set
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                )
                .map_err(|e| e.to_string())?;
                shared
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .absorb(sample, at_us);
                Ok(())
            };
            #[cfg(windows)]
            {
                let _ = (pid, window);
                windows_watch(child, lines, &mut record)
            }
            #[cfg(target_os = "linux")]
            {
                linux_sampler(pid, started, window, &mut record)
            }
            #[cfg(not(any(windows, target_os = "linux")))]
            {
                let _ = (pid, window, &mut record);
                Err("memory sampling is not implemented for this OS".into())
            }
        });
        Ok((Sampler { series, handle }, started))
    }
}

/// Reads one line of the Windows sampler's `key=value` output (Linux
/// reads `/proc` into a `Sample` directly).
#[cfg(any(windows, test))]
fn parse_sample(line: &str) -> Option<Sample> {
    let field = |key: &str| super::reading::token(line, key);
    let number = |key: &str| field(key).and_then(|v| v.parse().ok());
    let display = field("display=").unwrap_or("-");
    Some(Sample {
        working_set: number("ws=")?,
        os_peak_working_set: number("peakws="),
        private: number("pv=")?,
        display: if display == "-" {
            String::new()
        } else {
            display.to_string()
        },
        away_ms: number("away=").unwrap_or(0),
        foreground: field("fg=") == Some("1"),
        interactive: field("int=") == Some("1"),
        dark: match field("dark=") {
            Some("1") => Some(true),
            Some("0") => Some(false),
            _ => None,
        },
        minimized: field("min=") == Some("1"),
        windowed: field("win=") == Some("1"),
        kernel: number("k=").unwrap_or(0),
        user: number("u=").unwrap_or(0),
        idle: number("i=").unwrap_or(0),
        app: number("app=").unwrap_or(0),
        own: number("own=").unwrap_or(0),
        job: field("job=") == Some("1"),
    })
}

#[cfg(test)]
mod tests;
