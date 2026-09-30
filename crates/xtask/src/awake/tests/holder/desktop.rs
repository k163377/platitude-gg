//! Desktop log fixtures preserve the observed 2026-09-29 grammar; IDs are
//! anonymized. Tests own their log files and never read the user's logs.

use super::{FakeClaude, claim_file, scratch};

const FIXTURE: &str = r#"
$stamp='2026-09-29 18:58:35'
$mapped='2026-09-29 18:58:34 [info] Mapping internal session local_desktop to CLI session s'
$emitted='2026-09-29 18:58:35 [info] Emitted tool permission request request1 for browser:open_file in session local_desktop'
$answered='2026-09-29 19:42:04 [info] Received permission response for request1: once (tool: browser:open_file)'
$at=([DateTimeOffset]::new([DateTime]::ParseExact($stamp, 'yyyy-MM-dd HH:mm:ss', [Globalization.CultureInfo]::InvariantCulture))).ToUnixTimeMilliseconds()
function Candidate($session, $who, $id, $name, $asked, $start) {
  return @{ Key="r1.$session/$who/$id/$start"; Session=$session; Part=@($who,$id,$name,'input',"$start",'0',"$asked") }
}
function Check($condition, $message) { if (-not $condition) { throw $message } }
function Append($text) { [IO.File]::AppendAllText($path, $text + [char]10) }
function Asked($candidate) { return Get-DesktopAsked $state "r1.$($candidate.Session)" $candidate.Part }
$path=Join-Path $dir 'main.log'
$state=New-DesktopLog $path
$call=Candidate 's' '-' 'call1' 'mcp__Claude_Browser__navigate' '0' ($at-1000)
Append $mapped
Append $emitted
"#;

fn probe(body: &str) {
    let dir = scratch("desktop-log");
    std::fs::remove_file(dir.join(crate::awake::DESKTOP_OFF)).expect("enable fixture log");
    let script = format!(
        "$ErrorActionPreference='Stop'\n{}\n$dir='{}'\n{}\ntry {{\n{body}\n}} finally {{ Close-DesktopLog $state }}",
        crate::awake::desktop::SCRIPT,
        dir.display().to_string().replace('\'', "''"),
        FIXTURE,
    );
    let path = dir.join("probe.ps1");
    std::fs::write(&path, script).expect("the dedicated probe");
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-File"])
        .arg(path)
        .output()
        .expect("the log probe runs");
    assert!(
        out.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn a_desktop_dialog_and_split_response_override_only_the_matched_call() {
    probe(
        r#"
$other=Candidate 't' '-' 'call2' 'mcp__Claude_Browser__navigate' '0' ($at-1000)
Update-DesktopLog $state @($call,$other) $at
Check (Asked $call) $state.Status
Check (-not (Asked $other)) 'a different session was released'
[IO.File]::AppendAllText($path, $answered.Substring(0,60))
Update-DesktopLog $state @($call,$other) ($at+1)
Check (Asked $call) 'an incomplete response resumed work'
Append $answered.Substring(60)
Update-DesktopLog $state @($call,$other) ($at+2)
Check (-not (Asked $call)) $state.Status
Check ($call.Part[6] -eq '0') 'hook facts changed'
Update-DesktopLog $state @() ($at+3)
$new=Candidate 's' '-' 'call3' 'mcp__Claude_Browser__navigate' '1' ($at+2000)
Update-DesktopLog $state @($new) ($at+3000)
Check (Asked $new) 'an old grant resumed a subsequent call'
"#,
    );
}

#[test]
fn approval_resumes_childless_work_but_is_not_an_answer_to_a_question() {
    probe(
        r#"
$fetch=Candidate 's' '-' 'fetch1' 'WebFetch' '1' ($at-1000)
$question=Candidate 's' '-' 'q1' 'AskUserQuestion' '1' ($at-1000)
Append ($emitted.Replace('request1','fetch-request').Replace('browser:open_file','WebFetch'))
Append ($answered.Replace('request1','fetch-request').Replace('browser:open_file','WebFetch').Replace(': once ',': always '))
Append ($emitted.Replace('request1','question-request').Replace('browser:open_file','AskUserQuestion'))
Append ($answered.Replace('request1','question-request').Replace('browser:open_file','AskUserQuestion'))
Update-DesktopLog $state @($fetch,$question) $at
Check (-not (Asked $fetch)) $state.Status
Check (Asked $question) 'permission was confused with the answer to a question'
Check ($fetch.Part[6] -eq '1') 'permission hook evidence was overwritten'
"#,
    );
}

#[test]
fn ambiguous_subagents_and_unknown_decisions_never_guess_a_call() {
    probe(
        r#"
$other=Candidate 's' 'agent1' 'call2' 'mcp__Claude_Browser__navigate' '0' ($at-1000)
Update-DesktopLog $state @($call,$other) $at
Check (-not (Asked $call)) 'ambiguous parent released'
Check (-not (Asked $other)) 'ambiguous subagent released'
Update-DesktopLog $state @($call) ($at+1)
Check (-not (Asked $call)) 'ambiguity was silently reassigned after one call ended'
Append ($emitted.Replace('request1','unknown-request'))
Append ($answered.Replace('request1','unknown-request').Replace(': once ',': new-value '))
Update-DesktopLog $state @($call) ($at+2)
Check (-not (Asked $call)) 'unknown decision changed hook behavior'
Check ($state.Status -match 'unknown=1') $state.Status
"#,
    );
}

#[test]
fn rotation_larger_replacement_and_truncation_preserve_only_available_evidence() {
    probe(
        r#"
Update-DesktopLog $state @($call) $at
Check (Asked $call) $state.Status
Move-Item -LiteralPath $path -Destination (Join-Path $dir 'main1.log')
Append ('unrelated padding ' * 1000)
Append $answered
Update-DesktopLog $state @($call) ($at+1)
Check (-not (Asked $call)) $state.Status
Check ($state.Files.Count -eq 2) 'renamed file identity was lost'
Remove-Item -LiteralPath (Join-Path $dir 'main1.log')
Move-Item -LiteralPath $path -Destination (Join-Path $dir 'old.log')
Append ('replacement padding ' * 2000)
Append $mapped
Append $emitted
Update-DesktopLog $state @($call) ($at+2)
Check (Asked $call) 'larger replacement was mistaken for an append'
Check ($state.Status -match 'log-gap-replayed') $state.Status
[IO.File]::WriteAllText($path, '')
Append $mapped
Update-DesktopLog $state @($call) ($at+3)
Check (-not (Asked $call)) 'truncation retained an unavailable pending request'
"#,
    );
}

#[test]
fn unavailable_logs_disable_and_format_drift_restore_hook_behavior_with_diagnostics() {
    probe(
        r#"
Update-DesktopLog $state @($call) $at
Check (Asked $call) $state.Status
Remove-Item -LiteralPath $path
Update-DesktopLog $state @($call) ($at+1)
Check (-not (Asked $call)) 'missing log kept a stale waiting overlay'
Check ($state.Status -match '^fallback:') $state.Status
Append $mapped
Append $emitted
Update-DesktopLog $state @($call) ($at+2)
Check (Asked $call) 'the reader did not recover'
[IO.File]::WriteAllText((Join-Path $dir 'desktop-log.off'), '')
Update-DesktopLog $state @($call) ($at+3)
Check (-not (Asked $call)) 'disabled supplement still applied'
Check ($state.Status -eq 'disabled') $state.Status
Remove-Item -LiteralPath (Join-Path $dir 'desktop-log.off')
Append '2026-09-29 19:42:04 [info] Received permission response changed-format'
Update-DesktopLog $state @($call) ($at+4)
Check (-not (Asked $call)) 'changed log format left a stale overlay'
Check ($state.Status -match 'unrecognized-log-format') $state.Status
[IO.File]::WriteAllText($path, $mapped + [char]10 + $emitted + [char]10)
Update-DesktopLog $state @($call) ($at+5)
Check (Asked $call) 'a corrected log could not recover from format drift'
"#,
    );
}

#[test]
fn missing_permission_events_are_visible_without_disabling_other_sessions() {
    probe(
        r#"
$fetch=Candidate 't' '-' 'fetch1' 'WebFetch' '1' ($at-1000)
Update-DesktopLog $state @($fetch,$call) $at
Update-DesktopLog $state @($fetch,$call) ($at+5001)
Check (Asked $fetch) 'missing evidence resumed a permission prompt'
Check ($state.Status -match 'unmatched=1') $state.Status
Check (Asked $call) 'one missing event disabled a known request in another session'
"#,
    );
}

#[test]
fn every_dialog_must_resolve_and_late_mapping_still_finds_the_request() {
    probe(
        r#"
[IO.File]::WriteAllText($path, $emitted + [char]10)
Append ($emitted.Replace('request1','request2'))
Update-DesktopLog $state @($call) $at
Check (-not (Asked $call)) 'unmapped session was guessed'
Append $mapped
Append $answered
Update-DesktopLog $state @($call) ($at+1)
Check (Asked $call) 'one grant cleared another dialog'
Append ($answered.Replace('request1','request2'))
Update-DesktopLog $state @($call) ($at+2)
Check (-not (Asked $call)) $state.Status
Append $emitted
Update-DesktopLog $state @($call) ($at+3)
Check (-not (Asked $call)) 'a duplicate request erased its answer'
Close-DesktopLog $state
$state=New-DesktopLog $path
Update-DesktopLog $state @($call) ($at+4)
Check (-not (Asked $call)) 'restart lost the granted state'
"#,
    );
}

#[test]
fn desktop_file_events_drive_real_windows_requests_without_changing_claims() {
    use super::coordination::observed;
    use crate::awake::{Mark, apply};
    use std::io::Write;
    let dir = scratch("desktop-native");
    let log = dir.join("main.log");
    std::fs::write(&log, "").expect("the fixture log");
    std::fs::write(
        dir.join("desktop-log.path"),
        log.to_string_lossy().as_bytes(),
    )
    .expect("the isolated log path");
    std::fs::remove_file(dir.join(crate::awake::DESKTOP_OFF)).expect("enable supplement");
    let claude = FakeClaude::start(&dir);
    let started = crate::awake::now_ms();
    apply(&dir, &claim_file("s"), &claude.hook(), Mark::Prompt).expect("a turn");
    apply(
        &dir,
        &claim_file("s"),
        &claude.hook(),
        Mark::ToolStart {
            id: "browser1",
            name: "mcp__Claude_Browser__navigate",
            input: "fixture",
        },
    )
    .expect("the browser call");
    let revision = crate::awake::REVISION;
    observed(&dir, revision, "holding", started);
    let before = std::fs::read_to_string(dir.join(claim_file("s"))).unwrap();
    apply(&dir, &claim_file("t"), &claude.hook(), Mark::Prompt).expect("parallel work");
    let stamp = super::powershell("Get-Date -Format 'yyyy-MM-dd HH:mm:ss'")
        .trim()
        .to_string();
    let pending = format!(
        "{stamp} [info] Mapping internal session local_native to CLI session s\n{stamp} [info] Emitted tool permission request req_native for browser:open_file in session local_native\n"
    );
    let since = crate::awake::now_ms();
    std::fs::write(&log, &pending).unwrap();
    assert_eq!(super::next_reading(&dir), super::holding());
    apply(&dir, &claim_file("t"), &claude.hook(), Mark::Idle).expect("parallel work ends");
    assert_eq!(observed(&dir, revision, "released", since) & 3, 1);
    let since = crate::awake::now_ms();
    writeln!(
        std::fs::OpenOptions::new().append(true).open(&log).unwrap(),
        "{stamp} [info] Received permission response for req_native: once (tool: browser:open_file)"
    )
    .unwrap();
    assert_eq!(observed(&dir, revision, "holding", since) & 3, 0);
    assert_eq!(
        std::fs::read_to_string(dir.join(claim_file("s"))).unwrap(),
        before
    );
    let since = crate::awake::now_ms();
    apply(&dir, &claim_file("s"), &claude.hook(), Mark::Idle).expect("Stop");
    assert_eq!(observed(&dir, revision, "released", since) & 3, 1);
    apply(&dir, &claim_file("s"), &claude.hook(), Mark::Prompt).expect("a new turn");
    apply(
        &dir,
        &claim_file("s"),
        &claude.hook(),
        Mark::ToolStart {
            id: "fetch1",
            name: "WebFetch",
            input: "fixture",
        },
    )
    .expect("childless tool starts");
    apply(
        &dir,
        &claim_file("s"),
        &claude.hook(),
        Mark::Asked {
            id: Some("fetch1"),
            name: "WebFetch",
            input: "fixture",
        },
    )
    .expect("permission waits");
    assert_eq!(super::next_reading(&dir), super::released());
    let stamp = super::powershell("Get-Date -Format 'yyyy-MM-dd HH:mm:ss'")
        .trim()
        .to_string();
    let since = crate::awake::now_ms();
    writeln!(std::fs::OpenOptions::new().append(true).open(&log).unwrap(), "{stamp} [info] Emitted tool permission request req_fetch for WebFetch in session local_native\n{stamp} [info] Received permission response for req_fetch: once (tool: WebFetch)").unwrap();
    assert_eq!(observed(&dir, revision, "holding", since) & 3, 0);
    let since = crate::awake::now_ms();
    apply(&dir, &claim_file("s"), &claude.hook(), Mark::Idle).expect("childless tool ends");
    assert_eq!(observed(&dir, revision, "released", since) & 3, 1);
}
