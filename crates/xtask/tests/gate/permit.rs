//! The landing permit: a command that writes main goes through on the
//! user's own message, once, and not on the message after it.

use std::fs::File;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::support::{EXE, Sandbox, output_past_a_busy_image};

/// What `hook <event>` prints for `payload`. Empty is the hook's way of
/// saying it has no objection.
fn hook(sb: &Sandbox, event: &str, payload: &str) -> String {
    let file = sb.root.join(format!("{event}.json"));
    std::fs::write(&file, payload).expect("payload");
    let mut command = Command::new(EXE);
    command
        .args(["hook", event])
        .stdin(Stdio::from(File::open(&file).expect("payload open")));
    sb.env(&mut command);
    let output = output_past_a_busy_image(&mut command, || {}).expect("spawn xtask");
    assert!(
        output.status.success(),
        "hook {event} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn forward(dir: &Path) -> String {
    dir.display().to_string().replace('\\', "/")
}

/// The user says `text`, from the seat.
fn says(sb: &Sandbox, text: &str) -> String {
    hook(
        sb,
        "prompt-submit",
        &format!(
            "{{\"session_id\":\"gate-permit\",\"cwd\":\"{}\",\
             \"hook_event_name\":\"UserPromptSubmit\",\"prompt\":\"{text}\"}}",
            forward(&sb.seat)
        ),
    )
}

/// The session runs the sanctioned landing, approval flag and all, from the seat.
fn lands(sb: &Sandbox) -> String {
    hook(
        sb,
        "pre-shell",
        &format!(
            "{{\"session_id\":\"gate-permit\",\"cwd\":\"{}\",\
             \"hook_event_name\":\"PreToolUse\",\"tool_name\":\"Bash\",\
             \"tool_input\":{{\"command\":\"PGG_ALLOW_MAIN=1 cargo xtask land worktree-a\"}}}}",
            forward(&sb.seat)
        ),
    )
}

#[test]
fn a_landing_runs_on_the_users_message_once_and_not_on_the_next() {
    let sb = Sandbox::new("permit");

    // Nobody has asked: refused, and the refusal names the word that asks.
    let unasked = lands(&sb);
    assert!(
        unasked.contains("\"deny\"") && unasked.contains("反映"),
        "{unasked}"
    );

    // The user asks: the same command goes through, and once only.
    assert_eq!(says(&sb, "OK main反映"), "");
    assert_eq!(lands(&sb), "");
    let twice = lands(&sb);
    assert!(twice.contains("second time"), "{twice}");

    // A correction after the ask closes it, and the refusal quotes it.
    says(&sb, "直してmain反映");
    says(&sb, "ここも直して");
    let corrected = lands(&sb);
    assert!(
        corrected.contains("does not ask") && corrected.contains("ここも直して"),
        "{corrected}"
    );

    // A message about the landing is not an ask for one.
    says(&sb, "反映されてない部分がある、直して");
    assert!(lands(&sb).contains("does not ask"));

    // Asking again opens it again.
    says(&sb, "直してmain反映");
    assert_eq!(lands(&sb), "");
}
