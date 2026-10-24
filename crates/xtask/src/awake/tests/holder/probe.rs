//! The shipped holder's script run as a probe: its source as a holder of
//! a revision runs it, and a run of it with the OS request recorded, its
//! clock and commands as a test sets them.

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
        .replace("@CODEX_LOG@", crate::awake::codex::log::SCRIPT)
        .replace("@DIR@", &dir.display().to_string().replace('\'', "''"))
        .replace("@REVISION@", &revision.to_string())
        .replace(
            "@CODEX_HANDOFF@",
            &crate::awake::CODEX_HANDOFF_REVISION.to_string(),
        )
        .replace("@FORMAT@", "1")
        .replace("@RESERVED@", "0")
        .replace("@STARTING@", "starting")
        .replace("@DELEGATED@", "'Agent', 'Task'")
        .replace("@STARTS_PROCESSES@", "'Bash', 'PowerShell', 'Monitor'")
        .replace("@TTL@", "3600")
        .replace("@STOPPING@", &crate::awake::STOPPING.as_secs().to_string())
        .replace("@QUIET@", &crate::awake::QUIET.as_secs().to_string())
        .replace("@BUSY@", &crate::awake::holder::BUSY.to_string())
        .replace(
            "@BUSY_WINDOW@",
            &crate::awake::holder::TICK.as_millis().to_string(),
        )
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
/// day, the one that only runs forward, and the one that counts the time
/// awake (the forward one in a probe, which sleeps through nothing).
pub(super) const REAL_CLOCK: &str = "public static long Now() { return System.DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(); } \
     public static long Ticks() { return System.Diagnostics.Stopwatch.GetTimestamp() / (System.Diagnostics.Stopwatch.Frequency / 1000); } \
     public static long Awake() { return Ticks(); }";

/// Runs `body` — the shipped holder — as the holder of `revision` in
/// `dir`, with the OS request recorded in place of asked (`Last`, the last
/// request set), its clock read off `clock`, and the commands in `stubs`
/// replaced; the probe ends where a stub throws `PROBE_END`, and answers
/// what it printed. The recorder keeps the thread's request (`Held`), set
/// to begin with as a holding thread's, and answers it as Windows does — a
/// call without `ES_CONTINUOUS` changes nothing and answers it — and what
/// the system counts, in the shapes a test sets:
/// `Dropped`, the thread's request uncounted until the request changes;
/// `Stuck`, uncounted however it changes; `Deaf`, a count that shows no
/// request at all. A request kept on a thread of its own (`Kept`) is
/// counted unless the count is deaf.
pub(super) fn probe(
    dir: &std::path::Path,
    revision: u32,
    clock: &str,
    stubs: &str,
    body: &str,
) -> String {
    let script = format!(
        r#"$ErrorActionPreference = 'Stop'
Add-Type 'public static class PggAwake {{ public static uint Last=2147483649; public static uint Held=2147483649; public static bool Dropped; public static bool Stuck; public static bool Deaf; public static bool Kept; public static uint SetThreadExecutionState(uint flags) {{ uint was=Held; if ((flags & 2147483648u) != 0) {{ if (flags != Held) {{ Dropped=false; }} Held=flags; Last=flags; }} return was; }} public static uint SystemState() {{ if (Deaf) {{ return 0u; }} if (Kept) {{ return 1u; }} return (Dropped || Stuck) ? 0u : Held & 1u; }} public static void Keep() {{ Kept=true; }} public static void Unkeep() {{ Kept=false; }} {clock} }}'
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
