//! What the feeds settle onto the model's own properties, in a file of
//! their own (structure.md: a file whose bulk is tests lifts them to a
//! sibling).

use super::testkit::*;
use super::*;

/// The pair the sticky stand-in draws (`HeadPinRow`) comes off the
/// snapshot by name, so it and the row it stands for can never say
/// different numbers — and it keeps them where the row itself is gone,
/// which is exactly when the stand-in is the one on screen.
#[test]
fn the_stand_in_takes_its_counts_off_the_row_it_stands_for() {
    let mut main = local("main", false);
    main.ahead = 2;
    main.behind = 1;
    let mut model = section(
        "branches",
        Source::Locals(locals(vec![local("feature/topic-a", false), main])),
    );
    model.head_name = "main".to_string();
    // A pair that moved is news the sidebar has to be told about, or the
    // stand-in draws the numbers the listing before this one had.
    assert!(model.settle_head_marks());
    model.arrange();

    // One branch, one pair: the row reads it out of the same `BranchItem`.
    assert_eq!(model.head_row, 2);
    assert_eq!(numbers(&model, 2, Role::Ahead), 2);
    assert_eq!(numbers(&model, 2, Role::Behind), 1);
    assert_eq!(model.head_ahead, 2);
    assert_eq!(model.head_behind, 1);

    // A filter takes the row away and the stand-in takes a seat of its
    // own (`HeadPinRow.seated`) — with its pair, because it is read off
    // the snapshot rather than off the arranged rows.
    model.filter = "feature".to_string();
    model.arrange();
    assert_eq!(model.head_row, -1);
    assert_eq!(model.head_ahead, 2);
    assert_eq!(model.head_behind, 1);

    // Nothing to count where the branch is not in the listing at all: a
    // HEAD reported ahead of the first snapshot, or a branch made since
    // it.
    model.head_name = "made-since".to_string();
    assert!(model.settle_head_marks());
    assert_eq!(model.head_ahead, 0);
    assert_eq!(model.head_behind, 0);
}
