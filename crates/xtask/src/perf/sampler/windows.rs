//! The Windows sampler: one PowerShell, its C# compiled before the run
//! and held for it, and the line-at-a-time pipe it — and the attribution
//! script — speak on.

use std::process::{Command, Stdio};

use super::{Sample, parse_sample};
use crate::perf::SAMPLE_MS;

pub(in crate::perf) type Lines = std::io::Lines<std::io::BufReader<std::process::ChildStdout>>;

/// The process creation flag that starts a process with its primary
/// thread suspended: a pid with nothing executed yet, which the sampler
/// resumes once the process is in its job object.
pub(in crate::perf) const CREATE_SUSPENDED: u32 = 0x0000_0004;

/// How long the script is given to say `ready`, and later `resumed` — the
/// compile takes a fraction of it. A wait with no ceiling would hold every
/// build on the machine behind a measurement that never starts.
const SCRIPT_CEILING: std::time::Duration = std::time::Duration::from_secs(30);

/// Reads the script's lines until `wanted`, within [`SCRIPT_CEILING`],
/// and hands the rest back; a `note:` on the way is printed. The read
/// blocks, so it runs on a thread of its own; a script that never answers
/// leaves that thread on the pipe until the script is ended.
pub(in crate::perf) fn await_line(mut lines: Lines, wanted: &'static str) -> Result<Lines, String> {
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
                // "it": the caller names which script — the attribution
                // script waits here too.
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
pub(in crate::perf) fn end(child: &mut std::process::Child, what: &str) {
    if let Err(error) = child.kill() {
        println!("  note: could not end {what}: {error}");
    }
}

/// Starts the script and reads up to its `ready`: compiled, holding a job
/// object, and waiting on stdin for the process to watch.
pub(super) fn windows_arm(
    seconds: u64,
    software: bool,
) -> Result<(std::process::Child, Lines), String> {
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
pub(super) fn windows_watch(
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
/// ES_SYSTEM_REQUIRED` to hold only the machine (`--software`), and
/// `ES_CONTINUOUS` alone to let go.
///
/// Spliced into the scripts in decimal: PowerShell reads a hexadecimal
/// literal with the top bit set as a negative `Int32` and will not hand it
/// to a `uint` parameter.
pub(super) const AWAKE: u32 = 0x8000_0003;
pub(super) const SYSTEM_AWAKE: u32 = 0x8000_0001;
pub(super) const CONTINUOUS: u32 = 0x8000_0000;

/// The Win32 the sampler script calls (`windows_script` says what for).
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

/// The display's power state: a `NativeWindow` on a thread of its own
/// pumps the `GUID_CONSOLE_DISPLAY_STATE` broadcast (0 off, 1 on, 2
/// dimmed), which Windows sends on registration and on every change.
/// `Dark()` is `1` / `0` / `-` for off / on-or-dimmed / not answered yet.
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
/// Beside the process's memory it:
///
/// * holds the machine awake, and the screen too for a D3D run ([`AWAKE`]);
/// * says whether the window is up, in front or minimised, and whether the
///   session is locked;
/// * reads `GetSystemTimes` beside the process's own time, which tells a
///   slow application from a busy machine;
/// * counts the process's children as its own: the process arrives
///   suspended, joins a job object, and only then is resumed; `app=` is
///   the job's accounting, which keeps the time of children that already
///   exited. A process the job will not take (`note:`) is resumed all the
///   same and counted alone.
///
/// Two phases, on one pipe each way: the script compiles, makes the job
/// and says `ready`; the pid comes down stdin; `resumed` and then the
/// samples go up stdout.
///
/// The window handle is resolved once and held: `MainWindowHandle`
/// enumerates every top-level window, and `Refresh()` (which the counters
/// need) throws the cached one away — asking per tick would sweep the
/// desktop twice a tick.
///
/// The window is raised without taking focus (`SWP_NOMOVE | SWP_NOSIZE |
/// SWP_NOACTIVATE`) when it appears and, while it is not in front, on every
/// tenth tick: a covered window advances no animation, so the scroll bench
/// never starts (`measure::SCROLL_CEILING`). `HWND_TOP` does not beat a
/// topmost window, so a `HWND_TOPMOST` notification stays in front.
fn windows_script(seconds: u64, software: bool) -> String {
    // A software run's frames need no display: the screen is left alone
    // (`ES_DISPLAY_REQUIRED` would hold it on once a person lit it) and
    // the window is not raised into a person's face every second.
    let awake = if software { SYSTEM_AWAKE } else { AWAKE };
    let raise = u8::from(!software);
    // waits(paced): the sampler's tick — a reading every `SAMPLE_MS`, the loop
    // ending when the process it watches is gone or the run's window has passed
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
           Write-Output \"ws=$($p.WorkingSet64) peakws=$($p.PeakWorkingSet64) pv=$($p.PrivateMemorySize64) display=$display win=$win fg=$fg int=$int min=$min k=$kernel u=$user i=$idle app=$app own=$own job=$jobok away=$([PerfHost]::IdleMs()) dark=$([PerfDisplay]::Dark())\";\
           }} catch {{ if($p.HasExited){{break}}; throw }};\
           Start-Sleep -Milliseconds {SAMPLE_MS};\
         }}\
         }} finally {{ [void][PerfHost]::SetThreadExecutionState([uint32]{CONTINUOUS}) }}"
    )
}
