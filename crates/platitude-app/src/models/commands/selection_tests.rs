//! What a drag over the log takes, and where the wash it leaves is drawn.

use super::selection::{AT_CLOCK, AT_CMD, AT_GAP_CMD, AT_GAP_OUT, AT_OUT, clock_of};
use super::*;

/// One row's wash the way the tests read it: the three columns' runs as
/// `from:len,…`, and whether the line was taken whole; `None` for a row
/// the selection does not reach.
fn said(sel: &Optional<CommandWash>) -> Option<(String, String, String, bool)> {
    let spell = |runs: &Runs| {
        runs.iter()
            .map(|r| format!("{}:{}", r.from, r.len))
            .collect::<Vec<_>>()
            .join(",")
    };
    sel.as_ref().map(|w| {
        (
            spell(&w.clock),
            spell(&w.command),
            spell(&w.outcome),
            w.whole,
        )
    })
}

fn wash(
    clock: &str,
    command: &str,
    outcome: &str,
    whole: bool,
) -> Option<(String, String, String, bool)> {
    Some((
        clock.to_string(),
        command.to_string(),
        outcome.to_string(),
        whole,
    ))
}

fn row(args: &str, status: &str, result: &str, duration: &str, output: &str) -> CommandItem {
    CommandItem {
        clock: "12:03:17".to_string(),
        args: args.to_string(),
        full: format!("git {args}"),
        status: status.to_string(),
        result: result.to_string(),
        duration: duration.to_string(),
        output: output.to_string(),
        sel: Optional::none(),
    }
}

fn log(rows: Vec<CommandItem>) -> CommandsModel {
    let mut model = CommandsModel::default();
    for item in rows {
        model.push_unnotified(item);
    }
    model
}

/// A drag from the first character to the last.
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
    // 00:30Z read five hours west is 19:30 the day before.
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
    // The clock's own column: three places along is three bytes in.
    assert_eq!(model.hit(0, AT_CLOCK, 3), 3);
    // The command column starts after the clock and its tab.
    assert_eq!(model.hit(0, AT_CMD, 0), 9);
    assert_eq!(model.hit(0, AT_CMD, 4), 13);
    // A gap's two places are the two sides of its tab.
    assert_eq!(model.hit(0, AT_GAP_CMD, 0), 8);
    assert_eq!(model.hit(0, AT_GAP_CMD, 1), 9);
    assert_eq!(model.hit(0, AT_GAP_OUT, 0), 26);
    assert_eq!(model.hit(0, AT_GAP_OUT, 1), 27);
    // Past the end of the outcome: the line's length.
    assert_eq!(model.hit(0, AT_OUT, 999), 32);
}

#[test]
fn the_pane_is_handed_the_column_it_has_to_lay_out() {
    // A gap has nothing to lay out, so it is handed nothing.
    let model = log(vec![row("switch -- 3.2", "ok", "exit 1", "29 ms", "")]);
    assert_eq!(model.column(0, AT_CLOCK), "12:03:17");
    assert_eq!(model.column(0, AT_CMD), "git switch -- 3.2");
    assert_eq!(model.column(0, AT_OUT), "exit 1 29 ms");
    assert_eq!(model.column(0, AT_GAP_CMD), "");
    assert_eq!(model.column(1, AT_CMD), "");
}

#[test]
fn a_place_of_a_column_is_a_character_of_it() {
    // Each kanji is one place and three bytes. How wide it is drawn is the
    // row's question: counting columns here would misplace the press.
    let model = log(vec![row("add -- 日本語.txt", "ok", "", "5 ms", "")]);
    let at_cmd = 9;
    assert_eq!(model.hit(0, AT_CMD, 11), at_cmd + 11);
    assert_eq!(model.hit(0, AT_CMD, 12), at_cmd + 14);
    assert_eq!(model.hit(0, AT_CMD, 14), at_cmd + 20);
}

#[test]
fn a_press_below_the_line_is_the_end_of_it() {
    // The words under a failure cannot be pointed into.
    let model = log(vec![row(
        "switch nope",
        "failed",
        "exit 128",
        "12 ms",
        "fatal: no",
    )]);
    let end = model.line_at(0).unwrap().len();
    assert_eq!(model.hit(0, 9, 0), i32::try_from(end).unwrap());
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
    // A drag out past the panel's right edge names a byte past the line.
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
    // Row 0 runs from mid-command to its end: no clock, not whole.
    assert_eq!(said(&model.rows[0].sel), wash("", "4:9", "0:5", false));
    // Row 1 runs from its start to mid-command: no outcome.
    assert_eq!(said(&model.rows[1].sel), wash("0:8", "0:6", "", false));
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
    assert_eq!(said(&model.rows[0].sel), wash("0:8", "", "", false));
    assert_eq!(said(&model.rows[1].sel), None);
}

#[test]
fn a_wash_names_places_and_leaves_the_pixels_to_the_row() {
    // Runs are in places, whatever the font advances them by (`LineRuler`).
    let mut model = log(vec![row("add -- 日本語.txt", "ok", "", "5 ms", "")]);
    let end = model.line_at(0).unwrap().len();
    model.start_select(0, 9);
    model.drag_select(0, i32::try_from(end).unwrap());
    model.respell_row(0);
    // `git add -- ` is eleven places, the three kanji are one each, and
    // `.txt` is four.
    assert_eq!(said(&model.rows[0].sel), wash("", "0:18", "0:4", false));
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
