//! The shell guards from the outside: what `hook pre-shell` answers a
//! line, through the binary the harness runs and against a repository
//! laid out like this one.

use std::path::Path;

use crate::support::Sandbox;

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
