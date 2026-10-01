//! Telling a Qt list model what changed underneath it.
//!
//! qtbridge has no batch insert, no ranged `dataChanged` and no move, so
//! all three are spelled out through the proxy API the way
//! `QListModelBase::push` does it — hence the `unsafe`.
//!
//! The proxy is only ever reached through a shared reference: while a
//! notification runs, Qt re-enters it through its own pointer (`rowCount`,
//! `data`) and reads the model through the exclusive borrow handed to the
//! call (`&mut *self`). An exclusive reference to the proxy across the call
//! is what let the compiler drop that borrow before Qt could read it.
//!
//! Everything here runs on the Qt main thread inside one slot, so nothing
//! of Qt's runs between writing the rows and notifying.

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

/// One ranged `dataChanged` per inclusive `(first, last)` run of rows
/// rewritten in place (built by [`push_run`]). The length does not change,
/// so the view keeps its place. A model whose QObject side is not attached
/// has nobody to tell.
macro_rules! impl_notify_runs {
    ($ty:ty) => {
        impl $ty {
            #[expect(unsafe_code)]
            fn notify_runs(&mut self, runs: impl IntoIterator<Item = (usize, usize)>) {
                let Some(proxy) = self.try_get_rust_proxy_ptr() else {
                    return;
                };
                let root = qtbridge::qtbridge_type_lib::QModelIndex::default();
                for (first, last) in runs {
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

/// Extends the last run with row `i` if it follows on, else opens a new
/// one. Callers walk their rows in ascending order.
pub(crate) fn push_run(runs: &mut Vec<(usize, usize)>, i: usize) {
    match runs.last_mut() {
        Some((_, last)) if *last + 1 == i => *last = i,
        _ => runs.push((i, i)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runs_of(rows: impl IntoIterator<Item = usize>) -> Vec<(usize, usize)> {
        let mut runs = Vec::new();
        for i in rows {
            push_run(&mut runs, i);
        }
        runs
    }

    #[test]
    fn neighbours_join_one_run_and_a_gap_opens_another() {
        assert_eq!(runs_of([0, 1, 2, 5, 6, 9]), vec![(0, 2), (5, 6), (9, 9)]);
    }

    #[test]
    fn one_row_is_a_run_of_itself_wherever_it_sits() {
        assert_eq!(runs_of([0]), vec![(0, 0)]);
        assert_eq!(runs_of([7]), vec![(7, 7)]);
        assert!(runs_of([]).is_empty());
    }

    #[test]
    fn every_row_moving_costs_one_notification() {
        assert_eq!(runs_of(0..1000), vec![(0, 999)]);
    }

    #[test]
    fn a_row_offered_twice_opens_a_run_rather_than_running_backwards() {
        // Callers walk upwards, so this does not arise; pinned because the
        // alternative reading of the guard would emit (5, 4).
        let mut runs = vec![(3, 5)];
        push_run(&mut runs, 5);
        assert_eq!(runs, vec![(3, 5), (5, 5)]);
    }
}
