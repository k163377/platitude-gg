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

/// Desktop names its own dialogs by kind (`browser:open_file`), not by the
/// tool that raised one: `preview_start` and `browser_batch` open files and
/// sites as `navigate` does, and computer use asks for its apps.
#[test]
fn a_desktop_dialog_binds_to_any_call_of_the_server_that_raised_it() {
    probe(
        r#"
function Ask($id, $kind, $second) {
  Append ($emitted.Replace('request1',$id).Replace('browser:open_file',$kind).Replace('18:58:35',"18:58:$second"))
}
$preview=Candidate 's' '-' 'preview1' 'mcp__Claude_Browser__preview_start' '0' ($at-1000)
Update-DesktopLog $state @($preview) $at
Check (Asked $preview) $state.Status
Append $answered
Update-DesktopLog $state @($preview) ($at+1)
Check (-not (Asked $preview)) 'the grant did not resume preview_start'
$batch=Candidate 's' '-' 'batch1' 'mcp__Claude_Browser__browser_batch' '0' ($at+1000)
$access=Candidate 's' '-' 'access1' 'mcp__computer-use__request_access' '0' ($at+1000)
Ask 'site-request' 'browser:open_site' '36'
Update-DesktopLog $state @($batch,$access) ($at+2)
Check (Asked $batch) $state.Status
Check (-not (Asked $access)) 'a browser dialog was bound across servers'
$chrome=Candidate 's' '-' 'chrome1' 'mcp__claude-in-chrome__navigate' '0' ($at+2000)
$access=Candidate 's' '-' 'access2' 'mcp__computer-use__request_access' '0' ($at+2000)
Ask 'chrome-request' 'browser:domain_transition' '37'
Ask 'access-request' 'computer:request_access' '37'
Update-DesktopLog $state @($chrome,$access) ($at+3)
Check (Asked $chrome) $state.Status
Check (Asked $access) $state.Status
$navigate=Candidate 's' '-' 'nav1' 'mcp__Claude_Browser__navigate' '0' ($at+3000)
Ask 'stray-access' 'computer:request_access' '38'
Update-DesktopLog $state @($navigate) ($at+4)
Check (-not (Asked $navigate)) 'a computer-use dialog was bound to a browser call'
Check ($state.Status -match 'unmapped=1') $state.Status
$screen=Candidate 's' '-' 'screen1' 'mcp__computer-use__screenshot' '0' ($at+4000)
$shell=Candidate 's' '-' 'shell1' 'Bash' '0' ($at+4000)
Ask 'stray-file' 'browser:open_file' '39'
Update-DesktopLog $state @($screen,$shell) ($at+5)
Check (-not (Asked $screen)) 'a browser dialog was bound to a computer-use call'
Check (-not (Asked $shell)) 'a browser dialog was bound to a call outside the browser'
Check ($state.Status -match 'unmapped=1') $state.Status
"#,
    );
}

/// A reading binds a dialog from the claims it copied at its start, and a
/// call can start and ask within the reading, after the copy, while the
/// call before it still shows open there — a screenshot, then a navigate
/// that opens a file. What a reading bound from claims that changed under
/// it is bound again from the next reading's.
#[test]
fn a_dialog_bound_from_claims_that_changed_is_bound_again() {
    let said = stale_reading("desktop-stale", false);
    assert!(
        said.contains("READING1=2147483649 watching overlay=1"),
        "the first reading did not bind the screenshot: {said}"
    );
    assert!(
        said.contains("READING2=2147483648 watching overlay=1"),
        "the navigate waiting for a person was counted as work: {said}"
    );
}

/// A reading that could not compare its claims — a reader held the name —
/// never confirmed what it bound either.
#[test]
fn a_dialog_bound_by_a_reading_that_could_not_check_is_bound_again() {
    let said = stale_reading("desktop-unchecked", true);
    assert!(
        said.contains("READING1=2147483649 watching overlay=1"),
        "the first reading did not bind the screenshot: {said}"
    );
    assert!(
        said.contains("READING2=2147483648 watching overlay=1"),
        "the navigate waiting for a person was counted as work: {said}"
    );
}

/// Two readings of the shipped holder over one session: the screenshot's
/// call is open in the claims the first copies; once copied, the navigate
/// replaces it and asks. With `unchecked` the first reading's comparison
/// of its claims fails as an unreadable name does. What each reading asked
/// for and made of the log is printed as `READING<n>=`.
fn stale_reading(stem: &str, unchecked: bool) -> String {
    let dir = scratch(stem);
    std::fs::remove_file(dir.join(crate::awake::DESKTOP_OFF)).expect("enable supplement");
    let log = dir.join("main.log");
    std::fs::write(&log, "").expect("the fixture log");
    std::fs::write(
        dir.join("desktop-log.path"),
        log.to_string_lossy().as_bytes(),
    )
    .expect("the isolated log path");
    let claude = FakeClaude::start(&dir);
    let claim = dir.join(claim_file("s"));
    let start = crate::awake::now_ms() - 5000;
    let pid = claude.pid();
    std::fs::write(
        &claim,
        format!(
            "r1 working {pid} {start} a 0 -:shot:mcp__Claude_Browser__computer:x:{start}:0:0 - -"
        ),
    )
    .expect("the screenshot's call");
    let quoted = |path: &std::path::Path| path.display().to_string().replace('\'', "''");
    let race = format!(
        "if (-not $script:raced) {{ $script:raced = $true; \
         $s = [DateTime]::Now; $t = ([DateTimeOffset]$s).ToUnixTimeMilliseconds(); \
         [IO.File]::WriteAllText('{claim}', \"r1 working {pid} ${{t}} a 0 -:nav:mcp__Claude_Browser__navigate:x:${{t}}:0:0 - -\"); \
         $stamp = $s.ToString('yyyy-MM-dd HH:mm:ss', [Globalization.CultureInfo]::InvariantCulture); \
         [IO.File]::AppendAllText('{log}', \"$stamp [info] Mapping internal session local_s to CLI session s`n$stamp [info] Emitted tool permission request stale for browser:open_file in session local_s`n\") }}",
        claim = quoted(&claim),
        log = quoted(&log),
    );
    let revision = crate::awake::REVISION;
    let mut body = super::coordination::shipped(&dir, revision, &race)
        .replace("$holding = $false", "$holding = $true");
    let mut stubs = String::from(
        "$script:readings = 0\nfunction Wait-Event {\n  param($Timeout)\n  \
         $script:readings++\n  \
         [Console]::WriteLine(\"READING$($script:readings)=\" + [PggAwake]::Last + ' ' + $desktop.Status)\n  \
         if ($script:readings -lt 2) { return 'a claim changed' }\n  throw 'PROBE_END'\n}\n",
    );
    if unchecked {
        assert_eq!(body.matches("function Test-Named {").count(), 1);
        body = body.replace("function Test-Named {", "function Test-NamedShipped {");
        // The first reading's second look at its name, where it compares
        // its claims.
        stubs.push_str(
            "$script:named = 0\nfunction Test-Named {\n  $script:named++\n  \
             if ($script:named -eq 2) { throw [IO.IOException]::new('a reader holds the name') }\n  \
             return (Test-NamedShipped)\n}\n",
        );
    }
    super::coordination::probe(
        &dir,
        revision,
        super::coordination::REAL_CLOCK,
        &stubs,
        &body,
    )
}

/// Ambiguity stands through a reading whose bindings were undone: chosen
/// again as candidates end, it could fall on the call that runs.
#[test]
fn ambiguity_outlives_a_reading_whose_bindings_were_undone() {
    probe(
        r#"
$other=Candidate 's' 'agent1' 'call2' 'mcp__Claude_Browser__navigate' '0' ($at-1000)
Update-DesktopLog $state @($call,$other) $at
Check ($state.Status -match 'ambiguous=1') $state.Status
Undo-DesktopDecisions $state
Update-DesktopLog $state @($other) ($at+1)
Check ($state.Status -match '^watching .*ambiguous=1') $state.Status
Check (-not (Asked $other)) 'undone ambiguity picked the call left'
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
