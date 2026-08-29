//! The strip's own bookkeeping, off the Qt block: where a repository
//! already sits, what every tab is called against the ones standing
//! beside it, and what the hub is told once an act has landed.

use super::*;

impl TabsModel {
    /// Announces that the row in front is about to stop being it
    /// (`leaving_tab`). Silent with nothing in front, where there is no
    /// page to hand anything over.
    pub(super) fn leave_front(&mut self) {
        if self.current_index >= 0 {
            self.leaving_tab(self.current_index);
        }
    }

    /// Where the repository at `path` already sits in the strip, if it
    /// does. **The one place that answers this** — both ways a tab can
    /// appear (opening and restoring) ask here, so the two cannot come
    /// to different conclusions about the same folder.
    ///
    /// Compared by `repo::open_key` rather than by the string: the same
    /// folder arrives spelled differently depending on the way in, and
    /// the worktree row — the row naming the repository already open —
    /// is the one that arrives in git's spelling every time.
    ///
    /// Resolves every open tab's path, so the cost is one filesystem
    /// lookup per tab. That is bounded by the cap on the tab list and is
    /// paid only when someone asks for a repository.
    pub(super) fn position_of(&self, path: &str) -> Option<usize> {
        let key = platitude_core::repo::open_key(path);
        self.items
            .iter()
            .position(|t| platitude_core::repo::open_key(&t.repo_path) == key)
    }

    /// Names every tab against the strip it now stands in
    /// ([`tab_name::names_for`]).
    ///
    /// Called by opening, restoring and closing — the three acts that
    /// change which names are in the strip. **Not by a move**: the order
    /// is not what a name is settled against, and a `dataChanged` on the
    /// row being carried would be one the hand did not ask for.
    ///
    /// Rows that came out the same are left alone, so the ordinary case
    /// — a repository whose name nobody shares — costs no notification at
    /// all, and the strip re-measures only the tabs whose words moved
    /// (`TabStrip.settleTitleCap`).
    pub(super) fn settle_titles(&mut self) {
        let names = {
            let paths: Vec<&str> = self.items.iter().map(|t| t.repo_path.as_str()).collect();
            tab_name::names_for(&paths)
        };
        let mut runs = Vec::new();
        for (at, (item, name)) in self.items.iter_mut().zip(names).enumerate() {
            if item.title != name {
                item.title = name;
                push_run(&mut runs, at);
            }
        }
        self.notify_runs(runs);
    }

    /// Names the tab the front row is holding (`current_tab_id`).
    ///
    /// Called from [`TabsModel::report`], which every act on the strip
    /// ends with — so this cannot be left out of one. The signal goes out
    /// only on a change of tab, which is what makes a row closed to the
    /// left, or a tab carried past another, silent here: the strip
    /// renumbered, and the same repository is still in front.
    pub(super) fn settle_current(&mut self) {
        let id = usize::try_from(self.current_index)
            .ok()
            .and_then(|at| self.items.get(at))
            .map_or(-1, |tab| tab.tab_id);
        if id != self.current_tab_id {
            self.current_tab_id = id;
            self.current_index_changed();
        }
    }

    /// Hands the hub the tab strip as it stands. Opening, closing and
    /// switching are single acts rather than something that moves under a
    /// dragging hand, so they report as they happen; the file itself is
    /// still only written by the flush.
    ///
    /// **Called before the act's own `current_index_changed()`**, because
    /// this is also where the tab in front is named ([`settle_current`]) —
    /// and a notification that goes out with the row already moved and the
    /// tab not yet named is one where the two disagree: the page for the
    /// row arrived at has not been built, the page for the row left is
    /// still standing, and anything reading "the page in front" gets
    /// nothing.
    ///
    /// [`settle_current`]: TabsModel::settle_current
    pub(super) fn report(&mut self) {
        self.settle_current();
        // Named the way the file names it. The store normalises separators
        // on the way out anyway, so handing it the raw path would leave the
        // state held here unequal to the one on disk — harmless today only
        // because the flush compares against what it last wrote rather than
        // against the file.
        let paths = self
            .items
            .iter()
            .map(|t| platitude_core::settings::repo_key(&t.repo_path))
            .collect::<Vec<_>>();
        let active = usize::try_from(self.current_index).unwrap_or(0);
        Hub::with(|hub| hub.set_tabs_state(platitude_core::settings::TabsState { paths, active }));
    }
}

/// The tab's label before the strip has been consulted: the repository's
/// own folder name, which is what it is called wherever nobody shares it.
///
/// Asked of the same rule the whole strip is settled by
/// ([`tab_name::names_for`]), so a tab is never named twice over — the
/// row is pushed with this and [`TabsModel::settle_titles`] grows it if
/// the strip it landed in has a namesake standing in it.
pub(super) fn title_of(path: &str) -> String {
    tab_name::names_for(&[path])
        .into_iter()
        .next()
        .unwrap_or_default()
}

/// Where the row at `current` ends up once the row at `from` has been
/// taken out and put down at `to`.
///
/// Everything between the two shifts by one, towards the place the moved
/// row left. `current` is a position rather than a tab: a strip with
/// nothing in front of it says -1, and no move gives it a tab.
pub(crate) fn index_after_move(current: i32, from: usize, to: usize) -> i32 {
    let Ok(at) = usize::try_from(current) else {
        return current;
    };
    let landed = if at == from {
        to
    } else if from < at && at <= to {
        at - 1
    } else if to <= at && at < from {
        at + 1
    } else {
        at
    };
    i32::try_from(landed).unwrap_or(current)
}
