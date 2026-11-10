//! The shell guards from the outside: what `hook pre-shell` (Claude's) and
//! `hook codex-session` (Codex's) answer a line, through the binary the
//! harness runs and against a repository laid out like this one.

use std::path::Path;

use crate::support::Sandbox;

/// What Codex's PreToolUse hook prints for `command` in `dir` — as Codex
/// hands every shell line over, a `Bash` call — on `event`.
fn codex(sb: &Sandbox, dir: &Path, event: &str, command: &str) -> String {
    sb.hook(
        "codex-session",
        &format!(
            "{{\"session_id\":\"thread-1\",\"cwd\":\"{}\",\"hook_event_name\":\"{event}\",\
             \"turn_id\":\"turn-1\",\"tool_name\":\"Bash\",\"tool_use_id\":\"call-1\",\
             \"tool_input\":{{\"command\":\"{command}\"}}}}",
            dir.display().to_string().replace('\\', "/")
        ),
    )
}

/// The git hook does not see a refspec writing a remote's main, nor a line
/// that takes the thread id off the git it runs: Codex's own hook refuses
/// both, and lets a push of a branch of its own through.
#[test]
fn codex_is_refused_a_hand_written_main_and_a_line_that_unmarks_its_git() {
    let sb = Sandbox::new("shell-codex");
    let refused = |command: &str| {
        let answer = codex(&sb, &sb.seat, "PreToolUse", command);
        assert!(
            answer.contains("\"permissionDecision\":\"deny\""),
            "{command}: {answer}"
        );
        answer
    };
    for command in [
        "git push origin worktree-a:main",
        // Wherever in the line it stands.
        "git rebase main && git push --force origin HEAD:refs/heads/main",
        "git update-ref refs/heads/main worktree-a",
    ] {
        assert!(
            refused(command).contains("would write refs/heads/main"),
            "{command}"
        );
    }
    for command in [
        "CODEX_THREAD_ID= git merge --ff-only worktree-a",
        "PGG_GATE_SKIP=1 git commit -m x",
    ] {
        assert!(
            refused(command).contains("answers to the gate"),
            "{command}"
        );
    }
    for command in [
        "git push origin worktree-a",
        "git log main",
        "echo $CODEX_THREAD_ID",
    ] {
        assert_eq!(codex(&sb, &sb.seat, "PreToolUse", command), "", "{command}");
    }
    // Only before the call: the line has run by the time PostToolUse asks.
    assert_eq!(
        codex(
            &sb,
            &sb.seat,
            "PostToolUse",
            "git push origin worktree-a:main"
        ),
        ""
    );
}

/// What `hook pre-shell` prints for `command`, typed in `dir` by
/// `session`; empty is no objection.
fn pre_shell(sb: &Sandbox, dir: &Path, session: &str, command: &str) -> String {
    sb.hook(
        "pre-shell",
        &format!(
            "{{\"session_id\":\"{session}\",\"cwd\":\"{}\",\
             \"hook_event_name\":\"PreToolUse\",\"tool_name\":\"Bash\",\
             \"tool_input\":{{\"command\":\"{command}\"}}}}",
            dir.display().to_string().replace('\\', "/")
        ),
    )
}

/// Claude's guard answers for every git in the line that writes main, not
/// only the first git it holds back: a refspec writing a remote's main
/// passes no git hook. Windows reads environment names without case, so
/// the gate's names are refused however the line spells them.
#[test]
fn claude_is_refused_main_wherever_the_line_writes_it() {
    let sb = Sandbox::new("shell-claude-main");
    for command in [
        "git merge --ff-only worktree-b && git push origin HEAD:main",
        "PGG_ALLOW_REBASE=1 git rebase main && git push origin HEAD:main",
    ] {
        let answer = pre_shell(&sb, &sb.seat, "claude", command);
        assert!(
            answer.contains("would write refs/heads/main"),
            "{command}: {answer}"
        );
    }
    for command in [
        "$env:pgg_gate_skip='1'; git reset --hard HEAD~1",
        "$env:claudecode=''",
    ] {
        let answer = pre_shell(&sb, &sb.seat, "claude", command);
        assert!(
            answer.contains("answers to the gate"),
            "{command}: {answer}"
        );
    }
}

#[test]
fn a_wait_asked_again_is_held_where_the_work_it_waits_on_is_not() {
    let sb = Sandbox::new("shell-repeat");
    let wait = "tail -2 target/gate.log";

    for _ in 0..3 {
        assert_eq!(pre_shell(&sb, &sb.seat, "waiting", wait), "");
    }
    // The fourth, however it is spelled.
    let held = pre_shell(&sb, &sb.seat, "waiting", "tail  -2   target/gate.log");
    assert!(held.contains("run_in_background"), "{held}");

    // The ledger is one session's.
    assert_eq!(pre_shell(&sb, &sb.seat, "elsewhere", wait), "");

    // Cargo waits for itself, so it is never a poll.
    for _ in 0..4 {
        assert_eq!(pre_shell(&sb, &sb.seat, "waiting", "cargo xtask gate"), "");
    }

    // A session types in the checkout, then in the seat: the ledger is
    // beside the checkout either way.
    assert!(sb.repo.join(".repeats").join("waiting.tsv").is_file());
    assert!(!sb.seat.join(".repeats").exists());
    let held_across = pre_shell(&sb, &sb.repo, "waiting", wait);
    assert!(held_across.contains("run_in_background"), "{held_across}");
}

#[test]
fn a_file_printed_whole_is_held_and_names_the_part_to_take_instead() {
    let sb = Sandbox::new("shell-dump");
    let big = "crates/platitude-core/src/wide.rs";
    sb.write(
        &sb.seat,
        big,
        &"// a line with something on it\n".repeat(400),
    );
    sb.write(&sb.seat, "crates/platitude-core/src/small.rs", "// short\n");

    let held = pre_shell(&sb, &sb.seat, "reading", &format!("cat {big}"));
    assert!(held.contains("grep -n"), "{held}");
    assert!(held.contains(big), "{held}");

    // Bounded by what follows it, or not read here at all.
    for line in [
        format!("cat {big} | head -40"),
        format!("sed -n '1,40p' {big}"),
        "cat crates/platitude-core/src/small.rs".to_string(),
    ] {
        assert_eq!(pre_shell(&sb, &sb.seat, "reading", &line), "", "{line}");
    }

    if cfg!(windows) {
        let stub = pre_shell(&sb, &sb.seat, "scripting", "python probe.py");
        assert!(stub.contains("uv run --no-project python"), "{stub}");
        assert_eq!(
            pre_shell(
                &sb,
                &sb.seat,
                "scripting",
                "uv run --no-project python probe.py"
            ),
            ""
        );
    }
}
