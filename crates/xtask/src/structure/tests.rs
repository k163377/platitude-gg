use super::{
    Counted, Ledgered, SRC_CEILING, check_ledger, ledger_entries, names, read_baseline,
    rewrite_ledger, write_baseline,
};
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

fn paths(entries: &[Ledgered]) -> Vec<&str> {
    entries.iter().map(|entry| entry.path.as_str()).collect()
}

fn counted(path: &str, lines: usize) -> Counted {
    Counted {
        path: path.to_string(),
        lines,
        ceiling: SRC_CEILING,
    }
}

#[test]
fn reads_the_ledger_down_to_the_next_heading() {
    let section = "(台帳 — 行が消えたら分割済み)\n\n\
         - **`ui/AutoActDriver.qml`(1769 行)は割らない** — `runAutoAct()` の分岐\n\
         - **`crates/x/src/a.rs` も同じ理由**\n\
         \n## 次の見出し\n\n- **`ui/NotHere.qml`(9 行)は別の話**\n";
    let entries = ledger_entries(section, 0);
    assert_eq!(
        paths(&entries),
        ["ui/AutoActDriver.qml", "crates/x/src/a.rs"]
    );
    assert_eq!(
        entries[0].recorded.clone().map(|(lines, _)| lines),
        Some(1769)
    );
    // A bullet that records no length is still an entry: it has to be
    // told about, not waved through.
    assert_eq!(entries[1].recorded, None);
}

#[test]
fn points_the_recorded_digits_at_where_they_sit_in_the_whole_file() {
    let section = "- **`ui/Pane.qml`(700 行)は割らない** — 理由\n";
    let base = 4096;
    let (lines, digits) = ledger_entries(section, base)[0].recorded.clone().unwrap();
    assert_eq!(lines, 700);
    assert_eq!(&section[digits.start - base..digits.end - base], "700");
}

#[test]
fn reads_no_entry_out_of_prose_that_names_no_file() {
    let entries = ledger_entries("- `runAutoAct()` の分岐は割らない\n", 0);
    assert_eq!(paths(&entries), [""; 0]);
}

#[test]
fn reads_no_file_exemption_out_of_a_bullet_about_one_function() {
    // The fn ceiling keeps its entries in the same section; a file
    // named there in passing keeps its own ceiling.
    let section = "\n\
         - `parse/diff/parse.rs` の `parse_patch` は 196 行(上限 100)\n\
         - **`ui/Pane.qml`(700 行)は割らない** — 理由\n";
    assert_eq!(paths(&ledger_entries(section, 0)), ["ui/Pane.qml"]);
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
fn fails_a_ledgered_file_that_outgrew_the_length_its_entry_records() {
    let ledger = ledger_entries("- **`ui/Pane.qml`(700 行)は割らない** — 理由\n", 0);
    let (failures, shrunk) = check_ledger(&ledger, &[counted("crates/a/src/ui/Pane.qml", 701)]);
    assert!(shrunk.is_empty(), "{shrunk:?}");
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(failures[0].contains("701 lines, 1 more"), "{failures:?}");
}

#[test]
fn follows_a_ledgered_file_down_without_calling_it_a_failure() {
    let ledger = ledger_entries("- **`ui/Pane.qml`(700 行)は割らない** — 理由\n", 0);
    let (failures, shrunk) = check_ledger(&ledger, &[counted("crates/a/src/ui/Pane.qml", 690)]);
    assert!(failures.is_empty(), "{failures:?}");
    let digits = ledger[0].recorded.clone().unwrap().1;
    assert_eq!(shrunk, [(digits, 690)]);
}

#[test]
fn fails_an_entry_that_records_nothing_and_one_that_names_nothing() {
    let ledger = ledger_entries(
        "- **`ui/Pane.qml` は割らない** — 理由\n- **`ui/Gone.qml`(9 行)も**\n",
        0,
    );
    let (failures, shrunk) = check_ledger(&ledger, &[counted("crates/a/src/ui/Pane.qml", 700)]);
    assert!(shrunk.is_empty(), "{shrunk:?}");
    assert_eq!(failures.len(), 2, "{failures:?}");
    assert!(failures[0].contains("records no `(N 行)`"), "{failures:?}");
    assert!(failures[1].contains("names no file"), "{failures:?}");
}

#[test]
fn rewrites_the_recorded_numbers_and_none_of_the_prose_around_them() {
    let dir = test_dir("ledger-rewrite").unwrap();
    let path = dir.join("structure.md");
    let text = "- **`a.rs`(700 行)は割らない** — 理由(と括弧の続き)\n\
         - **`b.qml`(1200 行)も同じ理由**\n";
    let ledger = ledger_entries(text, 0);
    let shrunk = vec![
        (ledger[1].recorded.clone().unwrap().1, 9),
        (ledger[0].recorded.clone().unwrap().1, 44),
    ];
    rewrite_ledger(&path, text, shrunk).unwrap();
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "- **`a.rs`(44 行)は割らない** — 理由(と括弧の続き)\n\
         - **`b.qml`(9 行)も同じ理由**\n"
    );
    std::fs::remove_dir_all(&dir).unwrap();
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
