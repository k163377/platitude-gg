use std::collections::HashMap;
use std::sync::Arc;

use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, qobject};

use crate::hub::{Feed, Hub, PickMsg};
use crate::urlpath::file_url_to_path;

use super::{impl_move_notified, qml_register};

// ---------------------------------------------------------------------------
// TabsModel: open repositories (the tab strip)
// ---------------------------------------------------------------------------

#[derive(QModelItem, Default, Clone)]
pub struct TabItem {
    tab_id: i32,
    title: String,
    repo_path: String,
}

pub struct TabsModel {
    items: Vec<TabItem>,
    current_index: i32,
    /// Which *tab* is in front, as opposed to which row it sits in.
    ///
    /// The window builds a page for the tab in front and takes it down
    /// when that tab stops being in front (`Main.qml`), and that has to
    /// key off the tab rather than the row: a row closed to the left of
    /// the front one, or a tab carried across the strip, renumbers rows
    /// under a `currentIndex` that has not moved yet. Read off the row,
    /// the two disagree for as long as it takes both to settle, and the
    /// page in front is destroyed and rebuilt for nothing — with its
    /// session left open behind it, so the rebuilt page has an already
    /// opened repository that will not read itself again (measured: the
    /// graph stayed empty and `middle-close` waited out its watchdog).
    current_tab_id: i32,
    /// Answers about folders the picker handed over. Attached on the
    /// first question rather than at startup: a window that never opens
    /// the picker never has one to hear.
    picks: Arc<Feed<PickMsg>>,
    attached: bool,
}

impl Default for TabsModel {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            // No tab selected. Deriving this (0) points at a tab that does
            // not exist, and the UI reads "no repository open" as < 0.
            current_index: -1,
            current_tab_id: -1,
            picks: Arc::new(Feed::default()),
            attached: false,
        }
    }
}

impl QListModel for TabsModel {
    type Item = TabItem;

    fn len(&self) -> usize {
        self.items.len()
    }
    fn get(&self, index: usize) -> Option<&TabItem> {
        self.items.get(index)
    }
    fn remove_unnotified(&mut self, index: usize) -> TabItem {
        self.items.remove(index)
    }
    fn reset_unnotified(&mut self) {
        self.items.clear();
    }
    fn push_unnotified(&mut self, value: TabItem) {
        self.items.push(value);
    }
}

impl_move_notified!(TabsModel, items);

#[qobject(Base = QListModel, ConvertToCamelCase, NoQmlElement)]
impl TabsModel {
    qproperty!(
        "currentIndex",
        Member = current_index,
        Notify = current_index_changed
    );
    // Same signal as the row it is derived from: the two are settled
    // together (`settle_current`), so a reader that watches one has
    // already been told about the other.
    qproperty!(
        "currentTabId",
        Member = current_tab_id,
        Notify = current_index_changed
    );

    #[qsignal]
    fn current_index_changed(&mut self);

    /// The row at `index` is about to stop being the one in front.
    ///
    /// Emitted *before* `currentIndex` moves, which is the whole point of
    /// having it: the page being left has to hand over the layout the next
    /// one is laid out at and the words typed into its commit editor, and
    /// by the time `currentIndex` has moved every binding that watches it
    /// has already taken that page down. Nothing is emitted when the front
    /// tab does not change — a tab closed to the left of it renumbers the
    /// strip without the reader leaving anything.
    #[qsignal]
    fn leaving_tab(&mut self, index: i32);

    /// The folder picked in the dialog did not open. `kind` is `plain` /
    /// `bare` / `other`, `message` git's own words (`other` alone), and
    /// `near` the folder to bring the picker back up at.
    #[qsignal]
    fn open_rejected(&mut self, path: String, kind: String, message: String, near: String);

    /// Takes the folder picked in a FolderDialog (a `file://` URL).
    #[qslot]
    fn open_repository_url(&mut self, url: String) {
        self.open_picked_path(file_url_to_path(&url).to_string_lossy().into_owned());
    }

    /// Takes a picked folder as a plain path: check first, open second.
    ///
    /// The check costs one `git rev-parse` (実測 30–36ms on Windows,
    /// repository or not), which is why nothing is shown while it runs —
    /// and why the tab is not opened up front and closed again, which
    /// would flash a tab for the length of a frame or two.
    #[qslot]
    fn open_picked_path(&mut self, path: String) {
        let path_buf = std::path::PathBuf::from(path.trim());
        if path_buf.as_os_str().is_empty() {
            return;
        }
        // A repository already in the strip needs no asking: it opened
        // once. The strip moves to it, the way every other road into an
        // open one does (デザイン規約 §タブの所作).
        if let Some(position) = self.position_of(path.trim()) {
            self.set_current_index(position as i32);
            return;
        }
        if !self.attached {
            self.picks.attach(self.get_qml_method_invoker());
            self.attached = true;
        }
        let feed = Arc::clone(&self.picks);
        // With no runtime there is nothing to ask and nothing to wait
        // for, so the old road applies: open it and let the page say so.
        if Hub::with(|hub| hub.probe_repo(path_buf, feed)) != Some(true) {
            self.open_repository_path(path);
        }
    }

    #[qslot]
    fn drain(&mut self) {
        for msg in self.picks.drain() {
            match msg {
                PickMsg::Accepted { path } => {
                    self.open_repository_path(path.to_string_lossy().into_owned());
                }
                PickMsg::Rejected {
                    path,
                    near,
                    kind,
                    message,
                } => {
                    tracing::info!(path = %path.display(), kind, "picked folder refused");
                    self.open_rejected(
                        path.to_string_lossy().into_owned(),
                        kind.to_string(),
                        message,
                        near,
                    );
                }
            }
        }
    }

    /// Opens a plain filesystem path.
    ///
    /// A repository already in the strip is not opened a second time —
    /// the strip moves to the tab holding it (デザイン規約 §タブの所作).
    #[qslot]
    fn open_repository_path(&mut self, path: String) {
        // Trimmed once, here, so the string the tab keeps is the one it
        // was compared by.
        let path = path.trim().to_string();
        let path_buf = std::path::PathBuf::from(&path);
        if path_buf.as_os_str().is_empty() {
            return;
        }
        if let Some(position) = self.position_of(&path) {
            self.set_current_index(position as i32);
            return;
        }
        let title = title_of(&path_buf, &path);
        let Some(Some(tab_id)) = Hub::with(|hub| hub.open_tab(path_buf)) else {
            return;
        };
        self.push(TabItem {
            tab_id,
            title,
            repo_path: path,
        });
        self.leave_front();
        self.current_index = self.items.len() as i32 - 1;
        self.report();
        self.current_index_changed();
    }

    /// Puts back the tabs the last session had open.
    ///
    /// Only the active one gets a session here: restoring a window of tabs
    /// would otherwise spend one `git` startup per tab against the three
    /// second budget, for repositories nobody has looked at yet. The rest
    /// open when they are first selected.
    ///
    /// A path that is no longer a directory is dropped rather than shown
    /// as a broken tab — it is not something the reader did. One that is
    /// still there but is no longer a repository keeps its tab and reports
    /// itself the usual way, because that one is worth seeing.
    #[qslot]
    fn restore_tabs(&mut self) {
        let Some(saved) = Hub::with(|hub| hub.state().tabs.clone()) else {
            return;
        };
        let mut wanted = saved.active;
        // Where the active one landed when it turned out to be a second
        // copy: the tab already holding that repository, whatever the
        // shifting below does to the count.
        let mut wanted_held: Option<usize> = None;
        for (position, path) in saved.paths.iter().enumerate() {
            let path_buf = std::path::PathBuf::from(path);
            if !path_buf.is_dir() {
                tracing::info!(path = %path, "restored tab dropped: not there any more");
                // Everything after it shifts left, and the active one with
                // it if it was to the right.
                if position < saved.active {
                    wanted = wanted.saturating_sub(1);
                }
                continue;
            }
            // A file written before the strip refused duplicates can name
            // one repository twice, and the two entries need not be
            // spelled alike. Putting both back would restore the very
            // thing opening now declines to make.
            if let Some(held) = self.position_of(path) {
                tracing::info!(path = %path, "restored tab dropped: already open");
                if position == saved.active {
                    wanted_held = Some(held);
                } else if position < saved.active {
                    wanted = wanted.saturating_sub(1);
                }
                continue;
            }
            let title = title_of(&path_buf, path);
            let Some(Some(tab_id)) = Hub::with(|hub| hub.reserve_tab(path_buf)) else {
                continue;
            };
            self.push(TabItem {
                tab_id,
                title,
                repo_path: path.clone(),
            });
        }
        if self.items.is_empty() {
            return;
        }
        self.current_index = wanted_held.unwrap_or(wanted).min(self.items.len() - 1) as i32;
        self.report();
        self.current_index_changed();
    }

    #[qslot]
    fn close_tab(&mut self, tab_id: i32) {
        let closing = self.items.iter().position(|t| t.tab_id == tab_id);
        // The tab in front is being taken away along with its page, so it
        // says its goodbyes first — and before the hub closes the session,
        // so the page is still whole while it does (`leaving_tab`).
        if closing.is_some() && closing == usize::try_from(self.current_index).ok() {
            self.leave_front();
        }
        Hub::with(|hub| hub.close_tab(tab_id));
        if let Some(pos) = closing {
            self.remove(pos);
            // Closing a tab left of the active one shifts the active row
            // down; the index has to follow it, or the visible repository
            // silently becomes its right-hand neighbour.
            if (pos as i32) < self.current_index {
                self.current_index -= 1;
            }
            let len = self.items.len() as i32;
            if self.current_index >= len {
                self.current_index = len - 1;
            }
            self.report();
            self.current_index_changed();
        }
    }

    /// Takes the tab at `from` out of the strip and puts it down at `to`,
    /// where a hand carried it (デザイン規約 §タブの所作).
    ///
    /// The strip settles the order one neighbour at a time, so the two
    /// are next to each other every time a drag asks — but the whole
    /// distance is one call away, and that is the road the headless run
    /// carries a tab across the strip on.
    #[qslot]
    fn move_tab(&mut self, from: i32, to: i32) {
        let (Ok(from), Ok(to)) = (usize::try_from(from), usize::try_from(to)) else {
            return;
        };
        if from == to || from >= self.items.len() || to >= self.items.len() {
            return;
        }
        self.move_notified(from, to);
        // The tab being carried is the one in front (the press moves to
        // it before the drag begins), but the rows it was carried across
        // moved too, and each of them has to keep showing the repository
        // it was showing.
        let landed = index_after_move(self.current_index, from, to);
        let moved = landed != self.current_index;
        self.current_index = landed;
        self.report();
        if moved {
            self.current_index_changed();
        }
    }

    #[qslot]
    fn set_current_index(&mut self, index: i32) {
        if index != self.current_index && index >= -1 && index < self.items.len() as i32 {
            self.leave_front();
            self.current_index = index;
            self.report();
            self.current_index_changed();
        }
    }
}

impl TabsModel {
    /// Announces that the row in front is about to stop being it
    /// (`leaving_tab`). Silent with nothing in front, where there is no
    /// page to hand anything over.
    fn leave_front(&mut self) {
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
    fn position_of(&self, path: &str) -> Option<usize> {
        let key = platitude_core::repo::open_key(path);
        self.items
            .iter()
            .position(|t| platitude_core::repo::open_key(&t.repo_path) == key)
    }

    /// Names the tab the front row is holding (`current_tab_id`).
    ///
    /// Called from [`TabsModel::report`], which every act on the strip
    /// ends with — so this cannot be left out of one. The signal goes out
    /// only on a change of tab, which is what makes a row closed to the
    /// left, or a tab carried past another, silent here: the strip
    /// renumbered, and the same repository is still in front.
    fn settle_current(&mut self) {
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
    fn report(&mut self) {
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

/// The tab's label: the repository's own folder name.
fn title_of(path: &std::path::Path, whole: &str) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| whole.to_string())
}

/// Where the row at `current` ends up once the row at `from` has been
/// taken out and put down at `to`.
///
/// Everything between the two shifts by one, towards the place the moved
/// row left. `current` is a position rather than a tab: a strip with
/// nothing in front of it says -1, and no move gives it a tab.
fn index_after_move(current: i32, from: usize, to: usize) -> i32 {
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

qml_register!(TabsModel, "TabsModel", singleton = false);

#[cfg(test)]
mod tests {
    use super::index_after_move;

    #[test]
    fn the_tab_being_carried_lands_where_it_was_put_down() {
        assert_eq!(index_after_move(0, 0, 3), 3);
        assert_eq!(index_after_move(3, 3, 0), 0);
    }

    #[test]
    fn a_tab_carried_past_the_one_in_front_pushes_it_the_other_way() {
        // Carried rightwards from its left: everything it passed shifts
        // left to fill the gap.
        assert_eq!(index_after_move(1, 0, 3), 0);
        assert_eq!(index_after_move(3, 0, 3), 2);
        // And leftwards from its right: they shift right.
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
}
