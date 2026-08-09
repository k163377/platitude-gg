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
