//! What a change to a configuration owes — the workspace's build, a
//! crate's manifest, clippy's settings, the dependency policy: each the
//! steps that read it, and no more.

use crate::support::{Sandbox, set, without_always};

#[test]
fn a_build_input_change_owes_everything() {
    let sb = Sandbox::new("build-input");
    sb.write(&sb.seat, "Cargo.toml", "[workspace]\nresolver = \"2\"\n");
    sb.commit_all(&sb.seat, "chore: the resolver", &[]);
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(text.contains("everything (Cargo.toml)"), "{text}");
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

/// clippy's configuration is read by clippy alone: every crate's, on
/// both sides, and nothing a test or a build reads.
#[test]
fn a_lint_threshold_owes_clippy_alone() {
    let sb = Sandbox::new("clippy-config");
    sb.write(&sb.seat, "clippy.toml", "too-many-lines-threshold = 100\n");
    sb.commit_all(&sb.seat, "chore: a lint threshold", &[]);
    sb.gate_ok(&sb.seat, &[]);
    assert_eq!(
        without_always(&sb.ran()),
        set(&[
            "clippy platitude-app",
            "clippy platitude-core",
            "clippy xtask",
            "clippy-linux platitude-app",
            "clippy-linux platitude-core",
            "clippy-linux xtask",
        ])
    );
}

/// Nothing reads a configuration's comments: a change to them alone owes
/// what a document's does, even in the file that otherwise owes
/// everything.
#[test]
fn a_comment_in_a_configuration_owes_nothing() {
    let sb = Sandbox::new("toml-comment");
    sb.write(&sb.seat, "Cargo.toml", "# The workspace.\n[workspace]\n");
    sb.write(
        &sb.seat,
        "rust-toolchain.toml",
        "[toolchain]\n# The release.\nchannel = \"stable\"\n",
    );
    sb.commit_all(&sb.seat, "docs: the workspace", &[]);
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(!text.contains("everything"), "{text}");
    assert!(text.contains("no step reads these"), "{text}");
    sb.gate_ok(&sb.seat, &[]);
    assert!(without_always(&sb.ran()).is_empty());

    // A line of a multi-line string may start with `#`: a file holding one
    // is read as changed.
    let quoting =
        |line: &str| format!("[workspace]\n[workspace.metadata]\nnote = \"\"\"\n{line}\n\"\"\"\n");
    sb.write(&sb.seat, "Cargo.toml", &quoting("# one"));
    let quoted = sb.commit_all(&sb.seat, "chore: a note", &[]);
    sb.write(&sb.seat, "Cargo.toml", &quoting("# two"));
    sb.commit_all(&sb.seat, "chore: another note", &[]);
    let text = sb.gate_ok(&sb.seat, &["--main", &quoted, "--dry-run"]);
    assert!(text.contains("everything (Cargo.toml)"), "{text}");
}

/// A crate's manifest is read by every compile of the crate, and its
/// readers' through it; not by a crate that reads none of it.
#[test]
fn a_crates_manifest_owes_that_crate_and_its_readers() {
    let sb = Sandbox::new("manifest");
    let manifest = "crates/platitude-core/Cargo.toml";
    sb.write(&sb.seat, manifest, "[package]\nname = \"platitude-core\"\n");
    sb.write(
        &sb.seat,
        "crates/xtask/Cargo.toml",
        "[package]\nname = \"xtask\"\n",
    );
    let base = sb.commit_all(&sb.seat, "chore: manifests", &[]);
    sb.write(
        &sb.seat,
        manifest,
        "[package]\nname = \"platitude-core\"\n[features]\nprobe = []\n",
    );
    sb.commit_all(&sb.seat, "feat(core): a feature", &[]);
    let text = sb.gate_ok(&sb.seat, &["--main", &base, "--dry-run"]);
    assert!(!text.contains("everything"), "{text}");
    sb.gate_ok(&sb.seat, &["--main", &base]);
    let ran = without_always(&sb.ran());
    for owed in [
        "clippy platitude-core",
        "clippy-linux platitude-core",
        "clippy platitude-app",
        "test it (all)",
        "deny",
    ] {
        assert!(ran.contains(owed), "{owed} not run; ran: {ran:?}");
    }
    let tool: Vec<&String> = ran.iter().filter(|id| id.contains("xtask")).collect();
    assert!(tool.is_empty(), "{tool:?}");
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
