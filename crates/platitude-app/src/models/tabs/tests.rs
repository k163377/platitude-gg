//! Tests of how the strip says which tab is in front, through the acts that
//! move it (`TabsModel::carry` / `close` / `set_current_index`): read off
//! the tab, so carrying and closing rows renumbers nothing it holds.

use super::{TabItem, TabsModel};

/// A strip of the tabs `ids`, in that order, with `front` in front.
fn strip(ids: &[i32], front: Option<i32>) -> TabsModel {
    let mut model = TabsModel::default();
    for &id in ids {
        let path = format!("C:/repo{id}");
        model.items.push(TabItem::standing(
            id,
            format!("repo{id}"),
            path.clone(),
            path,
        ));
    }
    model.current_tab = front;
    model.settle_current();
    model
}

/// The strip's tabs, in order.
fn ids(model: &TabsModel) -> Vec<i32> {
    model.items.iter().map(|tab| tab.tab_id).collect()
}

/// Each row's own answer to "am I in front?".
fn flags(model: &TabsModel) -> Vec<bool> {
    model.items.iter().map(|tab| tab.current).collect()
}

#[test]
fn only_the_row_holding_the_tab_in_front_says_it_is() {
    let model = strip(&[1, 2, 3], Some(2));
    assert_eq!(model.current_index, 1);
    assert_eq!(flags(&model), [false, true, false]);
    assert_eq!(model.current_repo_name, "repo2");
}

#[test]
fn a_strip_with_no_tab_in_front_has_no_row_saying_it_is() {
    let model = strip(&[1, 2], None);
    assert_eq!(model.current_index, -1);
    assert_eq!(flags(&model), [false, false]);
    assert_eq!(model.current_repo_name, "");
}

#[test]
fn the_tab_carried_stays_in_front_where_it_was_put_down() {
    let mut model = strip(&[1, 2, 3, 4], Some(2));
    model.carry(1, 3);
    assert_eq!(ids(&model), [1, 3, 4, 2]);
    assert_eq!(model.current_index, 3);
    assert_eq!(flags(&model), [false, false, false, true]);
}

#[test]
fn a_tab_carried_past_the_one_in_front_moves_its_row_and_not_the_front() {
    let mut model = strip(&[1, 2, 3, 4], Some(2));
    // Rightwards past it: its row shifts left.
    model.carry(0, 3);
    assert_eq!(ids(&model), [2, 3, 4, 1]);
    assert_eq!(model.current_index, 0);
    assert_eq!(flags(&model), [true, false, false, false]);
    // Leftwards past it: its row shifts right.
    model.carry(3, 0);
    assert_eq!(ids(&model), [1, 2, 3, 4]);
    assert_eq!(model.current_index, 1);
    assert_eq!(flags(&model), [false, true, false, false]);
}

#[test]
fn a_tab_closed_to_the_left_leaves_the_one_in_front_in_front() {
    let mut model = strip(&[1, 2, 3], Some(3));
    model.close(1);
    assert_eq!(model.current_tab, Some(3));
    assert_eq!(model.current_index, 1);
    assert_eq!(flags(&model), [false, true]);
}

#[test]
fn a_tab_closed_to_the_right_leaves_the_one_in_front_where_it_is() {
    let mut model = strip(&[1, 2, 3], Some(1));
    model.close(3);
    assert_eq!(model.current_tab, Some(1));
    assert_eq!(model.current_index, 0);
    assert_eq!(flags(&model), [true, false]);
}

#[test]
fn the_front_closed_hands_the_front_to_its_right_neighbour() {
    let mut model = strip(&[1, 2, 3], Some(2));
    model.close(2);
    assert_eq!(model.current_tab, Some(3));
    assert_eq!(model.current_index, 1);
    assert_eq!(flags(&model), [false, true]);
}

#[test]
fn the_last_tab_closed_in_front_hands_the_front_to_the_new_last() {
    let mut model = strip(&[1, 2, 3], Some(3));
    model.close(3);
    assert_eq!(model.current_tab, Some(2));
    assert_eq!(model.current_index, 1);
    assert_eq!(flags(&model), [false, true]);
}

#[test]
fn the_only_tab_closed_leaves_none_in_front() {
    let mut model = strip(&[1], Some(1));
    model.close(1);
    assert_eq!(model.current_tab, None);
    assert_eq!(model.current_index, -1);
    assert_eq!(model.current_repo_name, "");
}

#[test]
fn a_row_put_in_front_brings_its_tab_and_none_takes_it_away() {
    let mut model = strip(&[1, 2, 3], Some(1));
    model.set_current_index(2);
    assert_eq!(model.current_tab, Some(3));
    assert_eq!(flags(&model), [false, false, true]);
    model.set_current_index(-1);
    assert_eq!(model.current_tab, None);
    assert_eq!(model.current_index, -1);
    assert_eq!(flags(&model), [false, false, false]);
}
