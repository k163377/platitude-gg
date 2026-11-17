//! What stage 1 (`gate --related`) runs: the host side of the reach,
//! the verbs narrowed to the lines that show what the change touched, no
//! commit stamp — and what the pre-merge gate after it is left to pay.

use crate::support::{Sandbox, set, without_always};

/// Stage 1 runs, of the verbs, the line that shows the changed popup, on
/// the host alone, and stamps no commit; the pre-merge gate after it pays
/// only what stage 1 left (反映前テストの機械化.md §段 1 / 段 2 / 段 3).
#[test]
fn stage_one_runs_the_lines_that_show_the_change_and_leaves_the_rest_to_the_premerge_gate() {
    let sb = Sandbox::new("related");
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/SettingsDialog.qml",
        "Popup {\n    property int x: 1\n}\n",
    );
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/Main.qml",
        "Item {\n    spacing: Theme.gap\n    StashPane {}\n    SettingsDialog {}\n}\n",
    );
    sb.write(
        &sb.seat,
        "crates/xtask/verb-lines.txt",
        "*\tStashPane\nstash --preset basic\t-\nsettings --preset basic\tSettingsDialog\n",
    );
    let base = sb.commit_all(&sb.seat, "feat(app-ui): a dialog", &[]);
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/SettingsDialog.qml",
        "Popup {\n    property int x: 2\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(app-ui): the dialog moved", &[]);
    let text = sb.gate_ok(&sb.seat, &["--main", &base, "--related"]);
    assert!(
        text.contains("verbs: 1 selected") && text.contains("is not stamped"),
        "{text}"
    );
    assert_eq!(
        without_always(&sb.ran()),
        set(&[
            "qmltest",
            "shipped",
            "shot-check",
            "records-check",
            "verify settings --preset basic"
        ])
    );
    let refused = sb.git(&sb.repo, &["merge", "--ff-only", "worktree-a"], &[]);
    assert!(
        refused.as_ref().is_err_and(|e| e.contains("no gate stamp")),
        "{refused:?}"
    );
    sb.gate_ok(&sb.seat, &["--main", &base]);
    assert_eq!(
        without_always(&sb.ran()),
        set(&[
            "qmltest-linux",
            "verify stash --preset basic",
            "verify-linux stash --preset basic",
            "bare"
        ])
    );
}

/// A Rust change is its own tests' in stage 1: the model a pane names puts
/// the pane in the reach, so the pre-merge gate owes the lines, but no line
/// shows a Rust file.
#[test]
fn stage_one_runs_no_verb_for_a_change_without_qml() {
    let sb = Sandbox::new("related-rust");
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/models.rs",
        "use platitude_core::stash;\npub struct StashModel;\n#[qobject]\nimpl StashModel {\n    \
         fn moved() {}\n}\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn m() {}\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(app): the model moved", &[]);
    let premerge = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(premerge.contains("verbs: 1 selected"), "{premerge}");
    let text = sb.gate_ok(&sb.seat, &["--related"]);
    assert!(
        text.contains("verbs: 0 selected by what they show"),
        "{text}"
    );
    let ran = without_always(&sb.ran());
    assert!(
        !ran.is_empty()
            && !ran.iter().any(|step| step.starts_with("verify")
                || step.ends_with("linux")
                || step == "bare"),
        "{ran:?}"
    );
}

/// A singleton stands in no item tree, so no line shows it: stage 1 runs
/// the lines that show the components reading it.
#[test]
fn stage_one_runs_what_shows_the_readers_of_a_changed_singleton() {
    let sb = Sandbox::new("related-singleton");
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/Theme.qml",
        "pragma Singleton
QtObject {
    readonly property int gap: 4
}
",
    );
    sb.commit_all(&sb.seat, "feat(app-ui): a gap of its own", &[]);
    let text = sb.gate_ok(&sb.seat, &["--related"]);
    assert!(
        text.contains("verbs: 1 selected by what they show"),
        "{text}"
    );
    assert!(sb.ran().contains("verify stash --preset basic"));
}
