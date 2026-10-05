//! Deterministic interleavings in the shipped holder, with the OS request
//! replaced by a recorder. The normal holder tests exercise the real API.

use super::{FakeClaude, claim_file, scratch};

struct OtherBuild(std::process::Child);

impl Drop for OtherBuild {
    fn drop(&mut self) {
        super::end_tree(self.0.id());
        let _ = self.0.wait();
    }
}

fn other_build(dir: &std::path::Path) -> OtherBuild {
    use std::os::windows::process::CommandExt;
    let revision = crate::awake::REVISION + 1;
    let script = crate::awake::holder::script(dir, 0).replace(
        &format!("$revision = '{}'", crate::awake::REVISION),
        &format!("$revision = '{revision}'"),
    );
    let path = dir.join("other-build.ps1");
    std::fs::write(&path, script).expect("the compatible build's script");
    let _held = crate::awake::lock(dir).expect("the holder's name is installed before it reads");
    let child = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-File"])
        .arg(path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(0x0800_0000)
        .spawn()
        .expect("the other build starts");
    std::fs::write(
        dir.join(format!("holder-r{revision}")),
        child.id().to_string(),
    )
    .expect("the other build's name");
    // A session runs the other build too: its hooks were heard.
    heard(dir, revision, crate::awake::now_ms());
    OtherBuild(child)
}

/// Stamps `revision`'s hooks as last heard at `at` ms.
fn heard(dir: &std::path::Path, revision: u32, at: u64) {
    std::fs::write(dir.join(format!("hooks-r{revision}")), at.to_string())
        .expect("the build's hook stamp");
}

pub(super) fn observed(dir: &std::path::Path, revision: u32, verdict: &str, since: u64) -> u32 {
    let row = crate::wait::until(
        "the holder's native request result",
        || {
            let _held = crate::awake::lock(dir).expect("read the observation under lock");
            std::fs::read_to_string(dir.join(format!("holder-r{revision}"))).unwrap_or_default()
        },
        |row| {
            let fields: Vec<_> = row.split_whitespace().collect();
            fields.get(1) == Some(&verdict)
                && fields
                    .get(2)
                    .and_then(|s| s.parse::<u64>().ok())
                    .is_some_and(|at| at >= since)
                && fields
                    .get(3)
                    .and_then(|s| s.parse::<u32>().ok())
                    .is_some_and(|previous| previous != 0)
        },
    );
    let previous = row.split_whitespace().nth(3).unwrap().parse().unwrap();
    let applied: u64 = row.split_whitespace().nth(4).unwrap().parse().unwrap();
    eprintln!(
        "Windows holder r{revision}: {row}; transition_after_mark_ms={}",
        applied.saturating_sub(since)
    );
    previous
}

#[test]
fn compatible_holders_apply_and_clear_real_windows_requests_together() {
    use crate::awake::{Mark, apply};
    let dir = scratch("native-requests");
    let claude = FakeClaude::start(&dir);
    let since = crate::awake::now_ms();
    apply(&dir, &claim_file("s"), &claude.hook(), Mark::Prompt).expect("a turn starts");
    let _other = other_build(&dir);
    for revision in [crate::awake::REVISION, crate::awake::REVISION + 1] {
        assert_eq!(
            observed(&dir, revision, "holding", since) & 3,
            0,
            "the holder started without a display request"
        );
    }
    apply(&dir, &claim_file("t"), &claude.hook(), Mark::Prompt).expect("another session starts");
    apply(
        &dir,
        &claim_file("t"),
        &claude.hook(),
        Mark::ToolStart {
            id: "question",
            name: "AskUserQuestion",
            input: "q",
        },
    )
    .expect("its question opens");
    apply(
        &dir,
        &claim_file("t"),
        &claude.hook(),
        Mark::Asked {
            id: Some("question"),
            name: "AskUserQuestion",
            input: "q",
        },
    )
    .expect("the question waits for a person");
    assert_eq!(
        super::next_reading(&dir),
        super::holding(),
        "the other session's question must not release s"
    );
    let since = crate::awake::now_ms();
    apply(&dir, &claim_file("s"), &claude.hook(), Mark::Idle)
        .expect("either build's Stop updates the shared activity");
    for revision in [crate::awake::REVISION, crate::awake::REVISION + 1] {
        assert_eq!(
            observed(&dir, revision, "released", since) & 3,
            1,
            "Windows returned the holder's previous system-only request"
        );
    }
    let since = crate::awake::now_ms();
    apply(
        &dir,
        &claim_file("t"),
        &claude.hook(),
        Mark::ToolEnd("question"),
    )
    .expect("the person's answer arrives");
    for revision in [crate::awake::REVISION, crate::awake::REVISION + 1] {
        assert_eq!(observed(&dir, revision, "holding", since) & 3, 0);
    }
    let since = crate::awake::now_ms();
    apply(&dir, &claim_file("t"), &claude.hook(), Mark::Idle).expect("the last session stops");
    for revision in [crate::awake::REVISION, crate::awake::REVISION + 1] {
        assert_eq!(observed(&dir, revision, "released", since) & 3, 1);
    }
}

/// The shipped holder from its directory on — past the type it declares
/// for itself — as a holder of `revision` in `dir` runs it, with
/// `during_query` run each time it has listed the processes: once the
/// claims are copied, and before anything is made of the listing.
pub(super) fn shipped(dir: &std::path::Path, revision: u32, during_query: &str) -> String {
    let source = crate::awake::holder::SCRIPT;
    let source = &source[source.find("$dir =").expect("the script's directory")..];
    let processes = format!(
        "{}\n$listing = ${{function:Get-Processes}}\nfunction Get-Processes {{\n  \
         $table = & $listing\n  {during_query}\n  return $table\n}}",
        crate::awake::processes::SCRIPT
    );
    source
        .replace("@PROCESSES@", &processes)
        .replace("@DESKTOP@", crate::awake::desktop::SCRIPT)
        .replace("@DIR@", &dir.display().to_string().replace('\'', "''"))
        .replace("@REVISION@", &revision.to_string())
        .replace("@FORMAT@", "1")
        .replace("@RESERVED@", "0")
        .replace("@STARTING@", "starting")
        .replace("@DELEGATED@", "'Agent', 'Task'")
        .replace("@STARTS_PROCESSES@", "'Bash', 'PowerShell', 'Monitor'")
        .replace("@TTL@", "3600")
        .replace(
            "@RETIRE@",
            &crate::awake::RETIRE_AFTER.as_secs().to_string(),
        )
        .replace("@STALE@", &crate::awake::HOLDER_STALE.as_secs().to_string())
        .replace("@SLACK@", "10")
        .replace("@HOLD@", "2147483649")
        .replace("@LET_GO@", "2147483648")
        .replace("@TICK@", "15")
        .replace(
            "@PACE@",
            &crate::awake::holder::PACE.as_millis().to_string(),
        )
        .replace("@BURST@", &crate::awake::holder::BURST.to_string())
        .replace(
            "@HISTORY@",
            &crate::awake::holder::HISTORY_BYTES.to_string(),
        )
}

/// The system's clocks, for the holder's own type in a probe: the time of
/// day, and the one that only runs forward.
pub(super) const REAL_CLOCK: &str = "public static long Now() { return System.DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(); } \
     public static long Ticks() { return System.Diagnostics.Stopwatch.GetTimestamp() / (System.Diagnostics.Stopwatch.Frequency / 1000); }";

/// Runs `body` — the shipped holder — as the holder of `revision` in
/// `dir`, with the OS request recorded in place of asked (`Last`), its
/// clock read off `clock`, and the commands in `stubs` replaced; the probe
/// ends where a stub throws `PROBE_END`, and answers what it printed.
pub(super) fn probe(
    dir: &std::path::Path,
    revision: u32,
    clock: &str,
    stubs: &str,
    body: &str,
) -> String {
    let script = format!(
        r#"$ErrorActionPreference = 'Stop'
Add-Type 'public static class PggAwake {{ public static uint Last=2147483649; public static uint SetThreadExecutionState(uint flags) {{ Last=flags; return 1; }} {clock} }}'
{stubs}
[IO.File]::WriteAllText('{}', "$PID")
try {{
{body}
}} catch {{ if ($_.Exception.Message -ne 'PROBE_END') {{ throw }} }}
"#,
        dir.join(format!("holder-r{revision}"))
            .display()
            .to_string()
            .replace('\'', "''")
    );
    let path = dir.join("probe.ps1");
    std::fs::write(&path, script).expect("the probe script");
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-File"])
        .arg(path)
        .output()
        .expect("the holder probe runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// One reading of a holder that asks for the machine as it starts, and
/// what it asks for after it (`REQUEST=`).
fn once(dir: &std::path::Path, revision: u32, during_query: &str) -> String {
    let body =
        shipped(dir, revision, during_query).replace("$holding = $false", "$holding = $true");
    let stubs = "function Wait-Event {\n  param($Timeout)\n  \
                 [Console]::WriteLine('REQUEST=' + [PggAwake]::Last)\n  throw 'PROBE_END'\n}";
    probe(dir, revision, REAL_CLOCK, stubs, &body)
}

/// How far apart the claims of a probe's stream change, in milliseconds.
const EVERY_MS: u64 = 100;

/// What a holder makes of `events` claims changing [`EVERY_MS`] apart —
/// one that asks for the machine as it starts (`held`), or not: how long
/// it waited after each before it read again, on a clock only the changes
/// and the waits move.
fn stream(dir: &std::path::Path, held: bool, events: usize) -> Vec<u64> {
    let mut body = shipped(dir, crate::awake::REVISION, "");
    if held {
        body = body.replace("$holding = $false", "$holding = $true");
    }
    // waits(measured): the stand-in for the holder's `Start-Sleep` — nothing sleeps; the wait asked for moves the probe's clock and is printed
    let stubs = format!(
        "$script:asked = 0\n\
         function Wait-Event {{\n  param($Timeout)\n  \
           if ($script:asked -ge {events}) {{ throw 'PROBE_END' }}\n  \
           $script:asked++\n  [PggAwake]::Clock += {EVERY_MS}\n  \
           [Console]::WriteLine('EVENT')\n  return 'a claim changed'\n}}\n\
         function Start-Sleep {{\n  param([int]$Milliseconds)\n  \
           [PggAwake]::Clock += $Milliseconds\n  \
           [Console]::WriteLine(\"SLEPT=$Milliseconds\")\n}}"
    );
    // As large as a real clock's milliseconds — past what an `Int32`
    // holds — while a wait takes an `Int32`, as the real `Start-Sleep`
    // does.
    let clock = "public static long Clock=1700000000000; \
                 public static long Now() { return Clock; } \
                 public static long Ticks() { return Clock; }";
    let said = probe(dir, crate::awake::REVISION, clock, &stubs, &body);
    let mut waits = Vec::new();
    for line in said.lines() {
        if line == "EVENT" {
            waits.push(0);
        } else if let Some(slept) = line.strip_prefix("SLEPT=") {
            let last = waits.last_mut().expect("a wait follows a change");
            *last += slept.parse::<u64>().expect("milliseconds");
        }
    }
    assert_eq!(waits.len(), events, "{said}");
    waits
}

/// While it holds the machine a holder answers a burst of changes at once,
/// then begins one reading each `PACE` — such a reading can only let go,
/// and every hook of every session at work asks for one. While it does not
/// hold it, no reading waits: a hold cannot.
#[test]
fn a_held_machine_s_readings_are_paced_past_a_burst() {
    let burst = usize::try_from(crate::awake::holder::BURST).expect("a count");
    let pace = u64::try_from(crate::awake::holder::PACE.as_millis()).expect("seconds");
    // The burst lasts a little longer than its count: the stream's own
    // time earns readings back.
    let spent = u64::from(crate::awake::holder::BURST) * pace / (pace - EVERY_MS);
    let events = usize::try_from(spent).expect("a count") + 3;
    let (held, _claude) = working("paced");
    let waits = stream(&held, true, events);
    assert!(
        waits[..burst].iter().all(|wait| *wait == 0),
        "a burst waited: {waits:?}"
    );
    assert_eq!(
        waits.last(),
        Some(&(pace - EVERY_MS)),
        "past the burst, a reading each pace: {waits:?}"
    );

    let free = scratch("unpaced");
    let claude = FakeClaude::start(&free);
    std::fs::write(
        free.join(claim_file("s")),
        format!(
            "r1 idle {} {} a 0 - - -",
            claude.pid(),
            crate::awake::now_ms()
        ),
    )
    .expect("the idle claim");
    let waits = stream(&free, false, events);
    assert!(
        waits.iter().all(|wait| *wait == 0),
        "a machine not held waited to read: {waits:?}"
    );
}

/// A reading lists the processes its calls' children are read from once
/// it has read the transcripts. A command that starts, and whose result is
/// written, after the claims' owners were listed still has its process
/// seen; read off the owners' listing, the result would show the call
/// ended with no process to it, and the holder would forget a command
/// that runs.
#[test]
fn a_command_started_once_the_owners_are_listed_keeps_its_call() {
    let dir = scratch("late-process");
    let mut claude = FakeClaude::start(&dir);
    let path = super::transcript(&dir.join("s.jsonl"), &[super::REPLY]);
    let start = crate::awake::now_ms();
    let claim = dir.join(claim_file("s"));
    std::fs::write(
        &claim,
        format!(
            "r1 idle {} {start} a 0 -:toolu_b:Bash:x:{start}:0:0 - {}",
            claude.pid(),
            path.display()
        ),
    )
    .expect("the stopped session's open call");
    // waits(paced): the probe's wait for the test's go-ahead — the file ends it
    let race = "if (-not $script:raced) { $script:raced = $true; \
                [IO.File]::WriteAllText(\"$dir\\listed\", ''); \
                while (-not (Test-Path -LiteralPath \"$dir\\started\")) { Start-Sleep -Milliseconds 20 } }";
    let probe = {
        let dir = dir.to_path_buf();
        std::thread::spawn(move || once(&dir, crate::awake::REVISION, race))
    };
    crate::wait::until(
        "the holder's listing of the owners",
        || dir.join("listed").exists(),
        |listed| *listed,
    );
    let command = claude.run();
    super::append(
        &path,
        &[&format!(
            r#"{{"type":"user","message":{{"role":"user","content":[{{"tool_use_id":"toolu_b","type":"tool_result","content":"ok"}}]}},"timestamp":"{}"}}"#,
            super::stamped(crate::awake::now_ms())
        )],
    );
    std::fs::write(dir.join("started"), "").expect("the go-ahead");
    let observed = probe.join().expect("the probe");
    assert!(super::alive(command));
    assert!(
        observed.contains("REQUEST=2147483649"),
        "a command started during the reading was let go of: {observed}"
    );
    assert!(
        std::fs::read_to_string(&claim).is_ok_and(|claim| claim.contains("toolu_b")),
        "the call of a command that runs was forgotten"
    );
}

/// The history names what a hold stands on, sorted: a session's turn, a
/// process running on past a turn, a wake-up scheduled. A process within
/// a turn that counts is the turn's, and no reason of its own.
#[test]
fn the_history_names_what_a_hold_stands_on() {
    let dir = scratch("reasons");
    let mut claude = FakeClaude::start(&dir);
    let start = crate::awake::now_ms();
    claude.run();
    let (owner, at) = (claude.pid(), crate::awake::now_ms());
    for (session, line) in [
        // In a turn, its shell call open and the call's process running.
        (
            "t",
            format!("r1 working {owner} {at} a 0 -:toolu_t:Bash:x:{start}:0:0 - -"),
        ),
        // Stopped, its shell call ended and the call's process running on.
        (
            "p",
            format!("r1 idle {owner} {at} a 0 -:toolu_p:Bash:x:{start}:{at}:0 - -"),
        ),
        // Stopped, a wake-up scheduled.
        (
            "w",
            format!("r1 idle {owner} {at} a {} - - -", at / 1000 + 600),
        ),
    ] {
        std::fs::write(dir.join(claim_file(session)), line).expect("a claim");
    }
    let observed = once(&dir, crate::awake::REVISION, "");
    assert!(observed.contains("REQUEST=2147483649"), "{observed}");
    assert_eq!(
        super::told(&dir),
        ["holding process:p,turn:t,wake:w", "quit -"]
    );
}

/// The history is cut back to its newer half once it passes its bound, at
/// a line's start, and the line just written stays.
#[test]
fn the_history_is_cut_back_to_its_newer_half() {
    let (dir, _claude) = working("history-cut");
    let bound = usize::try_from(crate::awake::holder::HISTORY_BYTES).expect("a size");
    let old = "1700000000000 r0 1 holding turn:an-older-session\n";
    let history = dir.join(crate::awake::HISTORY);
    std::fs::write(&history, old.repeat(bound / old.len() + 1)).expect("a history at its bound");
    once(&dir, crate::awake::REVISION, "");
    let text = std::fs::read_to_string(&history).expect("the history");
    assert!(
        (bound / 4..bound * 3 / 4).contains(&text.len()),
        "{} bytes are left of {bound}",
        text.len()
    );
    assert!(text.starts_with(old), "the cut fell inside a line");
    assert!(
        super::told(&dir).ends_with(&["holding turn:s".to_string(), "quit -".to_string()]),
        "the cut took the line it was written for"
    );
}

#[test]
fn a_reading_cannot_release_work_updated_during_its_process_query() {
    let dir = scratch("late-work");
    let claude = FakeClaude::start(&dir);
    let at = crate::awake::now_ms();
    let owner = claude.pid();
    std::fs::write(
        dir.join(claim_file("a")),
        format!(
            "r1 idle {owner} {at} a 0 -:bg:Bash:x:{}:{}:0 - -",
            at - 1000,
            at - 500
        ),
    )
    .expect("the completed shell call");
    std::fs::write(
        dir.join(claim_file("b")),
        format!("r1 idle {owner} {at} b 0 - - -"),
    )
    .expect("the idle session");
    let change = format!(
        "$guard = Enter-Lock; try {{ [IO.File]::WriteAllText(\"$dir\\{}\", \"r1 working {owner} $([PggAwake]::Now()) b 0 - - -\") }} finally {{ Exit-Lock $guard }}",
        claim_file("b")
    );
    let observed = once(&dir, crate::awake::REVISION, &change);
    assert!(
        observed.contains("REQUEST=2147483649"),
        "new work was released: {observed}"
    );
}

#[test]
fn a_new_holder_build_reads_the_existing_activity_protocol() {
    let dir = scratch("compatible-build");
    let claude = FakeClaude::start(&dir);
    std::fs::write(
        dir.join(claim_file("s")),
        format!(
            "r1 working {} {} a 0 - - -",
            claude.pid(),
            crate::awake::now_ms()
        ),
    )
    .expect("the existing activity");
    // Sessions run both builds.
    heard(&dir, crate::awake::REVISION, crate::awake::now_ms());
    heard(&dir, crate::awake::REVISION + 1, crate::awake::now_ms());
    let observed = once(&dir, crate::awake::REVISION + 1, "");
    assert!(
        observed.contains("REQUEST=2147483649"),
        "a build change lost activity: {observed}"
    );
    std::fs::write(
        dir.join(claim_file("s")),
        format!(
            "r1 idle {} {} a 0 - - -",
            claude.pid(),
            crate::awake::now_ms()
        ),
    )
    .expect("the new build's Stop uses the same protocol");
    let observed = once(&dir, crate::awake::REVISION, "");
    assert!(
        observed.contains("REQUEST=2147483648"),
        "the old build retained the finished turn: {observed}"
    );
}

/// A working claim of a live Claude process, so that no holder quits for
/// want of claims, and the process that keeps it alive.
fn working(stem: &str) -> (super::super::Scratch, FakeClaude) {
    let dir = scratch(stem);
    let claude = FakeClaude::start(&dir);
    std::fs::write(
        dir.join(claim_file("s")),
        format!(
            "r1 working {} {} a 0 - - -",
            claude.pid(),
            crate::awake::now_ms()
        ),
    )
    .expect("the working claim");
    (dir, claude)
}

/// A time its build's hooks were last heard at, past stepping down.
fn long_ago() -> u64 {
    let retire = u64::try_from(crate::awake::RETIRE_AFTER.as_millis()).expect("minutes");
    crate::awake::now_ms() - retire - 60_000
}

/// Another build's holder, beating now, whose hooks were last heard at
/// `heard_at`.
fn other_holder(dir: &std::path::Path, revision: u32, heard_at: u64) {
    std::fs::write(dir.join(format!("holder-r{revision}")), "1 holding 0 0 0")
        .expect("the other holder's name");
    heard(dir, revision, heard_at);
}

/// Whether the holder of `revision` went on to wait for its next reading
/// — its name kept, the probe's end reached — or stepped down.
fn stayed(dir: &std::path::Path, revision: u32, observed: &str) -> bool {
    let named = dir.join(format!("holder-r{revision}")).exists();
    assert_eq!(
        named,
        observed.contains("REQUEST="),
        "a holder either reads on under its name or quits without it: {observed}"
    );
    named
}

/// The holder of a build no session runs any more steps down for the
/// holder of one in use, which reads the same claims: it takes its name
/// off and lets go, and the build's next hook would start it again.
#[test]
fn a_quiet_build_s_holder_steps_down_for_one_in_use() {
    let (dir, _claude) = working("retire-for-use");
    let revision = crate::awake::REVISION;
    heard(&dir, revision, long_ago());
    other_holder(&dir, revision - 1, crate::awake::now_ms());
    let observed = once(&dir, revision, "");
    assert!(!stayed(&dir, revision, &observed), "it read on: {observed}");
}

/// Quiet hooks alone never take the machine's last holder down: a session
/// waiting on a long background run makes no hook either.
#[test]
fn the_last_holder_standing_never_steps_down() {
    let (dir, _claude) = working("retire-alone");
    let revision = crate::awake::REVISION;
    heard(&dir, revision, long_ago());
    let observed = once(&dir, revision, "");
    assert!(stayed(&dir, revision, &observed), "it quit: {observed}");
    assert!(observed.contains("REQUEST=2147483649"), "{observed}");
}

/// When every build's hooks are quiet, the lower revision steps down and
/// the higher stays, so two quiet holders never both go.
#[test]
fn among_quiet_builds_the_higher_revision_stays() {
    let revision = crate::awake::REVISION;
    let (lower, _claude) = working("retire-lower");
    heard(&lower, revision, long_ago());
    other_holder(&lower, revision + 1, long_ago());
    let observed = once(&lower, revision, "");
    assert!(
        !stayed(&lower, revision, &observed),
        "the lower stayed: {observed}"
    );

    let (higher, _claude) = working("retire-higher");
    heard(&higher, revision + 1, long_ago());
    other_holder(&higher, revision, long_ago());
    let observed = once(&higher, revision + 1, "");
    assert!(
        stayed(&higher, revision + 1, &observed),
        "the higher went: {observed}"
    );
}

/// A holder whose build's hooks are heard stays beside any other, and a
/// holder whose heartbeat went stale is nobody to step down for.
#[test]
fn a_build_in_use_or_a_stale_holder_keeps_a_holder_standing() {
    let revision = crate::awake::REVISION;
    let (used, _claude) = working("retire-used");
    heard(&used, revision, crate::awake::now_ms());
    other_holder(&used, revision + 1, crate::awake::now_ms());
    let observed = once(&used, revision, "");
    assert!(
        stayed(&used, revision, &observed),
        "a build in use stepped down: {observed}"
    );

    let (stale, _claude) = working("retire-stale");
    heard(&stale, revision, long_ago());
    other_holder(&stale, revision + 1, crate::awake::now_ms());
    let beat = crate::awake::HOLDER_STALE + std::time::Duration::from_secs(60);
    std::fs::File::options()
        .write(true)
        .open(stale.join(format!("holder-r{}", revision + 1)))
        .and_then(|name| name.set_modified(std::time::SystemTime::now() - beat))
        .expect("an old heartbeat");
    let observed = once(&stale, revision, "");
    assert!(
        stayed(&stale, revision, &observed),
        "it stepped down for a dead holder: {observed}"
    );
}

/// The shipped holder, started as a hook starts it, steps down beside a
/// build in use once its own build goes quiet: its process ends and lets
/// go, and the other build's holder reads on.
#[test]
fn a_started_holder_of_a_quiet_build_quits() {
    use crate::awake::{Mark, apply};
    let (dir, claude) = working("retire-started");
    let revision = crate::awake::REVISION;
    let pid = apply(&dir, &claim_file("s"), &claude.hook(), Mark::Working)
        .expect("a hook")
        .expect("the holder it started");
    assert_eq!(super::next_reading(&dir), super::holding());
    let since = crate::awake::now_ms();
    let _other = other_build(&dir);
    {
        // No hook of this build is heard again: a claim another build's
        // hook writes wakes the holder for its next reading.
        let _held = crate::awake::lock(&dir).expect("the lock");
        heard(&dir, revision, long_ago());
        std::fs::write(
            dir.join(claim_file("t")),
            format!(
                "r1 working {} {} a 0 - - -",
                claude.pid(),
                crate::awake::now_ms()
            ),
        )
        .expect("the other session's claim");
    }
    super::quit(pid);
    assert!(
        !dir.join(format!("holder-r{revision}")).exists(),
        "the holder that quit left its name"
    );
    assert!(
        dir.join(claim_file("t")).exists(),
        "stepping down took a claim with it"
    );
    observed(&dir, revision + 1, "holding", since);
}

/// Every hook stamps its build as heard, for that build's holder to know
/// a session still runs it.
#[test]
fn a_hook_stamps_its_build_as_heard() {
    use crate::awake::{Mark, apply};
    let dir = scratch("heard");
    let claude = FakeClaude::start(&dir);
    let before = crate::awake::now_ms();
    apply(&dir, &claim_file("s"), &claude.hook(), Mark::Prompt).expect("a turn starts");
    let said: u64 = std::fs::read_to_string(dir.join(format!("hooks-r{}", crate::awake::REVISION)))
        .expect("the stamp")
        .trim()
        .parse()
        .expect("milliseconds");
    assert!(said >= before, "stamped {said}, before {before}");
}
