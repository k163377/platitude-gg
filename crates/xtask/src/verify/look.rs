//! The look at a process that is still standing when the parent's
//! ceiling arrives: its threads, and on Windows a dump of it, each taken
//! by a diagnostic process of its own under a ceiling of its own
//! ([`bounded`]). What the record a stopped run leaves cannot say
//! (`super::wedge`): past `exiting` nothing inside the process is left
//! to say it.
//!
//! **A diagnostic is bounded, and its stdout is a file.** It is ended at
//! its ceiling and waited for, so the app's reaping never runs beside a
//! dumper still holding the process open; and what it wrote is read off
//! the disk once it has exited, so a child it left behind — holding what
//! would have been the pipe's write end — holds nothing here.

use std::fs::File;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::time::Duration;

use crate::wait::{Budget, LOOK_AGAIN, Wait};

#[cfg(windows)]
const DUMP_FILE: &str = "app.dmp";
/// The thread listing as the parent read it, whole, beside the pictures:
/// the line under the verdict shows the first few threads and counts the
/// rest ([`listed`]). Written by the listing itself, which is a process
/// on Windows ([`threads_of`]) and the stand-in a stalled look is made
/// of everywhere ([`look_at`]); the walk of `/proc` leaves none.
const THREADS_FILE: &str = "threads.txt";

/// How long a look at the app may take: the listing is one PowerShell
/// start and one query, the dump writes the process's whole memory to
/// disk. Both Windows-only — elsewhere the listing is a walk of `/proc`,
/// which takes no process and so has nothing to bound. **Ceilings on the
/// diagnostics, never on the run**: what they bound is a diagnostic that
/// stalls, and the answer to one is the line that says it did — the app
/// is reaped and the run reported the same either way ([`bounded`]).
#[cfg(windows)]
const LISTING_CEILING: Duration = Duration::from_secs(15);
#[cfg(windows)]
const DUMP_CEILING: Duration = Duration::from_secs(60);
/// The ceiling a look ordered to stall is ended at (`--fault-stall-look`).
/// The stand-in never answers, so any height ends it, and what the case
/// reads back is the ending and the words — never the height. Kept apart
/// from [`LISTING_CEILING`], which is set for a listing that does answer,
/// on a loaded machine; this one is only wall clock the check would pay
/// for nothing.
const STALLED_LOOK_CEILING: Duration = Duration::from_secs(1);

/// What only a look at the process can say once the trail has run out,
/// taken while the app still stands. Past `exiting` the exit has ended
/// every thread but the one it runs on before the loaded libraries are
/// given their detach, so **one thread alive is a process inside its
/// exit** and several is one that never got there. And on Windows a dump
/// of it, for the frame that stands still — unless the stop was ordered
/// (`--fault-hang`), whose cause needs no dump.
///
/// **Every look is bounded and ended at its ceiling** ([`bounded`]): a
/// diagnostic that stalls — a PowerShell that never gets past its start,
/// a dumper waiting on the process it dumps — is ended, said to have
/// been, and the run goes on to reap the app and report it. `stalled`
/// orders that stall (`--fault-stall-look`): the listing is a process
/// that never answers, ended at a ceiling of its own
/// ([`STALLED_LOOK_CEILING`]), so what the parent says about a look that
/// ran out of time can be checked rather than waited for
/// (`super::faults`).
pub(super) fn look_at(pid: u32, shot_dir: &Path, unordered: bool, stalled: bool) -> Vec<String> {
    let threads = if stalled {
        listing(bounded(
            "the thread listing",
            stand_in(),
            STALLED_LOOK_CEILING,
            Some(&shot_dir.join(THREADS_FILE)),
        ))
    } else {
        threads_of(pid, shot_dir)
    };
    let mut lines = vec![match threads {
        Ok(threads) => format!(
            "  threads at the ceiling: {} alive — {}",
            threads.len(),
            listed(&threads)
        ),
        Err(why) => format!("  threads at the ceiling: could not be listed — {why}"),
    }];
    if unordered {
        lines.push(dump_of(pid, shot_dir));
    }
    lines
}

/// What a bounded diagnostic came back with.
#[derive(Debug)]
enum Answer {
    /// It ended on its own: how, and what it had written by then to the
    /// file it was given for stdout — nothing where it was given none.
    Ended { status: ExitStatus, stdout: String },
    /// It stood past its ceiling and was ended here, after this long —
    /// or could not be, which is said rather than assumed, with the pid
    /// that may then still be running.
    OutOfTime {
        pid: u32,
        after: Duration,
        ended: Result<(), String>,
    },
    /// It could not be started, or asked after.
    Unstarted(String),
}

/// Runs `command` to its end or to `ceiling`, whichever comes first, and
/// says which. A diagnostic past its ceiling is ended and waited for
/// before this answers, so what comes after it — the reaping of the app
/// — never runs beside a dumper still holding the process open.
///
/// **Stdout goes to a file, never a pipe.** A pipe is read to its end,
/// and its end is every write handle closed — a child of the diagnostic
/// that inherited its stdout and outlives it holds one, and would hold
/// the answer for as long as it lives (the test that leaves such a child
/// behind waited its whole twenty seconds on the pipe). A file has no
/// other end: what the diagnostic wrote is read off the disk once it has
/// exited, and whatever still holds the file holds nothing here. `said`
/// is that file; `None` for a diagnostic whose words are not wanted.
fn bounded(what: &str, mut command: Command, ceiling: Duration, said: Option<&Path>) -> Answer {
    let stdout = match said.map(File::create) {
        None => Stdio::null(),
        Some(Ok(file)) => Stdio::from(file),
        Some(Err(error)) => {
            return Answer::Unstarted(format!(
                "{what} could not be given a file for what it says: {error}"
            ));
        }
    };
    command
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(Stdio::null());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => return Answer::Unstarted(format!("{what} could not be started: {error}")),
    };
    let mut wait = Wait::new(what, Budget::whole(ceiling), LOOK_AGAIN);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = said
                    .and_then(|path| std::fs::read(path).ok())
                    .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                    .unwrap_or_default();
                return Answer::Ended { status, stdout };
            }
            Ok(None) => {}
            Err(error) => {
                return Answer::Unstarted(format!("{what} could not be asked after: {error}"));
            }
        }
        if wait.look_again("its exit").is_err() {
            // Asking it to end is only asking; the wait is what says the process is
            // gone, and either failing is carried out as the answer.
            let ended = child
                .kill()
                .and_then(|()| child.wait())
                .map(|_| ())
                .map_err(|error| error.to_string());
            return Answer::OutOfTime {
                pid: child.id(),
                after: wait.elapsed(),
                ended,
            };
        }
    }
}

/// The threads a listing answered with, or why it could not answer —
/// worded for the line under the verdict, and the same words whichever
/// system's listing it was.
fn listing(answer: Answer) -> Result<Vec<String>, String> {
    match answer {
        Answer::Ended { status, stdout } if status.success() => {
            let threads: Vec<String> = stdout
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_string)
                .collect();
            if threads.is_empty() {
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

/// A process that stands in for a diagnostic that stalls: one that does
/// nothing for longer than any ceiling here, so that the ceiling is what
/// ends it. What every system has on hand — on Windows `ping`, counting
/// off seconds against the loopback, since there is no `sleep`.
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

/// The listing, from a PowerShell of its own: every thread's number,
/// its state, what it waits on, its description — what Rust and Qt name
/// their threads by: `tokio-rt-worker`, `QSGRenderThread`,
/// `pgg-deadline` — the module it started in, which names the system's
/// own threads (`ntdll.dll` for the thread pool, `WINMM.dll` for a
/// timer), and the processor time it has had. A survivor of the
/// teardown is told from the system's idle threads by the two in the
/// middle, which `Get-Process` does not carry: each is one Win32 call,
/// made from C# the way the dump is ([`DUMP_SCRIPT`]). A thread that
/// could not be opened is listed by its number alone, as `-@?`.
#[cfg(windows)]
const LISTING_SCRIPT: &str = "Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class PggThreads {
    [DllImport(\"kernel32.dll\", SetLastError = true)] static extern IntPtr OpenThread(uint access, bool inherit, uint tid);
    [DllImport(\"kernel32.dll\")] static extern bool CloseHandle(IntPtr handle);
    [DllImport(\"kernel32.dll\", SetLastError = true)] static extern int GetThreadDescription(IntPtr handle, out IntPtr description);
    [DllImport(\"kernel32.dll\")] static extern IntPtr LocalFree(IntPtr memory);
    [DllImport(\"ntdll.dll\")] static extern int NtQueryInformationThread(IntPtr handle, int kind, out IntPtr info, int length, IntPtr returned);
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
}";

/// Every thread of the process ([`LISTING_SCRIPT`]). Windows has no way
/// to this in std, and the one it has is the same PowerShell the reaper
/// reads its process tree from (`crate::reap`). The listing is written
/// whole beside the pictures ([`THREADS_FILE`]) and read back from there.
#[cfg(windows)]
fn threads_of(pid: u32, shot_dir: &Path) -> Result<Vec<String>, String> {
    let script = LISTING_SCRIPT.replace("PGG_PID", &pid.to_string());
    let mut command = Command::new("powershell");
    command.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
    listing(bounded(
        "the thread listing",
        command,
        LISTING_CEILING,
        Some(&shot_dir.join(THREADS_FILE)),
    ))
}

/// The same off `/proc`: the thread's name is there as well, which says
/// whose it is — a tokio worker, the deadline thread, a render thread.
/// A walk, not a process: it leaves no file beside the pictures.
#[cfg(unix)]
fn threads_of(pid: u32, _shot_dir: &Path) -> Result<Vec<String>, String> {
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
    Ok(threads)
}

/// The dump, written by `MiniDumpWriteDump` itself from a PowerShell of
/// its own — a process, so the ceiling can end it ([`bounded`]). Full
/// memory, the handles and the thread information ([`DUMP_KIND`]): what
/// a stopped exit is read off is the one thread's stack, `~*k` in
/// WinDbg, and what it holds is in the memory behind it.
///
/// **Not `rundll32 comsvcs.dll,MiniDump`**, the dumper every Windows
/// has on hand. What it writes here is a file whose header claims the
/// whole memory and holds none of it, under an ACL that names SYSTEM
/// and Administrators alone — a dump its own taker cannot open
/// (internal-docs/P3-確認事項.md §次の 1 回で何が読めるか). A file
/// PowerShell creates is the owner's, like any other.
///
/// `PGG_PID` and `PGG_PATH` are filled in by [`dump_of`]; the path is
/// inside a single-quoted literal, which a temp path has no quote to
/// break. The process exits with the Win32 error of the call, zero for
/// a dump written.
#[cfg(windows)]
const DUMP_SCRIPT: &str = "Add-Type -TypeDefinition @'
using System;
using System.IO;
using System.Runtime.InteropServices;
public static class PggDump {
    [DllImport(\"dbghelp.dll\", SetLastError = true)]
    static extern bool MiniDumpWriteDump(IntPtr process, uint pid, IntPtr file, int kind, \
     IntPtr exception, IntPtr user, IntPtr callback);
    public static int Write(int pid, string path, int kind) {
        var process = System.Diagnostics.Process.GetProcessById(pid);
        using (var file = new FileStream(path, FileMode.Create, FileAccess.ReadWrite, FileShare.None)) {
            return MiniDumpWriteDump(process.Handle, (uint)pid, file.SafeFileHandle.DangerousGetHandle(), \
             kind, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero) ? 0 : Marshal.GetLastWin32Error();
        }
    }
}
'@
exit [PggDump]::Write(PGG_PID, 'PGG_PATH', PGG_KIND)";

/// `MINIDUMP_TYPE` (minidumpapiset.h): `MiniDumpWithFullMemory` |
/// `MiniDumpWithHandleData` | `MiniDumpWithThreadInfo`.
#[cfg(windows)]
const DUMP_KIND: u32 = 0x2 | 0x4 | 0x1000;

/// A minidump of the process beside its pictures ([`DUMP_SCRIPT`]).
#[cfg(windows)]
fn dump_of(pid: u32, shot_dir: &Path) -> String {
    let path = shot_dir.join(DUMP_FILE);
    let script = DUMP_SCRIPT
        .replace("PGG_PID", &pid.to_string())
        .replace("PGG_PATH", &path.to_string_lossy())
        .replace("PGG_KIND", &DUMP_KIND.to_string());
    let mut command = Command::new("powershell");
    command.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
    match bounded("the dump", command, DUMP_CEILING, None) {
        Answer::Ended { status, .. } if status.success() => match std::fs::metadata(&path) {
            Ok(meta) => format!(
                "  a dump of the app: {} ({} MB) — WinDbg: `!analyze -v`, then `~*k` for the thread that stands",
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
        // What a dumper ended half way left is not a dump, and a file by
        // that name would be opened as one.
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
/// does not carry, and the thread list above names what waits where.
#[cfg(not(windows))]
fn dump_of(_pid: u32, _shot_dir: &Path) -> String {
    "  no dump on this system: the threads above are the whole of the look".to_string()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use super::{Answer, bounded, listing, stand_in};

    /// A diagnostic that stands past its ceiling is ended there and said
    /// to have been. Whether it is gone is asked of the machine, not of
    /// the answer.
    #[test]
    fn a_diagnostic_that_stalls_is_ended_at_its_ceiling() {
        // The stand-in stalls on purpose and the ceiling is what ends it;
        // the verdict is that it was ended, never how long that took.
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

    /// A diagnostic that ends on its own answers with its exit and what
    /// it wrote, whole — the listing is read off exactly that, and the
    /// file it was written to stays beside the pictures.
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
            Ok(vec!["one".to_string()])
        );
    }

    /// The words a stalled listing is reported by, which `wedge-check`
    /// reads back from the whole run (`super::super::faults`): a change
    /// here is a change there.
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

    /// The dump is one a person can open: written by the call itself, so
    /// the memory is behind the header and the file is its owner's to
    /// read — neither of which `comsvcs.dll,MiniDump` gives (`dump_of`).
    /// Taken of the stand-in, which is small and stands still.
    #[cfg(windows)]
    #[test]
    fn a_dump_of_a_standing_process_is_written_and_readable() {
        let dir = lanes("dump");
        let mut standing = stand_in()
            .stdout(std::process::Stdio::null())
            .spawn()
            .expect("a process to dump");

        let line = super::dump_of(standing.id(), &dir);

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
        // Full memory is the images and the heap, which is orders past a
        // header with the stacks alone.
        assert!(len > 1024 * 1024, "{len} bytes is a header, not a dump");
        drop(dump);
        std::fs::remove_dir_all(&dir).expect("the dump is its owner's to remove");
    }

    /// The listing names what the numbers alone could not — the thread's
    /// description and the module it started in — which is how a
    /// survivor of the teardown is told from the system's idle threads.
    /// The stand-in's main thread starts in its own image.
    #[cfg(windows)]
    #[test]
    fn a_listing_names_the_threads_of_a_standing_process() {
        let dir = lanes("listing");
        let mut standing = stand_in()
            .stdout(std::process::Stdio::null())
            .spawn()
            .expect("a process to list");

        let listed = super::threads_of(standing.id(), &dir);

        standing
            .kill()
            .and_then(|()| standing.wait())
            .expect("the stand-in ended");
        let threads = listed.expect("the stand-in's threads");
        assert!(!threads.is_empty());
        for line in &threads {
            assert!(line.contains('@') && line.contains(" cpu="), "{line}");
        }
        assert!(
            threads.iter().any(|line| line.contains("@ping.exe")),
            "{threads:?}"
        );
        // The whole listing is on the disk beside the pictures, the same
        // lines the answer was read off.
        let written = std::fs::read_to_string(dir.join(super::THREADS_FILE))
            .expect("the listing, on the disk");
        assert_eq!(
            written
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count(),
            threads.len()
        );
    }

    /// A process that says `one` and ends.
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

    /// A diagnostic can leave a child of its own behind — one that
    /// inherited its stdout and outlives it. The answer is the
    /// diagnostic's own exit and what it had written by then; what it
    /// left behind holds nothing here. The child left behind writes
    /// `done.txt` as its last act, so whether the answer waited for it
    /// is read off the disk and not off a clock.
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

    /// A process that ends at once and leaves a child behind, holding
    /// the stdout it was given for twenty seconds and then writing
    /// `done.txt` beside the script. The script is a file so that the
    /// shell's own quoting never enters the picture, and is named by its
    /// whole path: a machine with `NoDefaultCurrentDirectoryInExePath`
    /// set has a `cmd` that does not look in the current directory.
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

    /// A lanes directory of this test's own.
    fn lanes(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pgg-wedge-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a directory to hold lanes in");
        dir
    }
}
