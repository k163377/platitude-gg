//! Native hook activity and Desktop log fixtures meet in the shipped holder.
//! No test reads the user's logs or changes their environment settings.

use super::{FakeClaude, holding, next_reading, powershell, released, scratch};
use crate::awake::{Mark, Whose, claim_name, codex::apply_marks};
use std::path::Path;

fn mark(dir: &Path, codex: &FakeClaude, session: &str, who: Option<&str>, marks: &[Mark<'_>]) {
    let mut writer = codex.hook();
    writer.who = who.unwrap_or("-").into();
    apply_marks(
        dir,
        session,
        who.map_or(Whose::Session, Whose::Agent),
        &writer,
        marks,
    )
    .expect("native Codex activity");
}

fn start(id: &str) -> Mark<'_> {
    Mark::ToolStart {
        id,
        name: "CodexShell",
        input: "input",
    }
}

fn log_root(dir: &Path) -> std::path::PathBuf {
    let root = dir.join("codex-logs");
    std::fs::create_dir_all(&root).expect("fixture log root");
    std::fs::write(
        dir.join("codex-log.path"),
        root.to_string_lossy().as_bytes(),
    )
    .expect("fixture-only log location");
    std::fs::remove_file(dir.join("codex-log.off")).expect("enable fixture supplement");
    root
}

fn emit(root: &Path, message: &str) {
    let root = root.to_string_lossy().replace('\'', "''");
    let message = message.replace('\'', "''");
    powershell(&format!(
        "$day=Join-Path '{root}' ([DateTime]::UtcNow.ToString('yyyy/MM/dd')); \
         [void][IO.Directory]::CreateDirectory($day); \
         $path=Join-Path $day 'codex-desktop-fixture-100-t0-i1-123456-0.log'; \
         [IO.File]::AppendAllText($path, [DateTimeOffset]::UtcNow.ToString('o') + \
         ' debug [conversation-manager] {message}' + [char]10)"
    ));
}

fn ask(root: &Path, request: u32, thread: &str, item: &str) {
    emit(
        root,
        &format!(
            r#"Received server request id={request} method=item/commandExecution/requestApproval params={{"threadId":"{thread}","itemId":"{item}","command":"command with spaces and \\\"quotes\\\""}} rendererWebContentsId=1"#
        ),
    );
}

fn answer(root: &Path, request: u32) {
    emit(
        root,
        &format!(
            r#"Sending server response id={request} method=item/commandExecution/requestApproval rendererWebContentsId=1 response={{"decision":"accept"}}"#
        ),
    );
}

#[test]
fn codex_log_approval_resumes_before_result_and_auto_review_never_waits() {
    let dir = scratch("codex-log-approved");
    let root = log_root(&dir);
    let codex = FakeClaude::start_as(&dir, "codex.exe");
    mark(
        &dir,
        &codex,
        "parent",
        None,
        &[Mark::Prompt, start("build")],
    );
    assert_eq!(next_reading(&dir), holding());
    ask(&root, 7, "parent", "build");
    assert_eq!(next_reading(&dir), released(), "actual human approval wait");
    answer(&root, 8);
    assert_eq!(next_reading(&dir), released(), "unrelated response");
    answer(&root, 7);
    assert_eq!(
        next_reading(&dir),
        holding(),
        "approved tool without a result"
    );
    mark(
        &dir,
        &codex,
        "parent",
        None,
        &[Mark::ToolEnd("build"), Mark::Idle],
    );
    assert_eq!(next_reading(&dir), released());
    mark(&dir, &codex, "parent", None, &[Mark::Prompt, start("auto")]);
    emit(&root, "Automatic approval review started approvalId=review");
    assert_eq!(next_reading(&dir), holding(), "automatic review is work");
}

#[test]
fn codex_children_and_other_chats_do_not_share_log_waits() {
    let dir = scratch("codex-log-children");
    let root = log_root(&dir);
    let codex = FakeClaude::start_as(&dir, "codex.exe");
    mark(&dir, &codex, "parent", None, &[Mark::Prompt, Mark::Idle]);
    for who in ["one", "two"] {
        mark(
            &dir,
            &codex,
            "parent",
            Some(who),
            &[Mark::Working, start("same")],
        );
    }
    assert_eq!(
        next_reading(&dir),
        holding(),
        "idle parent, working children"
    );
    ask(&root, 1, "one", "same");
    assert_eq!(next_reading(&dir), holding(), "second child still runs");
    ask(&root, 2, "two", "same");
    assert_eq!(next_reading(&dir), released(), "all children wait");
    mark(&dir, &codex, "other", None, &[Mark::Prompt, start("same")]);
    assert_eq!(next_reading(&dir), holding(), "another chat runs");
    mark(&dir, &codex, "other", None, &[Mark::Idle]);
    assert_eq!(next_reading(&dir), released());
    answer(&root, 1);
    assert_eq!(next_reading(&dir), holding());
    mark(
        &dir,
        &codex,
        "parent",
        Some("one"),
        &[Mark::CallsEnd, Mark::Gone],
    );
    assert_eq!(next_reading(&dir), released(), "other child still waits");
}

#[test]
fn codex_mcp_waits_are_counted_per_child_and_per_call() {
    let dir = scratch("codex-mcp-children");
    let root = log_root(&dir);
    let codex = FakeClaude::start_as(&dir, "codex.exe");
    mark(&dir, &codex, "parent", None, &[Mark::Idle]);
    for who in ["one", "two"] {
        mark(
            &dir,
            &codex,
            "parent",
            Some(who),
            &[
                Mark::Working,
                Mark::ToolStart {
                    id: "mcp",
                    name: "mcp__browser__open",
                    input: "input",
                },
            ],
        );
    }
    assert_eq!(next_reading(&dir), holding());
    for (id, who) in [(1, "one"), (2, "two"), (3, "two")] {
        emit(
            &root,
            &format!(
                r#"Received server request id={id} method=mcpServer/elicitation/request params={{"threadId":"{who}","serverName":"browser"}} rendererWebContentsId=1"#
            ),
        );
    }
    assert_eq!(
        next_reading(&dir),
        released(),
        "both children wait on the same server"
    );
    emit(
        &root,
        r#"Sending server response id=2 method=mcpServer/elicitation/request rendererWebContentsId=1 response={"action":"accept","content":{}}"#,
    );
    assert_eq!(
        next_reading(&dir),
        released(),
        "the second dialog of the call still waits"
    );
    emit(
        &root,
        r#"Sending server response id=3 method=mcpServer/elicitation/request rendererWebContentsId=1 response={"action":"accept","content":{}}"#,
    );
    assert_eq!(
        next_reading(&dir),
        holding(),
        "one child resumed before its result"
    );
}

#[test]
fn codex_namespace_and_process_attribution_remain_separate() {
    use crate::awake::{apply, end_session};
    let dir = scratch("codex-isolation");
    let claude = FakeClaude::start(&dir);
    let mut codex = FakeClaude::start_as(&dir, "codex.exe");
    let r = claim_name("same-session", None);
    apply(&dir, &r, &claude.hook(), Mark::Prompt).expect("Claude prompt");
    mark(&dir, &codex, "same-session", None, &[Mark::Prompt]);
    end_session(&dir, "same-session").expect("Claude's session ends");
    assert!(dir.join("c1.same-session.claim").exists());
    assert_eq!(next_reading(&dir), holding());
    mark(&dir, &codex, "same-session", None, &[start("other")]);
    codex.run();
    mark(&dir, &codex, "same-session", None, &[Mark::Idle]);
    assert_eq!(
        next_reading(&dir),
        released(),
        "shared process children are not chat work"
    );
    apply(&dir, &r, &claude.hook(), Mark::Prompt).expect("Claude resumes");
    assert_eq!(next_reading(&dir), holding());
}

#[test]
fn each_namespace_requires_its_own_runtime_process() {
    use crate::awake::apply;
    let dir = scratch("codex-owner");
    let claude = FakeClaude::start(&dir);
    let codex = FakeClaude::start_as(&dir, "codex.exe");
    apply(&dir, "c1.wrong.claim", &claude.hook(), Mark::Prompt).expect("wrong Codex owner");
    apply(
        &dir,
        &claim_name("wrong", None),
        &codex.hook(),
        Mark::Prompt,
    )
    .expect("wrong Claude owner");
    assert_ne!(next_reading(&dir), holding());
    assert!(!dir.join("c1.wrong.claim").exists());
    assert!(!dir.join(claim_name("wrong", None)).exists());
}

#[test]
fn codex_log_bindings_are_retried_after_changed_or_unreadable_claims() {
    for unreadable in [false, true] {
        let dir = scratch("codex-snapshot");
        let root = log_root(&dir);
        let codex = FakeClaude::start_as(&dir, "codex.exe");
        let claim = dir.join("c1.s.claim");
        let pid = codex.pid();
        let start = crate::awake::now_ms() - 5000;
        std::fs::write(
            &claim,
            format!("r1 working {pid} {start} a 0 -:-.item:CodexShell:x:{start}:0:0 - -"),
        )
        .expect("initial call epoch");
        let quoted = |path: &Path| path.to_string_lossy().replace('\'', "''");
        let race = format!(
            r#"
if (-not $script:raced) {{
  $script:raced=$true
  $t=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
  [IO.File]::WriteAllText('{claim}',"r1 working {pid} $t a 0 -:-.item:CodexShell:x:${{t}}:0:0 - -")
  $day=Join-Path '{root}' ([DateTime]::UtcNow.ToString('yyyy/MM/dd'))
  [void][IO.Directory]::CreateDirectory($day)
  $path=Join-Path $day 'codex-desktop-fixture-100-t0-i1-123456-0.log'
  $line=[DateTimeOffset]::FromUnixTimeMilliseconds($t).ToString('o')+
    ' debug [conversation-manager] Received server request id=1 method=item/commandExecution/requestApproval params={{"threadId":"s","itemId":"item"}} rendererWebContentsId=1'
  [IO.File]::WriteAllText($path,$line+[char]10)
}}
"#,
            claim = quoted(&claim),
            root = quoted(&root)
        );
        let revision = crate::awake::REVISION;
        let mut body = super::coordination::shipped(&dir, revision, &race)
            .replace("$holding = $false", "$holding = $true");
        let mut stubs = String::from(
            r#"
$script:readings=0
function Wait-Event {
  param($Timeout)
  $script:readings++
  [Console]::WriteLine("READING$($script:readings)="+[PggAwake]::Last+' '+$codexLog.Status)
  if ($script:readings -lt 2) { return 'claim changed' }
  throw 'PROBE_END'
}
"#,
        );
        if unreadable {
            body = body.replace("function Test-Named {", "function Test-NamedShipped {");
            stubs.push_str(
                r#"
$script:named=0
function Test-Named {
  $script:named++
  if ($script:named -eq 2) { throw [IO.IOException]::new('fixture name read refused') }
  return (Test-NamedShipped)
}
"#,
            );
        }
        let said = super::coordination::probe(
            &dir,
            revision,
            super::coordination::REAL_CLOCK,
            &stubs,
            &body,
        );
        assert!(
            said.contains("READING1=2147483649 watching"),
            "stale reading released work: {said}"
        );
        assert!(
            said.contains("READING2=2147483648 watching"),
            "request did not bind the new epoch: {said}"
        );
    }
}
