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
    // Through a getter: a value QML wrote would not reach Rust (encode::tests::wire_qml).
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

    /// The tab in front is about to stop being it.
    ///
    /// Emitted *before* the front moves: the page being left hands over
    /// its layout and its commit editor's words, and once its row's flag
    /// has fallen the bindings have already taken that page down. Not
    /// emitted when the front tab stays. No row is named: it is always the
    /// tab in front, whose page the stack already holds.
    #[qsignal]
    pub(super) fn leaving_tab(&mut self);

    /// The tab in front is about to stand in another worktree, its page
    /// staying (`TabsModel::switch_worktree`).
    ///
    /// Emitted *before* the hub moves: the commit editor's words are filed
    /// under the worktree they were written in. Whatever else the page holds
    /// for the worktree being left goes here too; the graph and panes are the
    /// repository's and stay.
    #[qsignal]
    pub(super) fn leaving_worktree(&mut self);

    /// The other side of `leaving_worktree`: the tab in front now stands in
    /// the asked-for worktree, and its session is opening.
    #[qsignal]
    pub(super) fn stood_worktree(&mut self);

    /// Somebody asked to be shown the repository now in front, so the band
    /// travels to that tab's seat (`TabStrip.askFrontTab`), as far and as
    /// fast as a press on the stand-in. Three roads ask: a repository
    /// opened (picker, startup arguments) — its tab lands at the end, where
    /// the reader never scrolled, so a band that stayed put would bear
    /// every new tab out of view; an already open one asked for, or a tab
    /// stood in another worktree — the move is the answer, and an answer
    /// out of view is none; and the tabs a restart puts back.
    ///
    /// Not `current_index_changed`: a press or a carry moves that too, and
    /// there the reader has put the band where it is. Emitted after the
    /// front has moved, so the strip reads a model that has answered;
    /// `tab_id` is the tab asked for, now in front (`TabsModel::ask_front`).
    #[qsignal]
    pub(super) fn front_tab_asked(&mut self, tab_id: i32);

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
    /// worktree holding a branch, `_PGG_AUTO_OPEN`); a refusal shows on the
    /// tab's failure screen (`Ask::picked`).
    #[qslot]
    fn open_repository_path(&mut self, path: String) {
        self.ask(path, false);
    }

    /// Puts back the tabs the last session had open, in the order the
    /// reader left them in (`TabsModel::report` writes the strip's order).
    ///
    /// Tabs are reserved, not opened: a `git` startup per tab would spend
    /// the startup budget on repositories nobody has looked at. Each opens
    /// when first selected.
    ///
    /// A path no longer a directory is dropped (and logged) — out of the
    /// reader's hands; one still there but no longer a repository keeps
    /// its tab and shows its failure — worth showing.
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
            // own (`TabsModel::stand_home`); only a gone repository drops
            // the tab.
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
        let at = wanted_held.unwrap_or(wanted).min(self.items.len() - 1);
        self.current_tab = self.items.get(at).map(|tab| tab.tab_id);
        self.report();
        self.ask_front();
    }

    /// A tab's worktree would not open, so it is stood back in the
    /// repository's own (`TabsModel::stand_home`). The page asks before
    /// showing the refusal, so no failure screen flashes
    /// (`RepoTab::stand_home_asked`).
    #[qslot]
    fn stand_tab_home(&mut self, tab_id: i32) {
        self.stand_home(tab_id);
    }

    /// The worktree the tab holding `tab_id` stood in was asked to take a new
    /// folder: the tab stands in `path`, the new folder once git moved it or
    /// the old one after a refusal (`TabsModel::follow_tab_worktree`).
    #[qslot]
    fn follow_worktree(&mut self, tab_id: i32, path: String) {
        self.follow_tab_worktree(tab_id, path);
    }

    /// Closes the tab holding `tab_id` (`TabsModel::close`).
    #[qslot]
    fn close_tab(&mut self, tab_id: i32) {
        self.close(tab_id);
    }

    /// Takes the tab at `from` out of the strip and puts it down at `to`
    /// (`TabsModel::carry`).
    #[qslot]
    fn move_tab(&mut self, from: i32, to: i32) {
        let (Ok(from), Ok(to)) = (usize::try_from(from), usize::try_from(to)) else {
            return;
        };
        self.carry(from, to);
    }

    #[qslot]
    pub(super) fn set_current_index(&mut self, index: i32) {
        if index != self.current_index && index >= -1 && index < self.items.len() as i32 {
            self.leave_front();
            self.current_tab = usize::try_from(index)
                .ok()
                .and_then(|at| self.items.get(at))
                .map(|tab| tab.tab_id);
            self.report();
        }
    }
}
