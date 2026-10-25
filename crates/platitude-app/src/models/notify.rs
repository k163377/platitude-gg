//! Telling a Qt list model what changed underneath it.
//!
//! qtbridge has no batch insert, no ranged `dataChanged` and no move, so
//! all three are spelled out through the proxy API the way
//! `QListModelBase::push` does it — hence the `unsafe`.
//!
//! The proxy is only ever reached through a shared reference: while a
//! notification runs, Qt re-enters it through its own pointer (`rowCount`,
//! `data`) and reads the model through the exclusive borrow handed to the
//! call (`&mut *self`). Across the call, an exclusive reference to the
//! proxy lets an optimized build drop the hand-over of that borrow before
//! Qt reads it, and the first re-entrant read aborts.
//!
//! Everything here runs on the Qt main thread inside one slot, so nothing
//! of Qt's runs between writing the rows and notifying.

use std::ops::RangeInclusive;

/// Batch append with one begin/endInsertRows pair.
macro_rules! impl_extend_notified {
    ($ty:ty, $field:ident, $item:ty) => {
        impl $ty {
            #[expect(unsafe_code)]
            fn extend_notified(&mut self, batch: Vec<$item>) {
                if batch.is_empty() {
                    return;
                }
                let Some(proxy) = self.try_get_rust_proxy_ptr() else {
                    self.$field.extend(batch);
                    return;
                };
                let first = self.$field.len() as i32;
                let last = first + batch.len() as i32 - 1;
                // SAFETY: the registry's pointer for this value's attached
                // QObject, which outlives the slot we are in (main thread);
                // shared, as the module says.
                unsafe { &*proxy }.base_begin_insert_rows(
                    &mut *self,
                    &qtbridge::qtbridge_type_lib::QModelIndex::default(),
                    first,
                    last,
                );
                self.$field.extend(batch);
                // SAFETY: see above.
                unsafe { &*proxy }.base_end_insert_rows(&mut *self);
            }
        }
    };
}
pub(crate) use impl_extend_notified;

/// One row changing places, with one begin/endMoveRows pair around it —
/// not remove + insert, which rebuilds the delegate under the hand
/// (`TabStrip`).
///
/// `to` is the row's final index. Qt wants the row to insert *before*
/// while the list still holds it in both places: one further along when
/// moving right.
macro_rules! impl_move_notified {
    ($ty:ty, $field:ident) => {
        impl $ty {
            #[expect(unsafe_code)]
            fn move_notified(&mut self, from: usize, to: usize) {
                if from == to || from >= self.$field.len() || to >= self.$field.len() {
                    return;
                }
                let Some(proxy) = self.try_get_rust_proxy_ptr() else {
                    let item = self.$field.remove(from);
                    self.$field.insert(to, item);
                    return;
                };
                let root = qtbridge::qtbridge_type_lib::QModelIndex::default();
                let before = if to > from { to + 1 } else { to };
                // No list moved here nears i32::MAX rows.
                let (Ok(first), Ok(before)) = (i32::try_from(from), i32::try_from(before)) else {
                    return;
                };
                // SAFETY: as in `extend_notified`.
                unsafe { &*proxy }
                    .base_begin_move_rows(&mut *self, &root, first, first, &root, before);
                let item = self.$field.remove(from);
                self.$field.insert(to, item);
                // SAFETY: see above.
                unsafe { &*proxy }.base_end_move_rows(&mut *self);
            }
        }
    };
}
pub(crate) use impl_move_notified;

/// A list turned into `rows` as `edit` says ([`edit_between`]): the runs that
/// go and come told one at a time, the rows that stay rewritten from `rows`
/// and told as changed (their numbers and marks move with the rows around
/// them). A row that stays keeps its delegate — and the view its place
/// (`QQuickItemView::applyModelChanges`), where a reset sends it back to
/// the top.
macro_rules! impl_edit_notified {
    ($ty:ty, $field:ident, $item:ty) => {
        impl $ty {
            #[expect(unsafe_code)]
            fn edit_notified(&mut self, edit: &crate::models::notify::Edit, rows: Vec<$item>) {
                let mut rows: Vec<Option<$item>> = rows.into_iter().map(Some).collect();
                let root = qtbridge::qtbridge_type_lib::QModelIndex::default();
                // No list told here nears i32::MAX rows.
                let place = |at: usize| i32::try_from(at).unwrap_or(i32::MAX);
                let proxy = self.try_get_rust_proxy_ptr();
                for run in edit.gone.iter().rev() {
                    if let Some(proxy) = proxy {
                        // SAFETY: as in `extend_notified`.
                        unsafe { &*proxy }.base_begin_remove_rows(
                            &mut *self,
                            &root,
                            place(*run.start()),
                            place(*run.end()),
                        );
                    }
                    self.$field.drain(run.clone());
                    if let Some(proxy) = proxy {
                        // SAFETY: see above.
                        unsafe { &*proxy }.base_end_remove_rows(&mut *self);
                    }
                }
                for run in &edit.come {
                    if let Some(proxy) = proxy {
                        // SAFETY: see above.
                        unsafe { &*proxy }.base_begin_insert_rows(
                            &mut *self,
                            &root,
                            place(*run.start()),
                            place(*run.end()),
                        );
                    }
                    let fresh: Vec<$item> = run.clone().filter_map(|at| rows[at].take()).collect();
                    self.$field.splice(*run.start()..*run.start(), fresh);
                    if let Some(proxy) = proxy {
                        // SAFETY: see above.
                        unsafe { &*proxy }.base_end_insert_rows(&mut *self);
                    }
                }
                let mut changed = Vec::new();
                for &(_, now) in &edit.kept {
                    if let Some(row) = rows[now].take() {
                        self.$field[now] = row;
                        crate::models::notify::push_run(&mut changed, now);
                    }
                }
                self.notify_runs(changed);
            }
        }
    };
}
pub(crate) use impl_edit_notified;

/// One ranged `dataChanged` per run of rows rewritten in place (built by
/// [`push_run`]). The length does not change, so the view keeps its place.
/// A model whose QObject side is not attached has nobody to tell.
macro_rules! impl_notify_runs {
    ($ty:ty) => {
        impl $ty {
            #[expect(unsafe_code)]
            fn notify_runs(
                &mut self,
                runs: impl IntoIterator<Item = std::ops::RangeInclusive<usize>>,
            ) {
                let Some(proxy) = self.try_get_rust_proxy_ptr() else {
                    return;
                };
                let root = qtbridge::qtbridge_type_lib::QModelIndex::default();
                for (first, last) in runs.into_iter().map(std::ops::RangeInclusive::into_inner) {
                    // i32::MAX rows is far outside the memory budget.
                    let (Ok(first), Ok(last)) = (i32::try_from(first), i32::try_from(last)) else {
                        continue;
                    };
                    // SAFETY: as in `extend_notified`. `base_index` only
                    // builds an index.
                    let proxy = unsafe { &*proxy };
                    let top_left = proxy.base_index(&*self, first, 0, &root);
                    let bottom_right = proxy.base_index(&*self, last, 0, &root);
                    proxy.base_data_changed(&mut *self, &top_left, &bottom_right);
                }
            }
        }
    };
}
pub(crate) use impl_notify_runs;

/// Extends the last run with `row` if it follows on, else opens a new
/// one. Callers walk their rows in ascending order.
pub(crate) fn push_run(runs: &mut Vec<RangeInclusive<usize>>, row: usize) {
    match runs.last_mut() {
        Some(run) if *run.end() + 1 == row => *run = *run.start()..=row,
        _ => runs.push(row..=row),
    }
}

/// The runs `rows` make, in their order — what a test expects runs as
/// without spelling a list of one range.
#[cfg(test)]
pub(crate) fn runs_of(rows: impl IntoIterator<Item = usize>) -> Vec<RangeInclusive<usize>> {
    let mut runs = Vec::new();
    for row in rows {
        push_run(&mut runs, row);
    }
    runs
}

/// How one list of rows becomes another, in the terms Qt is told them: the
/// rows that go, as runs of their places before; the rows that come, as
/// runs of their places after; and the rows that stay, as `(before,
/// after)` pairs in order. A row that stays keeps its delegate — and the
/// view keeps whatever stands on it (rules-refs/app-ui.md 「外乱では閉じない
/// — 一覧」).
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Edit {
    pub(crate) gone: Vec<RangeInclusive<usize>>,
    pub(crate) come: Vec<RangeInclusive<usize>>,
    pub(crate) kept: Vec<(usize, usize)>,
}

/// The rows `after` keeps of `before`, both given as one key per row. Rows
/// are matched by key, the n-th of a key before with the n-th after; of the
/// matches, the longest run in the same order stays, and a row matched out
/// of that order goes and comes again (it changed places).
pub(crate) fn edit_between(before: &[u64], after: &[u64]) -> Edit {
    let mut places: std::collections::HashMap<u64, std::collections::VecDeque<usize>> =
        std::collections::HashMap::with_capacity(before.len());
    for (at, key) in before.iter().enumerate() {
        places.entry(*key).or_default().push_back(at);
    }
    let matched: Vec<(usize, usize)> = after
        .iter()
        .enumerate()
        .filter_map(|(now, key)| {
            places
                .get_mut(key)
                .and_then(std::collections::VecDeque::pop_front)
                .map(|was| (was, now))
        })
        .collect();
    let kept = in_order(&matched);
    let mut stays = vec![false; before.len()];
    let mut stood = vec![false; after.len()];
    for &(was, now) in &kept {
        stays[was] = true;
        stood[now] = true;
    }
    let mut edit = Edit {
        kept,
        ..Edit::default()
    };
    for (at, _) in stays.iter().enumerate().filter(|(_, stays)| !**stays) {
        push_run(&mut edit.gone, at);
    }
    for (at, _) in stood.iter().enumerate().filter(|(_, stood)| !**stood) {
        push_run(&mut edit.come, at);
    }
    edit
}

/// The longest run of `pairs` (in their order) whose first places rise —
/// patience sorting, so a list of tens of thousands of rows costs a
/// logarithm per row.
fn in_order(pairs: &[(usize, usize)]) -> Vec<(usize, usize)> {
    // `tails[k]`: the pair ending the best run of length k + 1 found so far;
    // `back[i]`: the pair before pair `i` in its run.
    let mut tails: Vec<usize> = Vec::new();
    let mut back: Vec<Option<usize>> = vec![None; pairs.len()];
    for (i, &(was, _)) in pairs.iter().enumerate() {
        let at = tails.partition_point(|&t| pairs[t].0 < was);
        back[i] = at.checked_sub(1).map(|k| tails[k]);
        if at == tails.len() {
            tails.push(i);
        } else {
            tails[at] = i;
        }
    }
    let mut run = Vec::with_capacity(tails.len());
    let mut at = tails.last().copied();
    while let Some(i) = at {
        run.push(pairs[i]);
        at = back[i];
    }
    run.reverse();
    run
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(before: &[u64], after: &[u64]) -> Edit {
        edit_between(before, after)
    }

    #[test]
    fn the_same_rows_stay_every_one() {
        let rows = [10, 11, 12];
        assert_eq!(
            edit(&rows, &rows),
            Edit {
                kept: vec![(0, 0), (1, 1), (2, 2)],
                ..Edit::default()
            }
        );
    }

    #[test]
    fn a_row_written_in_above_comes_and_the_rest_stay_one_place_down() {
        assert_eq!(
            edit(&[10, 11, 12], &[9, 10, 11, 12]),
            Edit {
                come: runs_of([0]),
                kept: vec![(0, 1), (1, 2), (2, 3)],
                ..Edit::default()
            }
        );
    }

    #[test]
    fn a_row_taken_out_goes_and_the_rows_below_move_up() {
        assert_eq!(
            edit(&[10, 11, 12, 13], &[10, 12, 13]),
            Edit {
                gone: runs_of([1]),
                kept: vec![(0, 0), (2, 1), (3, 2)],
                ..Edit::default()
            }
        );
    }

    #[test]
    fn runs_that_go_and_come_are_told_as_runs() {
        assert_eq!(
            edit(&[1, 2, 3, 4, 5, 6], &[1, 7, 8, 4, 9]),
            Edit {
                gone: vec![1..=2, 4..=5],
                come: vec![1..=2, 4..=4],
                kept: vec![(0, 0), (3, 3)],
            }
        );
    }

    #[test]
    fn a_row_that_changed_places_goes_and_comes_again() {
        // `30` moved to the top: keeping it would mean moving the two
        // others, which is more rows than it.
        assert_eq!(
            edit(&[10, 20, 30], &[30, 10, 20]),
            Edit {
                gone: runs_of([2]),
                come: runs_of([0]),
                kept: vec![(0, 1), (1, 2)],
            }
        );
    }

    #[test]
    fn equal_keys_match_in_their_order() {
        // Two rows with one key (a collision, or a row the list shows
        // twice) are matched first with first, second with second — and
        // of the two matches that cross (`6` and the second `5`), one
        // stays.
        assert_eq!(
            edit(&[5, 5, 6], &[5, 6, 5]),
            Edit {
                gone: runs_of([2]),
                come: runs_of([1]),
                kept: vec![(0, 0), (1, 2)],
            }
        );
    }

    #[test]
    fn from_nothing_every_row_comes_and_to_nothing_every_row_goes() {
        assert_eq!(
            edit(&[], &[1, 2]),
            Edit {
                come: runs_of([0, 1]),
                ..Edit::default()
            }
        );
        assert_eq!(
            edit(&[1, 2], &[]),
            Edit {
                gone: runs_of([0, 1]),
                ..Edit::default()
            }
        );
    }

    #[test]
    fn neighbours_join_one_run_and_a_gap_opens_another() {
        assert_eq!(runs_of([0, 1, 2, 5, 6, 9]), vec![0..=2, 5..=6, 9..=9]);
    }

    #[test]
    fn one_row_is_a_run_of_itself_wherever_it_sits() {
        assert_eq!(runs_of([0]), vec![0..=0]);
        assert_eq!(runs_of([7]), vec![7..=7]);
        assert!(runs_of([]).is_empty());
    }

    #[test]
    fn every_row_moving_costs_one_notification() {
        assert_eq!(runs_of(0..1000), vec![0..=999]);
    }

    #[test]
    #[expect(
        clippy::single_range_in_vec_init,
        reason = "the runs are a list of ranges, and a list of one run is a list of one"
    )]
    fn a_row_offered_twice_opens_a_run_rather_than_running_backwards() {
        // Callers walk upwards, so this does not arise; pinned because the
        // alternative reading of the guard would emit 5..=4.
        let mut runs = vec![3..=5];
        push_run(&mut runs, 5);
        assert_eq!(runs, vec![3..=5, 5..=5]);
    }
}
