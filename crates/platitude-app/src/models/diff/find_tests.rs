//! The cases for `find`: where the bar stands and which rows wear what.

use platitude_core::textfind::Options;

use super::DiffModel;
use super::Runs;
use super::selection_tests::{model, split_model};

/// The runs as the tests read them: `from:len,…`, `""` for none.
fn said(runs: &Runs) -> String {
    runs.iter()
        .map(|r| format!("{}:{}", r.from, r.len))
        .collect::<Vec<_>>()
        .join(",")
}

/// `(row, own found, own standing, pair found, pair standing)` of every
/// row wearing a mark.
fn worn(model: &DiffModel) -> Vec<(usize, String, String, String, String)> {
    model
        .lines
        .iter()
        .enumerate()
        .filter_map(|(row, item)| {
            let own = &item.marks.own;
            let pair = item.marks.pair.as_ref();
            let worn = (
                row,
                said(&own.found),
                said(&own.found_now),
                pair.map(|p| said(&p.found)).unwrap_or_default(),
                pair.map(|p| said(&p.found_now)).unwrap_or_default(),
            );
            (!worn.1.is_empty() || !worn.3.is_empty()).then_some(worn)
        })
        .collect()
}

fn plain(text: &str) -> (String, Options) {
    (text.to_string(), Options::default())
}

/// hunk / ctx `note one` / add `note two` / ctx `nothing` / ctx `note three`
/// on rows 0..4: `note` hits rows 1, 2 and 4.
fn three_notes() -> DiffModel {
    model(
        "\
--- a/f
+++ b/f
@@ -1,3 +1,4 @@
 note one
+note two
 nothing
 note three
",
        false,
    )
}

#[test]
fn the_place_is_asked_from_a_row_and_rounds_at_both_ends() {
    let mut m = three_notes();
    let (text, options) = plain("note");
    m.set_find_query(text, options);
    assert_eq!((m.find_count, m.find_at), (3, 0));
    // From a row: the first hit at or after it.
    assert_eq!(m.find_from_row(3), 4);
    assert_eq!(m.find_at, 3);
    // Past the last: round to the first.
    assert_eq!(m.find_from_row(5), 1);
    assert_eq!(m.find_at, 1);
    // On: 2, 3, then round to 1.
    assert_eq!(m.find_step(1), 2);
    assert_eq!(m.find_step(1), 4);
    assert_eq!(m.find_step(1), 1);
    assert_eq!(m.find_at, 1);
    // Back from the first: round to the last.
    assert_eq!(m.find_step(-1), 4);
    assert_eq!(m.find_at, 3);
    // From no place, back is the last and on is the first.
    m.find_at = 0;
    assert_eq!(m.find_step(-1), 4);
    m.find_at = 0;
    assert_eq!(m.find_step(1), 1);
    // The standing hit's row is said and worn.
    assert_eq!(m.find_row, 1);
    assert_eq!(
        worn(&m),
        [
            (1, "0:4".into(), "0:4".into(), String::new(), String::new()),
            (2, "0:4".into(), String::new(), String::new(), String::new()),
            (4, "0:4".into(), String::new(), String::new(), String::new()),
        ]
    );
}

#[test]
fn rows_laid_out_again_keep_the_place_unless_it_is_past_their_end() {
    let mut m = three_notes();
    let (text, options) = plain("note");
    m.set_find_query(text, options);
    m.find_step(-1);
    assert_eq!(m.find_at, 3);
    // The same rows again: the place stands.
    m.refind();
    assert_eq!((m.find_count, m.find_at, m.find_row), (3, 3, 4));
    // Rows with fewer hits: the place is given up.
    let fewer = model(
        "\
--- a/f
+++ b/f
@@ -1,1 +1,1 @@
 note one
",
        false,
    );
    m.lines = fewer.lines;
    m.shown = fewer.shown;
    m.refind();
    assert_eq!((m.find_count, m.find_at, m.find_row), (1, 0, -1));
    assert_eq!(
        worn(&m),
        [(1, "0:4".into(), String::new(), String::new(), String::new())]
    );
}

#[test]
fn lines_are_counted_through_every_hunk_of_every_patch() {
    let mut m = model(
        "\
--- a/f
+++ b/f
@@ -1,1 +1,1 @@
 a
@@ -3,1 +3,1 @@
 b
--- a/g
+++ b/g
@@ -1,1 +1,1 @@
 c
@@ -3,1 +3,1 @@
 target
",
        false,
    );
    let (text, options) = plain("target");
    m.set_find_query(text, options);
    let row = m.find_from_row(0);
    assert!(row >= 0);
    let row = usize::try_from(row).expect("a row");
    assert_eq!(m.source_line(0, row), Some("target"));
    assert_eq!(
        worn(&m),
        [(
            row,
            "0:6".into(),
            "0:6".into(),
            String::new(),
            String::new()
        )]
    );
}

#[test]
fn a_split_row_wears_a_hit_in_the_column_whose_line_it_is() {
    let mut m = split_model(
        "\
--- a/f
+++ b/f
@@ -1,3 +1,3 @@
 fn main() {
-    let a = 1;
+    let a = 2;
 }
",
    );
    // A shared line stands in both columns.
    let (text, options) = plain("main");
    m.set_find_query(text, options);
    m.find_from_row(0);
    assert_eq!(
        worn(&m),
        [(1, "3:4".into(), "3:4".into(), "3:4".into(), "3:4".into())]
    );
    // Each side's line in its own column: the old on the left, the new on
    // the right.
    let (text, options) = plain("= 2");
    m.set_find_query(text, options);
    m.find_from_row(0);
    assert_eq!(
        worn(&m),
        [(
            2,
            String::new(),
            String::new(),
            "10:3".into(),
            "10:3".into()
        )]
    );
    let (text, options) = plain("= 1");
    m.set_find_query(text, options);
    assert_eq!(
        worn(&m),
        [(
            2,
            "10:3".into(),
            String::new(),
            String::new(),
            String::new()
        )]
    );
}

/// Two hits that meet on a row are worn as one run, two that stand apart
/// as two; the one the bar stands on is worn alone.
#[test]
fn hits_that_meet_are_worn_as_one_and_hits_apart_as_two() {
    let mut m = model(
        "\
--- a/f
+++ b/f
@@ -1,2 +1,2 @@
 abab
 ab ab
",
        false,
    );
    let (text, options) = plain("ab");
    m.set_find_query(text, options);
    assert_eq!(m.find_from_row(0), 1);
    assert_eq!(
        worn(&m),
        [
            (1, "0:4".into(), "0:2".into(), String::new(), String::new()),
            (
                2,
                "0:2,3:2".into(),
                String::new(),
                String::new(),
                String::new()
            ),
        ]
    );
}

/// A hit over two rows is worn on each in its own places: the tab is
/// spelled to its stop at 4, 日 and 本 are a place each, the astral
/// glyph two. A match of the line break alone is no hit.
#[test]
fn a_hit_over_two_rows_is_worn_where_each_row_spells_it() {
    let mut m = model(
        "\
--- a/f
+++ b/f
@@ -1,2 +1,2 @@
 \t日本\u{1f600}x
 語y
",
        false,
    );
    let (text, options) = plain("本\u{1f600}x\n語");
    m.set_find_query(text, options);
    assert_eq!(m.find_from_row(0), 1);
    assert_eq!(
        worn(&m),
        [
            (1, "5:4".into(), "5:4".into(), String::new(), String::new()),
            (2, "0:1".into(), "0:1".into(), String::new(), String::new()),
        ]
    );
    m.set_find_query(
        "\\n".to_string(),
        Options {
            regex: true,
            ..Options::default()
        },
    );
    assert_eq!(m.find_count, 0);
    assert!(worn(&m).is_empty());
}

#[test]
fn hits_of_both_sides_overlapping_on_a_shared_line_are_worn_as_one() {
    let mut m = model(
        "\
--- a/f
+++ b/f
@@ -1,3 +1,3 @@
 keep
-old end
+new end
 keep
",
        false,
    );
    // The old side's hit runs into `ke` of the shared line, the new side's
    // into the whole of `keep`: the line wears the longer, once.
    m.set_find_query(
        "old end\\nke|new end\\nkeep".to_string(),
        Options {
            regex: true,
            ..Options::default()
        },
    );
    assert_eq!(m.find_count, 2);
    assert_eq!(
        worn(&m),
        [
            (2, "0:7".into(), String::new(), String::new(), String::new()),
            (3, "0:7".into(), String::new(), String::new(), String::new()),
            (4, "0:4".into(), String::new(), String::new(), String::new()),
        ]
    );
}
