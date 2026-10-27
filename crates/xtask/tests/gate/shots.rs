//! What the shot check is owed by, and what keeps its stamp: a change at
//! any boundary a picture a read lands in passes through — the check's
//! own closure, the runner it spawns, the knob the app reads and
//! publishes to QML, the harness that draws the mark, the product drawn
//! — asks it again; an edit no build reads keeps its stamp.

use crate::support::{Sandbox, without_always};

const KNOB: &str = "crates/platitude-app/src/harness/knobs.rs";

/// The miniature's picture path: the check reading the pictures through
/// the png module; the runner's option, across the process boundary; the
/// knob the app's singleton reads and publishes, across the boundary to
/// the app and the Rust/QML one; and the harness QML that mentions the
/// singleton.
fn with_the_picture_path(name: &str) -> (Sandbox, String) {
    let sb = Sandbox::new(name);
    sb.write(
        &sb.seat,
        "crates/xtask/src/main.rs",
        "mod demo;\nmod png;\nmod qmltest;\nmod seats;\nmod verify;\nfn main() {}\n",
    );
    sb.write(&sb.seat, "crates/xtask/src/png.rs", "pub fn decode() {}\n");
    sb.write(
        &sb.seat,
        "crates/xtask/src/verify/mod.rs",
        "mod options;\nmod shot_check;\npub fn judge() {}\n",
    );
    sb.write(
        &sb.seat,
        "crates/xtask/src/verify/options.rs",
        "pub fn fault() {}\n",
    );
    sb.write(
        &sb.seat,
        "crates/xtask/src/verify/shot_check.rs",
        "use crate::png;\npub fn run() { png::decode() }\n",
    );
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/main.rs",
        "mod harness;\nmod models;\nfn main() {}\n",
    );
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/harness/mod.rs",
        "pub mod knobs;\npub mod singleton;\n",
    );
    sb.write(&sb.seat, KNOB, &knob("", ""));
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/harness/singleton.rs",
        "use super::knobs;\npub struct Harness;\n#[qobject]\nimpl Harness {\n    \
         fn fault(&self) -> String { knobs::fault_owed_at_shot() }\n}\n",
    );
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/auto/WindowHarness.qml",
        "Item {\n    property string fault: Harness.faultOwedAtShot\n}\n",
    );
    let base = sb.commit_all(&sb.seat, "feat: the picture path", &[]);
    (sb, base)
}

/// The knob, answering `value`, under a comment of `note`.
fn knob(value: &str, note: &str) -> String {
    format!("{note}pub fn fault_owed_at_shot() -> String {{ String::from(\"{value}\") }}\n")
}

#[test]
fn a_change_at_each_boundary_of_a_picture_owes_the_shot_check() {
    let (sb, mut base) = with_the_picture_path("shot-boundaries");
    let boundaries = [
        (
            "crates/xtask/src/png.rs",
            "pub fn decode() { let _ = 1; }\n".to_string(),
            "the pictures' decoding, in the check's own closure",
        ),
        (
            "crates/xtask/src/verify/options.rs",
            "pub fn fault() { let _ = 1; }\n".to_string(),
            "the fault's option, across the process boundary to verify-ui",
        ),
        (
            KNOB,
            knob("app-during", ""),
            "the knob the app reads off its environment, across the boundary to the app",
        ),
        (
            "crates/platitude-app/src/auto/WindowHarness.qml",
            "Item {\n    property string fault: Harness.faultOwedAtShot\n    property bool marked: false\n}\n"
                .to_string(),
            "the harness QML that reads the singleton and draws the mark",
        ),
        (
            "crates/platitude-app/src/ui/StashPane.qml",
            "Item {\n    property var model: StashModel\n    property int rows: 1\n}\n".to_string(),
            "the product the pictures draw",
        ),
    ];
    for (file, text, why) in boundaries {
        sb.write(&sb.seat, file, &text);
        let head = sb.commit_all(&sb.seat, &format!("feat: {file}"), &[]);
        sb.gate_ok(&sb.seat, &["--main", &base]);
        let ran = without_always(&sb.ran());
        assert!(ran.contains("shot-check"), "{why}: {ran:?}");
        base = head;
    }
    // A change beside the path owes it nothing.
    sb.write(
        &sb.seat,
        "crates/platitude-core/tests/it/refs_integration.rs",
        "use crate::support;\nuse platitude_core::refs;\n#[test]\nfn r() { support::repo(); refs::refs(); let _ = 2; }\n",
    );
    sb.commit_all(&sb.seat, "test(core): refs", &[]);
    sb.gate_ok(&sb.seat, &["--main", &base]);
    let ran = without_always(&sb.ran());
    assert!(!ran.contains("shot-check"), "{ran:?}");
}

#[test]
fn an_edit_no_build_reads_keeps_the_shot_checks_stamp() {
    let (sb, base) = with_the_picture_path("shot-comment");
    sb.write(&sb.seat, KNOB, &knob("app-during", ""));
    let changed = sb.commit_all(&sb.seat, "feat(app): the knob", &[]);
    sb.gate_ok(&sb.seat, &["--main", &base]);
    assert!(sb.ran().contains("shot-check"));
    // The same knob under a comment: the key is of the code alone.
    sb.write(
        &sb.seat,
        KNOB,
        &knob("app-during", "/// The picture a read lands in, and when.\n"),
    );
    sb.commit_all(&sb.seat, "docs(app): the knob's comment", &[]);
    sb.gate_ok(&sb.seat, &["--main", &changed]);
    let ran = sb.ran();
    assert!(
        !ran.contains("shot-check"),
        "a comment keeps the stamp: {ran:?}"
    );
    let ledger = sb.ledger(&sb.seat);
    let row = ledger
        .iter()
        .find(|(_, id, _)| id == "shot-check")
        .expect("the check is in the plan, answered by its stamp");
    assert_eq!(row.2, "cached", "{row:?}");
}
