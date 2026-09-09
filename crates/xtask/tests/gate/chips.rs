//! The chip guard's git side: what a seat already has open is read from
//! the repository rather than from the payload, so a chip over this
//! session's own work is refused where a chip over anything else is not.

use std::fs::File;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::support::{EXE, Sandbox, output_past_a_busy_image};

/// What `hook pre-chip` prints for a chip whose prompt names `path`,
/// asked from `dir`. Empty is the hook's way of saying it has no
/// objection.
fn pre_chip(sb: &Sandbox, dir: &Path, path: &str) -> String {
    let payload = sb.root.join("chip.json");
    std::fs::write(
        &payload,
        format!(
            "{{\"session_id\":\"gate-chips\",\"cwd\":\"{}\",\
             \"hook_event_name\":\"PreToolUse\",\
             \"tool_name\":\"mcp__ccd_session__spawn_task\",\
             \"tool_input\":{{\"title\":\"1. 直す\",\"tldr\":\"直す\",\
             \"prompt\":\"{path} を直す\"}}}}",
            dir.display().to_string().replace('\\', "/")
        ),
    )
    .expect("payload");
    let mut command = Command::new(EXE);
    command
        .args(["hook", "pre-chip"])
        .stdin(Stdio::from(File::open(&payload).expect("payload open")));
    sb.env(&mut command);
    let output = output_past_a_busy_image(&mut command, || {}).expect("spawn xtask");
    assert!(
        output.status.success(),
        "hook pre-chip failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn a_chip_is_refused_over_the_work_the_seat_is_already_holding() {
    let sb = Sandbox::new("chips");
    let core = "crates/platitude-core/src/refs.rs";

    // Nothing open in the seat: the chip is work for a session that is
    // not this one, and the guard has nothing to say about it.
    assert_eq!(pre_chip(&sb, &sb.seat, core), "");

    // The same file, once this seat is changing it. An unstaged edit is
    // the ` M` line of a status listing, and the first such line is
    // where a reading that counts columns instead of reading the letters
    // off goes wrong — git's answer arrives trimmed.
    sb.write_refs(&sb.seat, 1);
    let refused = pre_chip(&sb, &sb.seat, core);
    assert!(refused.contains("already has open"), "{refused}");

    // And once it is committed rather than merely written: the branch
    // carries it until it lands, so the chip's session would still
    // start from a main without it.
    sb.commit_all(&sb.seat, "feat(core): refs", &[]);
    let carried = pre_chip(&sb, &sb.seat, core);
    assert!(carried.contains("already has open"), "{carried}");

    // A file neither side of the seat is touching still passes, and so
    // does the same chip asked from the primary checkout, which is on
    // main and holding nothing.
    assert_eq!(
        pre_chip(&sb, &sb.seat, "crates/platitude-core/src/stash.rs"),
        ""
    );
    assert_eq!(pre_chip(&sb, &sb.repo, core), "");
}
