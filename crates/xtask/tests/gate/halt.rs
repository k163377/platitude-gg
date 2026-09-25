//! A run's first red stops it (`gate::halt`): what was still running is
//! ended, what had not started is not, and each is filed as `halted`, so
//! the next run owes them again.

use crate::support::{ALWAYS, Sandbox};

/// The Linux step holds until the halt reaches it (`PGG_GATE_FAKE_HOLD` —
/// faked steps otherwise end as they start), and the host's red waits for
/// it to start (`PGG_GATE_FAKE_FAIL_AFTER`): what is checked is a running
/// step, never one the halt met at its door.
#[test]
fn the_first_red_ends_what_the_other_side_is_running() {
    let sb = Sandbox::new("halt-running");
    sb.write_refs(&sb.seat, 51);
    sb.commit_all(&sb.seat, "feat(core): fifty-one", &[]);
    let (ok, text) = sb.gate(
        &sb.seat,
        &[],
        &[
            ("PGG_GATE_FAKE_FAIL", "test platitude-core 1"),
            ("PGG_GATE_FAKE_HOLD", "test platitude-core 1 linux"),
            ("PGG_GATE_FAKE_FAIL_AFTER", "test platitude-core 1 linux"),
        ],
    );
    assert!(!ok, "{text}");
    assert!(
        text.contains("stopped at the first red (test platitude-core 1)"),
        "{text}"
    );
    assert!(
        !text.contains("side's ledger"),
        "every step answered for:\n{text}"
    );
    let ledger = sb.ledger(&sb.seat);
    assert!(
        ledger.iter().any(|(side, id, outcome)| side == "linux"
            && id == "test platitude-core 1 linux"
            && outcome == "halted"),
        "the held step was ended, not failed and not passed: {ledger:?}"
    );
    let first = sb.ran();
    assert!(
        first.contains("test platitude-core 1 linux"),
        "the held step had started before the red: {first:?}"
    );
    sb.gate_ok(&sb.seat, &[]);
    let again = sb.ran();
    assert!(again.contains("test platitude-core 1 linux"), "{again:?}");
}

/// Under `--keep-going` too, and each Linux step has a row saying so.
#[test]
fn a_red_always_step_starts_neither_side() {
    let sb = Sandbox::new("halt-always");
    sb.write_refs(&sb.seat, 52);
    sb.commit_all(&sb.seat, "feat(core): fifty-two", &[]);
    let (ok, text) = sb.gate(
        &sb.seat,
        &["--keep-going"],
        &[("PGG_GATE_FAKE_FAIL", "fmt")],
    );
    assert!(!ok, "{text}");
    assert!(text.contains("stopped at the first red (fmt)"), "{text}");
    let ran = sb.ran();
    assert!(
        ran.iter().all(|id| ALWAYS.contains(&id.as_str())),
        "nothing past the always-steps ran: {ran:?}"
    );
    let ledger = sb.ledger(&sb.seat);
    let linux: Vec<_> = ledger
        .iter()
        .filter(|(side, _, _)| side == "linux")
        .collect();
    assert!(
        !linux.is_empty(),
        "the plan owed the Linux side: {ledger:?}"
    );
    assert!(
        linux
            .iter()
            .all(|(_, _, outcome)| outcome == "halted" || outcome == "cached"),
        "{linux:?}"
    );
}
