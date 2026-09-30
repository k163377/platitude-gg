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
    OtherBuild(child)
}

fn observed(dir: &std::path::Path, revision: u32, verdict: &str, since: u64) -> u32 {
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

fn once(dir: &std::path::Path, revision: u32, during_query: &str) -> String {
    let source = crate::awake::holder::SCRIPT;
    let source = &source[source.find("$dir =").expect("the script's directory")..];
    let body = source
        .replace("@DIR@", &dir.display().to_string().replace('\'', "''"))
        .replace("@REVISION@", &revision.to_string())
        .replace("@FORMAT@", "1")
        .replace("@RESERVED@", "0")
        .replace("@STARTING@", "starting")
        .replace("@DELEGATED@", "'Agent', 'Task'")
        .replace("@STARTS_PROCESSES@", "'Bash', 'PowerShell', 'Monitor'")
        .replace("@TTL@", "3600")
        .replace("@SLACK@", "10")
        .replace("@HOLD@", "2147483649")
        .replace("@LET_GO@", "2147483648")
        .replace("@TICK@", "15")
        .replace("$holding = $false", "$holding = $true");
    let script = format!(
        r#"$ErrorActionPreference = 'Stop'
Add-Type 'public static class PggAwake {{ public static uint Last=2147483649; public static uint SetThreadExecutionState(uint flags) {{ Last=flags; return 1; }} public static long Now() {{ return System.DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(); }} }}'
function Get-CimInstance {{
  param($ClassName, $Filter)
  {during_query}
  return @()
}}
function Wait-Event {{
  param($Timeout)
  [Console]::WriteLine('REQUEST=' + [PggAwake]::Last)
  throw 'PROBE_END'
}}
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
