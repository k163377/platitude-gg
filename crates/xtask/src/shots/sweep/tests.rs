//! `sweep`'s own tests, in a file of their own (structure.md §分割).

use super::{
    Duration, FAREWELL, GRACE, Run, Scope, Whose, collect_expired, collectable, expired, mark_in,
    now_ms, pictures_named_in, referenced_pictures, replaced, taken_by,
};
use std::collections::BTreeSet;

fn run(seat: &str, session: &str, label: &str, at: u128) -> Run {
    Run {
        label: label.to_string(),
        verb: String::new(),
        seat: seat.to_string(),
        session: session.to_string(),
        at,
        ended: 0,
        side_by_side: false,
        shots: Vec::new(),
    }
}

/// The one thing a sweep must never do: reach a seat nobody named.
/// The board is shared, so the default has to be the narrow answer
/// and the whole board has to be asked for.
#[test]
fn a_prune_reaches_only_the_seats_it_was_given() {
    let mine = Scope::seats(vec!["c".to_string()]);
    assert!(mine.wants(&run("c", "s1", "x", 1)));
    assert!(!mine.wants(&run("a", "s1", "x", 1)));
    assert!(!mine.wants(&run("main", "s1", "x", 1)));
    // Naming several is still exactly those.
    let two = Scope::seats(vec!["c".to_string(), "d".to_string()]);
    assert!(two.wants(&run("d", "s1", "x", 1)));
    assert!(!two.wants(&run("e", "s1", "x", 1)));
    // A caller that resolved to no seat asked for no run (`--seat`
    // with nothing behind it is an error at the command line, and
    // this is the second lock).
    assert!(!Scope::seats(Vec::new()).wants(&run("c", "s1", "x", 1)));
    // And the sweep has to be spelled out.
    let every = Scope {
        whose: Whose::Everything,
        label: None,
    };
    assert!(every.wants(&run("a", "s1", "x", 1)));
    assert!(every.wants(&run("main", "", "x", 1)));
}

/// A mark follows the session, not the seat it sat in — and a run
/// stamped with no session is nobody's to mark.
#[test]
fn a_session_speaks_for_its_own_runs_and_no_others() {
    assert!(taken_by(&run("a", "s1", "x", 1), "s1"));
    assert!(
        taken_by(&run("main", "s1", "x", 1), "s1"),
        "wherever it sat"
    );
    assert!(
        !taken_by(&run("a", "s2", "x", 1), "s1"),
        "the next session in the same seat keeps its pictures"
    );
    assert!(
        !taken_by(&run("a", "", "x", 1), "s1"),
        "a hand-taken run is nobody's to mark"
    );
    assert!(
        !taken_by(&run("a", "", "x", 1), ""),
        "and two blanks are not a match"
    );
}

/// The whole of what a session's end may do to a picture, and how long
/// it takes to do it: the board it went out from under was one that
/// deleted here (module doc).
#[test]
fn a_mark_stands_for_a_day_and_an_unmarked_run_forever() {
    let ended = 1_800_000_000_000;
    let mut marked = run("a", "s1", "x", 1);
    marked.ended = ended;
    assert!(!expired(&marked, ended), "a mark just written stands");
    assert!(
        !expired(&marked, ended + FAREWELL.as_millis() - 1),
        "and stands until the day is out"
    );
    assert!(expired(&marked, ended + FAREWELL.as_millis()));
    assert!(
        !expired(&run("a", "s1", "x", 1), ended + FAREWELL.as_millis()),
        "a run nobody marked never expires"
    );
    assert!(
        !expired(&marked, ended - 1),
        "and a clock behind the mark is no reason to delete a picture"
    );
}

/// Dropping one abandoned approach leaves the rest of the seat's
/// work standing.
#[test]
fn a_named_label_narrows_the_sweep_to_itself() {
    let scope = Scope {
        whose: Whose::Seats(vec!["a".to_string()]),
        label: Some("header mock 0".to_string()),
    };
    assert!(scope.wants(&run("a", "s1", "header mock 0", 1)));
    assert!(!scope.wants(&run("a", "s1", "header mock 2", 1)));
    assert!(!scope.wants(&run("b", "s1", "header mock 0", 1)));
}

/// What a retake replaces, and what it leaves alone.
#[test]
fn a_retake_replaces_the_same_view_and_nothing_else() {
    let fresh = run("a", "s1", "graph-head", 200);
    assert!(replaced(&run("a", "s1", "graph-head", 100), &fresh));
    assert!(
        replaced(&run("a", "s2", "graph-head", 100), &fresh),
        "the seat and the name are the view; who took it is not"
    );
    assert!(
        !replaced(&run("a", "s1", "graph-head — linux", 100), &fresh),
        "the other side of Done is another picture, not this one again"
    );
    assert!(
        !replaced(&run("b", "s1", "graph-head", 100), &fresh),
        "another tree's picture says nothing about this one"
    );
    assert!(
        !replaced(&run("a", "s1", "graph-head", 300), &fresh),
        "a run taken later is not one this replaces"
    );
}

/// Collection waits out the window in which a run's own pictures are
/// written before the file that names them.
#[test]
fn only_an_old_picture_nothing_points_at_is_collected() {
    let referenced: BTreeSet<String> = ["img/kept.png".to_string()].into_iter().collect();
    let old = GRACE + Duration::from_secs(1);
    assert!(collectable("stray.png", &referenced, old));
    assert!(!collectable("kept.png", &referenced, old));
    assert!(
        !collectable("stray.png", &referenced, Duration::from_secs(1)),
        "a picture written a moment ago may be a run still writing"
    );
}

/// A run file that no longer parses still speaks for its pictures.
#[test]
fn a_reference_is_read_out_of_any_line_that_names_one() {
    let whole = "label\tx\nseat\ta\nat\t1\nshot\timg/a.png\tapp.png\t1\t1\n";
    assert_eq!(pictures_named_in(whole), vec!["img/a.png".to_string()]);
    // No label, no time: `parse_run` refuses it, and its pictures
    // are still spoken for.
    let broken = "shot\timg/b.png\tapp.png\t1\t1\nshot\timg/c.png\toverlay.png\t1\t1\n";
    assert_eq!(
        pictures_named_in(broken),
        vec!["img/b.png".to_string(), "img/c.png".to_string()]
    );
    assert!(pictures_named_in("label\tno pictures here\n").is_empty());
}

/// A runs directory that is not there is an empty set, not a panic:
/// `collect_orphans` runs on every rebuild, including the first.
#[test]
fn a_board_without_runs_references_nothing() {
    assert!(referenced_pictures(std::path::Path::new("no/such/runs")).is_empty());
}

/// The board a session wakes up to, asserted on the files themselves
/// because that is where a reader meets it: an end marks, a session
/// heard from again unmarks, and only a mark left standing for a day
/// takes anything off the board.
#[test]
fn a_sleeping_session_finds_its_pictures_where_it_left_them() {
    let board = temp_board("wake");
    put(&board, "one", "s1", "img/one.png");
    put(&board, "two", "s2", "img/two.png");
    put(&board, "hand", "", "img/hand.png");

    let now = now_ms().expect("a clock this side of the epoch");
    assert_eq!(
        mark_in(&board, "s1", now).expect("the end marks"),
        1,
        "one session's end reaches one session's runs"
    );
    assert_eq!(
        collect_expired(&board),
        0,
        "and takes nothing off the board"
    );
    assert!(board.join("img/one.png").is_file());

    // The wake: the same session, going on working.
    assert_eq!(mark_in(&board, "s1", 0).expect("the mark comes off"), 1);
    assert_eq!(
        mark_in(&board, "s1", 0).expect("and stays off"),
        0,
        "a session heard from twice rewrites nothing"
    );

    // A day later, for a session that never came back: the mark is
    // written in 1970, which is every FAREWELL there has ever been.
    assert_eq!(mark_in(&board, "s2", 1).expect("the end marks"), 1);
    assert_eq!(collect_expired(&board), 1);
    assert!(!board.join("runs/two.tsv").exists());
    assert!(
        !board.join("img/two.png").exists(),
        "a run leaves with its pictures"
    );
    assert!(
        board.join("runs/one.tsv").is_file() && board.join("runs/hand.tsv").is_file(),
        "and takes nobody else's"
    );
    std::fs::remove_dir_all(&board).expect("the temporary board goes");
}

/// A board of its own, under a name no other test writes into: these
/// run alongside one another in one process (CLAUDE.md ビルド・テスト).
fn temp_board(what: &str) -> std::path::PathBuf {
    let board = std::env::temp_dir().join(format!(
        "pg-shots-{what}-{}-{}",
        std::process::id(),
        now_ms().unwrap_or_default()
    ));
    for dir in ["runs", "img"] {
        std::fs::create_dir_all(board.join(dir)).expect("a board to write into");
    }
    board
}

/// One run on that board, with the picture it names beside it.
fn put(board: &std::path::Path, stem: &str, session: &str, picture: &str) {
    std::fs::write(board.join(picture), "not really a png").expect("a picture to point at");
    std::fs::write(
        board.join(format!("runs/{stem}.tsv")),
        format!(
            "label\t{stem}\nseat\ta\nsession\t{session}\nat\t1700000000000\n\
             shot\t{picture}\tapp.png\t1440\t900\n"
        ),
    )
    .expect("a run to mark");
}
