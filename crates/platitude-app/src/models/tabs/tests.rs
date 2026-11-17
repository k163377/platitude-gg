//! Tests of the tab strip's move arithmetic (`tabs::index_after_move`).

use super::tabs::index_after_move;

#[test]
fn the_tab_being_carried_lands_where_it_was_put_down() {
    assert_eq!(index_after_move(0, 0, 3), 3);
    assert_eq!(index_after_move(3, 3, 0), 0);
}

#[test]
fn a_tab_carried_past_the_one_in_front_pushes_it_the_other_way() {
    // Carried rightwards: the rows it passed shift left.
    assert_eq!(index_after_move(1, 0, 3), 0);
    assert_eq!(index_after_move(3, 0, 3), 2);
    // Leftwards: they shift right.
    assert_eq!(index_after_move(1, 3, 0), 2);
    assert_eq!(index_after_move(0, 3, 0), 1);
}

#[test]
fn a_move_that_stayed_on_one_side_leaves_the_front_tab_where_it_is() {
    assert_eq!(index_after_move(5, 0, 3), 5);
    assert_eq!(index_after_move(0, 1, 3), 0);
    assert_eq!(index_after_move(5, 3, 1), 5);
}

#[test]
fn a_strip_with_no_tab_in_front_gains_none_from_a_move() {
    assert_eq!(index_after_move(-1, 0, 2), -1);
}
