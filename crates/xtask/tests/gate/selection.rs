//! Which steps a change owes: what the reach of its diff selects, and
//! what it leaves alone.

use std::collections::BTreeSet;

use crate::support::{ALWAYS, Sandbox, set, without_always};

#[test]
fn a_docs_only_change_owes_the_always_steps_and_nothing_else() {
    let sb = Sandbox::new("docs");
    sb.write(&sb.seat, "internal-docs/notes.md", "# notes\n\nmore\n");
    sb.commit_all(&sb.seat, "docs: more", &[]);
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(text.contains("no step reads these"), "{text}");
    let text = sb.gate_ok(&sb.seat, &[]);
    assert!(text.contains("gate: PASS"), "{text}");
    assert_eq!(sb.ran(), set(&ALWAYS));
}

#[test]
fn markdown_named_by_code_or_under_build_inputs_only_owes_consistency_checks() {
    let sb = Sandbox::new("markdown-inputs");
    let paths = [
        "internal-docs/P3-確認事項.md",
        "CLAUDE.md",
        ".cargo/notes.md",
        "crates/platitude-app/assets/README.md",
        "crates/xtask/src/verify/README.md",
        "crates/platitude-core/src/README.MD",
    ];
    for path in paths {
        sb.write(&sb.repo, path, "# Before\n");
    }
    sb.write(&sb.repo, "crates/xtask/src/notices.rs",
        "pub fn notice() -> &'static str { \"internal-docs/P3-確認事項.md\" }\n#[test]\nfn notice_names_the_doc() { assert!(!notice().is_empty()); }\n");
    sb.write(
        &sb.repo,
        "crates/xtask/src/main.rs",
        "mod notices;\nfn main() {}\n",
    );
    sb.commit_all(
        &sb.repo,
        "test: markdown references",
        &[("PGG_GATE_SKIP", "1")],
    );
    sb.git_ok(&sb.seat, &["merge", "--ff-only", "main"]);
    for path in paths {
        sb.write(&sb.seat, path, "# After\n");
    }
    sb.commit_all(&sb.seat, "docs: edit named and nested markdown", &[]);
    sb.gate_ok(&sb.seat, &[]);
    assert_eq!(sb.ran(), set(&ALWAYS));

    sb.write_refs(&sb.seat, 2);
    sb.commit_all(&sb.seat, "feat(core): change beside markdown", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let ran = without_always(&sb.ran());
    assert!(ran.contains("test platitude-core 1"), "{ran:?}");
    assert!(
        !ran.iter().any(|id| id.starts_with("test xtask")),
        "{ran:?}"
    );
}

#[test]
fn a_core_change_owes_what_reads_it_and_nothing_beside_it() {
    let sb = Sandbox::new("core");
    sb.write(&sb.seat, "crates/platitude-core/src/stash.rs", "pub fn stash() { let _ = 1; }\n#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn t() { stash() }\n}\n");
    sb.commit_all(&sb.seat, "feat(core): stash", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let ran = without_always(&sb.ran());
    for owed in [
        "clippy platitude-core",
        "clippy-linux platitude-core",
        "clippy platitude-app",
        "test platitude-core 1",
        "test platitude-core 1 linux",
        "test platitude-app 1",
        "test it 1",
        "test it 1 linux",
        "shipped",
        "verify stash --preset basic",
        "verify-linux stash --preset basic",
        "bare",
    ] {
        assert!(ran.contains(owed), "{owed} not run; ran: {ran:?}");
    }
    for spared in ["clippy xtask", "test xtask 1"] {
        assert!(!ran.contains(spared), "{spared} ran; ran: {ran:?}");
    }
    // And the filters are the modules the reach holds.
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(
        text.contains("test platitude-core 1") && text.contains("test it 1"),
        "{text}"
    );
}

#[test]
fn a_leaf_core_change_stays_narrow() {
    let sb = Sandbox::new("leaf");
    sb.write_refs(&sb.seat, 2);
    sb.commit_all(&sb.seat, "feat(core): refs", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let ran = without_always(&sb.ran());
    assert_eq!(
        ran,
        set(&[
            "clippy platitude-core",
            "clippy-linux platitude-core",
            "test platitude-core 1",
            "test platitude-core 1 linux",
            "test it 1",
            "test it 1 linux",
            "bare",
        ]),
        "{ran:?}"
    );
}

#[test]
fn a_qml_change_owes_the_verbs_whose_census_names_it_and_no_rust_test() {
    let sb = Sandbox::new("qml");
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/StashPane.qml",
        "Item {\n    width: 1\n    property var model: StashModel\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(app-ui): pane", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let ran = without_always(&sb.ran());
    assert_eq!(
        ran,
        set(&[
            "qmltest",
            "qmltest-linux",
            "shipped",
            "verify stash --preset basic",
            "verify-linux stash --preset basic",
            "bare"
        ]),
        "{ran:?}"
    );
}

/// The final census does not contain a dialog a verb opened and closed.
/// Narrowing by that snapshot is only a candidate; a verb reached
/// through the dialog's owner still runs.
#[test]
fn a_qml_leaf_keeps_reached_verbs_and_reports_the_shadow_selection() {
    let sb = Sandbox::new("qml-shown");
    sb.write(
        &sb.seat,
        "crates/xtask/verb-census.txt",
        "# census\nstash --preset basic\tDriver Main StashPane\n\
         window --preset basic\tDriver Main\n",
    );
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/StashPane.qml",
        "Item {\n    width: 3\n    property var model: StashModel\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(app-ui): pane", &[]);
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(
        text.contains("verbs: 2 selected, 1 of them on the container too; 0 left")
            && text.contains("shadow candidate: 1"),
        "{text}"
    );
    sb.gate_ok(&sb.seat, &[]);
    let ran = without_always(&sb.ran());
    assert_eq!(
        ran,
        set(&[
            "qmltest",
            "qmltest-linux",
            "shipped",
            "verify stash --preset basic",
            "verify-linux stash --preset basic",
            "verify window --preset basic",
            "bare"
        ]),
        "{ran:?}"
    );
}

#[test]
fn a_dialog_closed_before_the_census_still_owes_its_escape_verb() {
    let sb = Sandbox::new("closed-dialog");
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/Main.qml",
        "Item { SettingsDialog {} }\n",
    );
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/SettingsDialog.qml",
        "Item { function escapeOut() {} }\n",
    );
    sb.write(
        &sb.seat,
        "crates/xtask/verb-census.txt",
        "settings\tMain SettingsDialog\nsettings-escape\tMain\n",
    );
    let base = sb.commit_all(&sb.seat, "test: dialog fixture", &[]);
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/SettingsDialog.qml",
        "Item { function escapeOut() { return false } }\n",
    );
    sb.commit_all(&sb.seat, "fix(ui): broken escape handler", &[]);
    let red = "verify settings-escape";
    let (ok, text) = sb.gate(&sb.seat, &["--main", &base], &[("PGG_GATE_FAKE_FAIL", red)]);
    assert!(!ok && text.contains("nothing stamped"), "{text}");
    assert!(sb.ran().contains(red), "{text}");
}

#[test]
fn a_harness_change_without_qml_edges_owes_every_recorded_verb() {
    let sb = Sandbox::new("harness-no-qml");
    sb.write(
        &sb.seat,
        "crates/xtask/src/main.rs",
        "mod qmltest;\nmod seats;\nmod verify;\n",
    );
    sb.write(
        &sb.seat,
        "crates/xtask/src/verify/mod.rs",
        "pub fn run() {}\n",
    );
    sb.write(
        &sb.seat,
        "crates/xtask/verb-census.txt",
        "stash --preset basic\tDriver Main StashPane\nwindow\tDriver Main\n",
    );
    let base = sb.commit_all(&sb.seat, "test: harness fixture", &[]);
    sb.write(
        &sb.seat,
        "crates/xtask/src/verify/mod.rs",
        "pub fn run() { return; }\n",
    );
    sb.commit_all(&sb.seat, "fix(verify): harness behavior", &[]);
    let red = "verify window";
    // Past its red, so that what ran is what the change owed.
    let (ok, text) = sb.gate(
        &sb.seat,
        &["--main", &base, "--keep-going"],
        &[("PGG_GATE_FAKE_FAIL", red)],
    );
    assert!(!ok && text.contains("nothing stamped"), "{text}");
    let ran = sb.ran();
    for verb in [
        red,
        "verify stash --preset basic",
        "verify-linux stash --preset basic",
    ] {
        assert!(ran.contains(verb), "{verb} not run; {ran:?}");
    }
}

/// A singleton stands in no item tree, so no run reports it; the app's
/// Rust is the binary every verb runs.
#[test]
fn a_singleton_and_the_apps_rust_owe_every_verb_the_census_holds() {
    let sb = Sandbox::new("qml-unnameable");
    let census = "# census\nstash --preset basic\tDriver Main StashPane\n\
                  window --preset basic\tDriver Main\n";
    let both = [
        "verify stash --preset basic",
        "verify window --preset basic",
        "verify-linux stash --preset basic",
    ];
    sb.write(&sb.seat, "crates/xtask/verb-census.txt", census);
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/Theme.qml",
        "pragma Singleton\nQtObject {\n    property int gap: 4\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(app-ui): a gap", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let ran = sb.ran();
    for verb in both {
        assert!(ran.contains(verb), "{verb} did not run; ran: {ran:?}");
    }

    sb.write(
        &sb.seat,
        "crates/platitude-app/src/models.rs",
        "use platitude_core::stash;\npub struct StashModel;\n#[qobject]\nimpl StashModel {}\n\
         #[cfg(test)]\nmod tests {\n    #[test]\n    fn m() { let _ = 2; }\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(app): a model", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let ran = sb.ran();
    for verb in both {
        assert!(ran.contains(verb), "{verb} did not run; ran: {ran:?}");
    }
}

/// The verbs of a side share nothing but the release the first one
/// builds, so under `--keep-going` a red one stops none of the others.
#[test]
fn a_red_verb_stops_no_other_verb_and_the_rerun_owes_it_alone() {
    let sb = Sandbox::new("red-verb");
    sb.write(
        &sb.seat,
        "crates/xtask/verb-census.txt",
        "# census\nstash --preset basic\tDriver Main StashPane\n\
         stash-menu --preset basic\tDriver Main StashPane\n\
         stash-open --preset basic\tDriver Main StashPane\n",
    );
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/StashPane.qml",
        "Item {\n    width: 2\n    property var model: StashModel\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(app-ui): pane", &[]);
    let red = "verify stash-open --preset basic";
    let (ok, text) = sb.gate(
        &sb.seat,
        &["--jobs", "2", "--keep-going"],
        &[("PGG_GATE_FAKE_FAIL", red)],
    );
    assert!(!ok && text.contains("nothing stamped"), "{text}");
    let ran = sb.ran();
    for verb in [
        "verify stash --preset basic",
        "verify stash-menu --preset basic",
        red,
        "verify-linux stash --preset basic",
    ] {
        assert!(ran.contains(verb), "{verb} did not run; ran: {ran:?}");
    }
    sb.gate_ok(&sb.seat, &["--jobs", "2"]);
    let again = without_always(&sb.ran());
    assert_eq!(again, set(&[red]), "{again:?}");
}

/// A verb stamped by another tree while this one queued built nothing
/// here; read as a build, the verbs after it would judge whatever release
/// lies in this tree — stale or absent. Nothing outside can land in that
/// window, so the runner's switch stands in (`PGG_GATE_FAKE_STAMP`).
#[test]
fn a_verb_stamped_while_it_queued_does_not_earn_the_block_its_no_build() {
    let sb = Sandbox::new("stamped-while-queued");
    sb.write(
        &sb.seat,
        "crates/xtask/verb-census.txt",
        "# census\nstash --preset basic\tDriver Main StashPane\n\
         stash-menu --preset basic\tDriver Main StashPane\n\
         stash-open --preset basic\tDriver Main StashPane\n",
    );
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/StashPane.qml",
        "Item {\n    width: 3\n    property var model: StashModel\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(app-ui): pane", &[]);
    // The host's first verb in plan order: the one that would build the
    // release for the rest.
    let first = "verify stash --preset basic";
    let (ok, text) = sb.gate(
        &sb.seat,
        &["--jobs", "2"],
        &[("PGG_GATE_FAKE_STAMP", first)],
    );
    assert!(ok, "{text}");
    assert!(
        text.contains("stamped elsewhere while this waited"),
        "the window was never entered: {text}"
    );
    let told = sb.told_not_to_build();
    assert!(
        !told.contains("verify stash-menu --preset basic"),
        "the verb after the stamped one was told to reuse a release nothing built here: {told:?}"
    );
    assert!(
        told.contains("verify stash-open --preset basic"),
        "the block never reached a verb that could reuse the build: {told:?}"
    );
}

#[test]
fn a_qtest_file_owes_the_qml_runner_and_nothing_the_app_is_built_for() {
    let sb = Sandbox::new("qmltest");
    sb.write(
        &sb.seat,
        "crates/platitude-app/tests/qml/tst_probe.qml",
        "import QtTest\nItem {\n    TestCase { name: \"Probe\" }\n}\n",
    );
    sb.commit_all(&sb.seat, "test(app-ui): probe", &[]);
    sb.gate_ok(&sb.seat, &[]);
    // A runner of its own: no census owes it a verb and nothing is built.
    let ran = without_always(&sb.ran());
    assert_eq!(ran, set(&["qmltest", "qmltest-linux"]), "{ran:?}");

    // The qmldir declares the singletons the tests resolve through. The
    // file above goes, so only this stands against main.
    std::fs::remove_file(sb.seat.join("crates/platitude-app/tests/qml/tst_probe.qml")).expect("rm");
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/qmldir",
        "module platitude.ui\nsingleton Theme 1.0 Theme.qml\nMain 1.0 Main.qml\nX 1.0 X.qml\n",
    );
    sb.commit_all(&sb.seat, "chore(app-ui): declare X", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let ran = without_always(&sb.ran());
    assert_eq!(ran, set(&["qmltest", "qmltest-linux"]), "{ran:?}");

    // The runner does not read their README: nothing is owed.
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/qmldir",
        "module platitude.ui\nsingleton Theme 1.0 Theme.qml\nMain 1.0 Main.qml\n",
    );
    sb.write(
        &sb.seat,
        "crates/platitude-app/tests/qml/README.md",
        "# how they are run\n",
    );
    sb.commit_all(&sb.seat, "docs: how they are run", &[]);
    let text = sb.gate_ok(&sb.seat, &[]);
    assert!(text.contains("no step reads these"), "{text}");
    let ran = without_always(&sb.ran());
    assert!(ran.is_empty(), "{ran:?}");

    // The staging is what a run resolves through; nothing else exercises
    // a change to the runner.
    sb.write(
        &sb.seat,
        "crates/xtask/src/qmltest.rs",
        "pub fn run() { let _ = 1; }\n",
    );
    sb.commit_all(&sb.seat, "fix(xtask): stage it differently", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let ran = sb.ran();
    assert!(ran.contains("qmltest"), "{ran:?}");
    assert!(ran.contains("qmltest-linux"), "{ran:?}");
}

#[test]
fn a_component_worn_as_another_s_root_is_covered_by_the_one_wearing_it() {
    let sb = Sandbox::new("worn");
    // `StashPane` is what the seeded census names, and this is one.
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/Bar.qml",
        "Item {\n    property int x: 1\n}\n",
    );
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/StashPane.qml",
        "Bar {\n    x: 2\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(app-ui): a bar under the pane", &[]);
    let text = sb.gate_ok(&sb.seat, &[]);
    // Nothing in an item tree ever answers to `Bar` — the run that showed
    // the pane is the run that showed it.
    assert!(!text.contains("no verb shows"), "{text}");
    let ran = sb.ran();
    assert!(ran.contains("verify stash --preset basic"), "{ran:?}");
}

#[test]
fn a_component_no_verb_shows_stops_the_gate_by_name() {
    let sb = Sandbox::new("uncovered");
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/Extra.qml",
        "Item {}\n",
    );
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/Main.qml",
        "Item {\n    StashPane {}\n    Extra {}\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(app-ui): extra", &[]);
    let (ok, text) = sb.gate(&sb.seat, &[], &[]);
    assert!(!ok, "{text}");
    assert!(
        text.contains("no verb shows") && text.contains("ui/Extra.qml"),
        "{text}"
    );
    assert!(
        !text.contains("ui/Main.qml\n") || text.contains("Main"),
        "{text}"
    );
    // The census is asked before anything runs, so `--verb` here records
    // too late and is no way out.
    let (ok, text) = sb.gate(&sb.seat, &["--verb", "extra"], &[]);
    assert!(!ok && text.contains("ui/Extra.qml"), "{text}");
    let way_out = text
        .lines()
        .find(|line| line.starts_with("Run a verb"))
        .unwrap_or_else(|| panic!("no way out said: {text}"));
    assert!(
        way_out.contains("cargo xtask verify-ui <verb>")
            && way_out.contains("commit it")
            && !way_out.contains("--verb"),
        "{way_out}"
    );
    assert!(sb.ran().is_empty());
    // A singleton is never owed a census.
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/Theme.qml",
        "pragma Singleton\nQtObject { property int x: 1 }\n",
    );
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/Main.qml",
        "Item {\n    StashPane {}\n}\n",
    );
    std::fs::remove_file(sb.seat.join("crates/platitude-app/src/ui/Extra.qml")).expect("rm");
    sb.commit_all(&sb.seat, "feat(app-ui): theme", &[]);
    sb.gate_ok(&sb.seat, &[]);
}

/// A row no run wrote: no step runs, and the file is left as it was.
#[test]
fn a_census_that_cannot_be_read_whole_stops_the_gate_by_its_row() {
    let sb = Sandbox::new("census-rows");
    let census = "# census\nstash --preset basic\tDriver Main StashPane\n\
                  stash --preset basic\tMain\n";
    sb.write(&sb.seat, "crates/xtask/verb-census.txt", census);
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/StashPane.qml",
        "Item {\n    property var model: StashModel\n    property int x: 1\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(app-ui): a census taken twice", &[]);
    let (ok, text) = sb.gate(&sb.seat, &[], &[]);
    assert!(!ok, "{text}");
    assert!(
        text.contains("crates/xtask/verb-census.txt:3: \"stash --preset basic\" again"),
        "{text}"
    );
    assert!(sb.ran().is_empty());
    assert_eq!(
        std::fs::read_to_string(sb.seat.join("crates/xtask/verb-census.txt")).expect("census"),
        census
    );
}

#[test]
fn an_xtask_change_leaves_the_app_alone_and_runs_on_both_sides() {
    let sb = Sandbox::new("xtask");
    sb.write(
        &sb.seat,
        "crates/xtask/src/seats.rs",
        "pub fn seat() {}\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn x() {}\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(xtask): seats", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let ran = without_always(&sb.ran());
    assert_eq!(
        ran,
        set(&[
            "clippy xtask",
            "clippy-linux xtask",
            "test xtask 1",
            "test xtask 1 linux",
        ]),
        "{ran:?}"
    );
}

#[test]
fn a_build_input_change_owes_everything() {
    let sb = Sandbox::new("lockfile");
    sb.write(&sb.seat, "Cargo.lock", "# lock\n# bumped\n");
    sb.commit_all(&sb.seat, "chore: bump", &[]);
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(text.contains("everything (Cargo.lock)"), "{text}");
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

/// The graph is read off the tree, so a removed source has no readers it
/// can name (`Main.qml` naming the pane resolves to nothing): everything
/// is the one reach that cannot miss them.
#[test]
fn a_source_taken_out_of_the_tree_owes_everything() {
    let sb = Sandbox::new("gone");
    std::fs::remove_file(sb.seat.join("crates/platitude-app/src/ui/StashPane.qml")).expect("rm");
    sb.commit_all(&sb.seat, "refactor(app-ui): drop the pane", &[]);
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(
        text.contains("everything (crates/platitude-app/src/ui/StashPane.qml is gone"),
        "{text}"
    );
    sb.gate_ok(&sb.seat, &[]);
    let ran = without_always(&sb.ran());
    for owed in [
        "shipped",
        "verify stash --preset basic",
        "test xtask (all)",
        "clippy platitude-app",
    ] {
        assert!(ran.contains(owed), "{owed} not run; ran: {ran:?}");
    }
}

/// Unlike a source: nothing outside its binary can name it, and the root
/// that declared it changes with it.
#[test]
fn a_test_taken_out_of_its_binary_owes_that_binary_and_nothing_else() {
    let sb = Sandbox::new("test-gone");
    std::fs::remove_file(
        sb.seat
            .join("crates/platitude-core/tests/it/refs_integration.rs"),
    )
    .expect("rm");
    sb.write(
        &sb.seat,
        "crates/platitude-core/tests/it/main.rs",
        "mod support;\nmod stash_integration;\n",
    );
    sb.commit_all(&sb.seat, "test(core): drop the refs tests", &[]);
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(!text.contains("everything"), "{text}");
    sb.gate_ok(&sb.seat, &[]);
    let ran = without_always(&sb.ran());
    assert_eq!(
        ran,
        set(&[
            "bare",
            "clippy platitude-core",
            "clippy-linux platitude-core",
            "test it (all)",
            "test it (all) linux",
        ]),
        "{ran:?}"
    );
}

/// git's rename detection would list the new name alone, and readers of
/// the old one would never be reached: read with renames off.
#[test]
fn a_renamed_source_is_seen_to_have_gone() {
    let sb = Sandbox::new("renamed");
    let pane = sb.seat.join("crates/platitude-app/src/ui/StashPane.qml");
    let text = std::fs::read_to_string(&pane).expect("the pane");
    std::fs::remove_file(&pane).expect("rm");
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/StashPanel.qml",
        &text,
    );
    sb.commit_all(&sb.seat, "refactor(app-ui): rename the pane", &[]);
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(
        text.contains("everything (crates/platitude-app/src/ui/StashPane.qml is gone"),
        "{text}"
    );
}

/// What the tier table moves (`verb-tiers.txt`): before a merge a full
/// line waits, a twin never runs, and only a linux line goes to the
/// container; the full gate owes every line but the twin on both sides.
#[test]
fn the_tier_table_says_which_lines_each_gate_owes_and_where() {
    let sb = Sandbox::new("tiers");
    sb.write(
        &sb.seat,
        "crates/xtask/verb-census.txt",
        "# census\nstash --preset basic\tDriver Main StashPane\n\
         stash-menu --preset basic\tDriver Main StashPane\n\
         stash-twin --preset basic\tDriver Main StashPane\n\
         stash-open --preset basic\tDriver Main StashPane\n",
    );
    sb.write(
        &sb.seat,
        "crates/xtask/verb-tiers.txt",
        "linux\tstash --preset basic\t-\tboth sides\n\
         full\tstash-menu --preset basic\tstash --preset basic\tpicture: the menu standing\n\
         twin\tstash-twin --preset basic\tstash --preset basic\tthe same run\n",
    );
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/StashPane.qml",
        "Item {\n    width: 4\n    property var model: StashModel\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(app-ui): pane", &[]);
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(
        text.contains("verbs: 2 selected, 1 of them on the container too; 2 left"),
        "{text}"
    );
    sb.gate_ok(&sb.seat, &[]);
    let verbs: BTreeSet<String> = sb
        .ran()
        .into_iter()
        .filter(|id| id.starts_with("verify"))
        .collect();
    assert_eq!(
        verbs,
        set(&[
            "verify stash --preset basic",
            "verify-linux stash --preset basic",
            "verify stash-open --preset basic",
        ]),
        "{verbs:?}"
    );
    let full = sb.gate_ok(&sb.seat, &["--all", "--dry-run"]);
    for owed in [
        "[host ] verify stash-menu --preset basic",
        "[linux] verify-linux stash-menu --preset basic",
        "[linux] verify-linux stash-open --preset basic",
    ] {
        assert!(
            full.contains(owed),
            "{owed} not owed by the full gate: {full}"
        );
    }
    assert!(
        !full.contains("stash-twin"),
        "a twin is owed by no gate: {full}"
    );
}

/// Waiting for the full gate would leave that component with no run
/// before the merge.
#[test]
fn a_full_line_that_alone_shows_a_reached_component_is_owed_before_the_merge() {
    let sb = Sandbox::new("tiers-alone");
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/auto/PerfDriver.qml",
        "Item {}\n",
    );
    sb.write(
        &sb.seat,
        "crates/xtask/verb-census.txt",
        "# census\nstash --preset basic\tDriver Main StashPane\n\
         perf none --preset perf\tDriver Main PerfDriver\n",
    );
    sb.write(
        &sb.seat,
        "crates/xtask/verb-tiers.txt",
        "full\tperf none --preset perf\t-\tperf: the perf tool walks it\n",
    );
    let base = sb.commit_all(&sb.seat, "test: a perf driver", &[]);
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/auto/PerfDriver.qml",
        "Item { width: 1 }\n",
    );
    sb.commit_all(&sb.seat, "fix(auto): the perf driver", &[]);
    let text = sb.gate_ok(&sb.seat, &["--main", &base, "--dry-run"]);
    assert!(
        text.contains("[host ] verify perf none --preset perf"),
        "the perf driver's one witness waits for nobody: {text}"
    );
    assert!(!text.contains("verify-linux perf"), "{text}");
}

/// The path the real tree has: the QtTest runner reads the QtTest tree,
/// and the harness is built with the runner. A QtTest change reaches the
/// harness as data a tool reads, which is no change to the harness or to
/// the code clippy reads; the runner's own tests may read the data. A
/// change to code the harness is built with still owes every recorded
/// line (`graph::Carried`).
#[test]
fn a_qtest_change_reaches_the_harness_as_data_and_owes_no_verb() {
    let sb = Sandbox::new("harness-as-data");
    sb.write(
        &sb.seat,
        "crates/xtask/src/main.rs",
        "mod qmltest;\nmod seats;\nmod verify;\nfn main() {}\n",
    );
    sb.write(
        &sb.seat,
        "crates/xtask/src/qmltest.rs",
        "pub fn run() -> &'static str { \"crates/platitude-app/tests/qml\" }\n\
         #[cfg(test)]\nmod tests {\n    #[test]\n    fn q() { assert!(!super::run().is_empty()) }\n}\n",
    );
    sb.write(
        &sb.seat,
        "crates/xtask/src/seats.rs",
        "pub fn seat() {}\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn x() {}\n}\n",
    );
    sb.write(
        &sb.seat,
        "crates/xtask/src/verify/mod.rs",
        "use crate::{qmltest, seats};\npub fn run() { qmltest::run(); seats::seat(); }\n",
    );
    sb.write(
        &sb.seat,
        "crates/platitude-app/tests/qml/tst_probe.qml",
        "import QtTest\nItem {\n    TestCase { name: \"Probe\" }\n}\n",
    );
    sb.write(
        &sb.seat,
        "crates/xtask/verb-census.txt",
        "stash --preset basic\tDriver Main StashPane\nwindow\tDriver Main\n",
    );
    let base = sb.commit_all(&sb.seat, "test: harness fixture", &[]);

    sb.write(
        &sb.seat,
        "crates/platitude-app/tests/qml/tst_probe.qml",
        "import QtTest\nItem {\n    TestCase { name: \"Probe\"; function test_it() {} }\n}\n",
    );
    sb.commit_all(&sb.seat, "test(app-ui): probe", &[]);
    sb.gate_ok(&sb.seat, &["--main", &base]);
    let ran = without_always(&sb.ran());
    // The runner is in reach as a reader of the data: its own tests run,
    // its crate's clippy does not (no code of it moved); no verb, and
    // nothing the app is built for.
    assert_eq!(
        ran,
        set(&[
            "qmltest",
            "qmltest-linux",
            "test xtask 1",
            "test xtask 1 linux",
        ]),
        "{ran:?}"
    );

    sb.write(
        &sb.seat,
        "crates/xtask/src/seats.rs",
        "pub fn seat() { let _ = 1; }\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn x() {}\n}\n",
    );
    sb.commit_all(&sb.seat, "fix(xtask): what the harness is built with", &[]);
    sb.gate_ok(&sb.seat, &["--main", &base]);
    let ran = without_always(&sb.ran());
    // The branch's reach holds the runner's tests as well as the seat's.
    for owed in [
        "clippy xtask",
        "test xtask 2",
        "verify stash --preset basic",
        "verify-linux stash --preset basic",
        "verify window",
    ] {
        assert!(ran.contains(owed), "{owed} not run; ran: {ran:?}");
    }
}

/// The other path the real tree has: a tool reads the app's source tree,
/// so a core change reaches the tool as data through the app. The tool's
/// own tests are owed (they may read the data); its crate's clippy is not
/// planned at all, since no code of it moved (`graph::Carried`).
#[test]
fn a_product_change_reaching_a_tool_as_data_owes_its_tests_and_no_clippy() {
    let sb = Sandbox::new("tool-as-data");
    sb.write(
        &sb.seat,
        "crates/xtask/src/main.rs",
        "mod qmltest;\nmod seats;\nfn main() { seats::seat(); }\n",
    );
    sb.write(
        &sb.seat,
        "crates/xtask/src/seats.rs",
        "pub fn seat() -> &'static str { \"crates/platitude-app/src\" }\n\
         #[cfg(test)]\nmod tests {\n    #[test]\n    fn s() { assert!(!super::seat().is_empty()) }\n}\n",
    );
    let base = sb.commit_all(&sb.seat, "test: a tool that reads the app", &[]);
    sb.write(
        &sb.seat,
        "crates/platitude-core/src/stash.rs",
        "pub fn stash() { let _ = 1; }\n#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn t() { stash() }\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(core): stash", &[]);
    let text = sb.gate_ok(&sb.seat, &["--main", &base, "--dry-run"]);
    assert!(
        text.contains("crates/xtask/src/seats.rs") && !text.contains("clippy xtask"),
        "{text}"
    );
    sb.gate_ok(&sb.seat, &["--main", &base]);
    let ran = without_always(&sb.ran());
    for owed in [
        "clippy platitude-core",
        "clippy platitude-app",
        "test xtask 1",
        "test xtask 1 linux",
    ] {
        assert!(ran.contains(owed), "{owed} not run; ran: {ran:?}");
    }
    for spared in ["clippy xtask", "clippy-linux xtask"] {
        assert!(!ran.contains(spared), "{spared} ran; ran: {ran:?}");
    }
}

/// A suite of the tool's own shoots the runner, and the runner reads the
/// product tree and the hook. The suite lays out its own product, so a
/// product change leaves its green standing; a change to the runner's
/// code, or to the hook it reads off the real tree, does not.
#[test]
fn a_tool_suites_green_stands_across_a_product_change_and_falls_with_what_it_reads() {
    let sb = Sandbox::new("suite-key");
    sb.write(
        &sb.seat,
        "crates/xtask/src/main.rs",
        "mod qmltest;\nmod seats;\nfn main() { seats::seat(); }\n",
    );
    let reads = |n: u32| {
        format!(
            "pub fn seat() -> [&'static str; 2] {{ let _ = {n}; [\"crates/platitude-app/src/ui\", \
             \".githooks/reference-transaction\"] }}\n"
        )
    };
    sb.write(&sb.seat, "crates/xtask/src/seats.rs", &reads(0));
    sb.write(
        &sb.seat,
        "crates/xtask/tests/probe.rs",
        "const EXE: &str = env!(\"CARGO_BIN_EXE_xtask\");\n#[test]\nfn shoots() { assert!(!EXE.is_empty()); }\n",
    );
    let base = sb.commit_all(&sb.seat, "test: a suite of the tool's own", &[]);
    let suite = ["test probe (all)", "test probe (all) linux"];

    sb.write(&sb.seat, "crates/xtask/src/seats.rs", &reads(1));
    sb.commit_all(&sb.seat, "feat(xtask): seats", &[]);
    sb.gate_ok(&sb.seat, &["--main", &base]);
    let ran = sb.ran();
    for owed in suite {
        assert!(ran.contains(owed), "{owed} not run; ran: {ran:?}");
    }

    // The product moved under the branch: the suite's green stands.
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/StashPane.qml",
        "Item {\n    width: 5\n    property var model: StashModel\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(app-ui): pane", &[]);
    let text = sb.gate_ok(&sb.seat, &["--main", &base]);
    let ran = sb.ran();
    for spared in suite {
        assert!(
            !ran.contains(spared),
            "{spared} ran again for a product change; ran: {ran:?}\n{text}"
        );
    }
    for standing in [
        "cached [host ] test probe (all)",
        "cached [linux] test probe (all) linux",
    ] {
        assert!(
            text.contains(standing),
            "{standing} not in the plan:\n{text}"
        );
    }
    assert!(ran.contains("verify stash --preset basic"), "{ran:?}");

    // The hook the runner reads off the real tree moved: it runs again.
    let hook = sb.seat.join(".githooks").join("reference-transaction");
    let script = std::fs::read_to_string(&hook).expect("the hook");
    std::fs::write(&hook, format!("{script}# probe\n")).expect("the hook");
    sb.commit_all(&sb.seat, "chore: the hook", &[]);
    sb.gate_ok(&sb.seat, &["--main", &base]);
    let ran = sb.ran();
    for owed in suite {
        assert!(ran.contains(owed), "{owed} not run; ran: {ran:?}");
    }

    // And so does the runner's own code.
    sb.write(&sb.seat, "crates/xtask/src/seats.rs", &reads(2));
    sb.commit_all(&sb.seat, "feat(xtask): seats again", &[]);
    sb.gate_ok(&sb.seat, &["--main", &base]);
    let ran = sb.ran();
    for owed in suite {
        assert!(ran.contains(owed), "{owed} not run; ran: {ran:?}");
    }
}

/// An unreadable tier table is no empty table: empty, the selected lines
/// would all be the host's and none the container's. The gate stops
/// before its first step, and says which file.
#[test]
fn a_tier_table_that_cannot_be_read_stops_the_gate_before_anything_runs() {
    let sb = Sandbox::new("tiers-unread");
    let table = sb.seat.join("crates/xtask/verb-tiers.txt");
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/StashPane.qml",
        "Item {\n    width: 6\n    property var model: StashModel\n}\n",
    );
    std::fs::write(&table, b"linux\tstash --preset basic\t-\t\xff\n").expect("bytes");
    sb.commit_all(&sb.seat, "chore: a table that is not text", &[]);
    let (ok, text) = sb.gate(&sb.seat, &[], &[]);
    assert!(
        !ok && text.contains("verb-tiers.txt") && text.contains("UTF-8"),
        "{text}"
    );
    assert!(
        sb.ran().is_empty(),
        "a plan that could not be made ran something"
    );

    sb.git_ok(&sb.seat, &["rm", "-q", "crates/xtask/verb-tiers.txt"]);
    sb.commit_all(&sb.seat, "chore: no table", &[]);
    let (ok, text) = sb.gate(&sb.seat, &[], &[]);
    assert!(!ok && text.contains("verb-tiers.txt"), "{text}");
    assert!(
        sb.ran().is_empty(),
        "a plan that could not be made ran something"
    );

    // The table back, the line goes to both sides as it says.
    sb.write(
        &sb.seat,
        "crates/xtask/verb-tiers.txt",
        "linux\tstash --preset basic\t-\tthe sandbox's line on both sides\n",
    );
    sb.commit_all(&sb.seat, "chore: the table back", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let ran = sb.ran();
    for owed in [
        "verify stash --preset basic",
        "verify-linux stash --preset basic",
    ] {
        assert!(ran.contains(owed), "{owed} not run; ran: {ran:?}");
    }
}
