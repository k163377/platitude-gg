//! `board`'s own tests, in a file of their own (structure.md §分割).

use super::{Run, Shot, load_runs, one_line, parse_run, place, slug, write_run};

#[test]
fn a_run_survives_the_round_trip() {
    let text = "label\tthe chip's badge\nverb\trow-card\nseat\ta\nsession\ts-1\n\
                at\t1700000000000\nshot\timg/x.png\tapp.png\t1440\t900\n";
    let run = parse_run(text).expect("a whole run parses");
    assert_eq!(run.label, "the chip's badge");
    assert_eq!(run.seat, "a");
    assert_eq!(run.session, "s-1");
    assert_eq!(run.at, 1_700_000_000_000);
    assert_eq!(run.shots.len(), 1);
    assert_eq!(run.shots[0].width, 1440);
}

/// A before/after has to come back off the board as one thing: the
/// two are read abreast, and each half keeps the word over it.
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

/// Every other run is read one picture at a time, and says so by
/// carrying no such line — including the ones written before the
/// board could put two pictures side by side.
#[test]
fn a_run_from_before_pairs_is_read_one_at_a_time() {
    let text = "label\tthe chip's badge\nseat\ta\nat\t1700000000000\n\
                shot\timg/x.png\tapp.png\t1440\t900\n";
    let run = parse_run(text).expect("a whole run parses");
    assert!(!run.side_by_side);
    assert!(run.shots[0].caption.is_empty());
}

/// A run written before the board carried sessions is still a run —
/// one that belongs to no session, which is the reading that keeps a
/// session's sweep off it.
#[test]
fn a_run_from_before_sessions_belongs_to_none() {
    let text = "label\tthe chip's badge\nseat\ta\nat\t1700000000000\n\
                shot\timg/x.png\tapp.png\t1440\t900\n";
    let run = parse_run(text).expect("a whole run parses");
    assert!(run.session.is_empty());
}

/// The mark a session's end leaves is read back a day later to decide
/// whether the pictures go, by a command that has only the file to go
/// on — so it has to survive the file, and a run nobody has marked has
/// to come back out unmarked.
#[test]
fn a_session_s_end_is_written_where_the_next_command_reads_it() {
    let runs = std::env::temp_dir().join(format!("pg-shots-mark-{}", std::process::id()));
    std::fs::create_dir_all(&runs).expect("a runs directory to write into");
    let mut run = Run {
        label: "the chip's badge".to_string(),
        verb: String::new(),
        seat: "a".to_string(),
        session: "s-1".to_string(),
        at: 1_700_000_000_000,
        ended: 0,
        side_by_side: false,
        shots: vec![Shot {
            file: "img/x.png".to_string(),
            from: "app.png".to_string(),
            caption: String::new(),
            width: 1,
            height: 1,
        }],
    };
    let path = runs.join("run.tsv");
    place(&path, &run).expect("the run is written");
    let text = std::fs::read_to_string(&path).expect("and read back");
    assert!(
        !text.contains("ended"),
        "a run whose session is still somebody's carries no mark: {text}"
    );
    assert_eq!(parse_run(&text).expect("a whole run parses").ended, 0);

    run.ended = 1_800_000_000_000;
    place(&path, &run).expect("the mark is written over it");
    let text = std::fs::read_to_string(&path).expect("and read back");
    assert_eq!(
        parse_run(&text).expect("a whole run parses").ended,
        1_800_000_000_000
    );
    std::fs::remove_dir_all(&runs).expect("the temporary board goes");
}

/// A file missing what a run *is* is skipped, not guessed at.
#[test]
fn a_run_without_a_picture_is_not_a_run() {
    assert!(parse_run("label\tnamed\nat\t1\n").is_none());
    assert!(parse_run("at\t1\nshot\timg/x.png\tapp.png\t1\t1\n").is_none());
    assert!(parse_run("label\tnamed\nshot\timg/x.png\tapp.png\t1\t1\n").is_none());
}

/// Japanese labels are the common case, and none of it may reach a
/// file name.
#[test]
fn a_slug_keeps_to_ascii() {
    assert_eq!(slug("チップの余白"), "shot");
    assert_eq!(slug("chip padding: top-left"), "chip-padding-top-left");
    assert!(slug(&"x".repeat(200)).len() <= 40);
}

#[test]
fn a_value_never_carries_a_separator() {
    assert_eq!(one_line("two\tlines\nhere"), "two lines here");
}

/// The board is read top down in the order the runs were put up, so
/// what `load_runs` answers is the reading order itself — never the
/// order the directory happened to hand the files back.
#[test]
fn runs_come_back_in_the_order_they_went_up() {
    let runs = std::env::temp_dir().join(format!("pg-shots-order-{}", std::process::id()));
    std::fs::create_dir_all(&runs).expect("a runs directory to write into");
    let run = |at| Run {
        label: "x".to_string(),
        verb: String::new(),
        seat: "a".to_string(),
        session: String::new(),
        at,
        ended: 0,
        side_by_side: false,
        shots: vec![Shot {
            file: "img/x.png".to_string(),
            from: "app.png".to_string(),
            caption: String::new(),
            width: 1,
            height: 1,
        }],
    };
    // Written out of order, so a directory order that reached the
    // page would show up here as one.
    for at in [3, 1, 2] {
        write_run(&runs, &format!("{at}-a-x"), &run(at)).expect("the run is written");
    }
    assert_eq!(
        load_runs(&runs).iter().map(|r| r.at).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    std::fs::remove_dir_all(&runs).expect("the temporary board goes");
}
