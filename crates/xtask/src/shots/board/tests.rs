//! `board`'s own tests, in a file of their own (structure.md §分割).

use super::{
    NEEDS_A_NAME, Run, Shot, load_runs, not_a_seat, one_line, parse_run, slug, write_run,
    written_label,
};

/// Held where the name is typed: once the pictures are on the board,
/// renaming a run means taking it again.
#[test]
fn a_run_is_named_in_the_language_the_board_is_read_in() {
    let named = written_label("チップの余白").expect("a name its reader can read");
    assert_eq!(named, "チップの余白");
    // One Japanese character is the whole of the rule.
    assert!(written_label("AskBar.settled を待った絵").is_ok());
    let refusal = written_label("the chip's padding").expect_err("a name written past the rule");
    assert!(
        refusal.contains("Japanese"),
        "a refusal has to say the rule it is holding to: {refusal}"
    );
    // No name and the wrong language are different mistakes.
    assert_eq!(
        written_label("   ").expect_err("a run with no name at all"),
        NEEDS_A_NAME
    );
}

/// The refusal hands back the one command that fixes it.
#[test]
fn a_picture_is_taken_in_a_seat_or_not_at_all() {
    assert!(not_a_seat("a").is_none());
    assert!(not_a_seat("f").is_none());
    for nowhere in ["main", "?", "spike-tree"] {
        let refusal = not_a_seat(nowhere).unwrap_or_else(|| panic!("{nowhere} is not a seat"));
        assert!(
            refusal.contains("cargo xtask seat"),
            "a refusal has to say what to do instead: {refusal}"
        );
    }
    assert!(
        not_a_seat("spike-tree").is_some_and(|refusal| refusal.contains("spike-tree")),
        "and name the tree it is refusing, so the reader knows where they are"
    );
}

#[test]
fn a_run_survives_the_round_trip() {
    let text = "label\tthe chip's badge\nverb\trow-card\nseat\ta\n\
                at\t1700000000000\nshot\timg/x.png\tapp.png\t1440\t900\n";
    let run = parse_run(text).expect("a whole run parses");
    assert_eq!(run.label, "the chip's badge");
    assert_eq!(run.seat, "a");
    assert_eq!(run.at, 1_700_000_000_000);
    assert_eq!(run.shots.len(), 1);
    assert_eq!(run.shots[0].width, 1440);
}

#[test]
fn a_pair_is_still_a_pair_after_a_rebuild() {
    let text = "label\tthe stopped landing\nseat\ta\nat\t1700000000000\n\
                abreast\t1\n\
                shot\timg/x.png\tapp.png\t1440\t900\tbefore\n\
                shot\timg/y.png\tapp.png\t1440\t900\tafter\n";
    let run = parse_run(text).expect("a whole run parses");
    assert!(run.side_by_side);
    assert_eq!(run.shots.len(), 2);
    assert_eq!(run.shots[0].caption, "before");
    assert_eq!(run.shots[1].caption, "after");
}

/// No `abreast` line and no captions: read one picture at a time.
#[test]
fn a_run_from_before_pairs_is_read_one_at_a_time() {
    let text = "label\tthe chip's badge\nseat\ta\nat\t1700000000000\n\
                shot\timg/x.png\tapp.png\t1440\t900\n";
    let run = parse_run(text).expect("a whole run parses");
    assert!(!run.side_by_side);
    assert!(run.shots[0].caption.is_empty());
}

/// A line the format does not know (`session`) is read past.
#[test]
fn a_run_stamped_with_a_session_is_read_past_it() {
    let text = "label\tthe chip's badge\nseat\ta\nsession\ts-1\nat\t1700000000000\n\
                shot\timg/x.png\tapp.png\t1440\t900\n";
    let run = parse_run(text).expect("a whole run parses");
    assert_eq!(run.seat, "a");
    assert_eq!(run.shots.len(), 1);
}

#[test]
fn a_run_without_a_picture_is_not_a_run() {
    assert!(parse_run("label\tnamed\nat\t1\n").is_none());
    assert!(parse_run("at\t1\nshot\timg/x.png\tapp.png\t1\t1\n").is_none());
    assert!(parse_run("label\tnamed\nshot\timg/x.png\tapp.png\t1\t1\n").is_none());
}

#[test]
fn a_slug_keeps_to_ascii() {
    assert_eq!(slug("チップの余白"), "shot");
    assert_eq!(slug("AskBar.settled を待った絵"), "askbar-settled");
    assert!(slug(&"x".repeat(200)).len() <= 40);
}

#[test]
fn a_value_never_carries_a_separator() {
    assert_eq!(one_line("two\tlines\nhere"), "two lines here");
}

/// The reading order, whatever order the directory lists the files in.
#[test]
fn runs_come_back_in_the_order_they_went_up() {
    let runs = crate::yard::Yard::new("shots-order");
    let run = |at| Run {
        label: "x".to_string(),
        verb: String::new(),
        seat: "a".to_string(),
        at,
        side_by_side: false,
        shots: vec![Shot {
            file: "img/x.png".to_string(),
            from: "app.png".to_string(),
            caption: String::new(),
            width: 1,
            height: 1,
        }],
    };
    // Written out of order, so a directory order would show.
    for at in [3, 1, 2] {
        write_run(&runs, &format!("{at}-a-x"), &run(at)).expect("the run is written");
    }
    assert_eq!(
        load_runs(&runs).iter().map(|r| r.at).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
}
