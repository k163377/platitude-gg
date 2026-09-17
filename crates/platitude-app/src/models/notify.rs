//! Telling a Qt list model what changed underneath it.
//!
//! qtbridge has no batch insert, no ranged `dataChanged` and no move, so
//! all three are spelled out through the public proxy API — the same
//! pattern `QListModelBase::push` uses, and the reason these are the only
//! places in `models` that say `unsafe`
//! (see .claude/rules/app-ui.md "Qt Bridges の要点(罠)").
//!
//! Everything here runs on the Qt main thread inside one slot: the rows
//! are already written when the notification goes out, and nothing of
//! Qt's runs in between.

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
                // SAFETY: same pattern as QListModelBase::push — the proxy
                // pointer stays valid while the QObject side is attached,
                // and we are on the Qt main thread inside a slot.
                unsafe { &mut *proxy }.base_begin_insert_rows(
                    &mut *self,
                    &qtbridge::qtbridge_type_lib::QModelIndex::default(),
                    first,
                    last,
                );
                self.$field.extend(batch);
                // SAFETY: see above.
                unsafe { &mut *proxy }.base_end_insert_rows(&mut *self);
            }
        }
    };
}
pub(crate) use impl_extend_notified;

/// One row changing places, with one begin/endMoveRows pair around it.
///
/// A move leaves the item standing and only tells the view where it
/// went. A remove followed by an insert takes the view's delegate down
/// and builds another one, and the row being moved is the one under the
/// hand (`TabStrip`).
///
/// `to` is where the row ends up once it has left `from` — the ordinary
/// reading of a move, and the one the caller can check against its own
/// list afterwards. Qt asks for the row the item is inserted
/// **before** while the list still holds it in both places, which is one
/// further along when the row travels right.
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
                // A row Qt cannot be handed an index for is a list past
                // two billion rows, which no strip of tabs reaches.
                let (Ok(first), Ok(before)) = (i32::try_from(from), i32::try_from(before)) else {
                    return;
                };
                // SAFETY: same pattern as `extend_notified` — the proxy
                // pointer stays valid while the QObject side is attached,
                // and we are on the Qt main thread inside a slot.
                unsafe { &mut *proxy }
                    .base_begin_move_rows(&mut *self, &root, first, first, &root, before);
                let item = self.$field.remove(from);
                self.$field.insert(to, item);
                // SAFETY: see above.
                unsafe { &mut *proxy }.base_end_move_rows(&mut *self);
            }
        }
    };
}
pub(crate) use impl_move_notified;

/// One ranged `dataChanged` per run of rows that were rewritten in place.
///
/// Nothing here changes the list's length, so the view keeps its place and
/// QML's own bindings on those rows re-read themselves. Runs are inclusive
/// `(first, last)` row indices — [`push_run`] is what builds them — and a
/// model whose QObject side is not attached has nobody to tell.
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
                    // A row Qt cannot be handed an index for is a list
                    // past two billion rows, which is past the memory
                    // budget by orders of magnitude.
                    let (Ok(first), Ok(last)) = (i32::try_from(first), i32::try_from(last)) else {
                        continue;
                    };
                    // SAFETY: same pattern as `extend_notified` — the
                    // proxy pointer stays valid while the QObject side is
                    // attached, and we are on the Qt main thread inside a
                    // slot. `base_index` only builds an index.
                    let proxy = unsafe { &mut *proxy };
                    let top_left = proxy.base_index(&*self, first, 0, &root);
                    let bottom_right = proxy.base_index(&*self, last, 0, &root);
                    proxy.base_data_changed(&mut *self, &top_left, &bottom_right);
                }
            }
        }
    };
}
pub(crate) use impl_notify_runs;

/// Adds row `i` to the run being built if it carries on from the last one,
/// and opens a new run otherwise.
///
/// Callers walk their rows in ascending order, so a run is one unbroken
/// stretch of changed rows and costs one `dataChanged`: a list where every
/// row moved is one notification, and one where every other row moved is
/// one per island.
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
