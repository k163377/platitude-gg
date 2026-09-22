use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, qobject};

use crate::encode::{Fields, Listed, Record, field};
use crate::hub::{Feed, Hub, OpenMsg};
use crate::urlpath::file_url_to_path;

use super::{impl_move_notified, impl_notify_runs, push_run, qml_register, tab_name};

/// One tab of the strip as the settings list offers it: the name it is
/// shown by, and the work tree path git is run in.
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
// The siblings reach this through `use super::*`, the way `graph` hands
// its row item down: the name a tab lands with belongs to the strip.
use strip::{Landing, landing_for, title_of};

// ---------------------------------------------------------------------------
// TabsModel: open repositories (the tab strip)
// ---------------------------------------------------------------------------

#[derive(QModelItem, Default, Clone)]
pub struct TabItem {
    tab_id: i32,
    title: String,
    /// The repository the tab shows: its own working copy, which is
    /// what the tab is named after and what says whether two folders
    /// are one repository (`repo::Place::repo`). **Not what git is run
    /// in** — that is `copy_path`, and the two differ for every tab
    /// standing in a linked copy.
    repo_path: String,
    /// The working copy the tab is standing in — `repo_path` where that
    /// is the repository's own, one of its linked copies otherwise.
    /// What the session opens, and what the strip moves to instead of
    /// opening a second time. **The hover reads this one**: a path put
    /// out under the hand is where the reader is standing
    /// (デザイン規約 §hover のツールチップ).
    copy_path: String,
    /// That copy by name, and empty where the tab stands in the
    /// repository's own — which is what says whether the strip draws
    /// anything after the name at all
    /// ([`copy_name_of`]; デザイン規約 §タブの所作).
    copy_name: String,
}

impl TabItem {
    /// One row of the strip: the tab showing the repository whose own
    /// working copy is `repo`, standing in `copy`.
    ///
    /// The one door, so the name the strip says the copy by cannot be
    /// left out of a road that opens a tab — there are four
    /// (opening, standing elsewhere, putting the last session back, and
    /// a folder that would not open standing for itself).
    fn standing(tab_id: i32, title: String, repo: String, copy: String) -> Self {
        Self {
            tab_id,
            title,
            copy_name: copy_name_of(&repo, &copy),
            repo_path: repo,
            copy_path: copy,
        }
    }

    /// Stands this row in another working copy of the repository it is
    /// already showing (`TabsModel::switch_copy`): the folder git is
    /// run in, and the name the strip draws after the tab's own.
    ///
    /// **The repository does not move**, so neither does what the tab is
    /// called — and the row keeps its id, because the page is built on
    /// that (`Hub::restand_tab`).
    fn stand_in(&mut self, copy: String) {
        self.copy_name = copy_name_of(&self.repo_path, &copy);
        self.copy_path = copy;
    }
}

/// What the strip says after the name: the folder git made the linked
/// copy in, and nothing at all for the repository's own copy.
///
/// Told apart by `repo::open_key`, the key the strip judges two folders
/// by everywhere else (`landing_for`): the two paths arrive from one
/// answer of git's when a tab is opened, but a tab put back from the
/// file carries the two spellings that answer was written down with.
fn copy_name_of(repo: &str, copy: &str) -> String {
    let key = platitude_core::repo::open_key;
    if key(repo) == key(copy) {
        return String::new();
    }
    crate::urlpath::path_leaf(copy).to_string()
}

/// A folder somebody asked for, waiting on git to say where it opens
/// (`TabsModel::ask`).
struct Ask {
    /// The folder as it was handed over — the path a refusal names,
    /// since a refused folder has no working copy to name instead.
    path: String,
    /// The reader's hand is on a folder picker, so a refusal has
    /// somewhere to go back to. Every other road shows a refusal on the
    /// tab's own failure screen (デザイン規約 §可否・警告の出し場所).
    picked: bool,
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
    /// The strip as a list: one record per tab (`OpenRepo`), the name it
    /// is shown by and its work tree path.
    ///
    /// For the readers that want the whole strip at once — the settings
    /// screen's repository chooser, which offers exactly the
    /// repositories standing in the strip and calls each of them what
    /// the tab does. A view can walk the rows; a list bound to a
    /// property cannot, and a name a reader picked has to lead back to
    /// a path.
    open_repos: OpenRepos,
    /// Answers about folders somebody asked for. Attached on the first
    /// question: a window that opens nothing never has one to hear.
    asks: Arc<Feed<OpenMsg>>,
    attached: bool,
    /// The folders asked for and not yet answered, in the order they
    /// were asked for, the one git is being asked about at the front.
    ///
    /// **One at a time, and the queue is why**: answers come back in
    /// whatever order git finishes, and the strip is an order —
    /// `PGG_AUTO_OPEN` naming three repositories has to put three tabs
    /// down in the order it named them.
    asking: VecDeque<Ask>,
    /// Whether anything in that queue is still waiting on git
    /// (`opening`).
    opening: bool,
}

impl Default for TabsModel {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            // No tab selected. Deriving this (0) points at a tab that does
            // not exist, and the UI reads "no repository open" as < 0.
            current_index: -1,
            current_tab_id: -1,
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
    /// The seat a tab standing in another working copy takes over
    /// (`TabsModel::switch_copy`) — the one act that puts a row back
    /// where one just left.
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
