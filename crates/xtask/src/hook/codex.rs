//! Codex lifecycle events share sleep activity and the landing permit, and
//! a shell line that writes main by hand or takes its git out from under
//! the gate is refused (`codex_git`).
//! Seat claims remain conversation-owned throughout those events.

use super::payload::{deny, own_string_field, own_value_field};
use crate::awake::{Mark, codex::record, digest};

pub(super) fn run(input: &str) {
    if let Err(error) = super::codex_land::prompt(input) {
        eprintln!("Codex landing permit: {error}");
    }
    let refused = super::codex_git::refusal(input);
    if let Some(reason) = &refused {
        deny(reason);
    }
    let (Some(session), Some(cwd), Some(event)) = (
        own_string_field(input, "session_id"),
        own_string_field(input, "cwd"),
        own_string_field(input, "hook_event_name"),
    ) else {
        return;
    };
    if event == "SessionStart"
        && let Err(error) = crate::gate::hooks::install(std::path::Path::new(&cwd))
    {
        eprintln!("The gate's git hook could not be installed: {error}");
    }
    let agent = own_string_field(input, "agent_id");
    if matches!(event.as_str(), "SubagentStart" | "SubagentStop") && agent.is_none() {
        return;
    }
    let id = own_string_field(input, "tool_use_id");
    let name = own_string_field(input, "tool_name").unwrap_or_default();
    let name = activity_name(&name);
    let digest = digest(own_value_field(input, "tool_input").unwrap_or(""));
    let mut marks = Vec::new();
    match event.as_str() {
        "SessionStart" | "Stop" | "Interrupt" => marks.push(Mark::Idle),
        "SessionEnd" => marks.push(Mark::Gone),
        "SubagentStart" => marks.push(Mark::Working),
        "SubagentStop" => {
            marks.push(Mark::CallsEnd);
            marks.push(Mark::Gone);
        }
        "UserPromptSubmit" => marks.push(Mark::Prompt),
        "PreToolUse" => {
            marks.push(Mark::Working);
            // A refused call never runs, and no PostToolUse would close it.
            if let Some(id) = id.as_deref().filter(|_| refused.is_none()) {
                marks.push(Mark::ToolStart {
                    id,
                    name,
                    input: &digest,
                });
                if asks_person(name) {
                    marks.push(Mark::Asked {
                        id: Some(id),
                        name,
                        input: &digest,
                    });
                }
            }
        }
        "PostToolUse" => {
            marks.push(Mark::Working);
            if let Some(id) = id.as_deref() {
                marks.push(Mark::ToolEnd(id));
            }
        }
        // This hook precedes approval policy and can be allowed without
        // asking anyone. Only an actual Desktop server request proves a wait.
        "PermissionRequest" => return,
        _ => return,
    }
    // Activity is advisory; a failed power-control hook cannot veto work.
    if let Err(error) = record(&session, &cwd, agent.as_deref(), &marks) {
        eprintln!("Codex sleep activity: {error}");
    }
}

fn asks_person(name: &str) -> bool {
    matches!(
        name,
        "request_user_input" | "functions.request_user_input" | "AskUserQuestion" | "ExitPlanMode"
    )
}

/// Codex's process serves multiple chats. Its unrelated children cannot
/// be attributed by Claude's per-process shell time spans. Unified exec's
/// call remains open until its completion hook, including across polls.
fn activity_name(name: &str) -> &str {
    match name {
        "Bash" | "PowerShell" | "Monitor" => "CodexShell",
        "spawn_agent" | "collaboration.spawn_agent" | "wait_agent" | "collaboration.wait_agent" => {
            "Agent"
        }
        _ => name,
    }
}

#[cfg(test)]
mod tests {
    use super::{activity_name, asks_person};

    #[test]
    fn session_start_installs_the_git_hook_in_a_fresh_clone() {
        let yard = crate::yard::Yard::new("codex-install");
        let root = yard.to_str().unwrap();
        let git = |args: &[&str]| crate::subprocess::git_query(root, args);
        assert!(git(&["init"]).is_some());
        std::fs::create_dir_all(yard.join(".githooks")).unwrap();
        std::fs::write(
            yard.join(".githooks/reference-transaction"),
            include_str!("../../../../.githooks/reference-transaction"),
        )
        .unwrap();
        assert!(git(&["config", "--get", "core.hooksPath"]).is_none());
        let input = format!(
            r#"{{"session_id":"codex-install-test","cwd":"{}","hook_event_name":"SessionStart"}}"#,
            root.replace('\\', "/")
        );
        super::run(&input);
        let installed = git(&["config", "--get", "core.hooksPath"]).unwrap();
        assert!(
            std::path::Path::new(&installed)
                .join("reference-transaction")
                .is_file()
        );
        super::run(&input);
        assert_eq!(
            git(&["config", "--get", "core.hooksPath"]).as_deref(),
            Some(installed.as_str())
        );
    }

    #[test]
    fn native_prompt_registers_the_owned_seat_and_closes_on_the_next_message() {
        let yard = crate::yard::Yard::new("codex-land");
        let root = yard.to_str().unwrap();
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git")
                .arg("-C")
                .arg(root)
                .args(["-c", "user.name=test", "-c", "user.email=test@example.test"])
                .args(args)
                .output()
                .unwrap();
            assert!(out.status.success(), "{out:?}");
        };
        git(&["init"]);
        git(&["commit", "--allow-empty", "-m", "init"]);
        let seat = yard.join(".claude/worktrees/c");
        git(&[
            "worktree",
            "add",
            "-b",
            "worktree-c",
            seat.to_str().unwrap(),
        ]);
        git(&[
            "worktree",
            "lock",
            "--reason",
            "claude-seat codex-test",
            seat.to_str().unwrap(),
        ]);
        let input = |turn: &str, prompt: &str| {
            format!(
                r#"{{"session_id":"codex-test","cwd":"{}","hook_event_name":"UserPromptSubmit","turn_id":"{turn}","prompt":"{prompt}"}}"#,
                root.replace('\\', "/")
            )
        };
        super::run(&input("first", "/land"));
        assert!(super::super::permit::is_open(root, "codex-test"));
        let entry = super::super::permit::kept_file(root, "codex-test", "line").unwrap();
        let line = std::fs::read_to_string(&entry).unwrap();
        assert!(line.contains(&seat.to_string_lossy().replace('\\', "/")));
        super::super::permit::landed(root, "codex-test");
        super::run(&input("first", "/land"));
        assert!(!super::super::permit::is_open(root, "codex-test"));
        super::run(&input("second", "/land"));
        assert!(super::super::permit::is_open(root, "codex-test"));
        super::run(&input("third", "continue"));
        assert!(!super::super::permit::is_open(root, "codex-test"));
        assert!(!entry.exists());
        super::run(&input("first", "/land"));
        let replay_closed = !super::super::permit::is_open(root, "codex-test");
        let message_time = fallback(&yard, root);
        assert_eq!((replay_closed, message_time), (true, true));
    }

    fn fallback(yard: &std::path::Path, root: &str) -> bool {
        use super::super::{codex_land::sync_from, permit};
        let sessions = yard.join("sessions");
        std::fs::create_dir(&sessions).unwrap();
        let file = sessions.join("rollout-codex-test.jsonl");
        let at = crate::land::line::now() / 1000 + 2;
        let write = |turn: &str, prompt: &str, time: u64| {
            std::fs::write(&file, format!(
                "{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"codex-test\",\"thread_source\":\"user\"}}}}\n{{\"type\":\"response_item\",\"payload\":{{\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\"text\":\"{prompt}\"}}],\"internal_chat_message_metadata_passthrough\":{{\"turn_id\":\"{turn}\",\"create_time\":{time},\"content_item_metadata\":[{{\"provenance\":{{\"type\":\"user\"}}}}]}}}}}}\n"
            )).unwrap();
        };
        write("fallback", "/land", at);
        sync_from(root, "codex-test", &sessions).unwrap();
        assert!(permit::is_open(root, "codex-test"));
        let message_time =
            permit::asked(root, "codex-test").unwrap().millis() == u128::from(at) * 1000;
        crate::land::line::leave(root, "codex-test", "review decision");
        let entry = permit::kept_file(root, "codex-test", "line").unwrap();
        let left = std::fs::read_to_string(&entry).unwrap();
        sync_from(root, "codex-test", &sessions).unwrap();
        assert_eq!(std::fs::read_to_string(&entry).unwrap(), left);
        permit::wait_for_approval(root, "codex-test");
        sync_from(root, "codex-test", &sessions).unwrap();
        assert!(!permit::is_open(root, "codex-test"));
        write("next", "/land", at + 1);
        sync_from(root, "codex-test", &sessions).unwrap();
        permit::landed(root, "codex-test");
        sync_from(root, "codex-test", &sessions).unwrap();
        assert!(!permit::is_open(root, "codex-test"));
        write("old", "/land", at - 1);
        assert!(sync_from(root, "codex-test", &sessions).is_err());
        write("last", "continue", at + 2);
        sync_from(root, "codex-test", &sessions).unwrap();
        assert!(!permit::is_open(root, "codex-test"));
        assert!(!entry.exists());
        message_time
    }

    #[test]
    fn shell_children_are_not_attributed_across_codex_chats() {
        assert_eq!(activity_name("Bash"), "CodexShell");
        assert_eq!(activity_name("mcp__x__y"), "mcp__x__y");
        assert!(asks_person("request_user_input"));
        assert!(!asks_person("request_user_input_async"));
    }
}
