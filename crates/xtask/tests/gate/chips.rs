//! The chip guard's git side: what a seat already has open is read from
//! the repository, and a chip over it is refused.

use std::fs::File;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::support::{EXE, Sandbox, output_past_a_busy_image};

/// What `hook pre-chip` prints for a chip whose prompt names `path`,
/// asked from `dir`; empty is no objection.
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

    assert_eq!(pre_chip(&sb, &sb.seat, core), "");

    // An unstaged edit: the first ` M` line is where a reading that counts
    // columns goes wrong — git's answer arrives trimmed.
    sb.write_refs(&sb.seat, 1);
    let refused = pre_chip(&sb, &sb.seat, core);
    assert!(refused.contains("already has open"), "{refused}");

    // Committed: the branch carries it until it lands, so the chip's
    // session would still start from a main without it.
    sb.commit_all(&sb.seat, "feat(core): refs", &[]);
    let carried = pre_chip(&sb, &sb.seat, core);
    assert!(carried.contains("already has open"), "{carried}");

    // The primary checkout, on main, holds nothing.
    assert_eq!(
        pre_chip(&sb, &sb.seat, "crates/platitude-core/src/stash.rs"),
        ""
    );
    assert_eq!(pre_chip(&sb, &sb.repo, core), "");
}
