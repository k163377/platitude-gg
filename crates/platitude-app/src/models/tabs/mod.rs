use std::collections::VecDeque;
use std::sync::Arc;

use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, QmlObject, qobject};

use crate::encode::{Fields, Listed, Record, field};
use crate::hub::{Feed, Hub, OpenMsg};
use crate::urlpath::file_url_to_path;

use super::{impl_move_notified, impl_notify_runs, push_run, qml_register, tab_name};

/// One tab of the strip as the settings list offers it: its shown name
/// and the path of the worktree git is run in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenRepo {
    pub name: String,
    pub path: String,
}

/// The strip, in its own order.
pub type OpenRepos = Listed<OpenRepo>;

impl Record for OpenRepo {
    fn to_map(&self) -> qtbridge::qtbridge_type_lib::QVariantMap {
        Fields::new()
            .put("name", &self.name)
            .put("path", &self.path)
            .done()
    }

    fn from_map(map: &qtbridge::qtbridge_type_lib::QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            name: field(map, "name")?,
            path: field(map, "path")?,
        })
    }
}

mod qobject;
mod strip;

pub(super) use strip::index_after_move;
// The siblings reach these through `use super::*`.
use strip::{Landing, landing_for, title_of};

// ---------------------------------------------------------------------------
// TabsModel: open repositories (the tab strip)
// ---------------------------------------------------------------------------

#[derive(QModelItem, Default, Clone)]
pub struct TabItem {
    tab_id: i32,
    title: String,
    /// The repository's own worktree: what the tab is named after and
    /// what says whether two folders are one repository
    /// (`repo::Place::repo`). Not what git is run in — that is `worktree_path`.
    repo_path: String,
    /// The worktree the tab stands in (`repo_path` or one of its
    /// linked worktrees): what the session opens, and what the hover shows
    /// (デザイン規約 §hover のツールチップ).
    worktree_path: String,
    /// That worktree's folder name, empty in the repository's own — which
    /// says whether the strip draws anything after the name
    /// ([`worktree_name_of`]; デザイン規約 §タブの所作).
    worktree_name: String,
}

impl TabItem {
    /// One row of the strip: the repository whose own worktree is
    /// `repo`, standing in `worktree`. The one door rows are made through, so
    /// no road that opens a tab can leave `worktree_name` out.
    fn standing(tab_id: i32, title: String, repo: String, worktree: String) -> Self {
        Self {
            tab_id,
            title,
            worktree_name: worktree_name_of(&repo, &worktree),
            repo_path: repo,
            worktree_path: worktree,
        }
    }

    /// Stands this row in another worktree of the same repository
    /// (`TabsModel::switch_worktree`). The repository, title and id stay: the
    /// page is built on the id (`Hub::restand_tab`).
    fn stand_in(&mut self, worktree: String) {
        self.worktree_name = worktree_name_of(&self.repo_path, &worktree);
        self.worktree_path = worktree;
    }
}

/// What the strip says after the name: the linked worktree's folder name,
/// empty for the repository's own worktree.
///
/// Compared by `repo::open_key`, not `==`: a tab put back from the file
/// carries whatever spellings were written down.
fn worktree_name_of(repo: &str, worktree: &str) -> String {
    let key = platitude_core::repo::open_key;
    if key(repo) == key(worktree) {
        return String::new();
    }
    crate::urlpath::path_leaf(worktree).to_string()
}

/// A folder somebody asked for, waiting on git to say where it opens
/// (`TabsModel::ask`).
struct Ask {
    /// The folder as handed over — what a refusal names, having no
    /// worktree to name instead.
    path: String,
    /// Asked from the folder picker, so a refusal goes back to it; every
    /// other road shows the refusal on the tab's failure screen
    /// (デザイン規約 §可否・警告の出し場所).
    picked: bool,
}

pub struct TabsModel {
    items: Vec<TabItem>,
    current_index: i32,
    /// Which *tab* is in front, as opposed to which row it sits in.
    ///
    /// The page in front is keyed off this (`RepoPageStack`): closing a
    /// row to its left or carrying a tab renumbers rows before
    /// `currentIndex` moves, and a page keyed off the row is rebuilt in
    /// that gap with its session already open, never to read again.
    current_tab_id: i32,
    /// The front tab's repository and worktree names (`TabItem::worktree_name`),
    /// for the operation panel. Settled with the row, so the panel does
    /// not wait on the page's own git answer.
    ///
    /// The repository by its own folder name, not the strip's title: that
    /// one grows a parent folder beside a namesake (`tab_name::names_for`),
    /// and the panel has none.
    current_repo_name: String,
    current_worktree_name: String,
    /// The strip as one list (`OpenRepo` per tab), for the settings
    /// screen's repository chooser: a list bound to a property cannot walk
    /// the model's rows, and a picked name has to lead back to a path.
    open_repos: OpenRepos,
    /// Answers about asked-for folders; attached on the first ask.
    asks: Arc<Feed<OpenMsg>>,
    attached: bool,
    /// Folders asked for and not yet answered, in ask order; git is asked
    /// about the front one only, because answers return in whatever order
    /// git finishes and tabs must land in the order asked.
    asking: VecDeque<Ask>,
    /// Whether anything in that queue is still waiting on git
    /// (`opening`).
    opening: bool,
}

impl Default for TabsModel {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            // No tab: a derived 0 would point at a tab that does not exist;
            // the UI reads < 0 as none open.
            current_index: -1,
            current_tab_id: -1,
            current_repo_name: String::new(),
            current_worktree_name: String::new(),
            open_repos: OpenRepos::default(),
            asks: Arc::new(Feed::default()),
            attached: false,
            asking: VecDeque::new(),
            opening: false,
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
    fn insert_unnotified(&mut self, index: usize, value: TabItem) {
        self.items.insert(index, value);
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
