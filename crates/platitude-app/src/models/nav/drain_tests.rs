//! What the feeds settle onto the model's own properties, in a file of
//! their own (structure.md: a file whose bulk is tests lifts them to a
//! sibling).

use super::testkit::*;
use super::*;

/// The pair the sticky stand-in draws (`HeadPinRow`) comes off the
/// snapshot by name, so it and the row it stands for always say the
/// same numbers — and it keeps them where the row itself is gone,
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
    // the snapshot.
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

/// The stand-in wears the badge its row wears, the state on it
/// included: a branch measured against an upstream git cannot reach
/// keeps the mark and takes the warning, because the far side deleting
/// the ref is what took `has_remote` away (デザイン規約 §ref の種別).
/// **The name, not a flag** — the line the stand-in opens says it, and
/// both come off this one answer.
#[test]
fn the_stand_in_carries_the_upstream_that_is_not_here() {
    let mut pruned = local("release-1.2", true);
    pruned.upstream_gone = "origin/release-1.2".into();
    let mut model = section(
        "branches",
        Source::Locals(locals(vec![local("main", false), pruned])),
    );
    model.head_name = "release-1.2".to_string();
    assert!(model.settle_head_marks());
    assert_eq!(model.head_upstream_gone, "origin/release-1.2");
    // Nothing is left of the ordinary reason for the badge, which is
    // why the state has to ride on the mark rather than take it away.
    assert!(!model.head_has_remote);

    // The branch whose upstream is here leaves it empty, and so does a
    // HEAD the listing has never heard of.
    model.head_name = "main".to_string();
    assert!(model.settle_head_marks());
    assert_eq!(model.head_upstream_gone, "");
    model.head_name = "made-since".to_string();
    assert!(!model.settle_head_marks());
    assert_eq!(model.head_upstream_gone, "");
}
