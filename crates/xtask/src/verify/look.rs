//! The look at a process still standing when the parent's ceiling
//! arrives: its threads, and on Windows a dump of it — what the record a
//! stopped run leaves (`super::wedge`) cannot say, since past `exiting`
//! nothing inside the process is left to say it. Each is a diagnostic
//! process under a ceiling of its own, its stdout a file ([`bounded`]).

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use crate::subprocess::{Answer, bounded};

#[cfg(windows)]
const DUMP_FILE: &str = "app.dmp";
/// The whole thread listing, beside the pictures — the line under the
/// verdict shows only the first few ([`listed`]). Written by the listing
/// process itself.
const THREADS_FILE: &str = "threads.txt";

/// How long a look at the app may take: the listing is one PowerShell,
/// the dump writes the process's whole memory to disk. Windows only — the
/// walk of `/proc` takes no process.
#[cfg(windows)]
const LISTING_CEILING: Duration = Duration::from_secs(15);
#[cfg(windows)]
const DUMP_CEILING: Duration = Duration::from_secs(60);
/// The ceiling a look ordered to stall is ended at (`--fault-stall-look`):
/// the stand-in never answers, so any height ends it, and the listing's
/// own ceiling would only be wall clock paid for nothing.
const STALLED_LOOK_CEILING: Duration = Duration::from_secs(1);

/// The app's threads, taken while it still stands, and on Windows a dump
/// of it unless the stop was ordered (`--fault-hang`, whose cause needs
/// no dump). Past `exiting` the exit has ended every thread but its own,
/// so one thread alive is a process inside its exit.
///
/// Every look is ended at its ceiling ([`bounded`]) and the run goes on to
/// reap the app. `stalled` makes the listing a process that never answers
/// (`--fault-stall-look`), so that line can be checked (`super::faults`).
pub(super) fn look_at(pid: u32, shot_dir: &Path, unordered: bool, stalled: bool) -> Vec<String> {
    looked(pid, "the app", shot_dir, unordered, stalled, After::Reaped)
}

/// A look at a process that is not the app and is not reaped behind the
/// look — the container engine's session process, which every checkout
/// on the machine shares (`linux::witness`). Its threads and its dump
/// land in `dir` under the names the app's take, and nothing here
/// suspends a thread of it ([`After::GoesOn`]).
pub(crate) fn look_into(pid: u32, whose: &str, dir: &Path) -> Vec<String> {
    looked(pid, whose, dir, true, false, After::GoesOn)
}

/// What becomes of the process once it has been looked at, which decides
/// what a look may do to it. Every look is a process ended at a ceiling,
/// and one ended between suspending a thread and resuming it leaves that
/// thread suspended for good.
#[derive(Clone, Copy, PartialEq)]
enum After {
    /// The run that looked reaps it: each thread is suspended for its
    /// stack, and all of them for as long as the dump is written.
    Reaped,
    /// It goes on running. No thread of it is suspended from out here:
    /// the listing walks no stack, and the dump is written off a
    /// snapshot — the stacks are in the dump.
    GoesOn,
}

fn looked(
    pid: u32,
    whose: &str,
    shot_dir: &Path,
    unordered: bool,
    stalled: bool,
    after: After,
) -> Vec<String> {
    let threads = if stalled {
        listing(bounded(
            "the thread listing",
            stand_in(),
            STALLED_LOOK_CEILING,
            Some(&shot_dir.join(THREADS_FILE)),
        ))
    } else {
        threads_of(pid, shot_dir, after)
    };
    let mut lines = match threads {
        Ok(threads) => {
            let mut lines = vec![format!(
                "  threads at the ceiling: {} alive — {}",
                threads.lines.len(),
                listed(&threads.lines)
            )];
            // The one stack that names a stop inside the exit.
            if let Some(stack) = threads.main_stack() {
                lines.push(format!(
                    "  the main thread stands in: {stack} (every thread's stack: {THREADS_FILE})"
                ));
            }
            lines
        }
        Err(why) => vec![format!(
            "  threads at the ceiling: could not be listed — {why}"
        )],
    };
    if unordered {
        lines.push(dump_of(pid, whose, shot_dir, after));
    }
    lines
}

/// What a listing answered with: one line per thread, and each one's
/// stack where the listing walks them — Windows only, as
/// `stack <tid> <frames>` lines after the threads.
#[derive(Debug, PartialEq, Eq)]
struct Threads {
    lines: Vec<String>,
    /// `(tid, frames)`, the frames innermost first and joined by ` <- `.
    stacks: Vec<(String, String)>,
}

impl Threads {
    /// The main thread's stack, cut to what one line under the verdict
    /// can hold: the thread named `main`, or the first listed where none
    /// is. `None` where the listing walked nothing.
    fn main_stack(&self) -> Option<String> {
        const SHOWN: usize = 12;
        let tid = self
            .lines
            .iter()
            .find(|line| {
                line.split_whitespace()
                    .any(|word| word.starts_with("main@"))
            })
            .or_else(|| self.lines.first())?
            .split_whitespace()
            .next()?;
        let (_, frames) = self.stacks.iter().find(|(id, _)| id == tid)?;
        let frames: Vec<&str> = frames.split(" <- ").collect();
        let mut shown = frames
            .iter()
            .take(SHOWN)
            .copied()
            .collect::<Vec<_>>()
            .join(" <- ");
        if frames.len() > SHOWN {
            shown.push_str(" <- …");
        }
        Some(shown)
    }
}

/// The threads a listing answered with, or why it could not — worded for
/// the line under the verdict.
fn listing(answer: Answer) -> Result<Threads, String> {
    match answer {
        Answer::Ended { status, stdout } if status.success() => {
            let mut threads = Threads {
                lines: Vec::new(),
                stacks: Vec::new(),
            };
            for line in stdout
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
            {
                match line.strip_prefix("stack ") {
                    Some(rest) => {
                        let (tid, frames) = rest.split_once(' ').unwrap_or((rest, ""));
                        threads.stacks.push((tid.to_string(), frames.to_string()));
                    }
                    None => threads.lines.push(line.to_string()),
                }
            }
            if threads.lines.is_empty() {
                return Err("the listing answered with no threads".to_string());
            }
            Ok(threads)
        }
        Answer::Ended { status, .. } => Err(format!("the listing exited {status}")),
        Answer::OutOfTime {
            after,
            ended: Ok(()),
            ..
        } => Err(format!(
            "the listing ran out of time after {:.1}s and was ended",
            after.as_secs_f32()
        )),
        Answer::OutOfTime {
            pid,
            after,
            ended: Err(error),
        } => Err(format!(
            "the listing ran out of time after {:.1}s and could not be ended ({error}); pid {pid} \
             may still be running",
            after.as_secs_f32()
        )),
        Answer::Unstarted(why) => Err(why),
    }
}

/// A stand-in for a diagnostic that stalls: it outlasts every ceiling
/// here. On Windows `ping` against the loopback, since there is no `sleep`.
#[cfg(windows)]
fn stand_in() -> Command {
    let mut command = Command::new("ping");
    command.args(["-n", "600", "127.0.0.1"]);
    command
}

#[cfg(not(windows))]
fn stand_in() -> Command {
    let mut command = Command::new("sleep");
    command.arg("600");
    command
}

/// The first few threads on one line; the rest are counted.
fn listed(threads: &[String]) -> String {
    const SHOWN: usize = 8;
    let shown = threads
        .iter()
        .take(SHOWN)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    if threads.len() > SHOWN {
        format!("{shown}, … {} more", threads.len() - SHOWN)
    } else {
        shown
    }
}

/// The listing, from a PowerShell of its own: per thread its number,
/// state, wait reason, description (what Rust and Qt name threads by),
/// `@` the module it started in (which names the system's own threads),
/// and CPU time. The two in the middle tell a teardown survivor from the
/// system's idle threads and are Win32 calls `Get-Process` does not
/// carry; a thread that could not be opened shows `-@?`.
///
/// Then one `stack <tid> <frames>` line per thread, walked with `dbghelp`
/// (on every Windows, no debugger needed). With no symbols on the machine
/// a frame is off the nearest export: the module exact, the function a
/// neighbourhood. The symbol search path is empty and prompts are off, or
/// an `_NT_SYMBOL_PATH` naming a symbol server spends the listing's
/// ceiling on the network.
#[cfg(windows)]
const LISTING_SCRIPT: &str = "Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class PggThreads {
    [DllImport(\"kernel32.dll\", SetLastError = true)] static extern IntPtr OpenThread(uint access, bool inherit, uint tid);
    [DllImport(\"kernel32.dll\", SetLastError = true)] static extern IntPtr OpenProcess(uint access, bool inherit, uint pid);
    [DllImport(\"kernel32.dll\")] static extern bool CloseHandle(IntPtr handle);
    [DllImport(\"kernel32.dll\", SetLastError = true)] static extern int GetThreadDescription(IntPtr handle, out IntPtr description);
    [DllImport(\"kernel32.dll\")] static extern IntPtr LocalFree(IntPtr memory);
    [DllImport(\"kernel32.dll\")] static extern uint SuspendThread(IntPtr handle);
    [DllImport(\"kernel32.dll\")] static extern uint ResumeThread(IntPtr handle);
    [DllImport(\"kernel32.dll\")] static extern bool GetThreadContext(IntPtr handle, IntPtr context);
    [DllImport(\"kernel32.dll\", CharSet = CharSet.Unicode)] static extern IntPtr GetModuleHandle(string name);
    [DllImport(\"kernel32.dll\", CharSet = CharSet.Ansi)] static extern IntPtr GetProcAddress(IntPtr module, string name);
    [DllImport(\"kernel32.dll\", CharSet = CharSet.Unicode)] static extern uint K32GetModuleBaseNameW(IntPtr process, IntPtr module, StringBuilder name, uint size);
    [DllImport(\"ntdll.dll\")] static extern int NtQueryInformationThread(IntPtr handle, int kind, out IntPtr info, int length, IntPtr returned);
    [DllImport(\"dbghelp.dll\", SetLastError = true)] static extern bool SymInitialize(IntPtr process, string path, bool invade);
    [DllImport(\"dbghelp.dll\")] static extern bool SymCleanup(IntPtr process);
    [DllImport(\"dbghelp.dll\")] static extern uint SymSetOptions(uint options);
    [DllImport(\"dbghelp.dll\")] static extern ulong SymGetModuleBase64(IntPtr process, ulong address);
    [DllImport(\"dbghelp.dll\")] static extern bool SymFromAddr(IntPtr process, ulong address, out ulong displacement, IntPtr symbol);
    [DllImport(\"dbghelp.dll\")] static extern bool StackWalk64(uint machine, IntPtr process, IntPtr thread, IntPtr frame, IntPtr context, IntPtr readMemory, IntPtr functionTableAccess, IntPtr getModuleBase, IntPtr translateAddress);
    static IntPtr process = IntPtr.Zero;
    static IntPtr fta = IntPtr.Zero;
    static IntPtr gmb = IntPtr.Zero;
    public static string Describe(uint tid) {
        IntPtr handle = OpenThread(0x0040 | 0x0800, false, tid);
        if (handle == IntPtr.Zero) return \"|\";
        IntPtr description; string name = \"\";
        if (GetThreadDescription(handle, out description) >= 0) { name = Marshal.PtrToStringUni(description); LocalFree(description); }
        IntPtr start; string at = \"\";
        if (NtQueryInformationThread(handle, 9, out start, IntPtr.Size, IntPtr.Zero) == 0) at = start.ToInt64().ToString();
        CloseHandle(handle);
        return name + \"|\" + at;
    }
    public static string Begin(uint pid) {
        process = OpenProcess(0x0400 | 0x0010, false, pid);
        if (process == IntPtr.Zero) return \"the process could not be opened\";
        SymSetOptions(0x2 | 0x4 | 0x80000);
        if (!SymInitialize(process, \"\", true)) { CloseHandle(process); process = IntPtr.Zero; return \"symbols could not be initialised\"; }
        IntPtr dbghelp = GetModuleHandle(\"dbghelp.dll\");
        fta = GetProcAddress(dbghelp, \"SymFunctionTableAccess64\");
        gmb = GetProcAddress(dbghelp, \"SymGetModuleBase64\");
        return \"\";
    }
    public static void End() {
        if (process == IntPtr.Zero) return;
        SymCleanup(process);
        CloseHandle(process);
        process = IntPtr.Zero;
    }
    public static string Walk(uint tid) {
        if (process == IntPtr.Zero) return \"no process to walk in\";
        IntPtr thread = OpenThread(0x0008 | 0x0002 | 0x0040, false, tid);
        if (thread == IntPtr.Zero) return \"the thread could not be opened\";
        IntPtr raw = Marshal.AllocHGlobal(1232 + 16);
        IntPtr context = new IntPtr((raw.ToInt64() + 15L) & ~15L);
        IntPtr frame = Marshal.AllocHGlobal(512);
        IntPtr symbol = Marshal.AllocHGlobal(88 + 512);
        StringBuilder frames = new StringBuilder();
        try {
            if (SuspendThread(thread) == 0xFFFFFFFF) return \"the thread could not be suspended\";
            try {
                for (int i = 0; i < 1232; i++) Marshal.WriteByte(context, i, 0);
                Marshal.WriteInt32(context, 0x30, 0x100003);
                if (!GetThreadContext(thread, context)) return \"the context could not be read\";
                long rip = Marshal.ReadInt64(context, 0xF8);
                long rsp = Marshal.ReadInt64(context, 0x98);
                long rbp = Marshal.ReadInt64(context, 0xA0);
                for (int i = 0; i < 512; i++) Marshal.WriteByte(frame, i, 0);
                Marshal.WriteInt64(frame, 0, rip); Marshal.WriteInt32(frame, 12, 3);
                Marshal.WriteInt64(frame, 32, rbp); Marshal.WriteInt32(frame, 44, 3);
                Marshal.WriteInt64(frame, 48, rsp); Marshal.WriteInt32(frame, 60, 3);
                for (int depth = 0; depth < 24; depth++) {
                    if (!StackWalk64(0x8664, process, thread, frame, context, IntPtr.Zero, fta, gmb, IntPtr.Zero)) break;
                    ulong pc = (ulong)Marshal.ReadInt64(frame, 0);
                    if (pc == 0) break;
                    if (frames.Length > 0) frames.Append(\" <- \");
                    frames.Append(Name(symbol, pc));
                }
            } finally { ResumeThread(thread); }
        } finally {
            Marshal.FreeHGlobal(raw); Marshal.FreeHGlobal(frame); Marshal.FreeHGlobal(symbol);
            CloseHandle(thread);
        }
        return frames.Length == 0 ? \"no frames\" : frames.ToString();
    }
    static string Name(IntPtr symbol, ulong pc) {
        ulong moduleBase = SymGetModuleBase64(process, pc);
        string module = \"?\";
        if (moduleBase != 0) { StringBuilder name = new StringBuilder(260); if (K32GetModuleBaseNameW(process, new IntPtr((long)moduleBase), name, 260) > 0) module = name.ToString(); }
        for (int i = 0; i < 88 + 512; i++) Marshal.WriteByte(symbol, i, 0);
        Marshal.WriteInt32(symbol, 0, 88); Marshal.WriteInt32(symbol, 80, 500);
        ulong displacement;
        if (SymFromAddr(process, pc, out displacement, symbol)) {
            string function = Marshal.PtrToStringAnsi(new IntPtr(symbol.ToInt64() + 84));
            return module + \"!\" + function + \"+0x\" + displacement.ToString(\"x\");
        }
        if (moduleBase != 0) return module + \"+0x\" + (pc - moduleBase).ToString(\"x\");
        return \"0x\" + pc.ToString(\"x\");
    }
}
'@
$process = Get-Process -Id PGG_PID
$modules = @()
try { foreach ($module in $process.Modules) { $modules += [pscustomobject]@{ Name = $module.ModuleName; Base = $module.BaseAddress.ToInt64(); End = $module.BaseAddress.ToInt64() + $module.ModuleMemorySize } } } catch {}
foreach ($thread in $process.Threads) {
    $described = [PggThreads]::Describe([uint32]$thread.Id).Split('|')
    $name = $described[0]; if ($name -eq '') { $name = '-' }
    $in = '?'
    if ($described[1] -ne '') { $start = [int64]$described[1]; $module = $modules | Where-Object { $start -ge $_.Base -and $start -lt $_.End } | Select-Object -First 1; if ($module) { $in = $module.Name } else { $in = 'unmapped' } }
    $waiting = ''; if ($thread.ThreadState -eq 'Wait') { $waiting = $thread.WaitReason }
    '{0} {1} {2} {3}@{4} cpu={5}ms' -f $thread.Id, $thread.ThreadState, $waiting, $name, $in, [int]$thread.TotalProcessorTime.TotalMilliseconds
}
if (PGG_WALK -eq 1) {
    $began = [PggThreads]::Begin([uint32]PGG_PID)
    foreach ($thread in $process.Threads) {
        if ($began -ne '') { $walked = $began } else { $walked = [PggThreads]::Walk([uint32]$thread.Id) }
        'stack {0} {1}' -f $thread.Id, $walked
    }
    [PggThreads]::End()
}";

/// Every thread of the process ([`LISTING_SCRIPT`]), written whole beside
/// the pictures ([`THREADS_FILE`]) and read back from there. The stacks
/// are walked only of a process about to be reaped ([`After`]): the walk
/// suspends each thread.
#[cfg(windows)]
fn threads_of(pid: u32, shot_dir: &Path, after: After) -> Result<Threads, String> {
    let script = LISTING_SCRIPT
        .replace("PGG_PID", &pid.to_string())
        .replace("PGG_WALK", if after == After::Reaped { "1" } else { "0" });
    let mut command = Command::new("powershell");
    command.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
    listing(bounded(
        "the thread listing",
        command,
        LISTING_CEILING,
        Some(&shot_dir.join(THREADS_FILE)),
    ))
}

/// The same off `/proc`, with each thread's name. It leaves no file and no
/// stacks (`/proc/<pid>/task/<tid>/stack` is root's).
#[cfg(unix)]
fn threads_of(pid: u32, _shot_dir: &Path, _after: After) -> Result<Threads, String> {
    let tasks = std::fs::read_dir(format!("/proc/{pid}/task"))
        .map_err(|error| format!("/proc/{pid}/task could not be read: {error}"))?;
    let mut threads = Vec::new();
    for task in tasks.flatten() {
        let dir = task.path();
        let tid = task.file_name().to_string_lossy().into_owned();
        let comm = std::fs::read_to_string(dir.join("comm")).unwrap_or_default();
        let stat = std::fs::read_to_string(dir.join("stat")).unwrap_or_default();
        // The state is the field after the command's closing parenthesis,
        // which can itself hold spaces.
        let state = stat
            .rsplit_once(')')
            .and_then(|(_, rest)| rest.split_whitespace().next())
            .unwrap_or("?");
        let wchan = std::fs::read_to_string(dir.join("wchan")).unwrap_or_default();
        threads.push(format!(
            "{tid} {state} {} wchan={}",
            comm.trim(),
            wchan.trim()
        ));
    }
    if threads.is_empty() {
        return Err(format!("/proc/{pid}/task lists no threads"));
    }
    Ok(Threads {
        lines: threads,
        stacks: Vec::new(),
    })
}

/// The dump, written by `MiniDumpWriteDump` itself from a PowerShell of
/// its own — a process, so the ceiling can end it ([`bounded`]). Not
/// `rundll32 comsvcs.dll,MiniDump`: its file holds no memory and its own
/// taker cannot open it (internal-docs/ハング調査.md §次の 1 回で何が読めるか).
///
/// `PGG_PID`, `PGG_PATH`, `PGG_KIND` and `PGG_WRITE` are filled in by
/// [`dump_of`]; the path sits in a single-quoted literal, which a temp
/// path has no quote to break. Exits with the call's Win32 error, zero
/// for a dump written.
///
/// **Two ways to write it** ([`After`]). `Write` hands the dumper the
/// process itself, whose threads it suspends for as long as it writes.
/// `WriteOfASnapshot` hands it a snapshot (`PssCaptureSnapshot`, a clone
/// of the address space with the threads' contexts): the process is held
/// for that one call and runs on while the dump is written off the
/// clone, so a dumper ended at its ceiling leaves it as it was. The
/// dumper has to be told it was handed a snapshot, which is what the
/// callback is for (`IsProcessSnapshotCallback` = 16, answered
/// `S_FALSE`; minidumpapiset.h packs its structures to 4, which puts the
/// callback's kind at offset 12).
#[cfg(windows)]
const DUMP_SCRIPT: &str = "Add-Type -TypeDefinition @'
using System;
using System.IO;
using System.Runtime.InteropServices;
public static class PggDump {
    [DllImport(\"dbghelp.dll\", SetLastError = true)]
    static extern bool MiniDumpWriteDump(IntPtr process, uint pid, IntPtr file, int kind, \
     IntPtr exception, IntPtr user, IntPtr callback);
    [DllImport(\"kernel32.dll\")]
    static extern int PssCaptureSnapshot(IntPtr process, uint capture, uint context, out IntPtr snapshot);
    [DllImport(\"kernel32.dll\")] static extern int PssFreeSnapshot(IntPtr process, IntPtr snapshot);
    [DllImport(\"kernel32.dll\")] static extern IntPtr GetCurrentProcess();
    delegate int Asked(IntPtr param, IntPtr input, IntPtr output);
    static int ItIsASnapshot(IntPtr param, IntPtr input, IntPtr output) {
        if (Marshal.ReadInt32(input, 12) == 16) Marshal.WriteInt32(output, 0, 1);
        return 1;
    }
    public static int Write(int pid, string path, int kind) {
        var process = System.Diagnostics.Process.GetProcessById(pid);
        using (var file = new FileStream(path, FileMode.Create, FileAccess.ReadWrite, FileShare.None)) {
            return MiniDumpWriteDump(process.Handle, (uint)pid, file.SafeFileHandle.DangerousGetHandle(), \
             kind, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero) ? 0 : Marshal.GetLastWin32Error();
        }
    }
    public static int WriteOfASnapshot(int pid, string path, int kind) {
        var process = System.Diagnostics.Process.GetProcessById(pid);
        IntPtr snapshot;
        int captured = PssCaptureSnapshot(process.Handle, 0xAC0003BDu, 0x0010001Fu, out snapshot);
        if (captured != 0) return captured;
        Asked asked = ItIsASnapshot;
        IntPtr callback = Marshal.AllocHGlobal(16);
        try {
            Marshal.WriteIntPtr(callback, 0, Marshal.GetFunctionPointerForDelegate(asked));
            Marshal.WriteIntPtr(callback, 8, IntPtr.Zero);
            using (var file = new FileStream(path, FileMode.Create, FileAccess.ReadWrite, FileShare.None)) {
                return MiniDumpWriteDump(snapshot, (uint)pid, file.SafeFileHandle.DangerousGetHandle(), \
                 kind, IntPtr.Zero, IntPtr.Zero, callback) ? 0 : Marshal.GetLastWin32Error();
            }
        } finally {
            Marshal.FreeHGlobal(callback);
            PssFreeSnapshot(GetCurrentProcess(), snapshot);
            GC.KeepAlive(asked);
        }
    }
}
'@
exit [PggDump]::PGG_WRITE(PGG_PID, 'PGG_PATH', PGG_KIND)";

/// `MINIDUMP_TYPE` (minidumpapiset.h): `MiniDumpWithFullMemory` |
/// `MiniDumpWithHandleData` | `MiniDumpWithThreadInfo`.
#[cfg(windows)]
const DUMP_KIND: u32 = 0x2 | 0x4 | 0x1000;

/// A minidump of the process beside its pictures ([`DUMP_SCRIPT`]).
/// `whose` names the process in the line this answers.
#[cfg(windows)]
fn dump_of(pid: u32, whose: &str, shot_dir: &Path, after: After) -> String {
    let path = shot_dir.join(DUMP_FILE);
    let script = DUMP_SCRIPT
        .replace("PGG_PID", &pid.to_string())
        .replace("PGG_PATH", &path.to_string_lossy())
        .replace("PGG_KIND", &DUMP_KIND.to_string())
        .replace(
            "PGG_WRITE",
            match after {
                After::Reaped => "Write",
                After::GoesOn => "WriteOfASnapshot",
            },
        );
    let mut command = Command::new("powershell");
    command.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
    match bounded("the dump", command, DUMP_CEILING, None) {
        Answer::Ended { status, .. } if status.success() => match std::fs::metadata(&path) {
            Ok(meta) => format!(
                "  a dump of {whose}: {} ({} MB) — WinDbg: `!analyze -v`, then `~*k` for the thread that stands",
                path.display(),
                meta.len() / (1024 * 1024)
            ),
            Err(error) => format!(
                "  no dump: the dumper answered but left nothing at {} ({error})",
                path.display()
            ),
        },
        // The Win32 error of the call, or PowerShell's own one where the
        // type never compiled.
        Answer::Ended { status, .. } => {
            format!("  no dump: MiniDumpWriteDump exited {status}")
        }
        // A half-written file by that name would be opened as a dump.
        Answer::OutOfTime { pid, after, ended } => format!(
            "  no dump: the dumper ran out of time after {:.1}s and {}; {}",
            after.as_secs_f32(),
            match ended {
                Ok(()) => "was ended".to_string(),
                Err(error) =>
                    format!("could not be ended ({error}) — pid {pid} may still be running"),
            },
            match std::fs::remove_file(&path) {
                Ok(()) => "what it had written was removed".to_string(),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound =>
                    "it had written nothing".to_string(),
                Err(error) =>
                    format!("what it had written could not be removed ({error}) and is not a dump"),
            }
        ),
        Answer::Unstarted(why) => format!("  no dump: {why}"),
    }
}

/// Nothing on the other systems: a core needs a debugger the container
/// does not carry.
#[cfg(not(windows))]
fn dump_of(_pid: u32, _whose: &str, _shot_dir: &Path, _after: After) -> String {
    "  no dump on this system: the threads above are the whole of the look".to_string()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Answer, Threads, bounded, listing, stand_in};
    use crate::yard::Yard;

    /// The main thread is found by name, else the first listed, and its
    /// stack cut to a line's worth of frames.
    #[test]
    fn a_listing_carries_each_threads_stack_and_shows_the_mains() {
        let said = "7 Wait UserRequest main@app.exe cpu=1ms\n\
                    9 Wait EventPairLow -@ntdll.dll cpu=0ms\n\
                    stack 9 ntdll!NtWaitForWorkViaWorkerFactory+0x14\n\
                    stack 7 ntdll!NtDelayExecution+0x14 <- KERNELBASE!SleepEx+0x9e <- app+0x1234\n";
        let threads = listing(Answer::Ended {
            status: exit_status(0),
            stdout: said.to_string(),
        })
        .expect("a listing with stacks");
        assert_eq!(threads.lines.len(), 2);
        assert_eq!(threads.stacks.len(), 2);
        assert_eq!(
            threads.main_stack().as_deref(),
            Some("ntdll!NtDelayExecution+0x14 <- KERNELBASE!SleepEx+0x9e <- app+0x1234")
        );

        let unnamed = Threads {
            lines: vec!["3 Wait - -@x.exe cpu=0ms".to_string()],
            stacks: vec![(
                "3".to_string(),
                (1..=14)
                    .map(|n| format!("f{n}"))
                    .collect::<Vec<_>>()
                    .join(" <- "),
            )],
        };
        let shown = unnamed.main_stack().expect("the first thread's stack");
        assert!(shown.starts_with("f1 <- f2"), "{shown}");
        assert!(shown.ends_with("f12 <- …"), "{shown}");
        let unwalked = Threads {
            lines: vec!["3 S main wchan=0".to_string()],
            stacks: Vec::new(),
        };
        assert_eq!(unwalked.main_stack(), None);
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

    /// Whether it is gone is asked of the machine itself.
    #[test]
    fn a_diagnostic_that_stalls_is_ended_at_its_ceiling() {
        let ceiling = Duration::from_millis(300);

        let answer = bounded("a stalled listing", stand_in(), ceiling, None);

        let Answer::OutOfTime { pid, after, ended } = answer else {
            panic!("the stand-in answered: {answer:?}");
        };
        assert_eq!(ended, Ok(()), "ended and waited for");
        assert!(after >= ceiling, "{after:?}");
        assert!(
            !crate::subprocess::process_exists(pid),
            "pid {pid} still stands"
        );
    }

    /// What it wrote is the answer and stays on the disk; the listing is
    /// read off exactly that.
    #[test]
    fn a_diagnostic_that_answers_is_carried_whole() {
        let dir = lanes("answers");
        let said = dir.join("said.txt");

        let answer = bounded("an echo", says_one(), Duration::from_secs(60), Some(&said));

        let Answer::Ended { status, stdout } = answer else {
            panic!("the echo did not end: {answer:?}");
        };
        assert!(status.success(), "{status}");
        assert_eq!(stdout.trim(), "one");
        assert_eq!(
            std::fs::read_to_string(&said)
                .expect("what it said, on the disk")
                .trim(),
            "one"
        );
        assert_eq!(
            listing(Answer::Ended { status, stdout }),
            Ok(Threads {
                lines: vec!["one".to_string()],
                stacks: Vec::new(),
            })
        );
    }

    /// `wedge-check` reads these words back (`super::super::faults`): a
    /// change here is a change there.
    #[test]
    fn a_listing_out_of_time_says_so_and_whether_it_was_ended() {
        let ended = listing(Answer::OutOfTime {
            pid: 7,
            after: Duration::from_secs(15),
            ended: Ok(()),
        })
        .expect_err("out of time is no listing");
        assert_eq!(
            ended,
            "the listing ran out of time after 15.0s and was ended"
        );
        let standing = listing(Answer::OutOfTime {
            pid: 7,
            after: Duration::from_secs(15),
            ended: Err("refused".to_string()),
        })
        .expect_err("out of time is no listing");
        assert!(
            standing.contains("could not be ended (refused)"),
            "{standing}"
        );
        assert!(
            standing.contains("pid 7 may still be running"),
            "{standing}"
        );
    }

    /// The dump is one a person can open: the memory is behind the header
    /// and the file is its owner's. Taken of the stand-in, which is small
    /// and stands still.
    #[cfg(windows)]
    #[test]
    fn a_dump_of_a_standing_process_is_written_and_readable() {
        let dir = lanes("dump");
        let mut standing = stand_in()
            .stdout(std::process::Stdio::null())
            .spawn()
            .expect("a process to dump");

        let line = super::dump_of(standing.id(), "the app", &dir, super::After::Reaped);

        // Ended before anything is asserted, so a red leaves nothing
        // standing.
        standing
            .kill()
            .and_then(|()| standing.wait())
            .expect("the stand-in ended");
        assert!(line.starts_with("  a dump of the app:"), "{line}");
        let dump = std::fs::File::options()
            .read(true)
            .write(true)
            .open(dir.join(super::DUMP_FILE))
            .expect("the dump opens for its owner");
        let len = dump.metadata().expect("the dump's size").len();
        // Full memory is orders past a header with the stacks alone.
        assert!(len > 1024 * 1024, "{len} bytes is a header alone");
        drop(dump);
        std::fs::remove_dir_all(&dir).expect("the dump is its owner's to remove");
    }

    /// A process that goes on running is dumped off a snapshot: the same
    /// kind of file, and the process is still there to be ended by the
    /// test and by nothing before it.
    #[cfg(windows)]
    #[test]
    fn a_process_that_goes_on_is_dumped_off_a_snapshot_and_listed_without_a_walk() {
        let dir = lanes("dump-snapshot");
        let mut standing = stand_in()
            .stdout(std::process::Stdio::null())
            .spawn()
            .expect("a process to look into");

        let lines = super::look_into(standing.id(), "the stand-in", &dir);

        let alive = matches!(standing.try_wait(), Ok(None));
        standing
            .kill()
            .and_then(|()| standing.wait())
            .expect("the stand-in ended");
        assert!(alive, "the look ended the process it was to leave running");
        assert!(
            lines
                .iter()
                .any(|line| line.starts_with("  a dump of the stand-in:")),
            "{lines:?}"
        );
        assert!(
            !lines.iter().any(|line| line.contains("stands in:")),
            "a stack was walked of a process that goes on: {lines:?}"
        );
        let dump = std::fs::read(dir.join(super::DUMP_FILE)).expect("the dump");
        assert_eq!(&dump[..4], b"MDMP", "not a minidump");
        assert!(
            dump.len() > 1024 * 1024,
            "{} bytes is a header alone",
            dump.len()
        );
        let listed = std::fs::read_to_string(dir.join(super::THREADS_FILE)).expect("the listing");
        assert!(
            !listed.lines().any(|line| line.starts_with("stack ")),
            "{listed}"
        );
        std::fs::remove_dir_all(&dir).expect("the dump is its owner's to remove");
    }

    /// Every thread carries its description and start module; the
    /// stand-in's main thread starts in its own image.
    #[cfg(windows)]
    #[test]
    fn a_listing_names_the_threads_of_a_standing_process() {
        let dir = lanes("listing");
        let mut standing = stand_in()
            .stdout(std::process::Stdio::null())
            .spawn()
            .expect("a process to list");

        let listed = super::threads_of(standing.id(), &dir, super::After::Reaped);

        standing
            .kill()
            .and_then(|()| standing.wait())
            .expect("the stand-in ended");
        let threads = listed.expect("the stand-in's threads");
        assert!(!threads.lines.is_empty());
        for line in &threads.lines {
            assert!(line.contains('@') && line.contains(" cpu="), "{line}");
        }
        assert!(
            threads.lines.iter().any(|line| line.contains("@ping.exe")),
            "{threads:?}"
        );
        // Every thread was walked, and ping's main thread stands in the
        // system's sleep, named with no symbols on the machine.
        assert_eq!(threads.stacks.len(), threads.lines.len(), "{threads:?}");
        let main = threads
            .main_stack()
            .expect("the stand-in's main thread's stack");
        assert!(
            main.contains("ntdll") || main.contains("KERNELBASE") || main.contains("kernel32"),
            "{main}"
        );
        // The file holds the same lines the answer was read off.
        let written = std::fs::read_to_string(dir.join(super::THREADS_FILE))
            .expect("the listing, on the disk");
        assert_eq!(
            written
                .lines()
                .filter(|line| !line.trim().is_empty() && !line.starts_with("stack "))
                .count(),
            threads.lines.len()
        );
    }

    #[cfg(windows)]
    fn says_one() -> std::process::Command {
        let mut command = std::process::Command::new("cmd");
        command.args(["/c", "echo", "one"]);
        command
    }

    #[cfg(not(windows))]
    fn says_one() -> std::process::Command {
        let mut command = std::process::Command::new("sh");
        command.args(["-c", "echo one"]);
        command
    }

    /// A child that inherited the diagnostic's stdout and outlives it
    /// does not hold the answer. It writes `done.txt` as its last act, so
    /// whether the answer waited for it is read off the disk.
    ///
    /// This tree outlives the test: the child holds `said.txt` inside it,
    /// so Windows refuses the yard's removal ([`crate::yard::Yard`]'s `Drop`).
    #[test]
    fn a_child_the_diagnostic_leaves_behind_does_not_hold_the_answer() {
        let dir = lanes("left-behind");
        let done = dir.join("done.txt");

        let answer = bounded(
            "a listing that leaves a child behind",
            leaves_a_child_behind(&dir),
            Duration::from_secs(60),
            Some(&dir.join("said.txt")),
        );

        let Answer::Ended { status, .. } = answer else {
            panic!("the diagnostic did not end on its own: {answer:?}");
        };
        assert!(status.success(), "{status}");
        assert!(
            !done.exists(),
            "the answer waited for the child the diagnostic left behind"
        );
    }

    /// A process that ends at once and leaves a child holding its stdout
    /// for twenty seconds, then writing `done.txt`. The script is named by
    /// its whole path: with `NoDefaultCurrentDirectoryInExePath` set, `cmd`
    /// does not look in the current directory.
    #[cfg(windows)]
    fn leaves_a_child_behind(dir: &std::path::Path) -> std::process::Command {
        let script = dir.join("child.cmd");
        std::fs::write(
            &script,
            "@echo off\r\nstart /b cmd /c \"ping -n 20 127.0.0.1 >nul & echo x > done.txt\"\r\n",
        )
        .expect("a script that leaves a child behind");
        let mut command = std::process::Command::new("cmd");
        command.arg("/c").arg(script).current_dir(dir);
        command
    }

    #[cfg(not(windows))]
    fn leaves_a_child_behind(dir: &std::path::Path) -> std::process::Command {
        let script = dir.join("child.sh");
        std::fs::write(&script, "(sleep 20; echo x > done.txt) &\n")
            .expect("a script that leaves a child behind");
        let mut command = std::process::Command::new("sh");
        command.arg(script).current_dir(dir);
        command
    }

    /// A lanes directory of this test's own, gone when the test is.
    fn lanes(name: &str) -> Yard {
        Yard::new(&format!("look-{name}"))
    }
}
