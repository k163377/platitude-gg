//! The bound on a quiet hold: with nothing happening — no hook of any
//! session, no transcript of a claim written, no counted process at work
//! or started — for [`QUIET`](crate::awake::QUIET) of time awake, the
//! holder lets go, and holds again once something happens. Run as probes
//! of the shipped holder whose clock of time awake the steps move, the
//! last thing to happen moved back past the bound with it.

use super::coordination::{probe, shipped};
use super::{FakeClaude, claim_file, scratch};
use crate::awake::holder::{BUSY, TICK};

/// The time of day and the clock that only runs forward, the real ones;
/// the time awake, moved by the steps (`Clock`).
const MOVED_CLOCK: &str = "public static long Clock = 0; \
     public static long Now() { return System.DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(); } \
     public static long Ticks() { return System.Diagnostics.Stopwatch.GetTimestamp() / (System.Diagnostics.Stopwatch.Frequency / 1000); } \
     public static long Awake() { return Clock; }";

/// The bound, and a minute past it, in milliseconds.
fn past_the_bound() -> u128 {
    crate::awake::QUIET.as_millis() + 60_000
}

/// A step that moves the time awake on by the window the holder judges
/// the processes' work over. The holder's first judgement takes every
/// process there for one started.
fn a_window_on() -> String {
    format!("[PggAwake]::Clock += {}", TICK.as_millis())
}

/// [`a_window_on`], with the last thing to happen moved back past the
/// bound: quiet unless the reading finds something happening.
fn quiet_a_window_on() -> String {
    format!(
        "{}; $script:lastActive -= {}",
        a_window_on(),
        past_the_bound()
    )
}

/// What the history says of a hold let go a window after
/// [`quiet_a_window_on`] moved the last thing to happen.
fn released_quiet() -> String {
    format!(
        "released quiet:{}m",
        (past_the_bound() + TICK.as_millis()) / 60_000
    )
}

/// What `body` — a holder holding as it starts — asks for at each wait
/// (`AT<i> LAST=…`), `steps[i]` run before the reading after its `i`-th.
fn waits(dir: &std::path::Path, body: &str, steps: &[String]) -> String {
    let cases: String = steps
        .iter()
        .enumerate()
        .map(|(at, step)| format!("    {at} {{ {step} }}\n"))
        .collect();
    let stubs = format!(
        "$script:readings = 0\n\
         function Wait-Event {{\n  param($Timeout)\n  \
           [Console]::WriteLine('AT' + $script:readings + ' LAST=' + [PggAwake]::Last)\n  \
           if ($script:readings -ge {}) {{ throw 'PROBE_END' }}\n  \
           switch ($script:readings) {{\n{cases}  }}\n  \
           $script:readings++\n  return 'a claim changed'\n}}",
        steps.len()
    );
    let body = body.replace("$holding = $false", "$holding = $true");
    probe(dir, crate::awake::REVISION, MOVED_CLOCK, &stubs, &body)
}

/// Whether the waits `said` asked for the machine (`true`) or let it go,
/// in order from the first.
fn asked(said: &str, held: &[bool]) {
    for (at, holds) in held.iter().enumerate() {
        let last = if *holds {
            2_147_483_649u32
        } else {
            2_147_483_648
        };
        assert!(
            said.contains(&format!("AT{at} LAST={last}")),
            "AT{at}: {said}"
        );
    }
}

/// A step that writes `path` again: something happened.
fn touched(path: &std::path::Path) -> String {
    format!(
        "[IO.File]::SetLastWriteTimeUtc('{}', [DateTime]::UtcNow.AddMinutes(5))",
        path.display().to_string().replace('\'', "''")
    )
}

/// A step that goes on once `changed` — a PowerShell condition on the
/// processes of the command `owner` started at `start`, `$first` and
/// `$now` two listings of them as `<pid>/<start>` = processor ms, as the
/// step starts and now — holds, and fails the probe after a minute
/// without.
fn once_the_processes(owner: u32, start: u64, changed: &str) -> String {
    // waits(paced): a poll of the listing, spaced so its own listings leave the processors to the work it watches — the condition ends it
    format!(
        "function Get-Under {{ return [PggProcesses]::new().Tree({owner}, {start}, [int64]::MaxValue) }}; \
         $sw = [Diagnostics.Stopwatch]::StartNew(); $first = Get-Under; \
         do {{ \
           if ($sw.Elapsed.TotalSeconds -gt 60) {{ throw 'the processes never changed' }}; \
           Start-Sleep -Milliseconds 100; $now = Get-Under }} until ({changed})"
    )
}

/// A stopped session whose command still runs, owned by `claude`, its
/// transcript at `<dir>/s.jsonl`: the hold stands on the process alone.
fn running_command(dir: &std::path::Path, claude: &FakeClaude, start: u64) {
    let transcript = dir.join("s.jsonl");
    std::fs::write(&transcript, format!("{}\n", super::REPLY)).expect("a transcript");
    std::fs::write(
        dir.join(claim_file("s")),
        format!(
            "r1 idle {} {} a 0 -:toolu_b:Bash:x:{start}:0:0 - {}",
            claude.pid(),
            crate::awake::now_ms(),
            transcript.display()
        ),
    )
    .expect("the claim");
}

/// A hold that stands on a process doing nothing lets go once nothing has
/// happened for the bound, and stays let go however long nothing more
/// does; a hook, or the transcript written, holds it again.
#[test]
fn a_quiet_hold_lets_go_past_its_bound_until_something_happens() {
    let dir = scratch("quiet");
    let mut claude = FakeClaude::start(&dir);
    let start = crate::awake::now_ms();
    claude.run();
    running_command(&dir, &claude, start);
    let hook = dir.join("hooks-r99");
    std::fs::write(&hook, "0").expect("a build's hook stamp");
    let steps = [
        a_window_on(),
        quiet_a_window_on(),
        "[PggAwake]::Clock += 600000".to_string(),
        touched(&hook),
        quiet_a_window_on(),
        touched(&dir.join("s.jsonl")),
    ];
    let said = waits(&dir, &shipped(&dir, crate::awake::REVISION, ""), &steps);
    asked(&said, &[true, true, false, false, true, false, true]);
    let quiet = released_quiet();
    assert_eq!(
        super::told(&dir),
        [
            "holding process:s",
            &quiet,
            "holding process:s",
            &quiet,
            "holding process:s",
            "quit -"
        ]
    );
}

/// A counted process at work — its child taking a processor — is
/// something happening, however long no hook and no transcript says so.
#[test]
fn a_counted_process_at_work_is_something_happening() {
    let dir = scratch("quiet-busy");
    let mut claude = FakeClaude::start(&dir);
    let start = crate::awake::now_ms();
    let child = claude.run_busy();
    // The holder's first judgement takes the busy process for one started,
    // so what the step after it makes of the process is its work.
    crate::wait::until(
        "the busy process under the child",
        || {
            super::powershell(&format!(
                "@(Get-CimInstance Win32_Process -Filter \"ParentProcessId={child} AND Name='powershell.exe'\").Count"
            ))
            .trim()
            .to_string()
        },
        |count| !count.is_empty() && count != "0",
    );
    running_command(&dir, &claude, start);
    let busy = TICK.as_millis() * u128::from(BUSY) / 100 + 100;
    let grown = format!(
        "(($now.Keys | Where-Object {{ $first.ContainsKey($_) }} | ForEach-Object {{ $now[$_] - $first[$_] }} | Measure-Object -Sum).Sum -gt {busy})"
    );
    let steps = [
        a_window_on(),
        format!(
            "{}; {}",
            once_the_processes(claude.pid(), start, &grown),
            quiet_a_window_on()
        ),
    ];
    let said = waits(&dir, &shipped(&dir, crate::awake::REVISION, ""), &steps);
    asked(&said, &[true, true, true]);
    assert_eq!(super::told(&dir), ["holding process:s", "quit -"]);
}

/// A counted command that starts process after process — a loop waiting
/// on another seat's landing, polling — is something happening, however
/// little processor it takes.
#[test]
fn a_counted_command_starting_processes_is_something_happening() {
    let dir = scratch("quiet-polling");
    let mut claude = FakeClaude::start(&dir);
    let start = crate::awake::now_ms();
    claude.run_polling();
    running_command(&dir, &claude, start);
    let started = "@($now.Keys | Where-Object { -not $first.ContainsKey($_) }).Count -gt 0";
    let steps = [
        a_window_on(),
        format!(
            "{}; {}",
            once_the_processes(claude.pid(), start, started),
            quiet_a_window_on()
        ),
    ];
    let said = waits(&dir, &shipped(&dir, crate::awake::REVISION, ""), &steps);
    asked(&said, &[true, true, true]);
    assert_eq!(super::told(&dir), ["holding process:s", "quit -"]);
}

/// A claim changed while the holder reads — the reading done again — is
/// something happening, and the bound counts from it.
#[test]
fn a_claim_changed_under_a_reading_is_something_happening() {
    let dir = scratch("quiet-changed");
    let mut claude = FakeClaude::start(&dir);
    let start = crate::awake::now_ms();
    claude.run();
    running_command(&dir, &claude, start);
    let claim = dir
        .join(claim_file("s"))
        .display()
        .to_string()
        .replace('\'', "''");
    let changing = format!(
        "if ($script:change) {{ $script:change = $false; \
           [IO.File]::WriteAllText('{claim}', ([IO.File]::ReadAllText('{claim}') -replace ' a 0 ', ' a 1 ')) }}"
    );
    let back = format!("$script:lastActive -= {}", past_the_bound());
    let steps = [format!("{back}; $script:change = $true"), back];
    let said = waits(
        &dir,
        &shipped(&dir, crate::awake::REVISION, &changing),
        &steps,
    );
    asked(&said, &[true, true, false]);
    assert_eq!(
        super::told(&dir),
        [
            "holding process:s",
            &format!("released quiet:{}m", past_the_bound() / 60_000),
            "quit -"
        ]
    );
}
