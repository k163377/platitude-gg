//! The landing permit: the landing verb goes through on the user's own
//! message, once — and the once is a landing that moved main, not an
//! attempt. Git of the session's own never writes main at all.

use std::fs::File;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::support::{EXE, Sandbox, output_past_a_busy_image};

/// The session every payload here comes from, and the land runs as.
const SESSION: &str = "gate-permit";

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
            "{{\"session_id\":\"{SESSION}\",\"cwd\":\"{}\",\
             \"hook_event_name\":\"UserPromptSubmit\",\"prompt\":\"{text}\"}}",
            forward(&sb.seat)
        ),
    )
}

/// The session runs a shell line from `dir`, through the pre-shell hook.
fn shell(sb: &Sandbox, dir: &Path, command: &str) -> String {
    hook(
        sb,
        "pre-shell",
        &format!(
            "{{\"session_id\":\"{SESSION}\",\"cwd\":\"{}\",\
             \"hook_event_name\":\"PreToolUse\",\"tool_name\":\"Bash\",\
             \"tool_input\":{{\"command\":\"{command}\"}}}}",
            forward(dir)
        ),
    )
}

/// The session runs the sanctioned landing from the seat.
fn lands(sb: &Sandbox) -> String {
    shell(sb, &sb.seat, "cargo xtask land worktree-a")
}

fn stop(sb: &Sandbox) -> String {
    hook(
        sb,
        "stop",
        &format!(
            "{{\"session_id\":\"{SESSION}\",\"cwd\":\"{}\",\"stop_hook_active\":true}}",
            forward(&sb.seat)
        ),
    )
}

#[test]
fn a_landing_reminds_the_session_to_finish_the_request_before_main_moves() {
    let sb = Sandbox::new("permit-ready");
    let note = says(&sb, "直してmain反映");
    assert!(
        note.contains("Before landing")
            && note.contains("review findings")
            && note.contains("background tasks")
            && note.contains("rechecks before invoking land"),
        "{note}"
    );
    assert_eq!(says(&sb, "ここも直して"), "");
    assert_eq!(
        says(&sb, "<task-notification>main反映</task-notification>"),
        ""
    );
}

#[test]
fn necessary_corrections_after_landing_remain_possible_and_visible() {
    let mut sb = Sandbox::new("permit-correction");
    let seat = sb.root.join(".claude/worktrees/a");
    std::fs::create_dir_all(seat.parent().expect("seat parent")).expect("seat directory");
    sb.git_ok(
        &sb.repo,
        &["worktree", "move", &forward(&sb.seat), &forward(&seat)],
    );
    sb.seat = seat;
    sb.write_refs(&sb.seat, 22);
    sb.commit_all(&sb.seat, "feat(core): twenty-two", &[]);
    says(&sb, "main反映");
    let (ok, text) = sb.land_as("worktree-a", SESSION);
    assert!(ok, "{text}");
    assert_eq!(stop(&sb), "", "a clean landing leaves no work");
    assert_eq!(shell(&sb, &sb.seat, "cargo xtask seat"), "");
    sb.git_ok(
        &sb.repo,
        &[
            "worktree",
            "lock",
            "--reason",
            &format!("claude-seat {SESSION}"),
            &forward(&sb.seat),
        ],
    );
    let path = sb.seat.join("crates/platitude-core/src/refs.rs");
    let edit = hook(
        &sb,
        "pre-write",
        &format!(
            "{{\"session_id\":\"{SESSION}\",\"cwd\":\"{}\",\"tool_input\":{{\"file_path\":\"{}\"}}}}",
            forward(&sb.seat),
            forward(&path)
        ),
    );
    assert_eq!(edit, "", "a necessary correction is still allowed");
    sb.write_refs(&sb.seat, 23);
    let dirty = stop(&sb);
    assert!(
        dirty.contains("after the landing") && dirty.contains("uncommitted"),
        "{dirty}"
    );
    assert_eq!(shell(&sb, &sb.seat, "git commit -m fix"), "");
    sb.commit_all(&sb.seat, "fix(core): twenty-three", &[]);
    let committed = stop(&sb);
    assert!(
        committed.contains("after the landing") && committed.contains("1 commit"),
        "{committed}"
    );
    let refused = lands(&sb);
    assert!(
        refused.contains("second time") && refused.contains("Finish"),
        "{refused}"
    );
    assert_ne!(sb.main_sha(), sb.head(&sb.seat));
}

#[test]
fn a_landing_runs_on_the_users_message_until_one_moves_main_and_not_on_the_next() {
    let sb = Sandbox::new("permit");
    sb.write_refs(&sb.seat, 21);
    sb.commit_all(&sb.seat, "feat(core): twenty-one", &[]);

    // Nobody has asked: refused, and the refusal names the word that asks.
    let unasked = lands(&sb);
    assert!(
        unasked.contains("\"deny\"") && unasked.contains("反映") && unasked.contains("No message"),
        "{unasked}"
    );

    // The user asks: the command goes through — and through again after
    // a landing that moved nothing, since the ask stands unmet.
    assert!(says(&sb, "OK main反映").contains("Before landing"));
    assert_eq!(lands(&sb), "");
    sb.write(
        &sb.seat,
        "crates/platitude-core/src/extra.rs",
        "// unsaved\n",
    );
    let (ok, text) = sb.land_as("worktree-a", SESSION);
    assert!(!ok && text.contains("uncommitted"), "{text}");
    assert_eq!(lands(&sb), "", "a landing that stopped spent nothing");
    std::fs::remove_file(sb.seat.join("crates/platitude-core/src/extra.rs")).expect("clean");

    // The landing that moves main spends it: the next on the same
    // message is refused, and says why.
    let (ok, text) = sb.land_as("worktree-a", SESSION);
    assert!(ok, "{text}");
    assert_eq!(sb.main_sha(), sb.head(&sb.seat));
    let twice = lands(&sb);
    assert!(
        twice.contains("second time") && twice.contains("OK main反映"),
        "{twice}"
    );

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

    // Asking again opens it again, and what is not the user's message —
    // the context a summary carries back, a tagged event — leaves it as
    // it stands.
    says(&sb, "直してmain反映");
    assert_eq!(lands(&sb), "");
    says(
        &sb,
        "This session is being continued from a previous conversation that ran out of \
         context. The user said: ここも直して",
    );
    says(
        &sb,
        "<task-notification>the agent is done</task-notification>",
    );
    assert_eq!(lands(&sb), "");
}

/// Git of the session's own never writes main, whatever stands in front
/// of the line, permit or no permit: the landing verb is the one way,
/// and the refusal names it.
#[test]
fn a_sessions_own_git_does_not_write_main_on_any_flag() {
    let sb = Sandbox::new("permit-git");
    says(&sb, "main反映");
    for command in [
        "git merge --ff-only worktree-a",
        "PGG_ALLOW_MAIN=1 git merge --ff-only worktree-a",
        "git push . worktree-a:main",
        "git update-ref refs/heads/main worktree-a",
    ] {
        let refused = shell(&sb, &sb.repo, command);
        assert!(
            refused.contains("\"deny\"") && refused.contains("cargo xtask land <branch>"),
            "{command}: {refused}"
        );
    }
    // A merge in the seat lands on the seat's own branch, not on main.
    assert_eq!(shell(&sb, &sb.seat, "git merge --ff-only some-branch"), "");
}
