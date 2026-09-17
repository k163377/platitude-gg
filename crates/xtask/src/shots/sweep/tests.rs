//! `sweep`'s own tests, in a file of their own (structure.md §分割).

use super::{
    Duration, GRACE, Run, Scope, Whose, collectable, pictures_named_in, referenced_pictures,
    replaced,
};
use std::collections::BTreeSet;

fn run(seat: &str, label: &str, at: u128) -> Run {
    Run {
        label: label.to_string(),
        verb: String::new(),
        seat: seat.to_string(),
        at,
        side_by_side: false,
        shots: Vec::new(),
    }
}

/// A sweep reaches exactly the seats it was named.
/// The board is shared, so the default has to be the narrow answer
/// and the whole board has to be asked for.
#[test]
fn a_prune_reaches_only_the_seats_it_was_given() {
    let mine = Scope::seats(vec!["c".to_string()]);
    assert!(mine.wants(&run("c", "x", 1)));
    assert!(!mine.wants(&run("a", "x", 1)));
    assert!(!mine.wants(&run("main", "x", 1)));
    // Naming several is still exactly those.
    let two = Scope::seats(vec!["c".to_string(), "d".to_string()]);
    assert!(two.wants(&run("d", "x", 1)));
    assert!(!two.wants(&run("e", "x", 1)));
    // A caller that resolved to no seat asked for no run (`--seat`
    // with nothing behind it is an error at the command line, and
    // this is the second lock).
    assert!(!Scope::seats(Vec::new()).wants(&run("c", "x", 1)));
    // And the sweep has to be spelled out.
    let every = Scope {
        whose: Whose::Everything,
        label: None,
    };
    assert!(every.wants(&run("a", "x", 1)));
    assert!(every.wants(&run("main", "x", 1)));
}

/// Dropping one abandoned approach leaves the rest of the seat's
/// work standing.
#[test]
fn a_named_label_narrows_the_sweep_to_itself() {
    let scope = Scope {
        whose: Whose::Seats(vec!["a".to_string()]),
        label: Some("header mock 0".to_string()),
    };
    assert!(scope.wants(&run("a", "header mock 0", 1)));
    assert!(!scope.wants(&run("a", "header mock 2", 1)));
    assert!(!scope.wants(&run("b", "header mock 0", 1)));
}

/// What a retake replaces, and what it leaves alone. The seat and the
/// name are the whole of the view — a picture taken in one conversation
/// is replaced by the same picture taken in the next.
#[test]
fn a_retake_replaces_the_same_view_and_nothing_else() {
    let fresh = run("a", "graph-head", 200);
    assert!(replaced(&run("a", "graph-head", 100), &fresh));
    assert!(
        !replaced(&run("a", "graph-head — linux", 100), &fresh),
        "the other side of Done is another picture"
    );
    assert!(
        !replaced(&run("b", "graph-head", 100), &fresh),
        "another tree's picture says nothing about this one"
    );
    assert!(
        !replaced(&run("a", "graph-head", 300), &fresh),
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

/// A runs directory that is not there is an empty set:
/// `collect_orphans` runs on every rebuild, including the first.
#[test]
fn a_board_without_runs_references_nothing() {
    assert!(referenced_pictures(std::path::Path::new("no/such/runs")).is_empty());
}
