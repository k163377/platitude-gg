use std::collections::HashMap;
use std::sync::Arc;

use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, qobject};

use crate::encode::{FIELD_SEP, RECORD_SEP};
use crate::hub::{Feed, Hub, PickMsg};
use crate::urlpath::file_url_to_path;

use super::{impl_move_notified, impl_notify_runs, push_run, qml_register, tab_name};

mod qobject;
mod strip;

pub(super) use strip::index_after_move;
// The siblings reach this through `use super::*`, the way `graph` hands
// its row item down: the name a tab lands with belongs to the strip.
use strip::title_of;

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
    /// key off the tab: a row closed to the left of the front one, or a
    /// tab carried across the strip, renumbers rows under a
    /// `currentIndex` that has not moved yet. Read off the row, the two
    /// disagree for as long as it takes both to settle, and the page in
    /// front is destroyed and rebuilt for nothing — with its session
    /// left open behind it, so the rebuilt page has an already opened
    /// repository that will not read itself again (measured: the graph
    /// stayed empty and `middle-close` waited out its watchdog).
    current_tab_id: i32,
    /// The strip as a list: one record per tab, the name it is shown by
    /// and then its work tree path, packed the way every other list QML
    /// unpacks itself is.
    ///
    /// For the readers that want the whole strip at once — the settings
    /// screen's repository chooser, which offers exactly the
    /// repositories standing in the strip and calls each of them what
    /// the tab does. A view can walk the rows; a list bound to a
    /// property cannot, and a name a reader picked has to lead back to
    /// a path.
    open_repos: String,
    /// Answers about folders the picker handed over. Attached on the
    /// first question: a window that never opens the picker never has
    /// one to hear.
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
            open_repos: String::new(),
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
impl_notify_runs!(TabsModel);

qml_register!(TabsModel, "TabsModel", singleton = false);
