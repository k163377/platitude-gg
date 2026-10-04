//! Which changes move a version the product is built against, and owe
//! the full tier before a merge (反映前テストの機械化.md
//! §版を動かす変更は段 3).

use crate::support::Sandbox;

/// A census with one line of each tier the full gate adds.
fn tiered(name: &str) -> Sandbox {
    let sb = Sandbox::new(name);
    sb.write(
        &sb.seat,
        "crates/xtask/verb-census.txt",
        "# census\nstash --preset basic\tDriver Main StashPane\n\
         stash-menu --preset basic\tDriver Main StashPane\n\
         stash-open --preset basic\tDriver Main StashPane\n",
    );
    sb.write(
        &sb.seat,
        "crates/xtask/verb-tiers.txt",
        "linux\tstash --preset basic\t-\tboth sides\n\
         full\tstash-menu --preset basic\tstash --preset basic\tpicture: the menu standing\n",
    );
    sb.commit_all(&sb.seat, "chore: tiers", &[]);
    sb
}

/// The lines only the full gate owes, both sides.
const FULL_ONLY: [&str; 2] = [
    "[host ] verify stash-menu --preset basic",
    "[linux] verify-linux stash-open --preset basic",
];

/// A moved crate version owes stage 2 the full tier, without `--all`;
/// the daily tier stays the host's quick half.
#[test]
fn a_version_move_owes_the_full_tier() {
    let sb = tiered("version-move");
    sb.write(&sb.seat, "Cargo.lock", "# lock\n# bumped\n");
    sb.commit_all(&sb.seat, "chore: bump", &[]);
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(
        text.contains("everything (Cargo.lock moves a version), the full tier"),
        "{text}"
    );
    for owed in FULL_ONLY {
        assert!(text.contains(owed), "{owed} not owed: {text}");
    }
    let daily = sb.gate_ok(&sb.seat, &["--host-only", "--dry-run"]);
    assert!(!daily.contains("the full tier"), "{daily}");
    assert!(!daily.contains("stash-menu"), "{daily}");
}

/// Of the core's version module, the minimum's constant is the version;
/// the words beside it are a source change like any other.
#[test]
fn the_git_minimum_is_its_constant() {
    let sb = tiered("git-minimum");
    let version = "crates/platitude-core/src/version.rs";
    sb.write(
        &sb.seat,
        version,
        "/// The oldest git supported.\npub const MINIMUM_GIT: (u32, u32) = (2, 43);\n",
    );
    sb.commit_all(&sb.seat, "docs: the minimum", &[]);
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(!text.contains("the full tier"), "{text}");

    sb.write(
        &sb.seat,
        version,
        "/// The oldest git supported.\npub const MINIMUM_GIT: (u32, u32) = (2, 47);\n",
    );
    sb.commit_all(&sb.seat, "feat: a newer minimum", &[]);
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(
        text.contains(&format!(
            "everything ({version} moves a version), the full tier"
        )),
        "{text}"
    );
    for owed in FULL_ONLY {
        assert!(text.contains(owed), "{owed} not owed: {text}");
    }
}

/// Qt's version lives in a file of its own, which no source reads: a bump
/// there alone owes the full tier.
#[test]
fn a_qt_bump_owes_the_full_tier() {
    let sb = tiered("qt");
    sb.write(&sb.seat, ".qt-version", "6.11.0\n");
    sb.commit_all(&sb.seat, "chore: a newer Qt", &[]);
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(
        text.contains("everything (.qt-version moves a version), the full tier"),
        "{text}"
    );
    for owed in FULL_ONLY {
        assert!(text.contains(owed), "{owed} not owed: {text}");
    }
}

/// A workflow is CI's own: no step here reads one, and an edit to it
/// alone owes nothing but the always-steps — the workflow that carried
/// Qt's version before the pin had a file of its own, and the line that
/// carried it.
#[test]
fn a_workflow_edit_owes_no_step() {
    let sb = tiered("workflow");
    let workflow = ".github/workflows/ci.yml";
    sb.write(
        &sb.seat,
        workflow,
        "env:\n  QT_VERSION: \"6.11.0\"\njobs:\n  test:\n    runs-on: ubuntu-24.04\n",
    );
    sb.commit_all(&sb.seat, "ci: a job", &[]);
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(!text.contains("the full tier"), "{text}");
    // Among the files no step reads: a row of its own, where a changed
    // file is listed behind a `*`.
    assert!(
        text.contains("no step reads these") && text.contains(&format!("\n  {workflow}\n")),
        "{text}"
    );
    assert!(!text.contains("  run    "), "{text}");
}

/// Of the Dockerfile, the base image carries the container's git; a line
/// beside it widens the reach and no more.
#[test]
fn the_containers_base_image_is_its_version() {
    let sb = tiered("base-image");
    sb.write(&sb.seat, "ci/linux/Dockerfile", "FROM ubuntu\nRUN true\n");
    sb.commit_all(&sb.seat, "chore: a step", &[]);
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(text.contains("everything (ci/linux/Dockerfile)"), "{text}");
    assert!(!text.contains("the full tier"), "{text}");

    sb.write(
        &sb.seat,
        "ci/linux/Dockerfile",
        "FROM ubuntu:26.04\nRUN true\n",
    );
    sb.commit_all(&sb.seat, "chore: a newer base", &[]);
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(
        text.contains("everything (ci/linux/Dockerfile moves a version), the full tier"),
        "{text}"
    );
    for owed in FULL_ONLY {
        assert!(text.contains(owed), "{owed} not owed: {text}");
    }
}
