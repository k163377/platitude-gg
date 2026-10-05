//! What a change to a file the code takes as it stands owes — one it
//! embeds or opens: its readers' tests and what shows them, and no code
//! of the readers moved.

use crate::support::{Sandbox, without_always};

/// The app's entry builds in the harness's QML module, which the stopped
/// runs load through the app: a change to what it builds in owes them,
/// and no code of the entry moved.
#[test]
fn what_the_entry_builds_in_owes_the_stopped_runs() {
    let sb = Sandbox::new("entry-builds-in");
    let module = "crates/platitude-app/src/auto/qmldir";
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/main.rs",
        "mod models;\nconst HARNESS: &str = \"auto/qmldir\";\nfn main() { let _ = HARNESS; }\n",
    );
    sb.write(&sb.seat, module, "module platitude.auto\n");
    let base = sb.commit_all(&sb.seat, "feat(app): the harness's module", &[]);
    sb.write(
        &sb.seat,
        module,
        "module platitude.auto\nDriver 1.0 Driver.qml\n",
    );
    sb.commit_all(&sb.seat, "feat(auto): the driver", &[]);
    sb.gate_ok(&sb.seat, &["--main", &base]);
    let ran = without_always(&sb.ran());
    assert!(ran.contains("wedge-check"), "{ran:?}");
    assert!(!ran.contains("clippy platitude-app"), "{ran:?}");
}
