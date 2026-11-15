//! What a drag over the log takes, and where the wash it leaves is drawn.

use super::selection::{AT_CLOCK, AT_CMD, AT_GAP_CMD, AT_GAP_OUT, AT_OUT, clock_of};
use super::*;

/// The two positions the way the slots build them, from the signed values
/// QML hands.
fn source(row: i32, byte: i32) -> SourcePosition {
    SourcePosition::new(row, byte).unwrap()
}

fn layout(row: i32, utf16: i32) -> LayoutPosition {
    LayoutPosition::new(row, utf16).unwrap()
}

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
    model.start_select(source(0, 0));
    model.drag_select(SourcePosition {
        row: last,
        byte: end,
    });
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
    assert_eq!(model.hit(AT_CLOCK, layout(0, 3)), 3);
    // The command column starts after the clock and its tab.
    assert_eq!(model.hit(AT_CMD, layout(0, 0)), 9);
    assert_eq!(model.hit(AT_CMD, layout(0, 4)), 13);
    // A gap's two places are the two sides of its tab.
    assert_eq!(model.hit(AT_GAP_CMD, layout(0, 0)), 8);
    assert_eq!(model.hit(AT_GAP_CMD, layout(0, 1)), 9);
    assert_eq!(model.hit(AT_GAP_OUT, layout(0, 0)), 26);
    assert_eq!(model.hit(AT_GAP_OUT, layout(0, 1)), 27);
    // Past the end of the outcome: the line's length.
    assert_eq!(model.hit(AT_OUT, layout(0, 999)), 32);
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
    assert_eq!(model.hit(AT_CMD, layout(0, 11)), at_cmd + 11);
    assert_eq!(model.hit(AT_CMD, layout(0, 12)), at_cmd + 14);
    assert_eq!(model.hit(AT_CMD, layout(0, 14)), at_cmd + 20);
}

/// `git add -- 日😀.txt` in the command column, which starts at byte 9:
/// `git add -- ` is eleven places, the kanji one and the astral glyph two,
/// so places 11, 12 and 14 are bytes 9 + 11, 9 + 14 and 9 + 18.
fn kanji_astral() -> CommandsModel {
    log(vec![
        row("add -- 日😀.txt", "ok", "", "5 ms", ""),
        row("status", "ok", "", "1 ms", ""),
    ])
}

#[test]
fn a_place_of_a_column_counts_utf16_units() {
    let model = kanji_astral();
    let cmd = |place| model.hit(AT_CMD, layout(0, place));
    assert_eq!([cmd(11), cmd(12), cmd(14)], [20, 23, 27]);
    // Between the astral glyph's two units the nearer end wins, ties right.
    assert_eq!(cmd(13), 27);
    // A row past the rows is no line at all.
    assert_eq!(model.hit(AT_CLOCK, layout(9, 3)), 0);
}

#[test]
fn a_drag_between_two_places_copies_what_lies_between_them() {
    let mut down = kanji_astral();
    let from = down.hit(AT_CMD, layout(0, 11));
    let to = down.hit(AT_CMD, layout(0, 14));
    down.start_select(source(0, from));
    down.drag_select(source(0, to));
    assert_eq!(down.copied(), "日😀");
    let mut up = kanji_astral();
    up.start_select(source(0, to));
    up.drag_select(source(0, from));
    assert_eq!(up.copied(), "日😀", "dragged the other way");
    // From the gap's near side, the tab comes along.
    down.start_select(source(0, down.hit(AT_GAP_CMD, layout(0, 0))));
    down.drag_select(source(0, down.hit(AT_CMD, layout(0, 3))));
    assert_eq!(down.copied(), "\tgit");
    // From the last column of one row into the first of the next.
    down.start_select(source(0, down.hit(AT_OUT, layout(0, 0))));
    down.drag_select(source(1, down.hit(AT_CLOCK, layout(1, 2))));
    assert_eq!(down.copied(), "5 ms\n12");
}

/// A cut inside a column is counted from that column's own start, and a
/// column the cut only touches at its first byte wears nothing.
#[test]
fn a_wash_inside_a_column_counts_from_that_column() {
    let mut model = kanji_astral();
    model.start_select(source(0, 20));
    model.drag_select(source(0, 27));
    model.respell_row(0);
    assert_eq!(said(&model.rows[0].sel), wash("", "11:3", "", false));
    // Byte 32 is the first of the outcome column, `5 ms`.
    model.drag_select(source(0, 32));
    model.respell_row(0);
    assert_eq!(said(&model.rows[0].sel), wash("", "11:7", "", false));
    assert_eq!(model.copied(), "日😀.txt\t");
}

#[test]
fn a_drag_with_nothing_selected_selects_nothing() {
    let mut model = kanji_astral();
    assert!(!model.drag_select(source(0, 5)));
    assert_eq!(model.sel_ends, None);
    assert_eq!(model.copied(), "");
}

#[test]
fn a_drag_that_stays_on_one_place_moves_nothing() {
    let mut model = kanji_astral();
    model.start_select(source(0, 9));
    assert!(model.drag_select(source(0, 20)));
    assert!(!model.drag_select(source(0, 20)), "the same place again");
    assert!(model.drag_select(source(0, 23)));
    assert_eq!(model.copied(), "git add -- 日");
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
    assert_eq!(model.hit(9, layout(0, 0)), i32::try_from(end).unwrap());
}

#[test]
fn a_drag_across_one_command_takes_the_characters_it_crossed() {
    let mut model = log(vec![row("switch -- 3.2", "ok", "", "29 ms", "")]);
    model.start_select(source(0, 9));
    model.drag_select(source(0, 26));
    assert_eq!(model.copied(), "git switch -- 3.2");
}

#[test]
fn a_drag_the_other_way_reads_the_same() {
    let mut model = log(vec![row("switch -- 3.2", "ok", "", "29 ms", "")]);
    model.start_select(source(0, 26));
    model.drag_select(source(0, 9));
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
    model.respell_row(0);
    assert_eq!(
        said(&model.rows[0].sel).map(|wash| wash.3),
        Some(true),
        "and its wash says so"
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
    model.start_select(source(0, 9));
    model.drag_select(source(0, 24));
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
    // A command still running has no outcome yet: the outcome column's place
    // is past its line, the tab before it not there.
    let mut model = log(vec![row("fetch origin", "running", "", "", "")]);
    let end = model.line_at(0).unwrap().len();
    let off = model.hit(AT_OUT, layout(0, 0));
    assert!(usize::try_from(off).unwrap() > end, "a place past the line");
    model.start_select(source(0, 0));
    model.drag_select(source(0, off));
    assert_eq!(model.copied(), "12:03:17\tgit fetch origin");
    model.respell_row(0);
    assert_eq!(
        said(&model.rows[0].sel).map(|wash| wash.3),
        Some(true),
        "its wash takes the line whole"
    );
}

#[test]
fn nothing_is_selected_until_the_hand_moves() {
    let mut model = log(vec![row("add --all", "ok", "", "23 ms", "")]);
    model.start_select(source(0, 12));
    assert_eq!(model.copied(), "");
}

#[test]
fn each_row_is_told_where_the_wash_falls_on_it() {
    let mut model = log(vec![
        row("add --all", "ok", "", "23 ms", ""),
        row("reset --quiet", "ok", "", "1 ms", ""),
    ]);
    // From the middle of the first command to the middle of the second.
    model.start_select(source(0, 13));
    model.drag_select(source(1, 15));
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
    model.start_select(source(0, 0));
    model.drag_select(source(0, 8));
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
    model.start_select(source(0, 9));
    model.drag_select(SourcePosition { row: 0, byte: end });
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
    model.start_select(source(1, 9));
    model.drag_select(source(2, 18));
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
    model.start_select(source(0, 9));
    model.drag_select(source(1, 12));
    model.shift_selection(1);
    assert_eq!(model.sel_ends, None, "let go, not left holding nothing");
    assert_eq!(model.copied(), "");
}

// ---- a double-click takes a word (`take_word`) ---------------------------

/// `12:03:17\tgit switch -- 3.2\t29 ms` over
/// `12:03:17\tgit push origin main\texit 128 1.2 s`, which failed.
fn two_commands() -> CommandsModel {
    log(vec![
        row("switch -- 3.2", "ok", "", "29 ms", ""),
        row(
            "push origin main",
            "failed",
            "exit 128",
            "1.2 s",
            "fatal: no",
        ),
    ])
}

/// What a double-click on place `place` of column `at` of row `row` copies.
fn word_at(model: &mut CommandsModel, row: i32, at: i32, place: i32) -> String {
    model.take_word(at, layout(row, place));
    model.copied()
}

#[test]
fn a_double_click_takes_the_word_under_the_pointer_in_its_column() {
    let mut model = two_commands();
    assert_eq!(word_at(&mut model, 0, AT_CMD, 5), "switch");
    assert_eq!(
        said(&model.rows[0].sel),
        wash("", "4:6", "", false),
        "washed over the word alone"
    );
    assert_eq!(word_at(&mut model, 0, AT_CMD, 11), "-", "a dash alone");
    assert_eq!(
        word_at(&mut model, 0, AT_CLOCK, 1),
        "12",
        "the clock's hour"
    );
    assert_eq!(word_at(&mut model, 0, AT_OUT, 0), "29", "the time taken");
    assert_eq!(
        word_at(&mut model, 1, AT_OUT, 2),
        "exit",
        "a failure's line, without git's words"
    );
}

#[test]
fn a_double_click_on_a_gap_below_the_line_or_past_its_end_takes_nothing() {
    let mut model = two_commands();
    let text = model.line_at(0).unwrap().text();
    // Each with the byte its second press lands on (`hit`).
    for (at, place, pressed, said) in [
        (AT_GAP_CMD, 0, 8, "the gap before the command"),
        (AT_GAP_OUT, 0, 26, "the gap after it"),
        (9, 0, text.len(), "git's words below the line"),
        (AT_OUT, 5, text.len(), "past the end of `29 ms`"),
    ] {
        model.take_word(AT_CMD, layout(0, 5));
        model.start_select(source(0, i32::try_from(pressed).unwrap()));
        assert!(
            !model.take_word(at, layout(0, place)),
            "{said}: nothing moved"
        );
        assert_eq!(model.copied(), "", "{said}");
        assert_eq!(model.sel_word, None, "{said}");
        // A drag behind it goes place by place from the press.
        model.drag_select(source(0, 12));
        assert_eq!(
            model.copied(),
            text[pressed.min(12)..pressed.max(12)],
            "{said}: the drag behind it"
        );
    }
}

#[test]
fn a_drag_behind_a_double_click_grows_word_by_word_across_rows() {
    let mut model = two_commands();
    model.take_word(AT_CMD, layout(0, 5));
    // Into `push` on the next row: byte 9 is where its command starts.
    model.drag_select(source(1, 9 + 5));
    assert_eq!(model.copied(), "switch -- 3.2\t29 ms\n12:03:17\tgit push");
    // Back into `git` on the first row.
    model.drag_select(source(0, 9 + 1));
    assert_eq!(model.copied(), "git switch");
    // A plain press after it drags place by place again.
    model.start_select(source(0, 9 + 5));
    model.drag_select(source(0, 9 + 7));
    assert_eq!(model.copied(), "wi");
}

#[test]
fn a_double_clicked_word_moves_with_its_row_or_goes_with_it() {
    let mut model = two_commands();
    model.take_word(AT_CMD, layout(1, 5));
    model.shift_selection(1);
    assert_eq!(model.sel_word, Some(SelectionEnds::word(0, 13..17)));
    model.take_word(AT_CMD, layout(0, 5));
    model.shift_selection(1);
    assert_eq!(model.sel_word, None, "its row fell off");
    // The word's row stays, but the drag behind it reached the row that
    // fell off: the selection goes, and the word with it.
    model.take_word(AT_CMD, layout(1, 5));
    model.drag_select(source(0, 9 + 5));
    model.shift_selection(1);
    assert_eq!(model.sel_ends, None);
    assert_eq!(model.sel_word, None, "no word stands beside no selection");
}
