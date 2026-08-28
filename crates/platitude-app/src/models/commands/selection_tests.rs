//! What a drag over the log takes, and where the wash it leaves is drawn.

use super::selection::{AT_CLOCK, AT_CMD, AT_GAP_CMD, AT_GAP_OUT, AT_OUT, clock_of};
use super::*;

/// One measured column of the mono font, and what a wide glyph costs
/// beyond the two it is counted as. Round numbers rather than a real
/// font's: what is being tested is the walk, and a hit is a boundary
/// either way.
const CHAR_W: f64 = 10.0;
const WIDE_DELTA: f64 = -3.0;

fn row(args: &str, state: &str, result: &str, duration: &str, output: &str) -> CommandItem {
    CommandItem {
        clock: "12:03:17".to_string(),
        args: args.to_string(),
        full: format!("git {args}"),
        state: state.to_string(),
        result: result.to_string(),
        duration: duration.to_string(),
        output: output.to_string(),
        sel: String::new(),
    }
}

fn log(rows: Vec<CommandItem>) -> CommandsModel {
    let mut model = CommandsModel::default();
    for item in rows {
        model.push_unnotified(item);
    }
    model
}

/// The whole of every row, which is what a drag from the first character
/// to the last leaves behind.
fn take_all(model: &mut CommandsModel) {
    let last = model.rows.len() - 1;
    let end = model.line_at(last).map_or(0, |line| line.len());
    model.start_select(0, 0);
    model.drag_select(i32::try_from(last).unwrap(), i32::try_from(end).unwrap());
}

#[test]
fn the_clock_is_read_in_the_zone_the_display_side_named() {
    // 2026-08-28 03:03:17Z, and the two offsets either side of it.
    let noon = 1_787_886_197_000;
    assert_eq!(clock_of(noon, 0), "03:03:17");
    assert_eq!(clock_of(noon, -540), "12:03:17");
    assert_eq!(clock_of(noon, 300), "22:03:17");
}

#[test]
fn a_day_that_wraps_backwards_is_still_a_time_of_day() {
    // 00:30Z read five hours west is half past seven the evening before,
    // which is a negative day-of-year the arithmetic must not carry.
    assert_eq!(
        clock_of(
            1_787_876_000_000 - 1_787_876_000_000 % 86_400_000 + 1_800_000,
            300
        ),
        "19:30:00"
    );
}

#[test]
fn a_row_is_its_columns_joined_by_tabs() {
    let model = log(vec![row("switch -- 3.2", "ok", "", "29 ms", "")]);
    assert_eq!(
        model.line_at(0).unwrap().text(),
        "12:03:17\tgit switch -- 3.2\t29 ms"
    );
}

#[test]
fn how_it_went_is_one_column_of_two_words() {
    let model = log(vec![row("switch nope", "failed", "exit 128", "12 ms", "")]);
    assert_eq!(
        model.line_at(0).unwrap().text(),
        "12:03:17\tgit switch nope\texit 128 12 ms"
    );
}

#[test]
fn a_command_still_running_has_no_third_column_yet() {
    let model = log(vec![row("fetch origin", "running", "", "", "")]);
    assert_eq!(
        model.line_at(0).unwrap().text(),
        "12:03:17\tgit fetch origin"
    );
}

#[test]
fn a_press_lands_on_the_byte_it_is_over() {
    let model = log(vec![row("switch -- 3.2", "ok", "", "29 ms", "")]);
    // The clock's own column: three characters along is three bytes in.
    assert_eq!(model.hit(0, AT_CLOCK, 3.0 * CHAR_W, CHAR_W, WIDE_DELTA), 3);
    // The command column starts after the clock and its tab.
    assert_eq!(model.hit(0, AT_CMD, 0.0, CHAR_W, WIDE_DELTA), 9);
    assert_eq!(model.hit(0, AT_CMD, 4.0 * CHAR_W, CHAR_W, WIDE_DELTA), 13);
    // The two gaps hold one tab each, and which side of it the press was
    // on is the whole answer.
    assert_eq!(model.hit(0, AT_GAP_CMD, 0.1, CHAR_W, WIDE_DELTA), 8);
    assert_eq!(model.hit(0, AT_GAP_CMD, 0.9, CHAR_W, WIDE_DELTA), 9);
    assert_eq!(model.hit(0, AT_GAP_OUT, 0.1, CHAR_W, WIDE_DELTA), 26);
    assert_eq!(model.hit(0, AT_GAP_OUT, 0.9, CHAR_W, WIDE_DELTA), 27);
    // And past the end of the outcome, the line's own length.
    assert_eq!(model.hit(0, AT_OUT, 999.0, CHAR_W, WIDE_DELTA), 32);
}

#[test]
fn a_press_below_the_line_is_the_end_of_it() {
    // The block of words under a failure cannot be pointed into, so a
    // press there is the line it belongs to, taken to its end.
    let model = log(vec![row(
        "switch nope",
        "failed",
        "exit 128",
        "12 ms",
        "fatal: no",
    )]);
    let end = model.line_at(0).unwrap().len();
    assert_eq!(
        model.hit(0, 9, 0.0, CHAR_W, WIDE_DELTA),
        i32::try_from(end).unwrap()
    );
}

#[test]
fn a_drag_across_one_command_takes_the_characters_it_crossed() {
    let mut model = log(vec![row("switch -- 3.2", "ok", "", "29 ms", "")]);
    model.start_select(0, 9);
    model.drag_select(0, 26);
    assert_eq!(model.copied(), "git switch -- 3.2");
}

#[test]
fn a_drag_the_other_way_reads_the_same() {
    let mut model = log(vec![row("switch -- 3.2", "ok", "", "29 ms", "")]);
    model.start_select(0, 26);
    model.drag_select(0, 9);
    assert_eq!(model.copied(), "git switch -- 3.2");
}

#[test]
fn rows_come_out_as_lines_in_the_order_they_ran() {
    let mut model = log(vec![
        row("add --all", "ok", "", "23 ms", ""),
        row("reset --quiet", "ok", "", "1 ms", ""),
    ]);
    take_all(&mut model);
    assert_eq!(
        model.copied(),
        "12:03:17\tgit add --all\t23 ms\n12:03:17\tgit reset --quiet\t1 ms"
    );
}

#[test]
fn a_line_taken_whole_brings_gits_own_words_with_it() {
    let mut model = log(vec![row(
        "switch nope",
        "failed",
        "exit 128",
        "12 ms",
        "fatal: invalid reference: nope\nhint: try again",
    )]);
    take_all(&mut model);
    assert_eq!(
        model.copied(),
        "12:03:17\tgit switch nope\texit 128 12 ms\n\tfatal: invalid reference: nope\n\thint: try again"
    );
}

#[test]
fn a_line_taken_in_part_leaves_them_behind() {
    // There is no way on screen to point at part of the block, so it
    // comes with a line taken end to end and not otherwise.
    let mut model = log(vec![row(
        "switch nope",
        "failed",
        "exit 128",
        "12 ms",
        "fatal: no",
    )]);
    model.start_select(0, 9);
    model.drag_select(0, 24);
    assert_eq!(model.copied(), "git switch nope");
}

#[test]
fn a_successful_command_never_brings_its_stderr() {
    let mut model = log(vec![row(
        "fetch origin",
        "ok",
        "",
        "1.42 s",
        "From github.com:o/r",
    )]);
    take_all(&mut model);
    assert_eq!(model.copied(), "12:03:17\tgit fetch origin\t1.42 s");
}

#[test]
fn a_hand_that_ran_off_the_end_stops_at_it() {
    // The end names a byte past the line, which is what a drag out to the
    // right of the panel says. Uncut it addresses nothing and the whole
    // selection reads as empty (2026-08-28 実測).
    let mut model = log(vec![row("add --all", "ok", "", "23 ms", "")]);
    model.start_select(0, 0);
    model.drag_select(0, 9999);
    assert_eq!(model.copied(), "12:03:17\tgit add --all\t23 ms");
}

#[test]
fn nothing_is_selected_until_the_hand_moves() {
    let mut model = log(vec![row("add --all", "ok", "", "23 ms", "")]);
    model.start_select(0, 12);
    assert_eq!(model.copied(), "");
}

#[test]
fn each_row_is_told_where_the_wash_falls_on_it() {
    let mut model = log(vec![
        row("add --all", "ok", "", "23 ms", ""),
        row("reset --quiet", "ok", "", "1 ms", ""),
    ]);
    // From the middle of the first command to the middle of the second.
    model.start_select(0, 13);
    model.drag_select(1, 15);
    model.respell_row(0);
    model.respell_row(1);
    // The first row keeps its clock out of it and runs to the end, so its
    // command and outcome columns both carry a run and the line is not
    // whole.
    assert_eq!(model.rows[0].sel, "|4:0:9:0|0:0:5:0|0");
    // The second starts at its first character, so the clock carries one
    // too -- and it stops inside the command, so nothing is on the third.
    assert_eq!(model.rows[1].sel, "0:0:8:0|0:0:6:0||0");
}

#[test]
fn a_row_the_selection_does_not_reach_is_told_nothing() {
    let mut model = log(vec![
        row("add --all", "ok", "", "23 ms", ""),
        row("reset --quiet", "ok", "", "1 ms", ""),
    ]);
    model.start_select(0, 0);
    model.drag_select(0, 8);
    model.respell_row(0);
    model.respell_row(1);
    assert_eq!(model.rows[0].sel, "0:0:8:0|||0");
    assert_eq!(model.rows[1].sel, "");
}

#[test]
fn a_wide_glyph_is_counted_where_it_is_drawn() {
    // A path in kanji advances two columns and comes from a fallback that
    // does not advance exactly two of them, so the wash needs both counts
    // (`DiffPane.wideDelta`, the same pair the diff is washed with).
    let mut model = log(vec![row("add -- 日本語.txt", "ok", "", "5 ms", "")]);
    let end = model.line_at(0).unwrap().len();
    model.start_select(0, 9);
    model.drag_select(0, i32::try_from(end).unwrap());
    model.respell_row(0);
    // `git add -- ` is 11 columns with no wide glyph in it, then three
    // wide ones worth two columns each, then `.txt`.
    assert_eq!(model.rows[0].sel, "|0:0:21:3|0:0:4:0|0");
}

#[test]
fn the_ends_move_down_with_the_rows_that_fall_off() {
    let mut model = log(vec![
        row("one", "ok", "", "1 ms", ""),
        row("two", "ok", "", "1 ms", ""),
        row("three", "ok", "", "1 ms", ""),
    ]);
    model.start_select(1, 9);
    model.drag_select(2, 18);
    model.rows.remove(0);
    model.shift_selection(1);
    assert_eq!(model.copied(), "git two\t1 ms\n12:03:17\tgit three");
}

#[test]
fn a_selection_whose_head_went_over_the_edge_is_let_go() {
    let mut model = log(vec![
        row("one", "ok", "", "1 ms", ""),
        row("two", "ok", "", "1 ms", ""),
    ]);
    model.start_select(0, 9);
    model.drag_select(1, 12);
    model.shift_selection(1);
    assert_eq!(model.copied(), "");
}
