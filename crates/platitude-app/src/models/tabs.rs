use std::collections::HashMap;

use qtbridge::{QListModel, QListModelBase, QModelItem, qobject};

use crate::hub::Hub;
use crate::urlpath::file_url_to_path;

use super::qml_register;

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
}

impl Default for TabsModel {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            // No tab selected. Deriving this (0) points at a tab that does
            // not exist, and the UI reads "no repository open" as < 0.
            current_index: -1,
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

#[qobject(Base = QListModel, ConvertToCamelCase, NoQmlElement)]
impl TabsModel {
    qproperty!(
        "currentIndex",
        Member = current_index,
        Notify = current_index_changed
    );

    #[qsignal]
    fn current_index_changed(&mut self);

    /// Opens the folder picked in a FolderDialog (a `file://` URL).
    #[qslot]
    fn open_repository_url(&mut self, url: String) {
        self.open_repository_path(file_url_to_path(&url).to_string_lossy().into_owned());
    }

    /// Opens a plain filesystem path.
    #[qslot]
    fn open_repository_path(&mut self, path: String) {
        let path_buf = std::path::PathBuf::from(path.trim());
        if path_buf.as_os_str().is_empty() {
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
        self.current_index = self.items.len() as i32 - 1;
        self.current_index_changed();
        self.report();
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
        self.current_index = wanted.min(self.items.len() - 1) as i32;
        self.current_index_changed();
        self.report();
    }

    #[qslot]
    fn close_tab(&mut self, tab_id: i32) {
        Hub::with(|hub| hub.close_tab(tab_id));
        if let Some(pos) = self.items.iter().position(|t| t.tab_id == tab_id) {
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
            self.current_index_changed();
            self.report();
        }
    }

    #[qslot]
    fn set_current_index(&mut self, index: i32) {
        if index != self.current_index && index >= -1 && index < self.items.len() as i32 {
            self.current_index = index;
            self.current_index_changed();
            self.report();
        }
    }
}

impl TabsModel {
    /// Hands the hub the tab strip as it stands. Opening, closing and
    /// switching are single acts rather than something that moves under a
    /// dragging hand, so they report as they happen; the file itself is
    /// still only written by the flush.
    fn report(&self) {
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
qml_register!(TabsModel, "TabsModel", singleton = false);
