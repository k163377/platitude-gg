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
    pub(super) minimized: bool,
    pub(super) windowed: bool,
    /// Whole-machine processor time, in 100ns units, cumulative.
    /// `kernel` includes `idle`, as `GetSystemTimes` reports it.
    pub(super) kernel: u64,
    pub(super) user: u64,
    pub(super) idle: u64,
    /// The measured process's own processor time, same units.
    pub(super) app: u64,
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
    /// included ([`Sample::away_ms`]). Evidence rather than a gate, and
    /// it reads as one number only: above `WAKE_SECS` the wake helper
    /// stopped, and the display timer is no longer being held off.
    pub(super) away_ms: u64,
}

impl Conditions {
    fn absorb(&mut self, sample: &Sample, previous: Option<&Sample>) {
        self.samples += 1;
        self.away_ms = self.away_ms.max(sample.away_ms);
        if sample.interactive {
            self.interactive += 1;
            self.blind = 0;
        } else {
            self.blind += 1;
            self.longest_blind = self.longest_blind.max(self.blind);
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
    ///
    /// Deliberately not a warning: a run taken while the machine was
    /// doing something else is not a slower application, and publishing
    /// it as one is the whole failure this exists to stop.
    pub(super) fn complaint(&self, limits: &Limits) -> Option<String> {
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
        if self.displays.len() > 1 {
            return Some(format!(
                "the window moved between screens ({}) — one run must stay on one screen, whose \
                 refresh rate the frames are read against",
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
}

impl Series {
    fn absorb(&mut self, sample: Sample) {
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

/// Samples until the measured process is reaped or the outer watchdog ends.
pub(super) fn sample_memory(
    pid: u32,
    deadline: Instant,
    started: Instant,
    mut csv: std::fs::File,
) -> Sampler {
    let series = Arc::new(Mutex::new(Series::default()));
    let shared = Arc::clone(&series);
    let handle = std::thread::spawn(move || {
        writeln!(
            csv,
            "parent_elapsed_us,working_set_bytes,private_bytes,display_name,windowed,foreground,\
             interactive,minimized,kernel_100ns,user_100ns,idle_100ns,process_100ns"
        )
        .map_err(|e| e.to_string())?;
        let mut record = |sample: Sample| -> Result<(), String> {
            writeln!(
                csv,
                "{},{},{},{},{},{},{},{},{},{},{},{}",
                started.elapsed().as_micros(),
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
            )
            .map_err(|e| e.to_string())?;
            shared
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .absorb(sample);
            Ok(())
        };
        #[cfg(windows)]
        {
            windows_sampler(pid, deadline, &mut record)
        }
        #[cfg(target_os = "linux")]
        {
            linux_sampler(pid, deadline, &mut record)
        }
        #[cfg(not(any(windows, target_os = "linux")))]
        {
            let _ = (pid, deadline);
            Err("memory sampling is not implemented for this OS".into())
        }
    });
    Sampler { series, handle }
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
        minimized: field("min=") == Some("1"),
        windowed: field("win=") == Some("1"),
        kernel: number("k=").unwrap_or(0),
        user: number("u=").unwrap_or(0),
        idle: number("i=").unwrap_or(0),
        app: number("app=").unwrap_or(0),
    })
}

#[cfg(windows)]
fn windows_sampler(
    pid: u32,
    deadline: Instant,
    record: &mut dyn FnMut(Sample) -> Result<(), String>,
) -> Result<(), String> {
    use std::io::{BufRead, BufReader};
    let seconds = deadline.saturating_duration_since(Instant::now()).as_secs() + 5;
    let script = windows_script(pid, seconds);
    let mut child = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().ok_or("memory sampler stdout missing")?;
    let mut error = None;
    for line in BufReader::new(stdout).lines() {
        let line = match line {
            Ok(line) => line,
            Err(e) => {
                error = Some(e.to_string());
                break;
            }
        };
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
/// ES_DISPLAY_REQUIRED` to hold the screen on, and `ES_CONTINUOUS` alone
/// to let go of it again.
///
/// Spelled in decimal because PowerShell reads a hexadecimal literal with
/// the top bit set as a negative `Int32` and then refuses to hand it to a
/// `uint` parameter.
#[cfg(windows)]
const AWAKE: u32 = 0x8000_0003;
#[cfg(windows)]
const CONTINUOUS: u32 = 0x8000_0000;

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
/// in front however often this fires.
#[cfg(windows)]
fn windows_script(pid: u32, seconds: u64) -> String {
    format!(
        "$ErrorActionPreference='Stop';\
         Add-Type -AssemblyName System.Windows.Forms;\
         Add-Type -TypeDefinition @'\n\
using System;\n\
using System.Runtime.InteropServices;\n\
public static class PerfHost {{\n\
  [DllImport(\"user32.dll\")] public static extern IntPtr GetForegroundWindow();\n\
  [DllImport(\"user32.dll\")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);\n\
  [DllImport(\"user32.dll\")] [return: MarshalAs(UnmanagedType.Bool)] public static extern bool IsIconic(IntPtr h);\n\
  [DllImport(\"user32.dll\")] [return: MarshalAs(UnmanagedType.Bool)] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);\n\
  [StructLayout(LayoutKind.Sequential)] public struct LASTINPUT {{ public uint size; public uint at; }}\n\
  [DllImport(\"user32.dll\")] [return: MarshalAs(UnmanagedType.Bool)] public static extern bool GetLastInputInfo(ref LASTINPUT info);\n\
  [DllImport(\"kernel32.dll\")] public static extern uint GetTickCount();\n\
  public static uint IdleMs() {{\n\
    var info = new LASTINPUT(); info.size = (uint)Marshal.SizeOf(typeof(LASTINPUT));\n\
    if (!GetLastInputInfo(ref info)) return 0;\n\
    return unchecked(GetTickCount() - info.at);\n\
  }}\n\
  [DllImport(\"kernel32.dll\")] public static extern uint SetThreadExecutionState(uint flags);\n\
  [DllImport(\"kernel32.dll\")] [return: MarshalAs(UnmanagedType.Bool)] public static extern bool GetSystemTimes(out long idle, out long kernel, out long user);\n\
}}\n\
'@;\
         [void][PerfHost]::SetThreadExecutionState([uint32]{AWAKE});\
         try {{\
         $p=Get-Process -Id {pid} -ErrorAction SilentlyContinue;\
         $hwnd=[IntPtr]::Zero;$display='-';$ticks=0;$int=1;$lockpids=@();\
         $end=(Get-Date).AddSeconds({seconds});\
         while($p -ne $null -and -not $p.HasExited -and (Get-Date) -lt $end){{\
           try {{\
           $p.Refresh();\
           if($hwnd -eq [IntPtr]::Zero){{\
             $hwnd=$p.MainWindowHandle;\
             if($hwnd -ne [IntPtr]::Zero){{\
               $display=[System.Windows.Forms.Screen]::FromHandle($hwnd).DeviceName;\
               [void][PerfHost]::SetWindowPos($hwnd,[IntPtr]0,0,0,0,0,0x0013);\
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
             if($owner -eq {pid}){{$fg=1}};\
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
             if($fg -eq 0 -and $min -eq 0 -and $ticks % 10 -eq 0){{\
               [void][PerfHost]::SetWindowPos($hwnd,[IntPtr]0,0,0,0,0,0x0013);\
             }};\
           }};\
           $idle=0;$kernel=0;$user=0;\
           [void][PerfHost]::GetSystemTimes([ref]$idle,[ref]$kernel,[ref]$user);\
           [void][PerfHost]::SetThreadExecutionState([uint32]{AWAKE});\
           Write-Output \"ws=$($p.WorkingSet64) pv=$($p.PrivateMemorySize64) display=$display win=$win fg=$fg int=$int min=$min k=$kernel u=$user i=$idle app=$($p.TotalProcessorTime.Ticks) away=$([PerfHost]::IdleMs())\";\
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
    let until = Instant::now() + ceiling;
    let mut last: Option<Host> = None;
    let mut said = false;
    loop {
        let now = host_sample()?;
        if let Some(previous) = &last {
            let busy = now.busy_percent_since(previous);
            if now.interactive && busy <= limits.quiet_percent {
                return Ok(());
            }
            if !said {
                said = true;
                println!(
                    "  waiting for a quiet machine ({}, {busy:.1}% busy)…",
                    if now.interactive {
                        "awake"
                    } else {
                        "session locked"
                    }
                );
            }
        }
        if Instant::now() >= until {
            return Err(format!(
                "the machine did not go quiet within {}s — measure it when nothing else is \
                 running, or pass --allow-noisy to publish the numbers anyway",
                ceiling.as_secs()
            ));
        }
        last = Some(now);
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

/// Keeps the screen awake for as long as it is alive.
///
/// **A dark screen is an unmeasurable machine.** Nothing is composited to
/// a display that is off, so no frames arrive, so the animation the
/// scroll bench is driven by never advances — the same standstill a
/// locked session produces. A measurement nobody is sitting at is idle
/// by definition, so whatever the display timer is set to, it runs out.
///
/// **`ES_DISPLAY_REQUIRED` is not enough**, twice over: it holds only
/// while it is held, so a request that lives for the length of a run
/// holds nothing over the build and the waits *between* runs — and it
/// does not wake a screen that is already dark, which is what every run
/// after the first then starts against. What wakes a dark screen is
/// input, so this sends some: a mouse move of zero pixels, which moves
/// no cursor and interrupts nobody's typing.
///
/// **It has to die with its parent, and `Drop` is not enough.** A killed
/// xtask never unwinds — `taskkill`, a stopped task, an abort — and a
/// loop that only `Drop` stops would then hold the display awake and
/// inject input for the rest of the machine's uptime, with nothing able
/// to find it (`xtask kill` reaps `platitude-gg` images, and killing by
/// image name is denied). So the loop asks whether its parent is still
/// there on every pass: the leak is bounded by one interval.
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
/// its parent check may be and how far `Sample::away_ms` can climb.
#[cfg(windows)]
const WAKE_SECS: u64 = 20;

#[cfg(windows)]
pub(super) fn keep_awake() -> Awake {
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
           [void][PerfWake]::SetThreadExecutionState([uint32]{AWAKE});\
           [PerfWake]::mouse_event(0x0001,0,0,0,[IntPtr]::Zero);\
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
    // entirely: the screen goes dark mid-invocation and every run after
    // it dies at `measure::SCROLL_CEILING` blaming a covered window.
    if child.is_none() {
        println!("  note: could not start the screen-awake helper — a dark screen will spoil runs");
    }
    Awake(child)
}

#[cfg(not(windows))]
pub(super) fn keep_awake() -> Awake {
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

#[cfg(target_os = "linux")]
fn linux_sampler(
    pid: u32,
    deadline: Instant,
    record: &mut dyn FnMut(Sample) -> Result<(), String>,
) -> Result<(), String> {
    let status = format!("/proc/{pid}/status");
    while Instant::now() < deadline {
        let sample = linux_sample_once(pid);
        if sample.working_set == 0 && sample.private == 0 && !std::path::Path::new(&status).exists()
        {
            break;
        }
        record(sample)?;
        std::thread::sleep(Duration::from_millis(SAMPLE_MS));
    }
    Ok(())
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
    }
    sample
}

#[cfg(test)]
mod tests {
    use super::{Conditions, Limits, Sample, Series, parse_host, parse_sample};

    fn sample(ws: u64, fg: bool, tick: u64, app: u64, idle: u64) -> Sample {
        Sample {
            working_set: ws,
            private: ws / 2,
            display: "\\\\.\\DISPLAY2".into(),
            foreground: fg,
            interactive: true,
            away_ms: 0,
            minimized: false,
            windowed: true,
            // One tick of a 24-thread machine: 24 cores * 100ms in 100ns units.
            kernel: tick * 24_000_000,
            user: 0,
            idle,
            app,
        }
    }

    #[test]
    fn a_sampler_line_parses_into_a_tick() {
        let line =
            "ws=123 pv=456 display=\\\\.\\DISPLAY1 win=1 fg=1 int=1 min=0 k=7 u=8 i=9 app=10";
        let parsed = parse_sample(line).expect("a whole line parses");
        assert_eq!(parsed.working_set, 123);
        assert_eq!(parsed.private, 456);
        assert_eq!(parsed.display, "\\\\.\\DISPLAY1");
        assert!(parsed.windowed && parsed.foreground && parsed.interactive && !parsed.minimized);
        assert_eq!(
            (parsed.kernel, parsed.user, parsed.idle, parsed.app),
            (7, 8, 9, 10)
        );
        // Before the window exists the display is a dash, not a name.
        let bare =
            parse_sample("ws=1 pv=1 display=- win=0 fg=0 int=1 min=0 k=0 u=0 i=0 app=0").unwrap();
        assert!(bare.display.is_empty() && !bare.windowed && bare.interactive);
        assert!(parse_sample("PowerShell said something else entirely").is_none());
    }

    /// The between-runs wait reads the machine alone. A locked session is
    /// the one thing it can see that the counters cannot say.
    #[test]
    fn a_host_line_parses_into_the_machine_alone() {
        let awake = parse_host("int=1 k=7 u=8 i=9").expect("a whole line parses");
        assert!(awake.interactive);
        assert_eq!((awake.kernel, awake.user, awake.idle), (7, 8, 9));
        let locked = parse_host("int=0 k=7 u=8 i=9").expect("a whole line parses");
        assert!(!locked.interactive);
        // Half the capacity went somewhere other than idle.
        let later = parse_host("int=1 k=17 u=8 i=14").expect("a whole line parses");
        assert!((later.busy_percent_since(&awake) - 50.0).abs() < 0.001);
        // Two reads of the same instant divide by nothing.
        assert!(awake.busy_percent_since(&awake).abs() < f64::EPSILON);
        assert!(parse_host("PowerShell said something else entirely").is_none());
    }

    #[test]
    fn the_peak_and_the_last_reading_come_off_one_series() {
        let mut series = Series::default();
        series.absorb(sample(100, true, 1, 0, 24_000_000));
        series.absorb(sample(300, true, 2, 0, 48_000_000));
        series.absorb(sample(200, true, 3, 0, 72_000_000));
        assert_eq!(series.peak_working_set, 300);
        assert_eq!(series.last.map(|s| s.working_set), Some(200));
        assert_eq!(series.conditions.samples, 3);
    }

    #[test]
    fn an_idle_machine_reads_as_no_foreign_load() {
        let mut series = Series::default();
        // Every 100ns of capacity went to idle across all three ticks.
        series.absorb(sample(100, true, 1, 0, 24_000_000));
        series.absorb(sample(100, true, 2, 0, 48_000_000));
        assert!(series.conditions.busy_percent.abs() < 0.001);
        assert!(series.conditions.complaint(&Limits::default()).is_none());
    }

    #[test]
    fn work_the_measured_process_did_is_not_foreign_load() {
        let mut series = Series::default();
        // Half the machine busy, and all of it the app's own.
        series.absorb(sample(100, true, 1, 0, 24_000_000));
        series.absorb(sample(100, true, 2, 12_000_000, 36_000_000));
        assert!((series.conditions.busy_percent - 50.0).abs() < 0.001);
        assert!(series.conditions.foreign_percent.abs() < 0.001);
        assert!(series.conditions.complaint(&Limits::default()).is_none());
    }

    #[test]
    fn a_busy_machine_is_refused_and_names_the_share() {
        let mut series = Series::default();
        series.absorb(sample(100, true, 1, 0, 24_000_000));
        series.absorb(sample(100, true, 2, 0, 36_000_000));
        let complaint = series
            .conditions
            .complaint(&Limits::default())
            .expect("half the machine went elsewhere");
        assert!(complaint.contains("50.0%"), "{complaint}");
        assert!(series.conditions.complaint(&Limits::OPEN).is_none());
    }

    /// Losing the front is recorded and does not spoil the run: a window
    /// that is not in front is still composited, and this application
    /// does not reliably take the focus off the shell that started it
    /// (every tick of a healthy run can read `fg=0`).
    #[test]
    fn a_window_that_lost_the_front_is_still_a_reading() {
        let mut series = Series::default();
        for tick in 1..=10 {
            series.absorb(sample(100, tick > 3, tick, 0, tick * 24_000_000));
        }
        assert!(series.conditions.complaint(&Limits::default()).is_none());
        assert_eq!(series.conditions.foreground_share(), Some(0.7));
    }

    /// A locked session is the one the window cannot be seen through.
    #[test]
    fn a_locked_session_is_refused() {
        let mut series = Series::default();
        for tick in 1..=10 {
            let mut tick_sample = sample(100, false, tick, 0, tick * 24_000_000);
            tick_sample.interactive = tick < 8;
            series.absorb(tick_sample);
        }
        let complaint = series
            .conditions
            .complaint(&Limits::default())
            .expect("three ticks in a row with nobody able to look");
        assert!(
            complaint.contains("for 3 of 10 sampled ticks"),
            "{complaint}"
        );
        assert!(series.conditions.complaint(&Limits::OPEN).is_none());
    }

    /// The secure desktop flashing past — a consent prompt, a focus
    /// change — is not a lock, and refusing on it costs a retake plus
    /// the wait before it.
    #[test]
    fn a_blink_of_the_secure_desktop_is_not() {
        let mut series = Series::default();
        for tick in 1..=10 {
            let mut tick_sample = sample(100, true, tick, 0, tick * 24_000_000);
            tick_sample.interactive = tick != 4 && tick != 5;
            series.absorb(tick_sample);
        }
        assert_eq!(series.conditions.longest_blind, 2);
        assert!(series.conditions.complaint(&Limits::default()).is_none());
    }

    /// Two short blinks are not one long one: the gate reads the longest
    /// unbroken stretch, not the total.
    #[test]
    fn scattered_blinks_do_not_add_up_to_a_lock() {
        let mut series = Series::default();
        for tick in 1..=12 {
            let mut tick_sample = sample(100, true, tick, 0, tick * 24_000_000);
            tick_sample.interactive = tick % 5 != 0;
            series.absorb(tick_sample);
        }
        assert_eq!(series.conditions.interactive, 10);
        assert_eq!(series.conditions.longest_blind, 1);
        assert!(series.conditions.complaint(&Limits::default()).is_none());
    }

    /// A window is mapped where the platform puts it and only then moved
    /// onto the screen the run asked for, so where it was in the first
    /// second is not held against it — but a move after that is the
    /// window wandering off the screen whose refresh the frames are read
    /// against.
    #[test]
    fn a_window_that_changed_screens_after_settling_is_refused() {
        let mut settling = Conditions::default();
        let arrived = sample(100, true, 1, 0, 24_000_000);
        let mut elsewhere = arrived.clone();
        elsewhere.display = "\\\\.\\DISPLAY1".into();
        settling.absorb(&elsewhere, None);
        for _ in 0..12 {
            settling.absorb(&arrived, None);
        }
        assert!(settling.complaint(&Limits::default()).is_none());
        assert_eq!(settling.displays, ["\\\\.\\DISPLAY2"]);

        let mut wandered = settling.clone();
        wandered.absorb(&elsewhere, None);
        let complaint = wandered
            .complaint(&Limits::default())
            .expect("a second screen once the window was placed");
        assert!(complaint.contains("moved between screens"), "{complaint}");
    }

    #[test]
    fn a_minimised_window_is_refused_before_anything_else() {
        let mut conditions = Conditions::default();
        let mut down = sample(100, false, 1, 0, 24_000_000);
        down.minimized = true;
        conditions.absorb(&down, None);
        conditions.absorb(&down, None);
        assert!(
            conditions
                .complaint(&Limits::default())
                .is_some_and(|c| c.contains("minimised"))
        );
    }
}
