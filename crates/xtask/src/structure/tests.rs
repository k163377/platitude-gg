use super::{
    Counted, check_ledger, code_lines, comment_share, ledger_entries, names, read_baseline,
    write_baseline,
};
use crate::yard::Yard;
use std::collections::BTreeMap;

/// A directory of this test's own, gone when the test is.
fn test_dir(label: &str) -> Yard {
    Yard::new(&format!("structure-{label}"))
}

fn paths(entries: &[String]) -> Vec<&str> {
    entries.iter().map(String::as_str).collect()
}

fn counted(path: &str, code: usize) -> Counted {
    Counted {
        path: path.to_string(),
        code,
        physical: code,
    }
}

#[test]
fn counts_a_line_once_it_carries_anything_outside_a_comment() {
    let source = "\
        use a::b;\n\
        \n\
        // a whole line of comment\n\
        /// and a doc line\n\
        fn f() {} // trailing comment\n\
        /* opened here\n\
           still comment\n\
        */ closed(); // and code after the close\n\
        /* one-liner */\n\
        let url = \"https://example.test\";\n";
    // use / fn / the line the block closes on / the URL: four.
    assert_eq!(code_lines(source), 4);
}

#[test]
fn counts_nothing_in_a_file_that_is_all_comment() {
    assert_eq!(code_lines("//! module doc\n//! more of it\n\n"), 0);
    assert_eq!(code_lines(""), 0);
}

#[test]
fn reports_the_share_a_file_spends_on_comment_and_blank() {
    assert_eq!(comment_share(100, 68), 32);
    assert_eq!(comment_share(4, 4), 0);
    // An empty file divides by nothing.
    assert_eq!(comment_share(0, 0), 0);
}

#[test]
fn reads_the_ledger_down_to_the_next_heading() {
    let section = "(台帳 — 行が消えたら分割済み)\n\n\
         - **`ui/AutoActDriver.qml` は割らない** — `runAutoAct()` の分岐\n\
         - **`crates/x/src/a.rs` も同じ理由**\n\
         \n## 次の見出し\n\n- **`ui/NotHere.qml` は別の話**\n";
    assert_eq!(
        paths(&ledger_entries(section)),
        ["ui/AutoActDriver.qml", "crates/x/src/a.rs"]
    );
}

#[test]
fn reads_no_entry_out_of_prose_that_names_no_file() {
    let entries = ledger_entries("- `runAutoAct()` の分岐は割らない\n");
    assert_eq!(paths(&entries), [""; 0]);
}

#[test]
fn reads_no_file_exemption_out_of_a_bullet_about_one_function() {
    // The fn ceiling keeps its entries in the same section; a file
    // named there in passing keeps its own backstop.
    let section = "\n\
         - `parse/diff/parse.rs` の `parse_patch` は上限 100 を超える\n\
         - **`ui/Pane.qml` は割らない** — 理由\n";
    assert_eq!(paths(&ledger_entries(section)), ["ui/Pane.qml"]);
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
fn asks_nothing_of_a_ledgered_file_but_that_it_is_still_there() {
    let ledger = ledger_entries("- **`ui/Pane.qml` は割らない** — 理由\n");
    // However long it has become.
    let failures = check_ledger(&ledger, &[counted("crates/a/src/ui/Pane.qml", 4_000)]);
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn fails_an_entry_whose_file_was_split_away() {
    let ledger = ledger_entries("- **`ui/Pane.qml` は割らない** — 理由\n- **`ui/Gone.qml` も**\n");
    let failures = check_ledger(&ledger, &[counted("crates/a/src/ui/Pane.qml", 700)]);
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(failures[0].contains("names no file"), "{failures:?}");
}

#[test]
fn round_trips_a_baseline_through_the_file_it_writes() {
    let dir = test_dir("baseline-test");
    let path = dir.join("baseline.txt");
    let entries = BTreeMap::from([
        ("crates/a/src/b.rs".to_string(), 501),
        ("crates/a/tests/it/c.rs".to_string(), 1200),
    ]);
    write_baseline(&path, &entries).unwrap();
    assert_eq!(read_baseline(&path).unwrap(), Some(entries));
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains('\r'), "baseline must stay LF: {text:?}");
    assert!(text.ends_with("501 crates/a/src/b.rs\n1200 crates/a/tests/it/c.rs\n"));
}

#[test]
fn reads_a_missing_baseline_as_the_first_run() {
    let dir = test_dir("baseline-absent");
    let path = dir.join("baseline.txt");
    assert_eq!(read_baseline(&path).unwrap(), None);
}
