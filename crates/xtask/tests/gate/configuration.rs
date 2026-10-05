//! What a change to a configuration owes — the workspace's build inputs
//! and the dependency policy.

use crate::support::{Sandbox, set, without_always};

#[test]
fn a_build_input_change_owes_everything() {
    let sb = Sandbox::new("build-input");
    sb.write(&sb.seat, "clippy.toml", "too-many-lines-threshold = 100\n");
    sb.commit_all(&sb.seat, "chore: a lint threshold", &[]);
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(text.contains("everything (clippy.toml)"), "{text}");
    assert!(!text.contains("the full tier"), "{text}");
    sb.gate_ok(&sb.seat, &[]);
    let ran = without_always(&sb.ran());
    for owed in [
        "test platitude-core (all)",
        "test platitude-app (all)",
        "test xtask (all)",
        "test it (all)",
        "clippy platitude-core",
        "clippy xtask",
        "qmltest",
        "shipped",
        "verify stash --preset basic",
        "bare",
        "deny",
    ] {
        assert!(ran.contains(owed), "{owed} not run; ran: {ran:?}");
    }
}

#[test]
fn a_policy_change_owes_cargo_deny_and_nothing_else() {
    let sb = Sandbox::new("deny");
    sb.write(
        &sb.seat,
        "deny.toml",
        "[bans]\nmultiple-versions = \"allow\"\n",
    );
    sb.commit_all(&sb.seat, "chore(deps): a ban", &[]);
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(!text.contains("no step reads these"), "{text}");
    sb.gate_ok(&sb.seat, &[]);
    assert_eq!(without_always(&sb.ran()), set(&["deny"]));
}

/// The policy is read against the closure: a change to neither leaves the
/// step unselected, however far it reaches.
#[test]
fn a_source_change_owes_no_policy_check() {
    let sb = Sandbox::new("deny-source");
    sb.write_refs(&sb.seat, 14);
    sb.commit_all(&sb.seat, "feat(core): refs", &[]);
    sb.gate_ok(&sb.seat, &[]);
    assert!(!sb.ran().contains("deny"));
}
