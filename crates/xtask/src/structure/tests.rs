use super::{ledger_entries, names, read_baseline, write_baseline};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

fn test_dir(label: &str) -> std::io::Result<std::path::PathBuf> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    loop {
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "pg-structure-{label}-{}-{serial}",
            std::process::id()
        ));
        match std::fs::create_dir(&dir) {
            Ok(()) => return Ok(dir),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
}

#[test]
fn reads_the_ledger_down_to_the_next_heading() {
    let section = "(台帳 — 行が消えたら分割済み)\n\n\
         - **`ui/AutoActDriver.qml`(1769 行)は割らない** — `runAutoAct()` の分岐\n\
         - **`crates/x/src/a.rs` も同じ理由**\n\
         \n## 次の見出し\n\n- **`ui/NotHere.qml` は別の話**\n";
    assert_eq!(
        ledger_entries(section),
        ["ui/AutoActDriver.qml", "crates/x/src/a.rs"]
    );
}

#[test]
fn reads_no_entry_out_of_prose_that_names_no_file() {
    assert_eq!(
        ledger_entries("- `runAutoAct()` の分岐は割らない\n"),
        [""; 0]
    );
}

#[test]
fn reads_no_file_exemption_out_of_a_bullet_about_one_function() {
    // The fn ceiling keeps its entries in the same section; a file
    // named there in passing keeps its own ceiling.
    let section = "\n\
         - `parse/diff/parse.rs` の `parse_patch` は 196 行(上限 100)\n\
         - **`ui/Pane.qml`(700 行)は割らない** — 理由\n";
    assert_eq!(ledger_entries(section), ["ui/Pane.qml"]);
}

#[test]
fn matches_a_ledger_entry_only_at_a_path_boundary() {
    assert!(names(
        "ui/AutoActDriver.qml",
        "crates/a/src/ui/AutoActDriver.qml"
    ));
    assert!(names("crates/a/src/nav.rs", "crates/a/src/nav.rs"));
    assert!(!names("nav.rs", "crates/a/src/graph_nav.rs"));
    assert!(!names("ui/Pane.qml", "crates/a/src/ui/PaneX.qml"));
}

#[test]
fn round_trips_a_baseline_through_the_file_it_writes() {
    let dir = test_dir("baseline-test").unwrap();
    let path = dir.join("baseline.txt");
    let entries = BTreeMap::from([
        ("crates/a/src/b.rs".to_string(), 501),
        ("crates/a/tests/it/c.rs".to_string(), 1200),
    ]);
    write_baseline(&path, &entries).unwrap();
    assert_eq!(read_baseline(&path).unwrap(), Some(entries));
    // The header is comments, and the body is one file per line.
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains('\r'), "baseline must stay LF: {text:?}");
    assert!(text.ends_with("501 crates/a/src/b.rs\n1200 crates/a/tests/it/c.rs\n"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn reads_a_missing_baseline_as_the_first_run() {
    let dir = test_dir("baseline-absent").unwrap();
    let path = dir.join("baseline.txt");
    assert_eq!(read_baseline(&path).unwrap(), None);
    std::fs::remove_dir_all(&dir).unwrap();
}
