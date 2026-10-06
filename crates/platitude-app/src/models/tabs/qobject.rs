//! Everything QML sees of the tab strip — one `#[qobject]` block, which
//! cannot be split (structure.md §分割).

use super::*;

#[qobject(Base = QListModel, ConvertToCamelCase, NoQmlElement)]
impl TabsModel {
    qproperty!(
        "currentIndex",
        Member = current_index,
        Notify = current_index_changed
    );
    // Shares the row's signal: both are settled together
    // (`settle_current`).
    qproperty!(
        "currentTabId",
        Member = current_tab_id,
        Notify = current_index_changed
    );
    // Their own signal: standing the front tab in another worktree changes
    // them without moving the front.
    qproperty!(
        "currentRepoName",
        Member = current_repo_name,
        Notify = front_names_changed
    );
    qproperty!(
        "currentWorktreeName",
        Member = current_worktree_name,
        Notify = front_names_changed
    );

    // Its own signal: a name settling or a carry changes it without moving
    // the front.
    // Through a getter: a value QML wrote would not reach Rust (encode::wire_qml).
    qproperty!("openRepos", Read = open_repos, Notify = open_repos_changed);
    fn open_repos(&self) -> &OpenRepos {
        &self.open_repos
    }

    // True while an asked-for folder waits on git (`TabsModel::asking`).
    // Tabs arrive over later frames, so whatever reads the whole strip (a
    // headless run, a picture) waits for this to go false.
    qproperty!("opening", Member = opening, Notify = opening_changed);

    #[qsignal]
    pub(super) fn current_index_changed(&mut self);

    #[qsignal]
    pub(super) fn open_repos_changed(&mut self);

    #[qsignal]
    pub(super) fn front_names_changed(&mut self);

    #[qsignal]
    pub(super) fn opening_changed(&mut self);

    /// The row at `index` is about to stop being the one in front.
    ///
    /// Emitted *before* `currentIndex` moves: the page being left hands
    /// over its layout and its commit editor's words, and once
    /// `currentIndex` has moved the bindings have already taken that page
    /// down. Not emitted when the front tab stays.
    #[qsignal]
    pub(super) fn leaving_tab(&mut self, index: i32);

    /// The row at `index` is about to stand in another worktree, its
    /// page staying (`TabsModel::switch_worktree`).
    ///
    /// Emitted *before* the hub moves: the commit editor's words are filed
    /// under the worktree they were written in. Whatever else the page holds
    /// for the worktree being left goes here too; the graph and panes are the
    /// repository's and stay.
    #[qsignal]
    pub(super) fn leaving_worktree(&mut self, index: i32);

    /// The other side of `leaving_worktree`: the tab at `index` now stands in
    /// the asked-for worktree, and its session is opening.
    #[qsignal]
    pub(super) fn stood_worktree(&mut self, index: i32);

    /// Somebody asked to be shown the repository now in front, so the band
    /// travels to that tab's seat (デザイン規約 §タブの所作).
    ///
    /// Not `current_index_changed`: a press or a carry moves that too, and
    /// there the reader has put the band where it is. Emitted after the
    /// index has moved, so the strip reads a model that has answered.
    #[qsignal]
    pub(super) fn front_tab_asked(&mut self);

    /// The folder picked in the dialog did not open. `kind` is `plain` /
    /// `bare` / `other`, `message` git's own words (`other` alone), and
    /// `near` the folder to bring the picker back up at.
    #[qsignal]
    pub(super) fn open_rejected(
        &mut self,
        path: String,
        kind: String,
        message: String,
        near: String,
    );

    /// Takes the folder picked in a FolderDialog (a `file://` URL).
    #[qslot]
    fn open_repository_url(&mut self, url: String) {
        self.open_picked_path(file_url_to_path(&url).to_string_lossy().into_owned());
    }

    /// Takes a picked folder as a plain path — the road whose refusals
    /// have a picker to go back to.
    #[qslot]
    fn open_picked_path(&mut self, path: String) {
        self.ask(path, true);
    }

    /// Opens a plain filesystem path (a worktree row, the pill naming the
    /// worktree holding a branch, `PGG_AUTO_OPEN`); a refusal shows on the
    /// tab's failure screen (`Ask::picked`).
    #[qslot]
    fn open_repository_path(&mut self, path: String) {
        self.ask(path, false);
    }

    #[qslot]
    fn drain(&mut self) {
        for msg in self.asks.drain() {
            // One ask is out at a time (`TabsModel::asking`).
            let Some(ask) = self.asking.pop_front() else {
                continue;
            };
            self.land(&ask, msg);
        }
        self.start_asking();
        // After the next ask is out, so a non-empty queue never reads as
        // settled between two.
        self.settle_opening();
    }

    /// Puts back the tabs the last session had open.
    ///
    /// Tabs are reserved, not opened: a `git` startup per tab would spend
    /// the startup budget on repositories nobody has looked at. Each opens
    /// when first selected.
    ///
    /// A path no longer a directory is dropped; one still there but no
    /// longer a repository keeps its tab and shows its failure.
    #[qslot]
    fn restore_tabs(&mut self) {
        let Some(saved) = Hub::with(|hub| hub.state().tabs.clone()) else {
            return;
        };
        let mut wanted = saved.active;
        // Set when the active one was already held: that tab's row, which
        // the shifting of `wanted` below does not touch.
        let mut wanted_held: Option<usize> = None;
        for (position, saved_tab) in saved.tabs.iter().enumerate() {
            // Spelled for the screen as `TabsModel::ask` does: a
            // hand-edited file may not use `/`, and the hover reads what
            // the tab keeps.
            let worktree = crate::urlpath::shown_path(&saved_tab.path);
            let repo = crate::urlpath::shown_path(&saved_tab.repo);
            // A linked worktree that has gone falls back to the repository's
            // own (デザイン規約 §タブの所作「立てない所へは立たない」);
            // only a gone repository drops the tab.
            let worktree = if std::path::Path::new(&worktree).is_dir() {
                worktree
            } else {
                repo.clone()
            };
            let worktree_buf = std::path::PathBuf::from(&worktree);
            if !worktree_buf.is_dir() {
                tracing::info!(path = %worktree, "restored tab dropped: not there any more");
                if position < saved.active {
                    wanted = wanted.saturating_sub(1);
                }
                continue;
            }
            // A file can name one repository twice (one folder spelled two
            // ways, or two worktrees of it); only the first is put back, as
            // opening would. git is not asked: the file holds what a run
            // wrote when it did ask (`settings::TabRecord`).
            if let Landing::Show(held) | Landing::Switch(held) =
                landing_for(&self.items, &worktree, &repo)
            {
                tracing::info!(path = %worktree, "restored tab dropped: already open");
                if position == saved.active {
                    wanted_held = Some(held);
                } else if position < saved.active {
                    wanted = wanted.saturating_sub(1);
                }
                continue;
            }
            let title = title_of(&repo);
            let Some(Some(tab_id)) =
                Hub::with(|hub| hub.reserve_tab(worktree_buf, std::path::PathBuf::from(&repo)))
            else {
                continue;
            };
            self.push(TabItem::standing(tab_id, title, repo, worktree));
        }
        // Once the whole strip stands: a name settled against half of it
        // misses namesakes still to arrive.
        self.settle_titles();
        if self.items.is_empty() {
            return;
        }
        self.current_index = wanted_held.unwrap_or(wanted).min(self.items.len() - 1) as i32;
        self.report();
        self.current_index_changed();
        self.front_tab_asked();
    }

    /// A tab's worktree would not open, so it is stood back in the
    /// repository's own (`TabsModel::stand_home`). The page asks before
    /// showing the refusal, so no failure screen flashes
    /// (`RepoTab::stand_home_asked`).
    #[qslot]
    fn stand_tab_home(&mut self, tab_id: i32) {
        self.stand_home(tab_id);
    }

    #[qslot]
    fn close_tab(&mut self, tab_id: i32) {
        let closing = self.items.iter().position(|t| t.tab_id == tab_id);
        // The front tab says its goodbyes (`leaving_tab`) before the hub
        // closes its session, while the page is still whole.
        if closing.is_some() && closing == usize::try_from(self.current_index).ok() {
            self.leave_front();
        }
        Hub::with(|hub| hub.close_tab(tab_id));
        if let Some(pos) = closing {
            self.remove(pos);
            // The closed tab may have been another's namesake.
            self.settle_titles();
            // A close to the left shifts the active row down; the index
            // follows, or the front silently becomes its right neighbour.
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

    /// Takes the tab at `from` out of the strip and puts it down at `to`
    /// (デザイン規約 §タブの所作). A drag asks one neighbour at a time;
    /// the headless run carries the whole distance in one call.
    #[qslot]
    fn move_tab(&mut self, from: i32, to: i32) {
        let (Ok(from), Ok(to)) = (usize::try_from(from), usize::try_from(to)) else {
            return;
        };
        if from == to || from >= self.items.len() || to >= self.items.len() {
            return;
        }
        self.move_notified(from, to);
        // The rows the tab was carried across moved too, and the front
        // follows its own row.
        let landed = index_after_move(self.current_index, from, to);
        let moved = landed != self.current_index;
        self.current_index = landed;
        self.report();
        if moved {
            self.current_index_changed();
        }
    }

    #[qslot]
    pub(super) fn set_current_index(&mut self, index: i32) {
        if index != self.current_index && index >= -1 && index < self.items.len() as i32 {
            self.leave_front();
            self.current_index = index;
            self.report();
            self.current_index_changed();
        }
    }
}
