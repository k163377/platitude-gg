//! The machine with no process attached: held awake for the whole
//! invocation, and waited on between runs until it is quiet enough to
//! measure on.

#[cfg(windows)]
use std::process::{Command, Stdio};

#[cfg(windows)]
use super::windows::{AWAKE, CONTINUOUS, SYSTEM_AWAKE};
use super::{Limits, percent};

/// Waits between runs, while nothing is timed, until the machine is
/// unlocked and quiet enough to measure on, or says why it gave up.
pub(in crate::perf) fn wait_for_quiet(
    limits: &Limits,
    ceiling: std::time::Duration,
) -> Result<(), String> {
    if limits.quiet_percent.is_infinite() {
        return Ok(());
    }
    // Each look is a PowerShell whose own load is part of what it reads:
    // spaced wider than the usual look, so the looking does not keep the
    // machine busy.
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

/// Keeps the machine awake for the whole invocation, and — for a D3D run —
/// the display too: nothing is composited to a dark display, so the
/// scroll bench's animation stands still as under a lock.
///
/// `ES_DISPLAY_REQUIRED` alone does not wake a screen already dark, so this
/// injects a zero-pixel mouse move, which moves no cursor
/// (rules-refs/app-ui.md「画面は invocation の間ずっと起こしておく」).
///
/// A software run needs no display, so its helper injects nothing and asks
/// only `ES_SYSTEM_REQUIRED`, leaving the screen to whoever drives it.
///
/// The helper checks on every pass that its parent is alive, by start time
/// as well as pid (a pid is reused): a killed xtask never runs `Drop`, and
/// nothing else would find the loop (`xtask kill` reaps only `platitude-gg`
/// images).
pub(in crate::perf) struct Awake(Option<std::process::Child>);

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
pub(in crate::perf) fn keep_awake(display: bool) -> Awake {
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
    // Said out loud: the failure it causes names something else (a D3D
    // run dies at `measure::SCROLL_CEILING` blaming a covered window).
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
pub(in crate::perf) fn keep_awake(_display: bool) -> Awake {
    Awake(None)
}

/// The machine with no process attached: whether anybody could be looking
/// at it, and the whole-machine processor counters.
///
/// Not a `Sample`, whose keys mostly fall back to zero when missing: a key
/// renamed on one side only would read as an idle machine.
#[derive(Debug, Clone, Copy, Default)]
struct Host {
    /// The same reading as
    /// [`Sample::interactive`](super::Sample::interactive), taken the same
    /// way — its caveats are written there only.
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

/// No desktop to lock here, so the wait is only about the processor, in
/// USER_HZ ticks (the ratio is unit-free).
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

    /// A locked session is the one thing the between-runs wait sees that
    /// the counters cannot say.
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
