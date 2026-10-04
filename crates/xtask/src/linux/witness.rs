//! What stands on this host when the container engine gives no answer —
//! or refuses a listing, which an engine in order does not — kept before
//! anybody does anything about it. Ending the session, its process or
//! the machine changes what is there, and an engine that answers again
//! has nothing left to show.
//!
//! Taken without an administrator, so from what a user may read: the
//! engine's processes and every CLI still waiting on it, the session
//! process's threads and its memory (`verify::look_into`, which suspends
//! no thread of it — every checkout on the machine shares that process),
//! the host's memory, and when the session's disks were last attached and
//! detached. The service's dump and the engine's own trace need an
//! administrator and are a person's to add; the directory says how.
//!
//! **Once per session process and VM.** Every line that meets the same
//! silence would otherwise take the same picture, each later than the
//! first and further from what it is a picture of; they are pointed at
//! the first instead. The session process outlives its VMs, so the VM
//! standing is part of what a witness is of: the next VM's silence is
//! another one. A look asked for by hand (`linux witness`) is always
//! taken.
//!
//! **Under a directory of its own** ([`BASE`]), not the one container
//! runs leave their pictures in: that one is mounted into the gate's
//! container, and a process's memory is nothing to hand to what is built
//! in there.
//!
//! Windows only: the session process is wslc's. Elsewhere nothing is
//! taken and nothing is said.
//!
//! Nothing here asks the engine for work: the one thing put to its CLI
//! is the session listing, which the service answers without the VM.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::subprocess::{Answer, bounded, bounded_both_streams};

/// The engine's session process: the one of the engine's a user may open.
const SESSION: &str = "wslcsession.exe";

/// The directory under the system temp the witnesses stand in. No sweep
/// reads it (`verify::sweep_yesterdays_runs`): what bounds it is
/// [`forget_all_but_the_newest`].
const BASE: &str = "pgg-engine";

/// How many witnesses of each kind — taken by a failing line, taken by
/// hand — stand at once: each holds a process's memory.
const KEPT: usize = 3;

/// The file a witness is whole by: written last. A directory without it
/// is being taken, or was left by a taker that did not finish.
const WHOLE: &str = "README.txt";

/// The pid of the process taking a witness, written first.
const TAKER: &str = "taker.txt";

/// How long one of the host's own listings may take.
const ASKING_CEILING: Duration = Duration::from_secs(30);

/// One line per process: `name|pid|parent|started (UTC)|working set
/// MB|threads|command line`, the command line's own line breaks folded
/// to spaces. A start the caller may not read is empty.
const PROCESSES_SCRIPT: &str = "[Console]::OutputEncoding = [Text.Encoding]::UTF8
Get-CimInstance Win32_Process | Where-Object { $_.Name -match \
 '^(wslc|wslcsession|wslservice|wslrelay|wslhost|wsl|vmwp|vmcompute)\\.exe$' -or $_.Name -like 'vmmem*' } | \
 Sort-Object ProcessId | ForEach-Object {
    $started = ''
    if ($_.CreationDate) { $started = $_.CreationDate.ToUniversalTime().ToString('yyyyMMddHHmmss') }
    '{0}|{1}|{2}|{3}|{4}|{5}|{6}' -f $_.Name, $_.ProcessId, $_.ParentProcessId, $started, \
 [int]($_.WorkingSetSize / 1MB), $_.ThreadCount, ($_.CommandLine -replace '\\s+', ' ')
}
exit 0";

/// The host's memory, and the session's disks as the VHD driver last saw
/// them. The driver's messages are in the machine's language, so only
/// what does not translate is kept: the event, the file, the VM. A log
/// that holds no such event, or will not be read, is not this listing's
/// failure: it ends zero either way.
const HOST_SCRIPT: &str = "[Console]::OutputEncoding = [Text.Encoding]::UTF8
$os = Get-CimInstance Win32_OperatingSystem
'memory free {0} MB of {1} MB, commit free {2} MB of {3} MB, up since {4:u}' -f \
 [int]($os.FreePhysicalMemory / 1KB), [int]($os.TotalVisibleMemorySize / 1KB), \
 [int]($os.FreeVirtualMemory / 1KB), [int]($os.TotalVirtualMemorySize / 1KB), \
 $os.LastBootUpTime.ToUniversalTime()
'the session disks, newest first (UTC) - 50 created, 12 opened by the VM named, 17 attached, 18 detached:'
Get-WinEvent -FilterHashtable @{ LogName = 'Microsoft-Windows-VHDMP-Operational'; Id = 12, 17, 18, 50 } \
 -MaxEvents 600 -ErrorAction SilentlyContinue | Where-Object { $_.Message -match 'wslc' } | \
 Select-Object -First 60 | ForEach-Object {
    $disk = '?'; if ($_.Message -match '(storage|swap)\\.vhdx') { $disk = $Matches[0] }
    $vm = ''; if ($_.Message -match '\\{[0-9a-fA-F-]{36}\\}') { $vm = $Matches[0] }
    '{0:u} {1} {2} {3}' -f $_.TimeCreated.ToUniversalTime(), $_.Id, $disk, $vm
}
exit 0";

/// What a directory tells whoever opens it. `{LOOK}` is what the look
/// at the session process said of itself.
const README: &str =
    "What stood on this host when the container engine (wslc) gave no answer, or refused a listing.

account.txt     the command and how it ended, and when this was taken
processes.txt   the engine's processes and every wslc still waiting:
                name|pid|parent|started (UTC)|working set MB|threads|command line
host.txt        the host's memory, and when the session's disks were last attached and detached
sessions.txt    what `wslc system session list` said
wslcsession-*/  the session process: threads.txt (every thread, its state and what it waits on)
                and app.dmp (its memory, written off a snapshot - the stacks are in it)

{LOOK}

Not here, because each needs an administrator. They are worth the most BEFORE anything ends the
session - `wslc system session terminate`, ending wslcsession.exe, a reboot - and the first of
them needs nothing downloaded:

1. Task Manager > Details > Create memory dump file, of vmwp.exe (the VM's own process) and
   of wslservice.exe.
2. The engine's own trace, from an administrator's PowerShell, with the script of the installed
   version (`wsl --version`; https://github.com/microsoft/WSL, diagnostics/collect-wsl-logs.ps1
   at that tag): `collect-wsl-logs.ps1 -Dump`, left recording for a minute while one
   `wslc ps` waits, then stopped.

The dump holds the session process's whole memory, registry credentials included if it held
any: it is for this machine and for whoever is asked to read it. Put what an administrator adds
in a directory of its own: the witnesses here are held to the newest few of each kind.
";

/// The failure path's witness: where it is kept, as the tail of the
/// account a failing line carries — or nothing off Windows.
pub(super) fn beside(account: &str) -> String {
    if !cfg!(windows) {
        return String::new();
    }
    match take(account, false) {
        Ok(Taken::Now(dir, _)) => format!(
            " — what stood on this host is kept in {}: before anything ends the session \
             (`{} system session terminate`, ending {SESSION}, a reboot), its README says what \
             an administrator can still add",
            dir.display(),
            super::engine::NAME
        ),
        Ok(Taken::Earlier(dir)) => format!(
            " — what stood on this host when a line first met this is kept in {}",
            dir.display()
        ),
        Err(why) => format!(" — what stood on this host could not be kept ({why})"),
    }
}

/// `linux witness`: the same look, taken now whatever was taken before.
/// `line` is the verb and whatever was typed behind it, which is nothing;
/// `optioned` is whether an option of `linux`'s stood ahead of it, which
/// none applies to.
pub(super) fn by_hand(line: &[String], optioned: bool) -> Result<(), String> {
    if let Some(extra) = line.get(1) {
        return Err(format!("witness takes no arguments (got {extra:?})"));
    }
    if optioned {
        return Err("witness takes no options: it starts nothing and builds nothing".into());
    }
    if !cfg!(windows) {
        return Err(format!(
            "there is nothing to take here: the witness is of {SESSION}, which is wslc's"
        ));
    }
    let account = format!("taken by hand (`{}`)", super::WITNESS.line());
    match take(&account, true)? {
        Taken::Now(dir, look) => println!("kept in {}\n{look}", dir.display()),
        Taken::Earlier(dir) => println!("kept in {}", dir.display()),
    }
    Ok(())
}

enum Taken {
    /// Where, and what the look at the session process said of itself.
    Now(PathBuf, String),
    /// An earlier line took this one already, or is taking it.
    Earlier(PathBuf),
}

fn take(account: &str, by_hand: bool) -> Result<Taken, String> {
    let base = std::env::temp_dir().join(BASE);
    std::fs::create_dir_all(&base).map_err(|e| format!("{}: {e}", base.display()))?;
    let now = crate::note::now_secs();
    let asking = base.join(format!("asking-{}.txt", std::process::id()));
    let processes = said_by(powershell(PROCESSES_SCRIPT), &asking);
    let _ = std::fs::remove_file(&asking);
    let session = processes.as_deref().ok().and_then(session_of);
    let vm = processes
        .as_deref()
        .ok()
        .zip(session.as_ref())
        .and_then(|(listed, session)| vm_of(listed, session.pid));
    let dir = base.join(named(session.as_ref(), vm.as_deref(), now, by_hand));
    if !claimed(&dir)? {
        return Ok(Taken::Earlier(dir));
    }
    let wrote = |name: &str, text: &str| {
        let _ = std::fs::write(dir.join(name), text);
    };
    wrote(
        "account.txt",
        &format!(
            "{account}\n\ntaken at {} ({now} seconds since the epoch) by pid {} in {}\n",
            super::engine::clock(now),
            std::process::id(),
            crate::tree::workspace_root().display()
        ),
    );
    wrote(
        "processes.txt",
        &processes.unwrap_or_else(|why| format!("could not be listed: {why}\n")),
    );
    if let Err(why) = said_by(powershell(HOST_SCRIPT), &dir.join("host.txt")) {
        // Beside what the listing had written by then, never over it.
        wrote("host-ended.txt", &format!("{why}\n"));
    }
    let mut sessions = super::engine::command();
    sessions.args(["system", "session", "list"]);
    if let Answer::OutOfTime { after, .. } = bounded_both_streams(
        "the session listing",
        sessions,
        ASKING_CEILING,
        &dir.join("sessions.txt"),
    ) {
        wrote(
            "sessions.txt",
            &format!("no answer within {:.0}s\n", after.as_secs_f32()),
        );
    }
    let look = match &session {
        Some(session) => {
            let into = dir.join(format!("wslcsession-{}", session.pid));
            match std::fs::create_dir(&into) {
                Ok(()) => {
                    crate::verify::look_into(session.pid, "the engine's session process", &into)
                        .join("\n")
                }
                Err(error) => format!("  no look at {SESSION}: {}: {error}", into.display()),
            }
        }
        None => format!("  no {SESSION} was standing: nothing of it to take"),
    };
    wrote(WHOLE, &README.replace("{LOOK}", &look));
    forget_all_but_the_newest(&base);
    Ok(Taken::Now(dir, look))
}

/// Makes `dir` this process's to fill, and answers whether it is.
///
/// Making the directory is the claim: of the lines that meet one silence
/// together, one takes the witness and the rest are pointed at it. A
/// directory that is not whole ([`WHOLE`]) and whose taker ([`TAKER`]) is
/// no longer running was left half taken — a line ended in the middle —
/// and is taken again, since every later line would otherwise be pointed
/// at half a witness.
fn claimed(dir: &Path) -> Result<bool, String> {
    for _ in 0..2 {
        match std::fs::create_dir(dir) {
            Ok(()) => {
                let _ = std::fs::write(dir.join(TAKER), std::process::id().to_string());
                return Ok(true);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if !left_half_taken(dir) {
                    return Ok(false);
                }
                let _ = std::fs::remove_dir_all(dir);
            }
            Err(error) => return Err(format!("{}: {error}", dir.display())),
        }
    }
    Ok(false)
}

/// Whether `dir` is a witness nobody is taking any more and nobody
/// finished. One with no taker written yet is being claimed this moment.
fn left_half_taken(dir: &Path) -> bool {
    if dir.join(WHOLE).exists() {
        return false;
    }
    std::fs::read_to_string(dir.join(TAKER))
        .ok()
        .and_then(|pid| pid.trim().parse::<u32>().ok())
        .is_some_and(|taker| !crate::subprocess::process_exists(taker))
}

/// A PowerShell of its own for one of the listings above.
fn powershell(script: &str) -> Command {
    let mut command = Command::new("powershell");
    command.args(["-NoProfile", "-NonInteractive", "-Command", script]);
    command
}

/// What a listing wrote to `said`, or why there is nothing to read.
fn said_by(listing: Command, said: &Path) -> Result<String, String> {
    match bounded("a look at the host", listing, ASKING_CEILING, Some(said)) {
        Answer::Ended { status, stdout } if status.success() => Ok(stdout),
        Answer::Ended { status, .. } => Err(format!("the listing exited {status}")),
        Answer::OutOfTime { after, .. } => Err(format!(
            "the listing gave no answer within {:.0}s",
            after.as_secs_f32()
        )),
        Answer::Unstarted(why) => Err(why),
    }
}

/// The session process as [`PROCESSES_SCRIPT`] listed it.
#[derive(Debug, PartialEq)]
struct Session {
    pid: u32,
    /// `yyyyMMddHHmmss`, UTC — with the pid, what tells one session
    /// process from the next to carry its number.
    started: String,
}

/// The first session process a listing holds.
fn session_of(processes: &str) -> Option<Session> {
    processes.lines().find_map(|line| {
        let mut fields = line.split('|');
        (fields.next()? == SESSION).then_some(())?;
        let pid = fields.next()?.parse().ok()?;
        let started = fields.nth(1)?.to_string();
        Some(Session { pid, started })
    })
}

/// The VM the session `session` has up, by the first eight digits of the
/// id its relay was started with — or None with no VM up. The relay is
/// the session process's own child: WSL's distributions run one too,
/// under the service, for a VM that is not this engine's.
fn vm_of(processes: &str, session: u32) -> Option<String> {
    processes.lines().find_map(|line| {
        let mut fields = line.split('|');
        (fields.next()? == "wslrelay.exe").then_some(())?;
        (fields.nth(1)?.parse::<u32>().ok()? == session).then_some(())?;
        let (_, id) = line.split_once("--vm-id {")?;
        id.get(..8).map(str::to_string)
    })
}

/// The directory a witness is kept under: the session process and the
/// VM it is of, so every line meeting that silence names the same one.
/// With no VM up there is no instance to name it by, and the day stands
/// in; with no session process either, the hour. One taken by hand is
/// that moment's own.
fn named(session: Option<&Session>, vm: Option<&str>, now: u64, by_hand: bool) -> String {
    let of = match (session, vm) {
        (Some(session), Some(vm)) => format!("engine-{}-{}-{vm}", session.started, session.pid),
        (Some(session), None) => format!(
            "engine-{}-{}-novm-{}",
            session.started,
            session.pid,
            now / 86_400
        ),
        (None, _) => format!("engine-none-{}", now / 3600),
    };
    if by_hand {
        format!("{of}{BY_HAND}{now}")
    } else {
        of
    }
}

/// What a name carries when the witness was taken by hand.
const BY_HAND: &str = "-at-";

/// Takes away every witness but the newest [`KEPT`] of each kind. The
/// two kinds are held apart so that looks taken by hand, however many,
/// never push out the one a failing line took — the one nearest to
/// whatever it is a witness of.
fn forget_all_but_the_newest(base: &Path) {
    let Ok(entries) = std::fs::read_dir(base) else {
        return;
    };
    let mut witnesses: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .filter(|entry| {
            entry.file_name().to_string_lossy().starts_with("engine-") && entry.path().is_dir()
        })
        .filter_map(|entry| Some((entry.metadata().ok()?.modified().ok()?, entry.path())))
        .collect();
    witnesses.sort();
    let (by_hand, by_a_line): (Vec<_>, Vec<_>) = witnesses.into_iter().partition(|(_, dir)| {
        dir.file_name()
            .is_some_and(|name| name.to_string_lossy().contains(BY_HAND))
    });
    for kind in [by_hand, by_a_line] {
        let older = kind.len().saturating_sub(KEPT);
        for (_, dir) in kind.into_iter().take(older) {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Session, claimed, forget_all_but_the_newest, named, session_of, vm_of};

    /// Lines as the listing writes them: a process whose start and
    /// command line a user may not read, and the relay of a VM that is
    /// WSL's own and not this engine's.
    const PROCESSES: &str = "wslservice.exe|5704|1972||32|5|
wslc.exe|2748|41852|20261004023227|16|4|\"C:\\Program Files\\WSL\\wslc.exe\" image inspect x
wslrelay.exe|9100|5704|20261004040000|11|4|\"C:\\Program Files\\WSL\\wslrelay.exe\" --mode 1 --vm-id {aaaaaaaa-c6ab-462f-b77a-e9dc40bf9625}
wslrelay.exe|19640|29944|20261004050705|11|4|\"C:\\Program Files\\WSL\\wslrelay.exe\" --mode 2 --exit-event 2556 --port 3097672278 --vm-id {0e04b585-c6ab-462f-b77a-e9dc40bf9625}
wslcsession.exe|29944|2052|20261004035311|19|4|\"C:\\Program Files\\WSL\\wslcsession.exe\" -Embedding
vmmemwslc-cli-someone|41576|33780|20261004050704|976|59|
";

    #[test]
    fn the_session_process_is_read_off_the_listing_by_its_name() {
        assert_eq!(
            session_of(PROCESSES),
            Some(Session {
                pid: 29944,
                started: "20261004035311".to_string()
            })
        );
        assert_eq!(session_of("wslc.exe|2748|41852|2026|16|4|x\n"), None);
        assert_eq!(session_of(""), None);
    }

    /// The relay that names the VM is the session process's own child.
    #[test]
    fn the_vm_is_the_one_the_session_process_started_a_relay_for() {
        assert_eq!(vm_of(PROCESSES, 29944).as_deref(), Some("0e04b585"));
        assert_eq!(
            vm_of(PROCESSES, 5704).as_deref(),
            Some("aaaaaaaa"),
            "by the parent, not by which relay is listed first"
        );
        assert_eq!(vm_of(PROCESSES, 7), None);
        assert_eq!(
            vm_of("wslcsession.exe|29944|2052|2026|19|4|x\n", 29944),
            None
        );
    }

    /// Every line that meets one silence names one directory; the next
    /// VM of the same session process, or a look asked for by hand,
    /// another.
    #[test]
    fn a_witness_is_named_for_the_session_process_and_the_vm_it_is_of() {
        let session = session_of(PROCESSES);
        assert_eq!(
            named(session.as_ref(), Some("0e04b585"), 1_791_086_000, false),
            "engine-20261004035311-29944-0e04b585"
        );
        assert_eq!(
            named(session.as_ref(), Some("0e04b585"), 1_791_086_999, false),
            "engine-20261004035311-29944-0e04b585",
            "the moment is no part of the name"
        );
        assert_eq!(
            named(session.as_ref(), Some("11ea8c57"), 1_791_086_000, false),
            "engine-20261004035311-29944-11ea8c57",
            "the session's next VM is another witness"
        );
        assert_eq!(
            named(session.as_ref(), None, 1_791_086_000, false),
            "engine-20261004035311-29944-novm-20730"
        );
        assert_eq!(
            named(session.as_ref(), Some("0e04b585"), 1_791_086_000, true),
            "engine-20261004035311-29944-0e04b585-at-1791086000"
        );
        assert_eq!(named(None, None, 7200, false), "engine-none-2");
    }

    /// One taker of the lines that arrive together; a witness left half
    /// taken by a process that is gone is taken again; one that is whole,
    /// or whose taker still runs, is not.
    #[test]
    fn a_witness_has_one_taker_and_one_left_half_taken_is_taken_again() {
        let yard = crate::yard::Yard::new("witness-claim");

        let fresh = yard.join("engine-fresh");
        assert_eq!(claimed(&fresh), Ok(true));
        assert_eq!(
            claimed(&fresh),
            Ok(false),
            "its taker — this process — still runs"
        );

        let whole = yard.join("engine-whole");
        std::fs::create_dir(&whole).expect("a directory");
        std::fs::write(
            whole.join(super::TAKER),
            crate::subprocess::NO_SUCH_PID.to_string(),
        )
        .expect("the taker");
        std::fs::write(whole.join(super::WHOLE), "x").expect("the last file");
        assert_eq!(
            claimed(&whole),
            Ok(false),
            "a whole witness is nobody's to retake"
        );

        let half = yard.join("engine-half");
        std::fs::create_dir(&half).expect("a directory");
        std::fs::write(
            half.join(super::TAKER),
            crate::subprocess::NO_SUCH_PID.to_string(),
        )
        .expect("the taker");
        std::fs::write(half.join("processes.txt"), "x").expect("what it got to");
        assert_eq!(claimed(&half), Ok(true));
        assert!(
            !half.join("processes.txt").exists(),
            "the half-taken one's files were kept under the new taker"
        );
    }

    /// The names are made so that sorting them is not sorting by age:
    /// the oldest of each kind carries the last name.
    #[test]
    fn only_the_newest_of_each_kind_stand_and_nothing_else_is_touched() {
        let yard = crate::yard::Yard::new("witness-kept");
        let made = |name: &str, age: u64| {
            let dir = yard.join(name);
            std::fs::create_dir(&dir).expect("a directory");
            aged(&dir, age);
            dir
        };
        let oldest = made("engine-9", 50);
        let lines = [
            made("engine-1", 40),
            made("engine-2", 30),
            made("engine-3", 20),
        ];
        let oldest_by_hand = made("engine-9-at-1", 45);
        let by_hand = [
            made("engine-1-at-2", 35),
            made("engine-1-at-3", 25),
            made("engine-1-at-4", 15),
        ];
        let other = made("asking", 60);

        forget_all_but_the_newest(&yard);

        assert!(!oldest.exists(), "the oldest a line took stayed");
        assert!(!oldest_by_hand.exists(), "the oldest taken by hand stayed");
        for dir in lines.iter().chain(&by_hand).chain([&other]) {
            assert!(dir.exists(), "{} was taken", dir.display());
        }
    }

    /// Sets `dir`'s own time `minutes` back, so the order is the test's
    /// and not the clock's grain.
    fn aged(dir: &std::path::Path, minutes: u64) {
        // waits(measured): a time to write on a directory, compared with nothing but its neighbours'
        let at = std::time::SystemTime::now() - std::time::Duration::from_secs(60 * minutes);
        let mut open = std::fs::OpenOptions::new();
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            // A directory opens only with FILE_FLAG_BACKUP_SEMANTICS.
            open.write(true).custom_flags(0x0200_0000);
        }
        #[cfg(not(windows))]
        open.read(true);
        open.open(dir)
            .and_then(|handle| handle.set_modified(at))
            .expect("the directory's time");
    }

    /// The listings themselves, on the machine they are written for:
    /// they parse, end zero with or without an engine installed, and
    /// write what [`session_of`] and a reader are told to expect.
    #[cfg(windows)]
    #[test]
    fn the_hosts_listings_run_and_write_what_is_read_off_them() {
        let yard = crate::yard::Yard::new("witness-listings");

        let processes = super::said_by(
            super::powershell(super::PROCESSES_SCRIPT),
            &yard.join("processes.txt"),
        )
        .expect("the process listing");
        for line in processes.lines().filter(|line| !line.trim().is_empty()) {
            assert!(line.split('|').count() >= 7, "{line}");
        }

        let host = super::said_by(
            super::powershell(super::HOST_SCRIPT),
            &yard.join("host.txt"),
        )
        .expect("the host listing");
        assert!(host.starts_with("memory free "), "{host}");
        assert!(host.contains("the session disks"), "{host}");
    }
}
