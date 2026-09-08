//! The cases for `selection`. Beside it rather than in it (structure.md
//! §分割): everything here names only what `selection` publishes to `diff`.

use std::sync::Arc;

use platitude_core::parse::diff::parse_patch;

use super::DiffModel;

/// `\t` written out, so the tabs these cases turn on are visible in the
/// source of the test rather than hiding in its indentation.
const TAB: &str = "\t";

/// The rows laid out by hand rather than through `lay_out_rows`: that one
/// tells the QObject side its list was reset, and there is no QObject side
/// here (`rows::line_items` is the half both use).
fn model(patch: &str, coloured: bool) -> DiffModel {
    let patches = parse_patch(patch.as_bytes());
    let colors = if coloured {
        platitude_core::highlight::colors(&patches, None)
    } else {
        Default::default()
    };
    let mut model = DiffModel::default();
    model.lines = super::rows::line_items(&patches, true, &colors, None);
    model.shown = Some(Arc::new(patches));
    model
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
    model.start_select(1, 0);
    model.drag_select(4, 1);
    assert_eq!(model.copied_new(), "fn main() {\n    let a = 2;\n}");
    assert_eq!(model.copied_removed(), "    let a = 1;");
    assert!(model.sel_has_new);
    assert_eq!(model.sel_removed, 1);
}

#[test]
fn only_the_rows_the_copy_takes_are_washed() {
    let mut model = one_change();
    model.start_select(0, 0);
    model.drag_select(4, 1);
    let washed: Vec<&str> = model.lines.iter().map(|row| row.sel.as_str()).collect();
    // The heading and the removed line stay bare; the rest is whole.
    assert_eq!(washed, vec!["", "*", "", "*", "*"]);
}

#[test]
fn the_two_ends_are_cut_and_the_rows_between_them_are_not() {
    let mut model = one_change();
    model.start_select(1, 3);
    model.drag_select(3, 7);
    assert_eq!(model.copied_new(), "main() {\n    let");
    // The far end is a run of places along the line rather than the whole
    // of it.
    assert_eq!(model.lines[3].sel, "0:7");
}

#[test]
fn a_drag_made_upwards_reads_the_same_as_one_made_down() {
    let mut down = one_change();
    down.start_select(1, 3);
    down.drag_select(3, 7);
    let mut up = one_change();
    up.start_select(3, 7);
    up.drag_select(1, 3);
    assert_eq!(up.copied_new(), down.copied_new());
}

#[test]
fn one_removed_row_taken_whole_offers_the_old_line_and_nothing_new() {
    let mut model = one_change();
    model.select_whole_row(2);
    assert!(!model.sel_has_new);
    assert_eq!(model.sel_removed, 1);
    assert_eq!(model.copied_removed(), "    let a = 1;");
    assert!(model.copied_new().is_empty());
    assert!(
        model.lines[2].sel.is_empty(),
        "a removed line is not washed"
    );
}

#[test]
fn a_press_with_no_drag_behind_it_copies_nothing() {
    let mut model = one_change();
    model.start_select(1, 4);
    assert!(!model.sel_has_new);
    assert!(model.copied_new().is_empty());
}

#[test]
fn a_selection_says_which_places_it_holds() {
    let mut model = one_change();
    model.start_select(1, 2);
    model.drag_select(3, 4);
    assert!(model.holds(1, 2) && model.holds(2, 0) && model.holds(3, 4));
    assert!(!model.holds(1, 1));
    assert!(!model.holds(3, 5));
    assert!(!model.holds(4, 0));
}

#[test]
fn the_rows_going_take_the_selection_with_them() {
    // What a re-read does before it swaps the rows. No wash is written
    // back — the rows it would go on are the ones being replaced.
    let mut model = one_change();
    model.start_select(1, 0);
    model.drag_select(4, 1);
    model.forget_selection();
    assert!(!model.sel_active);
    assert!(!model.sel_has_new);
    assert_eq!(model.sel_removed, 0);
    assert!(model.copied_new().is_empty());
    assert!(model.copied_removed().is_empty());
    assert!(!model.holds(1, 0));
}

#[test]
fn a_copy_of_a_coloured_row_still_pastes_the_file_s_own_tabs() {
    // The trap `selection` exists for: a coloured row's `text` is markup
    // whose tabs have already been spelled out as `&nbsp;`, so a copy
    // taken off the screen would paste spaces.
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
    model.select_whole_row(3);
    assert_eq!(model.copied_new(), format!("{TAB}let a = 2;"));
    model.select_whole_row(2);
    assert_eq!(model.copied_removed(), format!("{TAB}let a = 1;"));
}

#[test]
fn a_place_along_a_row_comes_back_as_a_byte_of_its_line() {
    // What the pane brings back off the row's own layout, and what this
    // side makes of it: the row spells the tab as the four spaces that
    // reach its stop, so places 0..4 stand in it and the line's own bytes
    // start after them.
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
    assert_eq!(model.byte_at(3, 0), 0);
    assert_eq!(model.byte_at(3, 4), 1);
    assert_eq!(model.byte_at(3, 5), 2);
    // Past the end of the line is the end of the line, however far past —
    // the answer the blank right of a row's last character has to give.
    assert_eq!(model.byte_at(3, 9999), 11);
    // A row that names no line of any file answers about none.
    assert_eq!(model.byte_at(0, 4), 0);
    assert_eq!(model.byte_at(-1, 4), 0);
}

#[test]
fn a_wash_on_a_wide_line_names_places_and_not_columns() {
    // 日 is one place and two columns. The run says where the change
    // starts and how far it runs in places, because the row's layout is
    // what turns a place into an x and no count of columns can.
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
    model.start_select(2, 10);
    model.drag_select(2, 14);
    assert_eq!(model.lines[2].sel, "4:4");
    assert_eq!(model.copied_new(), "torn");
}
