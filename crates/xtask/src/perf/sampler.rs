//! What the measured process weighed, and what the machine around it was
//! doing while it weighed that.
//!
//! Both halves come off one resident sampler, because the second half is
//! what says whether the first half is a reading at all. A window that
//! lost the foreground, a session that locked, a screen that went to
//! sleep and a machine that was building something else all answer with
//! numbers that look exactly like a slow application.
//!
//! **One process for the whole run.** Windows keeps a single PowerShell
//! (spawning one per 100ms sample would perturb the very thing being
//! measured, and cannot keep the period either); Linux reads `/proc`
//! directly. The peak, the last reading and the host conditions are all
//! kept in [`Series`], which the parent reads whenever it likes rather
//! than starting a second sampler to ask.

use std::io::Write;
#[cfg(windows)]
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[cfg(target_os = "linux")]
use std::time::Duration;

use super::SAMPLE_MS;

/// How many sampled ticks the window is given to be placed before where
/// it is starts counting against it. One second at [`super::SAMPLE_MS`].
const SETTLED_TICKS: usize = 10;

/// How many ticks in a row must find nobody able to look at the screen
/// before the run is refused. Three, so 300ms at [`super::SAMPLE_MS`]:
/// long enough that a focus change or a consent prompt flashing past
/// does not cost a retake, and orders of magnitude short of the lock
/// this is here to catch.
const BLIND_TICKS: usize = 3;

/// One tick: the process, and the machine it was on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Sample {
    pub(super) working_set: u64,
    pub(super) private: u64,
    /// The OS device name of the screen the window was on, empty while
    /// there is no window yet. Never the friendly name Qt reports — those
    /// two do not match (rules-refs/app-ui.md §性能計測の画面対応).
    pub(super) display: String,
    /// Whether the window in front belonged to the measured process.
    /// Evidence rather than a gate: a visible window that is not in front
    /// is still composited and still presents frames, and the app does
    /// not always take the focus off the shell that started it.
    pub(super) foreground: bool,
    /// Whether somebody could have been looking at the screen — false
    /// says the session is locked, and a locked session stops the
    /// compositor presenting, which stops the frames and freezes the
    /// animation the scroll bench is driven by.
    ///
    /// Read off *who owns the foreground window*, which is the lock
    /// screen while the machine is locked and something else once it is
    /// not. Two nearby answers are both wrong, and both were measured:
    /// `GetForegroundWindow` returns a handle while locked (`LockApp`
    /// holds the foreground like any window), and `LockApp` is still
    /// running long after the machine is unlocked, so its mere existence
    /// says nothing either.
    ///
    /// **No foreground window at all is the third case**, and it is the
    /// secure desktop: a UAC prompt, Ctrl+Alt+Del, the credential
    /// provider, and the moments either side of a lock. That desktop
    /// owns the display, so the frames stop there too — which is why
    /// this reads false rather than falling through to true.
    pub(super) interactive: bool,
    /// Whether the display was off at this tick — `Some(true)` off,
    /// `Some(false)` on or dimmed, `None` where the power broadcast had
    /// not answered yet. Read off `GUID_CONSOLE_DISPLAY_STATE`, which is
    /// what the display timer and the power button both drive; nothing
    /// is composited to a display that is off, so a D3D swap chain
    /// presents no frame while this is true, whatever the window itself
    /// is doing — the software scene graph's frames need no display.
    pub(super) dark: Option<bool>,
    pub(super) minimized: bool,
    pub(super) windowed: bool,
    /// Whole-machine processor time, in 100ns units, cumulative.
    /// `kernel` includes `idle`, as `GetSystemTimes` reports it.
    pub(super) kernel: u64,
    pub(super) user: u64,
    pub(super) idle: u64,
    /// The measured process's processor time, same units — with its
    /// children's, where a job object took them ([`Sample::job`]). The
    /// app answers a repository by running git, and a `git status` over
    /// a hundred thousand files is eleven cores wide for a fifth of a
    /// second: counted as somebody else's, it tripped the peak gate on
    /// every run (ci/baseline/perf-windows-x64.md §計測条件).
    pub(super) app: u64,
    /// The process's own time alone, so the evidence can say how much of
    /// `app` was its children.
    pub(super) own: u64,
    /// Whether `app` counts the children: false where the process could
    /// not be put in the job object, and `app` is `own`.
    pub(super) job: bool,
    /// Milliseconds since the last input event of any kind — **the ones
    /// this harness injects included**.
    ///
    /// So it does not say whether a person was here. [`Awake`] sends a
    /// zero-pixel mouse move every `WAKE_SECS` for the whole invocation,
    /// and that is the same counter the display timer reads, so while
    /// the wake helper is alive this cannot climb past one interval.
    /// **A value above that says the wake helper is not running** — the
    /// one thing it is still good for, and the reason it is kept.
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
///
/// Read as a whole: a run is only a reading of the application if the
/// window stayed up, stayed in front, stayed on one screen, and nothing
/// else on the machine was competing for the cores.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Conditions {
    pub(super) samples: usize,
    /// Ticks that had a window at all, and of those the ones where it was
    /// in front and the ones where it was minimised.
    pub(super) windowed: usize,
    pub(super) foreground: usize,
    pub(super) minimized: usize,
    /// Ticks taken while somebody could have been looking at the screen.
    /// Short of `samples` means the session locked mid-run, or the
    /// secure desktop came up.
    pub(super) interactive: usize,
    /// The longest unbroken stretch of those, which is what the gate
    /// reads rather than the count: a lock lasts orders of magnitude
    /// longer than the blink a focus change or a consent prompt leaves,
    /// and refusing on one tick costs a whole retake plus the wait
    /// before it. The first of the two is the stretch in progress and
    /// says nothing once the run is over.
    pub(super) blind: usize,
    pub(super) longest_blind: usize,
    /// Ticks taken with the display off, and ticks taken with it on or
    /// dimmed; short of `samples` together means the broadcast had not
    /// answered. What they mean is the renderer's: a run drawing with
    /// D3D must never see a dark tick — nothing is composited to a dark
    /// display, and its frames stop — while a run drawing with the
    /// software scene graph (`--software`) may see either, and they are
    /// evidence of the conditions rather than a gate.
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
    /// The longest this run went with no input event at all, ours
    /// included ([`Sample::away_ms`]). It reads as one thing only:
    /// above `WAKE_SECS` the wake helper stopped, and the display timer
    /// is no longer being held off — which is a gate, because a dark
    /// screen otherwise shows up only in the scroll bench's frame count
    /// and a run measured without one publishes its numbers.
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
            // Not the first second of the window's life: it is mapped
            // where the platform first puts it and only then moved onto
            // the screen the run asked for, so a healthy run is seen on
            // two screens and the move the run itself makes would
            // otherwise read as the window wandering.
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

    /// The averages, taken over the whole span rather than over the
    /// per-tick numbers: an uneven cadence must not weight a short tick
    /// like a long one.
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

    /// Whether the machine was watched at all. Two ticks is the fewest
    /// that can answer anything: the processor counters are cumulative,
    /// so a rate needs a pair.
    pub(super) fn watched(&self) -> bool {
        self.samples >= 2
    }

    /// Why this run says nothing about the machine it ran on.
    ///
    /// **Not a host condition — the run's own.** A machine nobody
    /// sampled has not been shown to have done anything wrong, so
    /// taking the run again would produce the same nothing; what went
    /// missing is the sampler, and that travels with the run
    /// (`measure::Spoiled`). It is also not covered by `--allow-noisy`,
    /// which opens the gates on evidence rather than manufacturing it.
    pub(super) fn unwatched(&self) -> Option<String> {
        (!self.watched()).then(|| {
            format!(
                "the host was never sampled — {} tick(s), where a rate needs two",
                self.samples
            )
        })
    }

    /// Why this run is not a reading of the application, or nothing.
    /// `software` says the run drew with the software scene graph, whose
    /// frames reach no display: the screen's state is then evidence
    /// rather than a condition, and no helper injected input for it.
    ///
    /// Deliberately not a warning: a run taken while the machine was
    /// doing something else is not a slower application, and publishing
    /// it as one is the whole failure this exists to stop.
    pub(super) fn complaint(
        &self,
        limits: &Limits,
        pinned: Option<&str>,
        software: bool,
    ) -> Option<String> {
        if !self.watched() {
            // Nothing to say about the machine, so nothing is said. What
            // a run nobody watched is, is [`Self::unwatched`]'s answer.
            return None;
        }
        if limits.quiet_percent.is_infinite() {
            // Every gate below is open: `--allow-noisy` publishes what a
            // busy, covered or locked machine produced, and the report
            // still prints the conditions it was taken under.
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
        // Nothing injects input under --software, so the interval since
        // the last input says how long the person has been away, not
        // whether a helper died.
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
        // **Pinning a window asks; it does not decide.** The place is
        // written into the run's own settings and Qt honours it, but a
        // screen that was unplugged, asleep or renamed between the
        // enumeration and the launch leaves the window wherever the
        // platform put it — and the report divides the frame rate by
        // the refresh of the screen that was *asked* for. On this
        // machine that is 180Hz against 100Hz, so the same application
        // reads as 98% or as 176% of its screen.
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
    /// nothing. For D3D one dark tick is enough: the display timer ran
    /// out under the run, and every frame after that went nowhere. The
    /// software scene graph needs nothing of the screen, so a run
    /// drawing with it has no complaint here whatever the display did.
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
///
/// The shipped values are what this machine measures at rest with the
/// window up; they are a gate on the host, not a performance budget
/// (CLAUDE.md §性能予算).
#[derive(Debug, Clone, Copy)]
pub(super) struct Limits {
    /// The share of the screen's own refresh the scroll bench has to
    /// deliver. Not a performance budget — a floor under "was anything
    /// being presented at all": an occluded window, a screen that went to
    /// sleep and a locked session all stop the frames, and all of them
    /// otherwise read as a slow application (`perf::frames_delivered`).
    pub(super) frame_share: f64,
    pub(super) foreign_percent: f64,
    pub(super) peak_foreign_percent: f64,
    /// How busy the whole machine may be *before* a run starts, where
    /// there is no measured process to subtract. Looser than
    /// [`Limits::foreign_percent`] on purpose: a desktop at rest is not
    /// at zero, and the gate that matters is the one over the run itself.
    pub(super) quiet_percent: f64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            frame_share: 0.5,
            // Where something is competing for the machine, not where a
            // desktop is merely in use: an editor, a browser and a
            // container runtime sitting open cost a fraction of a
            // many-threaded machine that a window drawing on one thread
            // and a GPU does not notice, and a gate under that refuses
            // every run anybody could actually take. What genuinely
            // spoils a reading is measured directly instead — the
            // session locking, the window going down or moving, the
            // frames not arriving. Either way the share is printed, so a
            // record carries the conditions it was taken under rather
            // than only that they passed (the numbers are in
            // rules-refs/app-ui.md).
            foreign_percent: 35.0,
            peak_foreign_percent: 75.0,
            quiet_percent: 30.0,
        }
    }
}

impl Limits {
    /// Every gate open, for a run somebody asked for on a machine they
    /// know is busy. The numbers still come out; what goes away is the
    /// refusal to publish them.
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

/// What the sampler has seen so far. Shared rather than returned, so the
/// parent can read the settled value off the same series that produced
/// the peak instead of starting a second sampler beside it.
#[derive(Debug, Default)]
pub(super) struct Series {
    pub(super) peak_working_set: u64,
    pub(super) peak_private: u64,
    pub(super) last: Option<Sample>,
    first: Option<Sample>,
    pub(super) conditions: Conditions,
    /// Every tick's two memory numbers, on the parent's clock, so a
    /// moment the app names can be read back off the series it was
    /// sampled in (`fonts::FontWalk::weigh`). Ten a second for the
    /// length of a run: a few thousand.
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
        let held = self.series.lock().unwrap_or_else(|e| e.into_inner());
        Series {
            peak_working_set: held.peak_working_set,
            peak_private: held.peak_private,
            last: held.last.clone(),
            first: held.first.clone(),
            conditions: held.conditions.clone(),
            history: held.history.clone(),
        }
    }

    pub(super) fn finish(self) -> Result<Series, String> {
        let series = self.read();
        self.handle
            .join()
            .map_err(|_| "memory sampler panicked".to_string())??;
        Ok(series)
    }
}

/// The sampler with its script compiled and its job object made, waiting
/// to be told which process to watch.
///
/// Armed *before* the app starts, so the compile is not in the timed
/// window: a sampler started beside the app would spend the app's first
/// half second compiling C# on the machine the startup number is being
/// taken on. The process it is then told about is started suspended
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

/// The script's output, read a line at a time. The attribution script
/// speaks the same way (`perf::attribution`).
#[cfg(windows)]
pub(super) type Lines = std::io::Lines<std::io::BufReader<std::process::ChildStdout>>;

/// The process creation flag that starts a process with its primary
/// thread suspended: a pid with nothing executed yet, which the sampler
/// resumes once the process is in its job object.
#[cfg(windows)]
pub(super) const CREATE_SUSPENDED: u32 = 0x0000_0004;

/// How long the script is given to compile and say `ready`, and later to
/// join and resume the process and say `resumed`: seconds against a
/// compile measured in hundreds of milliseconds. A script that says
/// nothing in that time is a sampler that will never sample, and a wait
/// on it with no ceiling would hold the machine's every build behind a
/// measurement that never starts.
#[cfg(windows)]
const SCRIPT_CEILING: std::time::Duration = std::time::Duration::from_secs(30);

/// Arms the sampler for a run of at most `window`. `software` is the
/// run's renderer: drawing with the software scene graph, the display
/// is neither held on nor the window raised over whatever a person has
/// in front (`windows_script`).
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
                 process_100ns,own_100ns,children_counted,display_off"
            )
            .map_err(|e| e.to_string())?;
            let mut record = |sample: Sample| -> Result<(), String> {
                let at_us = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
                writeln!(
                    csv,
                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
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
                    // The screen's power state as the tick read it: the
                    // evidence of what the display did under the run
                    // (`Conditions::dark`).
                    match sample.dark {
                        Some(true) => "1",
                        Some(false) => "0",
                        None => "-",
                    },
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
                let _ = (pid, window);
                Err("memory sampling is not implemented for this OS".into())
            }
        });
        Ok((Sampler { series, handle }, started))
    }
}

/// Reads the script's lines until `wanted`, within [`SCRIPT_CEILING`],
/// and hands the rest back. A `note:` on the way is said out loud — the
/// one thing the script says that is not a sample: the job object could
/// not take the process, so its children go uncounted and the peak gate
/// sees them as somebody else's. The read blocks, so it runs on a thread
/// of its own and the ceiling is kept here; a script that never answers
/// leaves that thread on the pipe until the script is ended.
#[cfg(windows)]
pub(super) fn await_line(mut lines: Lines, wanted: &'static str) -> Result<Lines, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        loop {
            let answer = match lines.next() {
                Some(Ok(line)) if line.trim() == wanted => Ok(lines),
                Some(Ok(line)) => {
                    if let Some(note) = line.strip_prefix("note: ") {
                        println!("  sampler: {note}");
                    }
                    continue;
                }
                // Said of "it": every caller names which script this
                // was, and the attribution script waits here too.
                Some(Err(error)) => Err(format!("its output failed: {error}")),
                None => Err(format!("it ended before it said `{wanted}`")),
            };
            // A receiver that gave up on the ceiling is gone; nothing
            // else is left to tell.
            let _ = tx.send(answer);
            return;
        }
    });
    match crate::wait::receive(
        "its output",
        &format!("`{wanted}`"),
        &rx,
        crate::wait::Budget::whole(SCRIPT_CEILING),
    ) {
        Ok(answer) => answer,
        Err(expired) => Err(expired.to_string()),
    }
}

/// Ends a script that will not be read any further, and says so when it
/// would not end.
#[cfg(windows)]
pub(super) fn end(child: &mut std::process::Child, what: &str) {
    if let Err(error) = child.kill() {
        println!("  note: could not end {what}: {error}");
    }
}

/// Reads one line of the sampler's own `key=value` output. Only the
/// Windows sampler speaks in lines — Linux reads `/proc` into a `Sample`
/// directly — so outside a test build this exists on one platform.
#[cfg(any(windows, test))]
fn parse_sample(line: &str) -> Option<Sample> {
    let field = |key: &str| super::reading::token(line, key);
    let number = |key: &str| field(key).and_then(|v| v.parse().ok());
    let display = field("display=").unwrap_or("-");
    Some(Sample {
        working_set: number("ws=")?,
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

/// The script, compiled and holding a job object, up to the `ready` it
/// prints once it is waiting for a process to watch. Anything it prints
/// before that is PowerShell complaining, and a script that ends before
/// saying it is a sampler that will never sample.
#[cfg(windows)]
fn windows_arm(seconds: u64, software: bool) -> Result<(std::process::Child, Lines), String> {
    use std::io::BufRead;
    let script = windows_script(seconds, software);
    let mut child = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().ok_or("memory sampler stdout missing")?;
    let lines = std::io::BufReader::new(stdout).lines();
    match await_line(lines, "ready") {
        Ok(lines) => Ok((child, lines)),
        Err(said) => {
            end(&mut child, "the sampler");
            Err(format!("the memory sampler did not arm — {said}"))
        }
    }
}

/// Reads the armed script's samples until it stops — the process reaped,
/// or the script's own deadline passed.
#[cfg(windows)]
fn windows_watch(
    mut child: std::process::Child,
    lines: Lines,
    record: &mut dyn FnMut(Sample) -> Result<(), String>,
) -> Result<(), String> {
    let mut error = None;
    for line in lines {
        let line = match line {
            Ok(line) => line,
            Err(e) => {
                error = Some(e.to_string());
                break;
            }
        };
        if let Some(note) = line.strip_prefix("note: ") {
            println!("  sampler: {note}");
            continue;
        }
        let Some(sample) = parse_sample(&line) else {
            continue;
        };
        if let Err(e) = record(sample) {
            error = Some(e);
            break;
        }
    }
    if error.is_some() {
        child.kill().map_err(|e| e.to_string())?;
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if let Some(error) = error {
        return Err(error);
    }
    if !status.success() {
        return Err("memory sampler failed".into());
    }
    Ok(())
}

/// `SetThreadExecutionState` flags: `ES_CONTINUOUS | ES_SYSTEM_REQUIRED |
/// ES_DISPLAY_REQUIRED` to hold the screen on, `ES_CONTINUOUS |
/// ES_SYSTEM_REQUIRED` to hold only the machine awake while the screen
/// is left to whoever is driving it (`--software`), and `ES_CONTINUOUS`
/// alone to let go of both again.
///
/// Spelled in decimal because PowerShell reads a hexadecimal literal with
/// the top bit set as a negative `Int32` and then refuses to hand it to a
/// `uint` parameter.
#[cfg(windows)]
const AWAKE: u32 = 0x8000_0003;
#[cfg(windows)]
const SYSTEM_AWAKE: u32 = 0x8000_0001;
#[cfg(windows)]
const CONTINUOUS: u32 = 0x8000_0000;

/// The Win32 the sampler script calls: who is in front, the input idle
/// counter, the execution state, the whole-machine times, and the job
/// object the measured process is put in and resumed under
/// (`windows_script` says what each is for).
#[cfg(windows)]
const HOST_CLASS: &str = "public static class PerfHost {
  [DllImport(\"user32.dll\")] public static extern IntPtr GetForegroundWindow();
  [DllImport(\"user32.dll\")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport(\"user32.dll\")] [return: MarshalAs(UnmanagedType.Bool)] public static extern bool IsIconic(IntPtr h);
  [DllImport(\"user32.dll\")] [return: MarshalAs(UnmanagedType.Bool)] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [StructLayout(LayoutKind.Sequential)] public struct LASTINPUT { public uint size; public uint at; }
  [DllImport(\"user32.dll\")] [return: MarshalAs(UnmanagedType.Bool)] public static extern bool GetLastInputInfo(ref LASTINPUT info);
  [DllImport(\"kernel32.dll\")] public static extern uint GetTickCount();
  public static uint IdleMs() {
    var info = new LASTINPUT(); info.size = (uint)Marshal.SizeOf(typeof(LASTINPUT));
    if (!GetLastInputInfo(ref info)) return 0;
    return unchecked(GetTickCount() - info.at);
  }
  [DllImport(\"kernel32.dll\")] public static extern uint SetThreadExecutionState(uint flags);
  [DllImport(\"kernel32.dll\")] [return: MarshalAs(UnmanagedType.Bool)] public static extern bool GetSystemTimes(out long idle, out long kernel, out long user);
  [StructLayout(LayoutKind.Sequential)] public struct JOBACCT { public long TotalUserTime; public long TotalKernelTime; public long ThisPeriodTotalUserTime; public long ThisPeriodTotalKernelTime; public uint TotalPageFaultCount; public uint TotalProcesses; public uint ActiveProcesses; public uint TotalTerminatedProcesses; }
  [DllImport(\"kernel32.dll\", SetLastError=true)] public static extern IntPtr CreateJobObject(IntPtr attrs, string name);
  [DllImport(\"kernel32.dll\", SetLastError=true)] [return: MarshalAs(UnmanagedType.Bool)] public static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);
  [DllImport(\"kernel32.dll\")] [return: MarshalAs(UnmanagedType.Bool)] public static extern bool QueryInformationJobObject(IntPtr job, int cls, ref JOBACCT info, int size, IntPtr ret);
  public static long JobTime(IntPtr job) {
    var info = new JOBACCT();
    if (!QueryInformationJobObject(job, 1, ref info, Marshal.SizeOf(typeof(JOBACCT)), IntPtr.Zero)) return -1;
    return info.TotalUserTime + info.TotalKernelTime;
  }
  [DllImport(\"kernel32.dll\", SetLastError=true)] public static extern IntPtr OpenThread(uint access, bool inherit, uint tid);
  [DllImport(\"kernel32.dll\", SetLastError=true)] public static extern int ResumeThread(IntPtr thread);
  [DllImport(\"kernel32.dll\")] [return: MarshalAs(UnmanagedType.Bool)] public static extern bool CloseHandle(IntPtr handle);
  public static int Resume(uint tid) {
    var thread = OpenThread(2, false, tid);
    if (thread == IntPtr.Zero) return -1;
    var was = ResumeThread(thread);
    CloseHandle(thread);
    return was;
  }
}";

/// The display's power state, kept current beside whichever script asks.
/// A `NativeWindow` on a thread of its own pumps the power broadcast for
/// `GUID_CONSOLE_DISPLAY_STATE` (0 off, 1 on, 2 dimmed), which Windows
/// sends once on registration and again on every change; `Dark()` is
/// `1` / `0` / `-` for off / on-or-dimmed / not answered yet. Spliced
/// into the sampler script (`windows_script`). `Start()` waits up to a
/// second for the first answer, which in practice arrives within the
/// registration call.
#[cfg(windows)]
const DISPLAY_CLASS: &str = "public class PerfDisplay : System.Windows.Forms.NativeWindow {
  [DllImport(\"user32.dll\")] static extern IntPtr RegisterPowerSettingNotification(IntPtr h, ref Guid guid, int flags);
  [StructLayout(LayoutKind.Sequential, Pack=4)] struct PBS { public Guid PowerSetting; public uint DataLength; public byte Data; }
  static Guid ConsoleDisplayState = new Guid(\"6fe69556-704a-47a0-8f24-c28d936fda47\");
  static volatile int state = -1;
  public static string Dark() { int s = state; return s < 0 ? \"-\" : (s == 0 ? \"1\" : \"0\"); }
  protected override void WndProc(ref System.Windows.Forms.Message m) {
    if (m.Msg == 0x0218 && (int)m.WParam == 0x8013) {
      PBS s = (PBS)Marshal.PtrToStructure(m.LParam, typeof(PBS));
      if (s.PowerSetting == ConsoleDisplayState) state = s.Data;
    }
    base.WndProc(ref m);
  }
  public static void Start() {
    var pump = new System.Threading.Thread(() => {
      var w = new PerfDisplay();
      w.CreateHandle(new System.Windows.Forms.CreateParams());
      RegisterPowerSettingNotification(w.Handle, ref ConsoleDisplayState, 0);
      while (true) { System.Windows.Forms.Application.DoEvents(); System.Threading.Thread.Sleep(50); }
    });
    pump.IsBackground = true;
    pump.SetApartmentState(System.Threading.ApartmentState.STA);
    pump.Start();
    for (int i = 0; i < 40 && state < 0; i++) System.Threading.Thread.Sleep(25);
  }
}";

/// The whole of the Windows sampler, as one script held for the run.
///
/// Three things it does that a `Get-Process` loop does not, all of them
/// about the host rather than the process:
///
/// * **Keeps the screen on.** [`AWAKE`] for the length of the run,
///   released at the end. A blanked screen stops the compositor
///   presenting, and the frames the run is counting stop with it.
/// * **Says who is in front.** A window that lost the foreground, was
///   minimised, or vanished behind a locked session is not being drawn
///   at the rate the run reports.
/// * **Says what the rest of the machine did.** `GetSystemTimes` beside
///   the process's own processor time separates a slow application from a
///   busy machine.
/// * **Counts the process's children as its own.** The process arrives
///   suspended, is put in a job object, and is resumed only then; the
///   job's accounting — which keeps the time of children that have
///   already exited — is what `app=` reports. The app answers a
///   repository by running git, and git counted as somebody else's trips
///   the peak gate on every run of the corpus. A process the job will not
///   take (`note:`) is resumed all the same and counted alone.
///
/// Two phases, on one pipe each way: the script compiles, makes the job
/// and says `ready`; the pid comes down stdin; `resumed` and then the
/// samples go up stdout.
///
/// The window handle is resolved once and held: `Process.MainWindowHandle`
/// enumerates every top-level window on the desktop, and `Refresh()` (which
/// the counters need) throws the cached one away — so asking per tick
/// would cost two desktop-wide sweeps every 100ms, on the machine whose
/// business is exactly what the run is trying not to measure.
///
/// The window is raised — `SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE`, so
/// it comes to the top without taking anybody's focus — when it first
/// appears, and again on the slow cadence whenever something else has
/// the foreground. A covered window is not drawn, and a window that is
/// not drawn advances no animation, which is the scroll bench never
/// starting (`measure::SCROLL_CEILING`); raising it once only answers
/// the things that were already in the way. **`HWND_TOP` does not beat
/// a topmost window**, so a notification that sets `HWND_TOPMOST` stays
/// in front however often this fires. A software run raises it not at
/// all: its frames need no compositing, and a person at the machine
/// would otherwise have the window in their face every second.
#[cfg(windows)]
fn windows_script(seconds: u64, software: bool) -> String {
    // Drawing with the software scene graph the screen is left alone —
    // its frames need no display, and `ES_DISPLAY_REQUIRED` would hold
    // it on once a person lit it — so only the machine is kept from
    // sleeping; nor is the window raised, which a person at the machine
    // would otherwise have in their face every second.
    let awake = if software { SYSTEM_AWAKE } else { AWAKE };
    let raise = u8::from(!software);
    format!(
        "$ErrorActionPreference='Stop';\
         Add-Type -AssemblyName System.Windows.Forms;\
         Add-Type -TypeDefinition @'\n\
using System;\n\
using System.Runtime.InteropServices;\n\
{HOST_CLASS}\n\
{DISPLAY_CLASS}\n\
'@ -ReferencedAssemblies System.Windows.Forms;\
         [PerfDisplay]::Start();\
         $job=[PerfHost]::CreateJobObject([IntPtr]::Zero,$null);Write-Output 'ready';\
         $target=[int][Console]::In.ReadLine();$raise={raise};\
         [void][PerfHost]::SetThreadExecutionState([uint32]{awake});\
         try {{\
         $p=Get-Process -Id $target -ErrorAction SilentlyContinue;\
         $jobok=0;\
         if($p -ne $null){{\
           if([PerfHost]::AssignProcessToJobObject($job,$p.Handle)){{$jobok=1}}\
           else{{Write-Output \"note: the process did not join the job object (error $([Runtime.InteropServices.Marshal]::GetLastWin32Error())) - its children are not counted\"}};\
           foreach($th in $p.Threads){{ if([PerfHost]::Resume([uint32]$th.Id) -lt 0){{Write-Output \"note: thread $($th.Id) could not be resumed\"}} }};\
           Write-Output 'resumed';\
         }} else {{ Write-Output \"note: no process $target to watch - it ended before the sampler looked\" }};\
         $hwnd=[IntPtr]::Zero;$display='-';$ticks=0;$int=1;$lockpids=@();\
         $end=(Get-Date).AddSeconds({seconds});\
         while($p -ne $null -and -not $p.HasExited -and (Get-Date) -lt $end){{\
           try {{\
           $p.Refresh();\
           if($hwnd -eq [IntPtr]::Zero){{\
             $hwnd=$p.MainWindowHandle;\
             if($hwnd -ne [IntPtr]::Zero){{\
               $display=[System.Windows.Forms.Screen]::FromHandle($hwnd).DeviceName;\
               if($raise -eq 1){{[void][PerfHost]::SetWindowPos($hwnd,[IntPtr]0,0,0,0,0,0x0013)}};\
             }}\
           }} elseif($p.MainWindowHandle -eq [IntPtr]::Zero) {{\
             $hwnd=[IntPtr]::Zero;$display='-';\
           }} else {{\
             $display=[System.Windows.Forms.Screen]::FromHandle($hwnd).DeviceName;\
           }}\
           $win=0;$fg=0;$min=0;$owner=0;\
           $front=[PerfHost]::GetForegroundWindow();\
           if($front -ne [IntPtr]::Zero){{\
             [void][PerfHost]::GetWindowThreadProcessId($front,[ref]$owner);\
             if($owner -eq $target){{$fg=1}};\
           }};\
           if($ticks % 10 -eq 0){{\
             $lockpids=@((Get-Process LockApp,LogonUI -ErrorAction SilentlyContinue).Id);\
           }};\
           $ticks=$ticks+1;\
           $int=0;\
           if($front -ne [IntPtr]::Zero -and -not ($lockpids -contains $owner)){{$int=1}};\
           if($hwnd -ne [IntPtr]::Zero){{\
             $win=1;\
             if([PerfHost]::IsIconic($hwnd)){{$min=1}};\
             if($raise -eq 1 -and $fg -eq 0 -and $min -eq 0 -and $ticks % 10 -eq 0){{\
               [void][PerfHost]::SetWindowPos($hwnd,[IntPtr]0,0,0,0,0,0x0013);\
             }};\
           }};\
           $idle=0;$kernel=0;$user=0;\
           [void][PerfHost]::GetSystemTimes([ref]$idle,[ref]$kernel,[ref]$user);\
           [void][PerfHost]::SetThreadExecutionState([uint32]{awake});\
           $own=$p.TotalProcessorTime.Ticks;$app=$own;\
           if($jobok -eq 1){{$t=[PerfHost]::JobTime($job);if($t -ge 0){{$app=$t}}}};\
           Write-Output \"ws=$($p.WorkingSet64) pv=$($p.PrivateMemorySize64) display=$display win=$win fg=$fg int=$int min=$min k=$kernel u=$user i=$idle app=$app own=$own job=$jobok away=$([PerfHost]::IdleMs()) dark=$([PerfDisplay]::Dark())\";\
           }} catch {{ if($p.HasExited){{break}}; throw }};\
           Start-Sleep -Milliseconds {SAMPLE_MS};\
         }}\
         }} finally {{ [void][PerfHost]::SetThreadExecutionState([uint32]{CONTINUOUS}) }}"
    )
}

/// Waits until the machine is quiet enough to measure on, or says why it
/// gave up. Between runs, never during one: what it costs is a second
/// PowerShell, and the point is to spend it while nothing is being timed.
///
/// This is the answer to a person walking away mid-measurement: a
/// session somebody locked by hand, or a build somebody started, is
/// waited out instead of being published as a slow application. The
/// screen is held awake across this wait as across everything else —
/// [`keep_awake`] is held for the whole invocation, which is what makes
/// the gap between two runs no darker than a run.
pub(super) fn wait_for_quiet(limits: &Limits, ceiling: std::time::Duration) -> Result<(), String> {
    if limits.quiet_percent.is_infinite() {
        return Ok(());
    }
    // A look is a PowerShell of its own, and its own share of the load
    // being read: spaced further apart than the runner's usual look, so
    // the looking is not what keeps the machine busy.
    const QUIET_LOOK: std::time::Duration = std::time::Duration::from_millis(500);
    let mut wait = crate::wait::Wait::new(
        "the measurement",
        crate::wait::Budget::whole(ceiling),
        QUIET_LOOK,
    );
    let mut last: Option<Host> = None;
    let mut said = false;
    loop {
        let now = host_sample()?;
        if let Some(previous) = &last {
            let busy = now.busy_percent_since(previous);
            if now.interactive && busy <= limits.quiet_percent {
                return Ok(());
            }
            let state = if now.interactive {
                "awake"
            } else {
                "session locked"
            };
            if !said {
                said = true;
                println!("  waiting for a quiet machine ({state}, {busy:.1}% busy)…");
            }
            wait.saw(format!("{state}, {busy:.1}% busy"));
        }
        last = Some(now);
        wait.look_again("a quiet machine").map_err(|expired| {
            format!(
                "{expired} — measure it when nothing else is running, or pass --allow-noisy to \
                 publish the numbers anyway"
            )
        })?;
    }
}

/// Keeps the machine awake for as long as it is alive, and — for a run
/// whose frames need a display — the display too.
///
/// **A dark screen is an unmeasurable machine, for a D3D run.** Nothing
/// is composited to a display that is off, so no frames arrive, so the
/// animation the scroll bench is driven by never advances — the same
/// standstill a locked session produces. A measurement nobody is sitting
/// at is idle by definition, so whatever the display timer is set to, it
/// runs out.
///
/// **`ES_DISPLAY_REQUIRED` is not enough**, twice over: it holds only
/// while it is held, so a request that lives for the length of a run
/// holds nothing over the build and the waits *between* runs — and it
/// does not wake a screen that is already dark, which is what every run
/// after the first then starts against. What wakes a dark screen is
/// input, so this sends some: a mouse move of zero pixels, which moves
/// no cursor and interrupts nobody's typing.
///
/// **For a software run it is the opposite request.** Its frames need
/// no display, and a person at the machine may be turning the screen
/// on and off as they please — so that helper injects nothing and asks
/// only for `ES_SYSTEM_REQUIRED`, which keeps the machine from sleeping
/// (a build with the screen off and nobody typing is otherwise idle,
/// and the sleep timer runs out on it) while leaving the display to
/// whoever and whatever is driving it. It is held for the invocation for
/// the same reason the other is: the machine must not sleep between the
/// runs any more than during them.
///
/// **It has to die with its parent, and `Drop` is not enough.** A killed
/// xtask never unwinds — `taskkill`, a stopped task, an abort — and a
/// loop that only `Drop` stops would then hold the machine awake (and,
/// lit, inject input) for the rest of the machine's uptime, with nothing
/// able to find it (`xtask kill` reaps `platitude-gg` images, and
/// killing by image name is denied). So the loop asks whether its parent
/// is still there on every pass: the leak is bounded by one interval.
///
/// **The parent is identified by when it started, not by its number.**
/// A pid is reused, and a wake loop that only asked whether *something*
/// holds that number would outlive its parent for as long as whatever
/// took the number lives.
pub(super) struct Awake(Option<std::process::Child>);

impl Drop for Awake {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// How long the wake loop sleeps between passes, and so both how stale
/// its parent check may be and how far `Sample::away_ms` can climb
/// before [`Conditions::complaint`] reads it as the helper having died.
const WAKE_SECS: u64 = 20;

#[cfg(windows)]
pub(super) fn keep_awake(display: bool) -> Awake {
    // Holding the display pokes the input timer too, so an already-dark
    // screen comes back; a software run holds only the machine and
    // injects nothing.
    let (flags, poke) = if display {
        (
            AWAKE,
            "[PerfWake]::mouse_event(0x0001,0,0,0,[IntPtr]::Zero);",
        )
    } else {
        (SYSTEM_AWAKE, "")
    };
    let script = format!(
        "$ErrorActionPreference='Stop';\
         Add-Type -TypeDefinition @'\n\
using System;\n\
using System.Runtime.InteropServices;\n\
public static class PerfWake {{\n\
  [DllImport(\"kernel32.dll\")] public static extern uint SetThreadExecutionState(uint flags);\n\
  [DllImport(\"user32.dll\")] public static extern void mouse_event(uint flags, int dx, int dy, uint data, IntPtr extra);\n\
}}\n\
'@;\
         try {{\
         $born=(Get-Process -Id {pid} -ErrorAction SilentlyContinue).StartTime;\
         while($born -ne $null){{\
           [void][PerfWake]::SetThreadExecutionState([uint32]{flags});\
           {poke}\
           Start-Sleep -Seconds {WAKE_SECS};\
           $now=(Get-Process -Id {pid} -ErrorAction SilentlyContinue).StartTime;\
           if($now -ne $born){{break}};\
         }}\
         }} finally {{ [void][PerfWake]::SetThreadExecutionState([uint32]{CONTINUOUS}) }}",
        pid = std::process::id(),
    );
    let child = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok();
    // Said out loud, because the failure it causes names something else
    // entirely: for D3D, the screen goes dark mid-invocation and every
    // run after it dies at `measure::SCROLL_CEILING` blaming a covered
    // window; for a software run, the machine can sleep out from under
    // a long build.
    if child.is_none() {
        let consequence = if display {
            "a dark screen will spoil runs"
        } else {
            "the machine may sleep during a build"
        };
        println!("  note: could not start the awake helper — {consequence}");
    }
    Awake(child)
}

#[cfg(not(windows))]
pub(super) fn keep_awake(_display: bool) -> Awake {
    Awake(None)
}

/// The machine with no process attached: whether anybody could be looking
/// at it, and the whole-machine processor counters.
///
/// Its own type rather than an empty [`Sample`]: a `Sample` would have to
/// carry a second spelling of the sampler's field list, and every key but
/// two of those falls back to zero when it is missing — so a key renamed
/// on one side and not the other would read as a perfectly idle machine.
#[derive(Debug, Clone, Copy, Default)]
struct Host {
    /// False says nobody could be looking at this desktop. The same
    /// reading as [`Sample::interactive`], taken the same way and
    /// subject to the same caveats — do not restate them here, one copy
    /// of this drifting is what there is to avoid.
    interactive: bool,
    kernel: u64,
    user: u64,
    idle: u64,
}

impl Host {
    fn busy_percent_since(&self, previous: &Host) -> f64 {
        let capacity =
            self.kernel.saturating_sub(previous.kernel) + self.user.saturating_sub(previous.user);
        percent(
            capacity.saturating_sub(self.idle.saturating_sub(previous.idle)),
            capacity,
        )
    }
}

#[cfg(windows)]
fn host_sample() -> Result<Host, String> {
    let script = "$ErrorActionPreference='Stop';\
         Add-Type -TypeDefinition @'\n\
using System;\n\
using System.Runtime.InteropServices;\n\
public static class PerfIdle {\n\
  [DllImport(\"user32.dll\")] public static extern IntPtr GetForegroundWindow();\n\
  [DllImport(\"user32.dll\")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);\n\
  [DllImport(\"kernel32.dll\")] [return: MarshalAs(UnmanagedType.Bool)] public static extern bool GetSystemTimes(out long idle, out long kernel, out long user);\n\
}\n\
'@;\
         $idle=0;$kernel=0;$user=0;$owner=0;\
         [void][PerfIdle]::GetSystemTimes([ref]$idle,[ref]$kernel,[ref]$user);\
         $front=[PerfIdle]::GetForegroundWindow();\
         [void][PerfIdle]::GetWindowThreadProcessId($front,[ref]$owner);\
         $lockpids=@((Get-Process LockApp,LogonUI -ErrorAction SilentlyContinue).Id);\
         $int=0;\
         if($front -ne [IntPtr]::Zero -and -not ($lockpids -contains $owner)){$int=1};\
         Write-Output \"int=$int k=$kernel u=$user i=$idle\"";
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .map_err(|e| e.to_string())?;
    parse_host(String::from_utf8_lossy(&out.stdout).trim())
        .ok_or_else(|| "the host counters did not answer".to_string())
}

/// Nothing here has a desktop to lock, so the wait is only ever about the
/// processor. USER_HZ ticks rather than 100ns ones — the ratio is
/// unit-free, so only the shape has to match.
#[cfg(target_os = "linux")]
fn host_sample() -> Result<Host, String> {
    let stat = std::fs::read_to_string("/proc/stat").map_err(|e| e.to_string())?;
    let cpu = stat
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("cpu "))
        .ok_or("/proc/stat did not open with a cpu line")?;
    let values: Vec<u64> = cpu
        .split_whitespace()
        .filter_map(|v| v.parse().ok())
        .collect();
    let user = values.first().copied().unwrap_or(0) + values.get(1).copied().unwrap_or(0);
    Ok(Host {
        interactive: true,
        kernel: values.iter().sum::<u64>() - user,
        user,
        idle: values.get(3).copied().unwrap_or(0),
    })
}

/// `"int=1 k=7 u=8 i=9"` — the whole of what [`host_sample`] says.
#[cfg(any(windows, test))]
fn parse_host(line: &str) -> Option<Host> {
    let number = |key: &str| super::reading::token(line, key).and_then(|v| v.parse().ok());
    Some(Host {
        interactive: super::reading::token(line, "int=") == Some("1"),
        kernel: number("k=")?,
        user: number("u=")?,
        idle: number("i=")?,
    })
}

/// Samples `pid` at the samplers' pace for as long as the process stands,
/// and for `window` past `started` at the most. The window's end is
/// nobody's failure — `measure` ends the run at a deadline of its own
/// inside it, and this only has to outlast that — so the sampling is a
/// stand watched rather than a wait (`wait::stood`).
#[cfg(target_os = "linux")]
fn linux_sampler(
    pid: u32,
    started: Instant,
    window: Duration,
    record: &mut dyn FnMut(Sample) -> Result<(), String>,
) -> Result<(), String> {
    let status = format!("/proc/{pid}/status");
    let stretch = window.saturating_sub(started.elapsed());
    match crate::wait::stood(stretch, Duration::from_millis(SAMPLE_MS), || {
        let sample = linux_sample_once(pid);
        if sample.working_set == 0 && sample.private == 0 && !std::path::Path::new(&status).exists()
        {
            return Some(Ok(()));
        }
        record(sample).err().map(Err)
    }) {
        Err(ended) => ended,
        Ok(_stood_for) => Ok(()),
    }
}

/// Memory and whole-machine processor time. There is no window question
/// here: nothing on this side of the project measures a real window on
/// Linux (`perf::guard_the_window`, ci/linux).
#[cfg(target_os = "linux")]
fn linux_sample_once(pid: u32) -> Sample {
    let mut sample = Sample::default();
    let Ok(text) = std::fs::read_to_string(format!("/proc/{pid}/status")) else {
        return sample;
    };
    for line in text.lines() {
        let kb = |l: &str| {
            l.split_whitespace()
                .next()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0)
                * 1024
        };
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            sample.working_set = kb(rest);
        } else if let Some(rest) = line.strip_prefix("VmData:") {
            sample.private = kb(rest);
        }
    }
    // `/proc/stat`'s first line is in USER_HZ ticks; the ratios this feeds
    // are unit-free, so it is only the shape that has to match Windows.
    if let Ok(stat) = std::fs::read_to_string("/proc/stat")
        && let Some(cpu) = stat.lines().next().and_then(|l| l.strip_prefix("cpu "))
    {
        let values: Vec<u64> = cpu
            .split_whitespace()
            .filter_map(|v| v.parse().ok())
            .collect();
        sample.user = values.first().copied().unwrap_or(0) + values.get(1).copied().unwrap_or(0);
        sample.idle = values.get(3).copied().unwrap_or(0);
        sample.kernel = values.iter().sum::<u64>() - sample.user;
    }
    // The process's own share, in the same USER_HZ ticks. Everything after
    // the closing parenthesis is field 3 onwards, which is the only way to
    // index past a command name that may hold spaces and parentheses.
    if let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat"))
        && let Some(after) = stat.rfind(')').map(|at| &stat[at + 1..])
    {
        let field = |at: usize| -> u64 {
            after
                .split_whitespace()
                .nth(at)
                .and_then(|v| v.parse().ok())
                .unwrap_or(0)
        };
        sample.app = field(11) + field(12);
        sample.own = sample.app;
    }
    sample
}

#[cfg(test)]
mod tests;
