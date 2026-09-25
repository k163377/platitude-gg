//! The machine with no process attached: held awake for the whole
//! invocation, and waited on between runs until it is quiet enough to
//! measure on.

#[cfg(windows)]
use std::process::{Command, Stdio};

#[cfg(windows)]
use super::windows::{AWAKE, CONTINUOUS, SYSTEM_AWAKE};
use super::{Limits, percent};

/// Waits until the machine is quiet enough to measure on, or says why it
/// gave up. Between runs: what it costs is a second PowerShell, and the
/// point is to spend it while nothing is being timed.
///
/// This is the answer to a person walking away
/// mid-measurement: a session somebody locked by hand, or a
/// build somebody started, is waited out. The screen is held
/// awake across this wait as across everything else —
/// [`keep_awake`] is held for the whole invocation, which is
/// what makes the gap between two runs no darker than a run.
pub(crate) fn wait_for_quiet(limits: &Limits, ceiling: std::time::Duration) -> Result<(), String> {
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
/// the same reason the other is: the machine stays awake between the
/// runs as during them.
///
/// **It has to die with its parent, past any `Drop`.** A killed
/// xtask never unwinds — `taskkill`, a stopped task, an abort — and a
/// loop that only `Drop` stops would then hold the machine awake (and,
/// lit, inject input) for the rest of the machine's uptime, with nothing
/// able to find it (`xtask kill` reaps `platitude-gg` images, and
/// killing by image name is denied). So the loop asks whether its parent
/// is still there on every pass: the leak is bounded by one interval.
///
/// **The parent is identified by when it started.**
/// A pid is reused, and a wake loop that only asked whether *something*
/// holds that number would outlive its parent for as long as whatever
/// took the number lives.
pub(crate) struct Awake(Option<std::process::Child>);

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
/// before [`Conditions::complaint`](super::Conditions::complaint) reads it as the helper having died.
pub(super) const WAKE_SECS: u64 = 20;

#[cfg(windows)]
pub(crate) fn keep_awake(display: bool) -> Awake {
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
    // waits(paced): the wake loop's pass — `WAKE_SECS` between pokes, ending when
    // the parent it watches is gone
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
pub(crate) fn keep_awake(_display: bool) -> Awake {
    Awake(None)
}

/// The machine with no process attached: whether anybody could be looking
/// at it, and the whole-machine processor counters.
///
/// Its own type: a `Sample` would have to carry a second spelling of
/// the sampler's field list, and every key but two of those falls back
/// to zero when it is missing — so a key renamed on one side and not
/// the other would read as a perfectly idle machine.
#[derive(Debug, Clone, Copy, Default)]
struct Host {
    /// False says nobody could be looking at this desktop. The same
    /// reading as [`Sample::interactive`](super::Sample::interactive), taken the same way and
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
/// processor. USER_HZ ticks — the ratio is unit-free, so only the shape
/// has to match.
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
    let number = |key: &str| crate::perf::reading::token(line, key).and_then(|v| v.parse().ok());
    Some(Host {
        interactive: crate::perf::reading::token(line, "int=") == Some("1"),
        kernel: number("k=")?,
        user: number("u=")?,
        idle: number("i=")?,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_host;

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
}
