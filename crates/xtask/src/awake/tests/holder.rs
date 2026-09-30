//! The holder itself, run against stand-in Claude processes: a copy of
//! cmd.exe named claude.exe — the image name the holder asks an owner
//! for — that starts a child only when it is told to.

use std::io::Write;
use std::path::Path;

use super::{CHAIN_CLAUDE, CHAIN_DIR, claim_file, scratch};
use crate::awake::claim::{Claim, State, Writer, digest};
use crate::awake::{Mark, apply, holder_path};

/// A Claude process to own claims: idle but for the `conhost.exe` of its
/// hidden console, as a Claude process keeps, until it starts a command.
struct FakeClaude(std::process::Child);

impl FakeClaude {
    fn start(dir: &Path) -> Self {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let exe = dir.join("claude.exe");
        if !exe.exists() {
            let root = std::env::var("SystemRoot").expect("SystemRoot names the Windows directory");
            std::fs::copy(Path::new(&root).join("System32").join("cmd.exe"), &exe)
                .expect("a claude.exe to run");
        }
        let child = std::process::Command::new(&exe)
            .args(["/d", "/q", "/k"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .expect("the stand-in starts");
        Self(child)
    }

    fn pid(&self) -> u32 {
        self.0.id()
    }

    /// Starts a child that runs until it is ended — a tool, a background
    /// task, a server — and answers its pid.
    fn run(&mut self) -> u32 {
        let before = self.tasks();
        let stdin = self.0.stdin.as_mut().expect("the stand-in's input");
        stdin
            .write_all(b"start /b ping -n 600 127.0.0.1\r\n")
            .expect("the command reaches the stand-in");
        stdin.flush().expect("the command reaches the stand-in");
        let tasks = crate::wait::until(
            "the stand-in's new child",
            || self.tasks(),
            |tasks| tasks.iter().any(|task| !before.contains(task)),
        );
        tasks
            .into_iter()
            .find(|task| !before.contains(task))
            .expect("a new child")
    }

    /// What the session's own hook writes.
    fn hook(&self) -> Writer {
        Writer {
            claude: self.pid(),
            now_ms: crate::awake::now_ms(),
            seat: "a".into(),
            who: "-".into(),
            transcript: None,
        }
    }

    /// What the session's hook writes, its transcript at `path`.
    fn hook_with(&self, path: &Path) -> Writer {
        Writer {
            transcript: Some(path.display().to_string()),
            ..self.hook()
        }
    }

    /// What a subagent's hook writes, its own transcript at `path`.
    fn agent_hook_with(&self, agent: &str, path: &Path) -> Writer {
        Writer {
            who: agent.into(),
            ..self.hook_with(path)
        }
    }

    /// Its children but the console host, by pid.
    fn tasks(&self) -> Vec<u32> {
        let pid = self.pid();
        let listing = format!(
            "Get-CimInstance Win32_Process -Filter \"ParentProcessId={pid}\" | \
             Where-Object {{ $_.Name -ne 'conhost.exe' }} | ForEach-Object {{ $_.ProcessId }}"
        );
        powershell(&listing)
            .lines()
            .filter_map(|line| line.trim().parse().ok())
            .collect()
    }
}

impl Drop for FakeClaude {
    fn drop(&mut self) {
        // Its children go with it: a child outlives a plain kill of its
        // parent.
        end_tree(self.0.id());
        let _ = self.0.wait();
    }
}

/// Ends the holder named in `dir`, if one runs, and waits for it to be
/// gone, its files in `dir` with it: the test's directory goes next. While
/// a test unwinds, nothing here may panic: the name is read without the
/// lock, and a wait that runs out gives up rather than failing.
pub(super) fn end_the_holder(dir: &Path, unwinding: bool) {
    let pid = if unwinding {
        std::fs::read_to_string(holder_path(dir))
            .ok()
            .and_then(|text| text.split_whitespace().next()?.parse().ok())
    } else {
        holder_pid(dir)
    };
    // A holder that quit clears its name; a pid named by one that died may
    // be another process's by now.
    let Some(pid) = pid.filter(|pid| running_as(*pid, "powershell.exe")) else {
        return;
    };
    end_tree(pid);
    if !unwinding {
        quit(pid);
        return;
    }
    let mut wait = crate::wait::Wait::new(
        "a failed test's holder",
        crate::wait::Budget::SUITE,
        crate::wait::LOOK_AGAIN,
    );
    while alive(pid) && wait.look_again("its exit").is_ok() {}
}

fn powershell(script: &str) -> String {
    std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
        .expect("powershell runs")
}

/// The holder's name, split into its pid, its last verdict, and when it
/// read the claims that verdict comes from (0 before its first reading).
/// Read under the lock the holder rewrites it under: between the rewrite's
/// truncation and its write the file is empty. A directory the test has
/// taken down names no holder.
fn named(dir: &Path) -> Option<(u32, String, u64)> {
    if !dir.exists() {
        return None;
    }
    let _held = crate::awake::lock(dir).expect("the lock");
    let text = std::fs::read_to_string(holder_path(dir)).ok()?;
    let mut words = text.split_whitespace();
    let pid = words.next()?.parse().ok()?;
    let verdict = words.next().unwrap_or("").to_string();
    let read = words.next().and_then(|read| read.parse().ok()).unwrap_or(0);
    Some((pid, verdict, read))
}

fn holder_pid(dir: &Path) -> Option<u32> {
    named(dir).map(|(pid, _, _)| pid)
}

/// The holders started on a script installed in `dir`, running now: the
/// command that reads it travels base64-encoded, so it is decoded to be
/// read.
fn holders_for(dir: &Path) -> Vec<u32> {
    let named = format!(
        "$installed = '{}",
        dir.join("holder-")
            .display()
            .to_string()
            .replace('\'', "''")
    );
    // This listing's own command line spells the flag too, and fails to
    // decode: it is left out, and a failed decode reads as no script.
    let listing = format!(
        "Get-CimInstance Win32_Process -Filter \"Name='powershell.exe'\" | ForEach-Object {{ \
           if ($_.ProcessId -ne $PID -and $_.CommandLine -match '-EncodedCommand ([A-Za-z0-9+/=]+)$') {{ \
             $script = try {{ [Text.Encoding]::Unicode.GetString([Convert]::FromBase64String($Matches[1])) }} catch {{ '' }}; \
             if ($script.Contains('{}')) {{ $_.ProcessId }} }} }}",
        named.replace('\'', "''")
    );
    powershell(&listing)
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .collect()
}
fn end_tree(pid: u32) {
    let _ = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

fn alive(pid: u32) -> bool {
    listed(pid, &[])
}

/// Whether `pid` is running with the image name `image`.
fn running_as(pid: u32, image: &str) -> bool {
    listed(pid, &["/FI", &format!("IMAGENAME eq {image}")])
}

fn listed(pid: u32, filter: &[&str]) -> bool {
    std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}")])
        .args(filter)
        .args(["/NH", "/FO", "CSV"])
        .output()
        .is_ok_and(|out| String::from_utf8_lossy(&out.stdout).contains(&format!("\"{pid}\"")))
}

fn modified(path: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}

/// The verdict of the first reading the holder begins after this call —
/// one that sees every claim written and every process started or ended
/// before it; None once the holder has quit. A claim's write wakes the
/// holder at once; news no hook brings (a process, a transcript grown)
/// waits for its tick — a hook's write would also start a holder that had
/// quit, and hide one that should not have. A reading copies the claims
/// under the lock and names itself under it again later, so a name
/// written after this call may still be a reading copied before it: the
/// name says when its reading copied the claims, on the same clock.
fn next_reading(dir: &Path) -> Option<String> {
    let since = crate::awake::now_ms();
    crate::wait::until(
        "the holder's next reading",
        || named(dir),
        |named| named.as_ref().is_none_or(|(_, _, read)| *read >= since),
    )
    .map(|(_, verdict, _)| verdict)
}

/// The holder's next beat after this call, whatever its reading made of
/// the claims — a reading that could not copy them beats too.
fn next_beat(dir: &Path) {
    let since = {
        let _held = crate::awake::lock(dir).expect("the lock");
        modified(&holder_path(dir))
    };
    crate::wait::until(
        "the holder's next beat",
        || modified(&holder_path(dir)),
        |beat| beat.is_none() || *beat > since,
    );
}

fn holding() -> Option<String> {
    Some("holding".to_string())
}

fn released() -> Option<String> {
    Some("released".to_string())
}

fn quit(pid: u32) {
    crate::wait::until("the holder's exit", || alive(pid), |alive| !alive);
}

/// Ends `pid` and its tree, and waits until it is gone: `taskkill /F`
/// returns while the process may still be listed, and a reading that lists
/// it counts it.
fn ended(pid: u32) {
    end_tree(pid);
    crate::wait::until("the process's end", || alive(pid), |alive| !alive);
}

/// The System process's pid: always running, and no Claude's.
const SYSTEM: u32 = 4;

/// A claim of this revision owned by `pid`, working as of now.
fn claim_of(pid: u32) -> String {
    format!(
        "r{} working {pid} {} a 0 - - -",
        crate::awake::CLAIM_FORMAT,
        crate::awake::now_ms()
    )
}

/// Writes `line` as the claim at `path` under the lock, as a hook would —
/// with none of a hook's checks, for a claim no hook would write.
fn planted(dir: &Path, path: &Path, line: &str) {
    let _held = crate::awake::lock(dir).expect("the lock");
    std::fs::write(path, line).expect("a claim");
}

/// The session's claim, read under the lock: the holder rewrites it as it
/// forgets a call.
fn session(dir: &Path) -> Claim {
    let _held = crate::awake::lock(dir).expect("the lock");
    let text = std::fs::read_to_string(dir.join(claim_file("s"))).expect("the session's claim");
    Claim::parse(&text).unwrap_or_else(|| panic!("a claim that does not read: {text}"))
}

/// A background task's notice, written at `ms` since the epoch.
fn notice(ms: u64) -> String {
    format!(
        r#"{{"type":"user","message":{{"role":"user","content":"<task-notification><status>completed</status></task-notification>"}},"timestamp":"{}","origin":{{"kind":"task-notification"}}}}"#,
        stamped(ms)
    )
}

/// A transcript's `timestamp` for `ms` since the epoch,
/// `YYYY-MM-DDTHH:MM:SS.mmmZ` (civil from days, Howard Hinnant's).
fn stamped(ms: u64) -> String {
    let (days, rest) = (ms / 86_400_000, ms % 86_400_000);
    let z = i64::try_from(days).expect("a day count") + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        rest / 3_600_000,
        rest / 60_000 % 60,
        rest / 1000 % 60,
        rest % 1000
    )
}

/// The model's reply, as text.
const REPLY: &str = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"done"}]}}"#;

fn transcript(path: &Path, lines: &[&str]) -> std::path::PathBuf {
    let text: String = lines.iter().map(|line| format!("{line}\n")).collect();
    std::fs::write(path, text).expect("a transcript");
    path.to_path_buf()
}

fn append(path: &Path, lines: &[&str]) {
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(path)
        .expect("the transcript");
    for line in lines {
        file.write_all(format!("{line}\n").as_bytes())
            .expect("the transcript takes the line");
    }
}

/// The holder's script runs, takes the lock the hooks take, and — with no
/// claim to read — clears its name and exits.
#[test]
fn a_holder_with_no_claim_clears_its_name_and_exits() {
    let dir = scratch("holder-exits");
    let pid = {
        let _held = crate::awake::lock(&dir).expect("the lock");
        let pid = crate::awake::holder::spawn(&dir, std::process::id()).expect("a holder");
        std::fs::write(holder_path(&dir), pid.to_string()).expect("its name");
        pid
    };
    quit(pid);
    assert!(
        !holder_path(&dir).exists(),
        "the holder left its name behind"
    );
}

/// Another implementation's heartbeat cannot stand in for this holder:
/// each implementation owns its name even when they share activity.
#[test]
fn holders_of_two_revisions_ask_apart() {
    let dir = scratch("two-revisions");
    let claude = FakeClaude::start(&dir);
    let other = dir.join(format!("holder-r{}", crate::awake::REVISION + 1));
    std::fs::write(&other, "1 holding").expect("another revision's holder's name");
    let pid = apply(&dir, &claim_file("s"), &claude.hook(), Mark::Prompt)
        .expect("claimed")
        .expect("this revision's holder started");
    assert_eq!(holder_pid(&dir), Some(pid));
    assert_eq!(next_reading(&dir), holding());
    assert_eq!(
        std::fs::read_to_string(&other).ok().as_deref(),
        Some("1 holding"),
        "this revision's holder touched another's name"
    );
}

/// A claim of another protocol is its own protocol's holder's: this one
/// neither reads it, counts it, nor takes it down, and quits once no claim
/// of its own is left.
#[test]
fn a_claim_of_another_protocol_is_left_to_its_holder() {
    let dir = scratch("other-revision");
    let claude = FakeClaude::start(&dir);
    let pid = apply(&dir, &claim_file("s"), &claude.hook(), Mark::Idle)
        .expect("claimed")
        .expect("a holder started");
    assert_eq!(next_reading(&dir), released());

    // Well formed but for its protocol, under its protocol's name: a holder
    // deaf to the protocol would count it, working and fresh, with a live
    // owner.
    let other = dir.join(format!("r{}.t.claim", crate::awake::CLAIM_FORMAT + 1));
    let line = format!(
        "r{} working {} {} a 0 - - -",
        crate::awake::CLAIM_FORMAT + 1,
        claude.pid(),
        crate::awake::now_ms()
    );
    planted(&dir, &other, &line);
    assert_eq!(
        next_reading(&dir),
        released(),
        "a claim of another revision held this revision's machine"
    );

    apply(&dir, &claim_file("s"), &claude.hook(), Mark::Gone).expect("the session's claim goes");
    quit(pid);
    assert_eq!(
        std::fs::read_to_string(&other).ok(),
        Some(line),
        "the holder took down a claim it cannot read"
    );
}

/// A holder whose name another took quits at its next reading.
#[test]
fn a_holder_whose_name_was_taken_quits() {
    let dir = scratch("name-taken");
    let claude = FakeClaude::start(&dir);
    let pid = apply(&dir, &claim_file("s"), &claude.hook(), Mark::Prompt)
        .expect("claimed")
        .expect("a holder started");
    assert_eq!(next_reading(&dir), holding());
    {
        let _held = crate::awake::lock(&dir).expect("the lock");
        std::fs::write(holder_path(&dir), "1 holding").expect("the name, taken");
    }
    let started = apply(&dir, &claim_file("s"), &claude.hook(), Mark::Working).expect("a claim");
    assert_eq!(started, None, "a beating name started another holder");
    quit(pid);
}

/// A hook starting a holder reserves its name first and starts it outside
/// the lock: a fresh reservation starts no second holder, and one gone
/// stale — its start failed — lets the next claim start one, which takes
/// the name.
#[test]
fn a_reserved_name_starts_no_second_holder_until_it_goes_stale() {
    let dir = scratch("reserved");
    let claude = FakeClaude::start(&dir);
    planted(&dir, &holder_path(&dir), "4 starting");
    let started = apply(&dir, &claim_file("s"), &claude.hook(), Mark::Prompt).expect("claimed");
    assert_eq!(started, None, "a reserved name started another holder");

    std::fs::File::options()
        .write(true)
        .open(holder_path(&dir))
        .and_then(|name| {
            name.set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(60))
        })
        .expect("the reservation, gone stale");
    let pid = apply(&dir, &claim_file("s"), &claude.hook(), Mark::Working)
        .expect("claimed")
        .expect("a stale reservation let a holder start");
    assert_eq!(next_reading(&dir), holding());
    assert_eq!(holder_pid(&dir), Some(pid));
}

/// The holder takes the directory's lock only to copy the claims and to
/// write back — never while it asks after processes or reads transcripts,
/// which take their time on any seat and would hold up every other seat's
/// hooks and holders. Read off the script the holder runs.
#[test]
fn the_holder_asks_after_processes_and_transcripts_with_the_lock_let_go() {
    let mut rest = crate::awake::holder::SCRIPT;
    let mut sections = 0;
    while let Some(taken) = rest.find("$lock = Enter-Lock") {
        let held = &rest[taken..];
        let let_go = held
            .find("Exit-Lock $lock")
            .expect("every lock taken is let go");
        let section = &held[..let_go];
        for slow in [
            "Get-CimInstance",
            "Get-Children",
            "Get-Process",
            "Read-Tail",
            "Get-Tail",
            "Get-Stop",
            "Get-NoticeSince",
        ] {
            assert!(
                !section.contains(slow),
                "{slow} runs under the lock:\n{section}"
            );
        }
        sections += 1;
        rest = &held[let_go..];
    }
    assert_eq!(sections, 2, "the readings' lock sections");
}

/// A reader that holds the holder's name open and lets others only read —
/// a virus scanner, an indexer, a person's `Get-Content` — refuses the
/// holder's write of it: the holder writes it again at a later reading
/// rather than ending, since no holder would ask for the machine until a
/// hook found the name stale.
#[test]
fn a_reader_holding_its_name_costs_the_holder_a_beat_not_its_run() {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ: u32 = 1;
    let dir = scratch("name-held-open");
    let claude = FakeClaude::start(&dir);
    let path = transcript(&dir.join("s.jsonl"), &[REPLY]);
    let pid = apply(
        &dir,
        &claim_file("s"),
        &claude.hook_with(&path),
        Mark::Prompt,
    )
    .expect("claimed")
    .expect("a holder started");
    apply(&dir, &claim_file("s"), &claude.hook_with(&path), Mark::Idle).expect("stopped");
    assert_eq!(next_reading(&dir), released());

    let reader = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(holder_path(&dir))
        .expect("the name, held open for reading");
    // The holder rewrites the claim for the notice and writes its name in
    // the same reading, under the lock the claim is read under here.
    append(&path, &[&notice(crate::awake::now_ms())]);
    crate::wait::until(
        "the notice's turn",
        || session(&dir).state,
        |state| *state == State::Working,
    );
    drop(reader);

    apply(
        &dir,
        &claim_file("s"),
        &claude.hook_with(&path),
        Mark::Working,
    )
    .expect("a claim");
    assert_eq!(
        next_reading(&dir),
        holding(),
        "the holder ended on a refused write"
    );
    assert_eq!(holder_pid(&dir), Some(pid), "another holder took the name");
}

/// A claim another process holds open and shares with nobody — a scanner
/// — cannot be read: the holder leaves that reading, beating with what it
/// asked for before, and reads again later rather than ending.
#[test]
fn a_claim_held_open_costs_the_holder_a_reading_not_its_run() {
    use std::os::windows::fs::OpenOptionsExt;
    let dir = scratch("claim-held-open");
    let claude = FakeClaude::start(&dir);
    let pid = apply(&dir, &claim_file("s"), &claude.hook(), Mark::Prompt)
        .expect("claimed")
        .expect("a holder started");
    assert_eq!(next_reading(&dir), holding());

    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(dir.join(claim_file("s")))
        .expect("the claim, held open");
    let held_at = crate::awake::now_ms();
    // A claim written wakes the holder. The first beat after it may be a
    // reading that copied the claims before the hold; the second began
    // after the first ended, and could copy nothing.
    planted(&dir, &dir.join(claim_file("a")), &claim_of(SYSTEM));
    next_beat(&dir);
    next_beat(&dir);
    let (named_pid, verdict, read) = named(&dir).expect("the holder's name");
    assert_eq!(
        (named_pid, verdict.as_str()),
        (pid, "holding"),
        "a reading that could not copy the claims changed what was asked for"
    );
    assert!(read < held_at, "a reading copied a claim held open");
    drop(held);

    apply(&dir, &claim_file("s"), &claude.hook(), Mark::Working).expect("a claim");
    assert_eq!(
        next_reading(&dir),
        holding(),
        "the holder ended on a claim it could not read"
    );
    assert_eq!(holder_pid(&dir), Some(pid), "another holder took the name");
}

/// A claim is its Claude process's: one whose pid names another program —
/// a pid its Claude process left and another took — is taken down at the
/// next reading, and counts for nothing.
#[test]
fn a_claim_whose_owner_is_no_claude_is_taken_down() {
    let dir = scratch("not-claude");
    let claude = FakeClaude::start(&dir);
    apply(&dir, &claim_file("s"), &claude.hook(), Mark::Idle)
        .expect("claimed")
        .expect("a holder started");
    assert_eq!(next_reading(&dir), released());

    let stray = dir.join(claim_file("t"));
    planted(&dir, &stray, &claim_of(SYSTEM));
    assert_eq!(
        next_reading(&dir),
        released(),
        "a claim of another program held the machine"
    );
    assert!(!stray.exists(), "a claim of another program stood");
}

/// A holder whose last claim went but that cannot take its name down — a
/// reader holds the name open — stays, and takes it down at a later
/// reading: a name left standing would keep the next claim from starting a
/// holder until it went stale.
#[test]
fn a_holder_that_cannot_clear_its_name_stays_until_it_can() {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ: u32 = 1;
    let dir = scratch("name-kept");
    let claude = FakeClaude::start(&dir);
    let pid = apply(&dir, &claim_file("s"), &claude.hook(), Mark::Prompt)
        .expect("claimed")
        .expect("a holder started");
    assert_eq!(next_reading(&dir), holding());

    let reader = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(holder_path(&dir))
        .expect("the name, held open for reading");
    // The claim goes and a stray comes at once: the reading that takes the
    // stray down finds no claim left.
    let stray = dir.join(claim_file("a"));
    {
        let _held = crate::awake::lock(&dir).expect("the lock");
        std::fs::remove_file(dir.join(claim_file("s"))).expect("the claim goes");
        std::fs::write(&stray, claim_of(SYSTEM)).expect("a stray claim");
    }
    crate::wait::until(
        "the stray claim's removal",
        || stray.exists(),
        |there| !there,
    );
    drop(crate::awake::lock(&dir).expect("the lock, once that reading is done"));
    drop(reader);

    planted(&dir, &stray, &claim_of(SYSTEM));
    quit(pid);
    assert!(
        !holder_path(&dir).exists(),
        "the holder left its name behind"
    );
}

/// The encoding PowerShell's `-EncodedCommand` reads: .NET's
/// `Convert.ToBase64String` of `Encoding.Unicode.GetBytes`.
#[test]
fn a_script_is_encoded_as_powershell_decodes_it() {
    use crate::awake::holder::encoded;
    assert_eq!(encoded(""), "");
    assert_eq!(encoded("A"), "QQA=");
    assert_eq!(encoded("AB"), "QQBCAA==");
    assert_eq!(encoded("ABC"), "QQBCAEMA");
    assert_eq!(encoded("é"), "6QA=");
}

/// The command that starts the holder fits a Windows command line — 32,767
/// characters, the program's name included — for a checkout under a long
/// path: past it no holder starts, and a hook has nobody to tell.
#[test]
fn the_holder_s_command_fits_a_command_line() {
    let long = Path::new(r"C:\").join("d".repeat(240));
    let line = format!(
        "powershell -NoProfile -NonInteractive -Command \"{}\"",
        crate::awake::holder::create_command(&long, u32::MAX)
    );
    assert!(
        line.len() < 32_767,
        "the holder's command line runs {} characters",
        line.len()
    );
}
/// The holder's script is pinned to its revision: a holder whose name
/// beats serves every seat's compatible claims, so a script changed
/// under the same revision leaves the old one running until its last claim
/// goes.
#[test]
fn the_holder_s_script_is_pinned_to_its_revision() {
    assert_eq!(
        (
            crate::awake::REVISION,
            digest(&format!(
                "{}{}",
                crate::awake::holder::SCRIPT,
                crate::awake::desktop::SCRIPT
            ))
        ),
        (4, "8842f5f934a49a73".to_string()),
        "the holder's script changed: raise REVISION, then pin the new digest here"
    );
}

/// The hooks' lock and the holder's are one lock: PowerShell's
/// `FileStream.Lock` on the first byte is refused while Rust holds the
/// file.
#[test]
fn the_holder_cannot_take_the_lock_a_hook_holds() {
    let dir = scratch("lock");
    let held = crate::awake::lock(&dir).expect("the lock");
    let path = dir
        .join(crate::awake::LOCK)
        .display()
        .to_string()
        .replace('\'', "''");
    let script = format!(
        "$f = [IO.File]::Open('{path}', 'OpenOrCreate', 'ReadWrite', 'ReadWrite, Delete'); \
         try {{ $f.Lock(0, 1); 'took' }} catch [IO.IOException] {{ 'refused' }} finally {{ $f.Dispose() }}"
    );
    let ask = || powershell(&script).trim().to_string();
    assert_eq!(ask(), "refused");
    drop(held);
    assert_eq!(ask(), "took");
}

/// Passed as a filter rather than with `--exact`, like the chain's other
/// links.
const CARGO: &str = "stands_in_for_cargo_between_claude_and_a_hook";

/// Claude Code reads a hook's output until the pipe closes, and every
/// process holding a copy keeps it open: a holder that took one would keep
/// the hook running as long as the holder runs. The pipe closes while the
/// holder still runs.
#[test]
fn a_holder_lets_the_pipes_of_its_hook_close() {
    use std::io::Read;

    let dir = scratch("pipe");
    let claude = FakeClaude::start(&dir);
    let mut chain = std::process::Command::new(std::env::current_exe().expect("this test binary"))
        .args([CARGO, "--ignored", "--nocapture"])
        .env(CHAIN_DIR, dir.as_os_str())
        .env(CHAIN_CLAUDE, claude.pid().to_string())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("the chain starts");
    let mut out = chain.stdout.take().expect("the chain's output");
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut said = String::new();
        let read = out.read_to_string(&mut said).map(|_| said);
        tx.send(read).ok();
    });
    let said = crate::wait::heard("the hook's pipe", "its close", &rx).expect("the pipe reads");
    let pid = holder_pid(&dir).unwrap_or_else(|| panic!("no holder was named:\n{said}"));
    assert!(
        alive(pid),
        "the pipe closed only once the holder had gone:\n{said}"
    );
    assert!(
        chain.wait().expect("the chain is reaped").success(),
        "{said}"
    );
}

/// A working claim holds the machine for as long as its Claude process
/// lives; once the process is gone, the holder drops the claim within a
/// tick — no file says a process ended — and, with no claim left, quits.
#[test]
fn a_working_claim_holds_until_its_claude_process_is_gone() {
    let dir = scratch("owner-gone");
    let claude = FakeClaude::start(&dir);
    let pid = apply(&dir, &claim_file("s"), &claude.hook(), Mark::Prompt)
        .expect("claimed")
        .expect("the claim started a holder");
    assert_eq!(next_reading(&dir), holding());

    drop(claude);
    quit(pid);
    assert!(
        !dir.join(claim_file("s")).exists(),
        "the claim of a gone process stood"
    );
    assert!(
        !holder_path(&dir).exists(),
        "the holder left its name behind"
    );
}

/// Sessions claiming at the same moment start one holder: the claim and
/// the start happen under the directory's lock.
#[test]
fn claims_made_at_once_start_one_holder() {
    let dir = scratch("at-once");
    let claude = FakeClaude::start(&dir);
    let gate = std::sync::Arc::new(std::sync::Barrier::new(6));
    let claims: Vec<_> = (0..6)
        .map(|seat| {
            let (dir, gate, hook) = (
                dir.to_path_buf(),
                std::sync::Arc::clone(&gate),
                claude.hook(),
            );
            std::thread::spawn(move || {
                gate.wait();
                apply(&dir, &claim_file(&seat.to_string()), &hook, Mark::Working)
            })
        })
        .collect();
    let started: Vec<u32> = claims
        .into_iter()
        .filter_map(|claim| claim.join().expect("a claiming thread").expect("claimed"))
        .collect();
    assert_eq!(started.len(), 1, "holders started: {started:?}");
    assert_eq!(holder_pid(&dir), started.first().copied());
    assert_eq!(holders_for(&dir), started);
}

mod coordination;
mod counting;
mod desktop;
mod physical;

#[test]
fn an_installed_holder_handles_unicode_and_quotes_and_removes_its_script() {
    let dir = scratch("holder-東京 '");
    let claude = FakeClaude::start(&dir);
    let pid = apply(&dir, &claim_file("s"), &claude.hook(), Mark::Prompt)
        .expect("the installed script starts")
        .expect("its pid");
    assert_eq!(next_reading(&dir), holding());
    apply(&dir, &claim_file("s"), &claude.hook(), Mark::Gone).expect("the claim goes");
    quit(pid);
    assert!(
        !std::fs::read_dir(&*dir).unwrap().flatten().any(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.starts_with("holder-") && name.ends_with(".ps1")
        }),
        "a normally finished holder left its script"
    );
}
