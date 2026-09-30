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

fn once(dir: &std::path::Path, revision: u32, during_query: &str) -> String {
    let source = crate::awake::holder::SCRIPT;
    let source = &source[source.find("$dir =").expect("the script's directory")..];
    let body = source
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
