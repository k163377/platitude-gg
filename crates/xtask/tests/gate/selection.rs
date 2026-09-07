//! Which steps a change owes: what the reach of its diff selects, and
//! what it leaves alone.

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
    // And the filters are the modules the reach holds, not the crate.
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

/// A verb is owed for what its run showed, never for what merely reaches
/// it. `Main` reads every pane and every census line names `Main`, so a
/// pane's own leaf used to owe every verb in the file — of which only the
/// ones that built a pane can have taken a different picture.
#[test]
fn a_qml_leaf_owes_the_verbs_that_showed_it_and_not_the_ones_showing_its_readers() {
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
    assert!(text.contains("verbs: 1 of the 2"), "{text}");
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

/// What no census can name is owed by every verb. A singleton stands in
/// no item tree, so no run ever reports it and the file that changed
/// cannot be matched against a line; the same for the app's own Rust,
/// which is the binary every verb runs.
#[test]
fn a_singleton_and_the_apps_rust_owe_every_verb_the_census_holds() {
    let sb = Sandbox::new("qml-unnameable");
    let census = "# census\nstash --preset basic\tDriver Main StashPane\n\
                  window --preset basic\tDriver Main\n";
    let both = [
        "verify stash --preset basic",
        "verify window --preset basic",
        "verify-linux stash --preset basic",
        "verify-linux window --preset basic",
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
/// builds, so a red one stops none of the others: every verb the change
/// owes runs, the greens are stamped, and the run after the fix owes the
/// red alone.
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
    let (ok, text) = sb.gate(&sb.seat, &["--jobs", "2"], &[("PG_GATE_FAKE_FAIL", red)]);
    assert!(!ok && text.contains("nothing stamped"), "{text}");
    let ran = sb.ran();
    for verb in [
        "verify stash --preset basic",
        "verify stash-menu --preset basic",
        red,
        "verify-linux stash-open --preset basic",
    ] {
        assert!(ran.contains(verb), "{verb} did not run; ran: {ran:?}");
    }
    sb.gate_ok(&sb.seat, &["--jobs", "2"]);
    let again = without_always(&sb.ran());
    assert_eq!(again, set(&[red]), "{again:?}");
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
    // It stands in a runner of its own, so no census owes it a verb and
    // nothing here is built: not shipped, not a verb, not bare.
    let ran = without_always(&sb.ran());
    assert_eq!(ran, set(&["qmltest", "qmltest-linux"]), "{ran:?}");

    // The qmldir declares the singletons the tests resolve through, so it
    // is as much of the module as the components are. The file above goes
    // again, so what stands against main is this and nothing else.
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

    // Their README is not something the runner reads, so it is a document
    // like any other and nothing at all is owed for it.
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

    // The staging is what a run resolves through, so a change to the
    // runner is a change nothing else here would exercise.
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
    // A singleton stands in no item tree, so it is never owed a census.
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

/// The policy is read against the closure, not against the sources: a
/// change to neither leaves the step unselected, however far it reaches.
#[test]
fn a_source_change_owes_no_policy_check() {
    let sb = Sandbox::new("deny-source");
    sb.write_refs(&sb.seat, 14);
    sb.commit_all(&sb.seat, "feat(core): refs", &[]);
    sb.gate_ok(&sb.seat, &[]);
    assert!(!sb.ran().contains("deny"));
}

/// A source taken out of the tree has no readers the graph can name —
/// the graph is read off the tree, and `Main.qml` still naming the pane
/// resolves to nothing rather than to an edge — so its reach is
/// everything: the one reach that cannot miss the reader.
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

/// A test file taken out is not that: nothing outside its own binary can
/// name it, and the root that declared it changes with it — which is
/// the whole binary, what any change to the core's crate owes (its
/// clippy, the app's bare start), and nothing beside them.
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

/// A file moved is a file gone under its old name, and git's rename
/// detection would list the new name alone: the readers still naming
/// the old one would then never be reached. Read with renames off.
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
