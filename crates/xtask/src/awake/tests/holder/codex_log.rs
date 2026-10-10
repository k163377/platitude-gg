//! Installed Desktop's 26.1007.2314.0 grammar, with synthetic IDs and payloads.
//! The observer and setup run against test-owned files/settings only.

use super::scratch;

const FIXTURE: &str = r#"
$root=Join-Path $dir 'logs'
$day=Join-Path $root ([DateTime]::UtcNow.ToString('yyyy/MM/dd'))
[void][IO.Directory]::CreateDirectory($day)
$path=Join-Path $day 'codex-desktop-fixture-100-t0-i1-123456-0.log'
[IO.File]::WriteAllText($path,'')
[IO.File]::Delete((Join-Path $dir 'codex-log.off'))
$state=New-CodexLog $root
$at=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
$method='item/commandExecution/requestApproval'
function Check($condition,$message) { if (-not $condition) { throw $message } }
function Append($text) { [IO.File]::AppendAllText($path,$text+[char]10) }
function Line($message,$when=$at,$renderer=1) {
  return [DateTimeOffset]::FromUnixTimeMilliseconds($when).ToString('o')+
    " debug [conversation-manager] $message rendererWebContentsId=$renderer hostId=local"
}
function Ask($id,$thread,$item,$when=$at,$renderer=1) {
  Append (Line "Received server request id=$id method=$method params={""threadId"":""$thread"",""itemId"":""$item""}" $when $renderer)
}
function Answer($id,$when=$at,$renderer=1) {
  return Line "Sending server response id=$id method=$method response={""decision"":""accept""}" $when $renderer
}
function Candidate($id,$name='CodexShell',$start=($at-1),$asked='0',$thread='s') {
  return New-CodexCandidate "c1.$thread" @('-',$id,$name,'input',"$start",'0',$asked)
}
function Asked($candidate) { return Get-CodexAsked $state "c1.$($candidate.Thread)" $candidate.Part }
$call=Candidate 'item'
"#;

fn run(script: &str) {
    let dir = scratch("codex-log-probe");
    let path = dir.join("probe.ps1");
    let script = script.replace("@DIR@", &dir.to_string_lossy().replace('\'', "''"));
    std::fs::write(&path, script).expect("dedicated probe");
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-File"])
        .arg(path)
        .output()
        .expect("PowerShell probe runs");
    assert!(
        out.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn probe(body: &str) {
    run(&format!(
        "$ErrorActionPreference='Stop'\n$dir='@DIR@'\n{}\n{}\n{FIXTURE}\ntry {{\n{body}\n}} finally {{ Close-CodexLog $state }}",
        crate::awake::desktop::SCRIPT,
        crate::awake::codex::log::SCRIPT,
    ));
}

#[test]
fn codex_log_partial_lines_and_rotation_preserve_request_scope() {
    probe(
        r#"
Ask 1 's' 'item'
Update-CodexLog $state @($call) $at
Check (Asked $call) $state.Status
$answer=Answer 1
[IO.File]::AppendAllText($path,$answer.Substring(0,60))
Update-CodexLog $state @($call) ($at+1)
Check (Asked $call) 'partial answer released work'
$rotated=$path.Replace('-0.log','-1.log')
[IO.File]::Move($path,$rotated)
[IO.File]::AppendAllText($rotated,$answer.Substring(60)+[char]10)
[IO.File]::WriteAllText($path,(Answer 1 $at 2)+[char]10)
Update-CodexLog $state @($call) ($at+2)
Check (-not (Asked $call)) $state.Status
Check ($call.Part[6] -eq '0') 'hook evidence changed'
Check ($state.Status -match 'debug=observed age-ms=2') $state.Status
"#,
    );
}

#[test]
fn codex_log_multiple_roots_deduplicate_store_aliases_and_replay_truncation() {
    probe(
        r#"
$empty=Join-Path $dir 'empty'
[void][IO.Directory]::CreateDirectory($empty)
$state.Root=@($empty,$root,$root)
Ask 1 's' 'item'
Update-CodexLog $state @($call) $at
Check (Asked $call) $state.Status
Check ($state.Files.Count -eq 1 -and $state.Requests.Count -eq 1) 'aliased log files were counted twice'
[IO.File]::WriteAllText($path,'')
Update-CodexLog $state @($call) ($at+1)
Check (-not (Asked $call)) 'truncated evidence released work'
Check ($state.Status -match 'codex-log-gap-replayed') $state.Status
"#,
    );
}

#[test]
fn codex_log_unrelated_answers_and_reused_ids_never_resume_another_request() {
    probe(
        r#"
Ask 1 's' 'item'
Append (Answer 1 $at 2)
Update-CodexLog $state @($call) $at
Check (Asked $call) 'another renderer answered this request'
Append (Answer 1 ($at-100))
Update-CodexLog $state @($call) $at
Check (-not (Asked $call)) 'stale answer was accepted as a wait'
Check ($state.Overlay.Count -eq 0) 'stale answer changed hook facts'
Ask 1 's' 'item' ($at+1)
Append (Answer 1 ($at+2))
Update-CodexLog $state @($call) ($at+3)
Check ($state.Status -match 'ambiguous=1') $state.Status
Check ($state.Overlay.Count -eq 0) 'reused request id changed activity'
"#,
    );
}

#[test]
fn codex_log_binding_uses_item_epoch_and_can_discard_an_unconfirmed_reading() {
    probe(
        r#"
Ask 1 's' 'item'
$other=Candidate 'another'
Update-CodexLog $state @($other) $at
Check ($state.Status -match 'unmatched=1') $state.Status
Check (-not (Asked $other)) 'unmatched ID released another call'
Update-CodexLog $state @($call) $at
Check (Asked $call) 'exact ID did not bind'
Undo-CodexDecisions $state
Check (@($state.Requests.Values)[0].Bound -eq '') 'snapshot race kept a binding'
Update-CodexLog $state @($call) $at
Check (Asked $call) 'rolled-back request did not rebind'
Update-CodexLog $state @() ($at+1)
$later=Candidate 'item' 'CodexShell' ($at+2)
Update-CodexLog $state @($later) ($at+3)
Check (-not (Asked $later)) 'finished request released a later call'
"#,
    );
}

#[test]
fn codex_log_questions_permissions_and_mcp_keep_independent_waits() {
    probe(
        r#"
$method='item/tool/requestUserInput'
$question=Candidate 'item' 'request_user_input' ($at-1) '1'
Ask 1 's' 'item'
Append (Line 'Received server request id=2 method=item/tool/requestUserInput params={"threadId":"s","itemId":"async","isBlocking":false}')
Append (Line 'Sending server response id=1 method=item/tool/requestUserInput response={"answers":{"q":{"answers":["yes"]}}}')
$async=Candidate 'async' 'request_user_input_async'
Update-CodexLog $state @($question,$async) $at
Check (-not (Asked $question)) 'question response did not resume'
Check (-not (Asked $async)) 'asynchronous question blocked work'
$method='item/permissions/requestApproval'
Ask 3 's' 'item'
Update-CodexLog $state @($call) $at
Check (Asked $call) 'permissions request did not wait'
Append (Line 'Sending server response id=3 method=item/permissions/requestApproval response={"permissions":{},"scope":"turn"}')
Update-CodexLog $state @($call) $at
Check (-not (Asked $call)) 'empty permissions response did not resume'
$mcp=Candidate 'mcp' 'mcp__browser__open'
Append (Line 'Received server request id=4 method=mcpServer/elicitation/request params={"threadId":"s","serverName":"browser","mode":"form"}')
Append (Line 'Received server request id=5 method=mcpServer/elicitation/request params={"threadId":"s","serverName":"browser","mode":"openai/userVerification"}')
Update-CodexLog $state @($mcp) $at
$pending=Get-CodexPending $state 'c1.s' '-'
Check ($pending['browser'] -eq 1) 'verification was counted as elicitation'
Check ((Get-CodexPending $state 'c1.other' '-').Count -eq 0) 'MCP wait escaped its chat'
Append (Line 'Sending server response id=4 method=mcpServer/elicitation/request response={"action":"accept","content":{}}')
Update-CodexLog $state @($mcp) $at
Check ((Get-CodexPending $state 'c1.s' '-').Count -eq 0) 'answered elicitation stayed pending'
"#,
    );
}

#[test]
fn codex_log_mcp_waits_never_follow_a_replaced_call_or_a_changed_call_set() {
    probe(
        r#"
$one=Candidate 'one' 'mcp__browser__open'
$two=Candidate 'two' 'mcp__browser__read'
Append (Line 'Received server request id=1 method=mcpServer/elicitation/request params={"threadId":"s","serverName":"browser"}')
Update-CodexLog $state @($one,$two) $at
Check ($state.Status -match 'ambiguous=1') $state.Status
Check ((Get-CodexPending $state 'c1.s' '-').Count -eq 0) 'ambiguous request released another call'
Undo-CodexDecisions $state
Update-CodexLog $state @($one) $at
Check ((Get-CodexPending $state 'c1.s' '-').Count -eq 0) 'ambiguity vanished with an unconfirmed reading'
Append (Line 'Received server request id=2 method=mcpServer/elicitation/request params={"threadId":"s","serverName":"browser"}')
Append (Line 'Received server request id=3 method=mcpServer/elicitation/request params={"threadId":"s","serverName":"browser"}')
Update-CodexLog $state @($one) $at
Check ((Get-CodexPending $state 'c1.s' '-')['browser'] -eq 1) 'multiple dialogs counted as multiple calls'
$later=Candidate 'later' 'mcp__browser__open' ($at+1)
Update-CodexLog $state @($one,$later) ($at+2)
Check ((Get-CodexPending $state 'c1.s' '-')['browser'] -eq 1) 'dialogs released a later concurrent call'
Update-CodexLog $state @($later) ($at+2)
Check ((Get-CodexPending $state 'c1.s' '-').Count -eq 0) 'old elicitation released a new MCP call'
"#,
    );
}

#[test]
fn codex_log_mcp_and_item_waits_do_not_subtract_the_same_call_twice() {
    probe(
        r#"
$one=Candidate 'one' 'mcp__browser__open'
$two=Candidate 'two' 'mcp__browser__read'
Append (Line 'Received server request id=1 method=mcpServer/elicitation/request params={"threadId":"s","serverName":"browser"}')
Update-CodexLog $state @($one) $at
Ask 2 's' 'one'
Update-CodexLog $state @($one,$two) $at
Check (Asked $one) 'item-level wait did not bind'
Check (-not (Asked $two)) 'item-level wait released the other call'
Check ((Get-CodexPending $state 'c1.s' '-').Count -eq 0) 'elicitation counted the already-excluded call again'
"#,
    );
}

#[test]
fn codex_log_malformed_missing_and_ambiguous_evidence_falls_back_to_hooks() {
    probe(
        r#"
Ask 1 's' 'item'
$duplicate=Candidate 'item' 'CodexShell' ($at-2)
Update-CodexLog $state @($call,$duplicate) $at
Check ($state.Status -match 'ambiguous=1') $state.Status
Check (-not (Asked $call)) 'ambiguous item released work'
Append (Line 'Received server request id=2 method=item/commandExecution/requestApproval params={"threadId":"s","itemId":"item","command":"spaces { and \\"quotes\\""}')
Update-CodexLog $state @($call) $at
Check (-not $state.Enabled) 'malformed JSON did not discard overlay'
Check ($state.Status.StartsWith('fallback:')) $state.Status
Check (-not (Asked $call)) 'malformed input changed hook evidence'
[IO.File]::Delete($path)
Update-CodexLog $state @($call) $at
Check (-not $state.Enabled) 'missing logs did not fall back'
Check (-not (Asked $call)) 'missing logs released work'
"#,
    );
}

#[test]
fn codex_setup_is_idempotent_restores_exactly_and_preserves_later_edits() {
    run(&format!(
        r#"
$ErrorActionPreference='Stop'
$dir='@DIR@'
{}
function Check($condition,$message) {{ if (-not $condition) {{ throw $message }} }}
$script:level=$null; $script:writes=0
function Read-CodexLevel {{ return $script:level }}
function Write-CodexLevel($value) {{ $script:level=$value; $script:writes++ }}
function Notify-CodexLevel {{}}
Invoke-CodexSetup $dir $false
Invoke-CodexSetup $dir $false
Check ($level -eq 'debug' -and $writes -eq 1) 'setup was not idempotent'
Invoke-CodexSetup $dir $true
Check ($null -eq $level) 'missing value was not restored'
$script:level='info'
Invoke-CodexSetup $dir $false
Invoke-CodexSetup $dir $true
Check ($level -eq 'info') 'previous value was not restored'
Invoke-CodexSetup $dir $false
$script:level='trace'
Invoke-CodexSetup $dir $true
Check ($level -eq 'trace') 'later user change was overwritten'
$script:level='debug'; $before=$writes
Invoke-CodexSetup $dir $false
Invoke-CodexSetup $dir $true
Check ($level -eq 'debug' -and $writes -eq $before) 'pre-existing debug level was taken over'
$script:level='warning'
function Notify-CodexLevel {{ throw 'fixture broadcast failure' }}
$failed=$false
try {{ Invoke-CodexSetup $dir $false }} catch {{ $failed=$true }}
Check $failed 'notification failure was hidden'
Check (Test-Path -LiteralPath (Join-Path $dir 'codex-log-level.previous')) 'failed notification lost restoration ledger'
function Notify-CodexLevel {{}}
Invoke-CodexSetup $dir $true
Check ($level -eq 'warning') 'partially successful setup could not be undone'
"#,
        crate::awake::codex::setup::SCRIPT
    ));
}
