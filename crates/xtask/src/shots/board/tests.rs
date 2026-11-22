//! `board`'s own tests, in a file of their own (structure.md §分割).

use super::{
    NEEDS_A_NAME, Run, Shot, load_runs, not_a_seat, one_line, parse_run, picture_name, slug,
    write_run, written_label,
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

/// A verb, a `--part` word and a picture's own name are values written
/// too: the format has no escapes, so a newline in one would be read back
/// as a line of its own and a tab as a cell.
#[test]
fn no_value_written_can_write_a_line_of_its_own() {
    let runs = crate::yard::Yard::new("shots-one-line");
    let run = |verb: &str, from: &str, caption: &str| Run {
        label: "x".to_string(),
        verb: verb.to_string(),
        seat: "a".to_string(),
        at: 1,
        side_by_side: true,
        shots: vec![Shot {
            file: "img/x.png".to_string(),
            from: from.to_string(),
            caption: caption.to_string(),
            width: 1,
            height: 1,
        }],
    };
    let another = "before\nlabel\tanother";
    write_run(&runs, "1-a-x", &run(another, "app.png", "")).expect("written");
    let back = load_runs(&runs);
    assert_eq!(back[0].label, "x", "the verb wrote the run's name");
    assert_eq!(back[0].verb, "before label another");
    write_run(&runs, "1-a-x", &run("wip", "app.png", another)).expect("written");
    let back = load_runs(&runs);
    assert_eq!(back[0].label, "x", "the caption wrote the run's name");
    assert_eq!(back[0].shots[0].caption, "before label another");
    write_run(&runs, "1-a-x", &run("wip", "app\t1.png", "")).expect("written");
    let back = load_runs(&runs);
    assert_eq!(back.len(), 1, "a tab in the name split the shot's cells");
    assert_eq!(back[0].shots[0].from, "app 1.png");
}

/// The name the run file says is the name the copy is made under, so
/// it is on one line before either.
#[test]
fn a_picture_is_named_on_one_line_before_it_is_copied() {
    assert_eq!(
        picture_name(std::path::Path::new("shots/app\t1.png")),
        "app 1.png"
    );
    assert_eq!(picture_name(std::path::Path::new("..")), "shot.png");
}

/// What answers a proposal (`hook::proposal`): this seat's runs since the
/// moment, read abreast with something to read against — not a lone
/// picture, not another seat's pair, not a pair from before.
#[test]
fn a_comparison_is_a_seats_pictures_abreast_since_the_moment() {
    let yard = crate::yard::Yard::new("shots-compared");
    let repo = crate::seats::slashed(yard.as_ref());
    assert!(crate::subprocess::git_query(&repo, &["init", "-q"]).is_some());
    let runs = yard.join(".shots/runs");
    std::fs::create_dir_all(&runs).expect("the board's runs");
    let run = |label: &str, seat: &str, at, pictures: usize, abreast| Run {
        label: label.to_string(),
        verb: String::new(),
        seat: seat.to_string(),
        at,
        side_by_side: abreast,
        shots: (0..pictures)
            .map(|n| Shot {
                file: format!("img/{n}.png"),
                from: "app.png".to_string(),
                caption: String::new(),
                width: 1,
                height: 1,
            })
            .collect(),
    };
    for (stem, put) in [
        ("2-d-old", run("1. old pair", "d", 2, 2, true)),
        ("5-d-pair", run("2. pair", "d", 5, 2, true)),
        ("6-d-lone", run("3. lone", "d", 6, 1, false)),
        ("7-d-part", run("4. one part", "d", 7, 1, true)),
        ("8-e-pair", run("5. other seat", "e", 8, 2, true)),
    ] {
        write_run(&runs, stem, &put).expect("the run is written");
    }
    assert_eq!(super::compared_since(&repo, "d", 3), ["2. pair"]);
    assert_eq!(
        super::put_up_since(&repo, "d", 3),
        ["2. pair", "3. lone", "4. one part"]
    );
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
