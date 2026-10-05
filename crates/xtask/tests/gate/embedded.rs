//! What a change to a file the code takes as it stands owes — one it
//! embeds or opens: its readers' tests and what shows them, on the systems
//! that compile those readers; and, taken out, the builds that still
//! name it, which break.

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

/// The app embedding a note every system compiles, and an icon only a
/// Windows build does, read by the model the pane shows.
fn embedding(name: &str) -> (Sandbox, String) {
    let sb = Sandbox::new(name);
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/main.rs",
        "mod models;\n#[cfg(windows)]\nmod icon;\nfn main() {}\n",
    );
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/models.rs",
        &models("note.txt"),
    );
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/icon.rs",
        "pub const ICON: &[u8] = include_bytes!(\"../assets/icon.png\");\n",
    );
    sb.write(&sb.seat, "crates/platitude-app/assets/note.txt", "a note\n");
    sb.write(&sb.seat, "crates/platitude-app/assets/icon.png", "png\n");
    let base = sb.commit_all(&sb.seat, "feat(app): embedded files", &[]);
    (sb, base)
}

/// The model, embedding the note under `note`.
fn models(note: &str) -> String {
    format!(
        "use platitude_core::stash;\n#[cfg(windows)]\nuse crate::icon;\n\
         pub const NOTE: &str = include_str!(\"../assets/{note}\");\n\
         pub struct StashModel;\n#[qobject]\nimpl StashModel {{}}\n\
         #[cfg(test)]\nmod tests {{\n    #[test]\n    fn m() {{}}\n}}\n"
    )
}

/// The steps a gate against `base` ran on the container's side.
fn on_the_container(ran: &std::collections::BTreeSet<String>) -> Vec<&String> {
    ran.iter()
        .filter(|id| id.contains("linux") || *id == "bare")
        .collect()
}

#[test]
fn an_embedded_file_owes_what_embeds_it_and_taken_out_the_builds_that_name_it() {
    let (sb, base) = embedding("embedded");

    // New bytes: the readers' tests and the verbs that show them, where the
    // reader is compiled; no build breaks on them.
    sb.write(&sb.seat, "crates/platitude-app/assets/icon.png", "png 2\n");
    let drawn = sb.commit_all(&sb.seat, "feat(app): a new icon", &[]);
    sb.gate_ok(&sb.seat, &["--main", &base]);
    let ran = without_always(&sb.ran());
    assert!(on_the_container(&ran).is_empty(), "{ran:?}");
    assert_eq!(
        ran.contains("verify stash --preset basic"),
        cfg!(windows),
        "{ran:?}"
    );
    assert!(!ran.contains("clippy platitude-app"), "{ran:?}");

    // Renamed with its reader: the reader changed, on every system.
    std::fs::remove_file(sb.seat.join("crates/platitude-app/assets/note.txt")).expect("rm");
    sb.write(
        &sb.seat,
        "crates/platitude-app/assets/notes.txt",
        "a note\n",
    );
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/models.rs",
        &models("notes.txt"),
    );
    let renamed = sb.commit_all(&sb.seat, "refactor(app): the notes", &[]);
    sb.gate_ok(&sb.seat, &["--main", &drawn]);
    let ran = without_always(&sb.ran());
    for owed in ["clippy platitude-app", "clippy-linux platitude-app"] {
        assert!(ran.contains(owed), "{owed} not run; ran: {ran:?}");
    }

    // Taken out under a reader that still names it: every build of the
    // reader breaks, and every system's is owed.
    std::fs::remove_file(sb.seat.join("crates/platitude-app/assets/notes.txt")).expect("rm");
    let unnoted = sb.commit_all(&sb.seat, "refactor(app): no notes", &[]);
    let text = sb.gate_ok(&sb.seat, &["--main", &renamed, "--dry-run"]);
    assert!(!text.contains("everything"), "{text}");
    sb.gate_ok(&sb.seat, &["--main", &renamed]);
    let ran = without_always(&sb.ran());
    for owed in ["clippy platitude-app", "clippy-linux platitude-app"] {
        assert!(ran.contains(owed), "{owed} not run; ran: {ran:?}");
    }

    // The icon taken out: Windows' build breaks, and only it is owed.
    std::fs::remove_file(sb.seat.join("crates/platitude-app/assets/icon.png")).expect("rm");
    sb.commit_all(&sb.seat, "refactor(app): no icon", &[]);
    sb.gate_ok(&sb.seat, &["--main", &unnoted]);
    let ran = without_always(&sb.ran());
    assert!(on_the_container(&ran).is_empty(), "{ran:?}");
    assert_eq!(
        ran.contains("clippy platitude-app"),
        cfg!(windows),
        "{ran:?}"
    );
}
