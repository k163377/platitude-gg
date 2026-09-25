//! What the feeds settle onto the model's own properties.

use super::testkit::*;
use super::*;

/// The stand-in's pair (`HeadPinRow`) comes off the snapshot by name, so
/// it matches its row and outlives the row being filtered away — when
/// the stand-in is the one on screen.
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
    // A moved pair is news, or the stand-in keeps the last listing's.
    assert!(model.settle_head_marks());
    model.arrange();

    // One branch, one pair: the row reads it out of the same `BranchItem`.
    assert_eq!(model.head_row, 2);
    assert_eq!(numbers(&model, 2, Role::Ahead), 2);
    assert_eq!(numbers(&model, 2, Role::Behind), 1);
    assert_eq!(model.head_ahead, 2);
    assert_eq!(model.head_behind, 1);

    // A filter takes the row away; the stand-in keeps its pair.
    model.filter = "feature".to_string();
    model.arrange();
    assert_eq!(model.head_row, -1);
    assert_eq!(model.head_ahead, 2);
    assert_eq!(model.head_behind, 1);

    // Not in the listing: nothing to count.
    model.head_name = "made-since".to_string();
    assert!(model.settle_head_marks());
    assert_eq!(model.head_ahead, 0);
    assert_eq!(model.head_behind, 0);
}

/// The stand-in carries its row's unreachable upstream, by name: the far
/// side deleting the ref is what took `has_remote` away, so the badge's
/// state rides on the name (デザイン規約 §ref の種別).
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
    assert!(!model.head_has_remote);

    // Empty for an upstream that is here, and for a HEAD the listing lacks.
    model.head_name = "main".to_string();
    assert!(model.settle_head_marks());
    assert_eq!(model.head_upstream_gone, "");
    model.head_name = "made-since".to_string();
    assert!(!model.settle_head_marks());
    assert_eq!(model.head_upstream_gone, "");
}
