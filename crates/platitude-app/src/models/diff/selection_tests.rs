//! The cases for `selection`.

use std::sync::Arc;

use platitude_core::parse::diff::parse_patch;

use super::{DiffModel, Washed};
use crate::encode::Optional;

/// One side's wash the way the tests read it: `""` for none, `*` for
/// the whole line, and the runs as `from:len,…` for a cut one.
fn said(sel: &Optional<Washed>) -> String {
    match sel.as_ref() {
        None => String::new(),
        Some(w) if w.whole => "*".to_string(),
        Some(w) => w
            .runs
            .iter()
            .map(|r| format!("{}:{}", r.from, r.len))
            .collect::<Vec<_>>()
            .join(","),
    }
}

/// So the tabs these cases turn on are visible in their source.
const TAB: &str = "\t";

/// The rows laid out by hand (`rows::line_items`) — no QObject side here.
fn model(patch: &str, coloured: bool) -> DiffModel {
    let patches = parse_patch(patch.as_bytes());
    let colors = if coloured {
        platitude_core::highlight::colors(&patches, None)
    } else {
        Default::default()
    };
    DiffModel {
        lines: super::rows::line_items(&patches, true, &colors, None, false),
        shown: Some(Arc::new(patches)),
        ..DiffModel::default()
    }
}

/// hunk / ctx / del / add / ctx on rows 0..4.
fn one_change() -> DiffModel {
    model(
        "\
--- a/f
+++ b/f
@@ -1,3 +1,3 @@
 fn main() {
-    let a = 1;
+    let a = 2;
 }
",
        false,
    )
}

#[test]
fn a_drag_over_the_lot_takes_the_new_side_and_leaves_the_old_one() {
    let mut model = one_change();
    model.start_select(0, 1, 0);
    model.drag_select(0, 4, 1);
    assert_eq!(model.copied_new(), "fn main() {\n    let a = 2;\n}");
    assert_eq!(model.copied_removed(), "    let a = 1;");
    assert!(model.sel_has_new);
    assert_eq!(model.sel_removed, 1);
}

#[test]
fn only_the_rows_the_copy_takes_are_washed() {
    let mut model = one_change();
    model.start_select(0, 0, 0);
    model.drag_select(0, 4, 1);
    let washed: Vec<String> = model.lines.iter().map(|row| said(&row.sel)).collect();
    // The heading and the removed line stay bare; the rest is whole.
    assert_eq!(washed, vec!["", "*", "", "*", "*"]);
}

#[test]
fn the_two_ends_are_cut_and_the_rows_between_them_are_not() {
    let mut model = one_change();
    model.start_select(0, 1, 3);
    model.drag_select(0, 3, 7);
    assert_eq!(model.copied_new(), "main() {\n    let");
    assert_eq!(said(&model.lines[3].sel), "0:7");
}

#[test]
fn a_drag_made_upwards_reads_the_same_as_one_made_down() {
    let mut down = one_change();
    down.start_select(0, 1, 3);
    down.drag_select(0, 3, 7);
    let mut up = one_change();
    up.start_select(0, 3, 7);
    up.drag_select(0, 1, 3);
    assert_eq!(up.copied_new(), down.copied_new());
}

#[test]
fn one_removed_row_taken_whole_offers_the_old_line_and_nothing_new() {
    let mut model = one_change();
    model.select_whole_row(0, 2);
    assert!(!model.sel_has_new);
    assert_eq!(model.sel_removed, 1);
    assert_eq!(model.copied_removed(), "    let a = 1;");
    assert!(model.copied_new().is_empty());
    assert!(model.lines[2].sel.is_none(), "a removed line is not washed");
}

#[test]
fn a_press_with_no_drag_behind_it_copies_nothing() {
    let mut model = one_change();
    model.start_select(0, 1, 4);
    assert!(!model.sel_has_new);
    assert!(model.copied_new().is_empty());
}

#[test]
fn a_selection_says_which_places_it_holds() {
    let mut model = one_change();
    model.start_select(0, 1, 2);
    model.drag_select(0, 3, 4);
    assert!(model.holds(0, 1, 2) && model.holds(0, 2, 0) && model.holds(0, 3, 4));
    assert!(!model.holds(0, 1, 1));
    assert!(!model.holds(0, 3, 5));
    assert!(!model.holds(0, 4, 0));
}

#[test]
fn the_rows_going_take_the_selection_with_them() {
    // What a re-read does before it swaps the rows.
    let mut model = one_change();
    model.start_select(0, 1, 0);
    model.drag_select(0, 4, 1);
    model.forget_selection();
    assert!(!model.sel_active);
    assert!(!model.sel_has_new);
    assert_eq!(model.sel_removed, 0);
    assert!(model.copied_new().is_empty());
    assert!(model.copied_removed().is_empty());
    assert!(!model.holds(0, 1, 0));
}

#[test]
fn a_copy_of_a_coloured_row_still_pastes_the_file_s_own_tabs() {
    // The trap in `selection`'s module doc: tabs spelled as `&nbsp;` on
    // screen.
    let mut model = model(
        &format!(
            "\
diff --git a/src/a.rs b/src/a.rs
--- a/src/a.rs
+++ b/src/a.rs
@@ -1,3 +1,3 @@
 fn main() {{
-{TAB}let a = 1;
+{TAB}let a = 2;
 }}
"
        ),
        true,
    );
    assert!(
        model.lines[3].text.starts_with("<font"),
        "the row under test wears the theme's colours"
    );
    assert!(!model.lines[3].text.contains(TAB), "and its tab is spelled");
    model.select_whole_row(0, 3);
    assert_eq!(model.copied_new(), format!("{TAB}let a = 2;"));
    model.select_whole_row(0, 2);
    assert_eq!(model.copied_removed(), format!("{TAB}let a = 1;"));
}

#[test]
fn a_place_along_a_row_comes_back_as_a_byte_of_its_line() {
    // The row spells the tab as the four spaces to its stop, so places
    // 0..4 are byte 0 and the line's other bytes start after them.
    let model = model(
        &format!(
            "\
--- a/f
+++ b/f
@@ -1,3 +1,3 @@
 fn main() {{
-{TAB}let a = 1;
+{TAB}let a = 2;
 }}
"
        ),
        false,
    );
    assert_eq!(model.byte_at(0, 3, 0), 0);
    assert_eq!(model.byte_at(0, 3, 4), 1);
    assert_eq!(model.byte_at(0, 3, 5), 2);
    // Past the end is the end — a press right of the last character.
    assert_eq!(model.byte_at(0, 3, 9999), 11);
    // A heading and a missing row answer 0.
    assert_eq!(model.byte_at(0, 0, 4), 0);
    assert_eq!(model.byte_at(0, -1, 4), 0);
}

#[test]
fn a_wash_on_a_wide_line_names_places_and_not_columns() {
    // 日 is one place and two columns; runs count places, since only the
    // row's layout turns a place into an x.
    let mut model = model(
        "\
--- a/f
+++ b/f
@@ -1,1 +1,1 @@
-日本語 kept
+日本語 torn
",
        false,
    );
    model.start_select(0, 2, 10);
    model.drag_select(0, 2, 14);
    assert_eq!(said(&model.lines[2].sel), "4:4");
    assert_eq!(model.copied_new(), "torn");
}

/// The moving end as `(side, row, place)`, or nothing.
fn end_of(model: &DiffModel) -> Option<(i32, i32, i32)> {
    model
        .moving_end()
        .as_ref()
        .map(|end| (end.side, end.row, end.place))
}

#[test]
fn the_moving_end_follows_the_hand_in_the_places_its_row_spells() {
    // Row 3 spells its tab as the four spaces to the stop, so its byte 3
    // (after `\tle`) is place 6.
    let mut model = model(
        &format!(
            "\
--- a/f
+++ b/f
@@ -1,3 +1,3 @@
 fn main() {{
-{TAB}let a = 1;
+{TAB}let a = 2;
 }}
"
        ),
        false,
    );
    assert_eq!(end_of(&model), None, "nothing selected, no end");
    model.start_select(0, 1, 2);
    model.drag_select(0, 3, 3);
    assert_eq!(
        end_of(&model),
        Some((0, 3, 6)),
        "dragged down: the lower end"
    );
    model.drag_select(0, 1, 0);
    assert_eq!(
        end_of(&model),
        Some((0, 1, 0)),
        "back up past the start: the end the hand carried"
    );
    model.select_whole_row(0, 3);
    assert_eq!(
        end_of(&model),
        Some((0, 3, 14)),
        "a whole row ends at the line's end"
    );
    model.drop_selection();
    assert_eq!(end_of(&model), None);
}

// ---- the same text read side by side ----------------------------------

/// The rows laid out side by side (`encode::pair_rows`), coloured — a
/// fence is the highlighter's to say (`highlight::LineColors.fence`).
fn split_model(patch: &str) -> DiffModel {
    let patches = parse_patch(patch.as_bytes());
    let colors = platitude_core::highlight::colors(&patches, None);
    DiffModel {
        split: true,
        lines: super::rows::line_items(&patches, true, &colors, None, true),
        shown: Some(Arc::new(patches)),
        ..DiffModel::default()
    }
}

/// hunk / ctx / del+add / ctx on rows 0..3, the change read across row 2.
fn one_change_split() -> DiffModel {
    split_model(
        "\
--- a/f
+++ b/f
@@ -1,3 +1,3 @@
 fn main() {
-    let a = 1;
+    let a = 2;
 }
",
    )
}

/// What every row wears, on each side.
fn washes(model: &DiffModel) -> (Vec<String>, Vec<String>) {
    (
        model.lines.iter().map(|row| said(&row.sel)).collect(),
        model.lines.iter().map(|row| said(&row.pair_sel)).collect(),
    )
}

/// The two sides as a test spells them, in [`washes`]'s own shape.
fn sides(own: &[&str], pair: &[&str]) -> (Vec<String>, Vec<String>) {
    (
        own.iter().map(|s| s.to_string()).collect(),
        pair.iter().map(|s| s.to_string()).collect(),
    )
}

#[test]
fn a_drag_down_the_old_column_takes_the_old_file() {
    // The old column copies the old file, so `Copy removed lines` has
    // nothing left to offer.
    let mut model = one_change_split();
    model.start_select(0, 1, 0);
    model.drag_select(0, 3, 1);
    assert_eq!(model.copied_new(), "fn main() {\n    let a = 1;\n}");
    assert!(model.sel_has_new);
    assert_eq!(model.sel_removed, 0);
    assert!(model.copied_removed().is_empty());
    assert_eq!(
        washes(&model),
        sides(&["", "*", "*", "*"], &["", "", "", ""])
    );
}

#[test]
fn a_drag_down_the_new_column_takes_the_new_file_and_offers_the_removed_line_across() {
    let mut model = one_change_split();
    model.start_select(1, 1, 0);
    model.drag_select(1, 3, 1);
    assert_eq!(model.copied_new(), "fn main() {\n    let a = 2;\n}");
    // The removed line across from the added one is offered, whole.
    assert_eq!(model.sel_removed, 1);
    assert_eq!(model.copied_removed(), "    let a = 1;");
    assert_eq!(
        washes(&model),
        sides(&["", "", "", ""], &["", "*", "*", "*"])
    );
}

#[test]
fn a_press_in_the_other_column_moves_the_whole_wash_across() {
    // The rows inside both selections are the ones that change column,
    // so the ends are not enough to revisit.
    let mut model = one_change_split();
    model.start_select(0, 1, 0);
    model.drag_select(0, 3, 1);
    model.start_select(1, 2, 0);
    model.drag_select(1, 2, 3);
    assert_eq!(
        washes(&model),
        sides(&["", "", "", ""], &["", "", "0:3", ""])
    );
    assert_eq!(model.copied_new(), "   ");
}

#[test]
fn a_selection_is_of_one_column() {
    let mut model = one_change_split();
    model.start_select(1, 1, 3);
    model.drag_select(1, 3, 1);
    assert!(model.holds(1, 2, 0));
    assert!(
        !model.holds(0, 2, 0),
        "the same row in the other column is outside it"
    );
    // A drag that wandered into the other column stays in its own.
    model.drag_select(0, 3, 0);
    assert_eq!(model.sel_side, 1);
    assert_eq!(model.copied_new(), "main() {\n    let a = 2;\n");
}

#[test]
fn a_place_on_the_right_side_is_a_byte_of_the_line_on_the_right() {
    let model = one_change_split();
    // Row 2 holds `    let a = 1;` on the left and `    let a = 2;` on
    // the right; place 12 is the digit of each.
    assert_eq!(model.byte_at(0, 2, 12), 12);
    assert_eq!(model.byte_at(1, 2, 12), 12);
    assert_eq!(model.source_line(0, 2), Some("    let a = 1;"));
    assert_eq!(model.source_line(1, 2), Some("    let a = 2;"));
    // A context row is the same line on both sides; a heading is none.
    assert_eq!(model.source_line(1, 1), model.source_line(0, 1));
    assert_eq!(model.source_line(1, 0), None);
}

#[test]
fn an_empty_seat_takes_nothing_and_breaks_nothing() {
    // Two removed against one added: row 1 is the pair, row 2 a removed
    // line with an empty seat across. A drag down the new column over the
    // seat copies the lines either side of it.
    let mut model = split_model(
        "\
--- a/f
+++ b/f
@@ -1,3 +1,2 @@
-one
-two
+uno
 same
",
    );
    model.start_select(1, 1, 0);
    model.drag_select(1, 3, 4);
    assert_eq!(model.copied_new(), "uno\nsame");
    assert_eq!(model.sel_removed, 2);
    assert_eq!(model.copied_removed(), "one\ntwo");
    assert_eq!(washes(&model).1, vec!["", "*", "", "*"]);
    assert_eq!(
        model.byte_at(1, 2, 3),
        0,
        "the seat has no line to be a byte of"
    );
}

#[test]
fn the_marks_spell_each_side_s_facts() {
    // A conflict's combined diff read side by side: the fence and the
    // side it came from ride the row as the `own` and `pair` records of
    // its marks (`encode::Marks`), and the tally reads both.
    let model = split_model(
        "\
diff --cc f
index 1111111,2222222..0000000
--- a/f
+++ b/f
@@@ -1,3 -1,3 +1,5 @@@
  same
++<<<<<<< HEAD
 +ours
++=======
+ theirs
++>>>>>>> topic
",
    );
    let marks: Vec<super::super::super::encode::Marks> =
        model.lines.iter().map(|row| (*row.marks).clone()).collect();
    let context = &marks[1];
    assert!(
        !context.own.fence && context.own.side.is_empty(),
        "a context line carries nothing on its own side: {context:?}"
    );
    assert!(
        context
            .pair
            .as_ref()
            .is_some_and(|p| !p.fence && p.side.is_empty()),
        "nor on the other: {context:?}"
    );
    let pairs = || marks.iter().filter_map(|m| m.pair.as_ref());
    assert!(
        pairs().any(|p| p.fence),
        "a fence is on the new side: {marks:?}"
    );
    assert!(
        pairs().any(|p| p.side == "ours"),
        "our line is on the new side: {marks:?}"
    );
    assert!(
        pairs().any(|p| p.side == "theirs"),
        "their line is on the new side: {marks:?}"
    );
    assert_eq!(model.side_count("ours".into()), 1);
    assert_eq!(model.side_count("theirs".into()), 1);
}
