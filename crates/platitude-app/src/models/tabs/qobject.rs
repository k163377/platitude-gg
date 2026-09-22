//! Everything QML sees of the tab strip: the properties it binds to, the
//! acts it calls for, and the news it is told back.
//!
//! One `#[qobject]` block, and it cannot be split further — QMetaInfo is
//! built per file (app-ui.md).

use super::*;

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

    // Every tab, one record each — see the member. Its own signal: this changes
    // on a name settling and on a carry across the strip, neither of
    // which moves the tab in front.
    qproperty!(
        "openRepos",
        Member = open_repos,
        Notify = open_repos_changed
    );

    // A folder somebody asked for is still being placed: git has been
    // asked where it opens and has not answered yet (`TabsModel::asking`).
    //
    // **What "the strip is what was asked for" is read from.** Tabs
    // arrive over the frames after they are asked for now, so anything
    // reading the whole strip — a headless run measuring it, a picture
    // of it — waits for this to go false, or reads a strip that is
    // still filling (measured: a run photographed one tab of sixteen).
    qproperty!("opening", Member = opening, Notify = opening_changed);

    #[qsignal]
    pub(super) fn current_index_changed(&mut self);

    #[qsignal]
    pub(super) fn open_repos_changed(&mut self);

    #[qsignal]
    pub(super) fn opening_changed(&mut self);

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
    pub(super) fn leaving_tab(&mut self, index: i32);

    /// The row at `index` is about to stand in another working copy of
    /// the repository it is showing, and its page is staying
    /// (`TabsModel::switch_copy`).
    ///
    /// Emitted *before* the hub is pointed at that copy, for the reason
    /// [`TabsModel::leaving_tab`] is emitted before the index moves: the
    /// words in the commit editor are filed under the copy they were
    /// written in, and after the hub has moved they would be filed under
    /// the copy they are not about. Everything the page has that belongs
    /// to the copy being left goes here too — the graph and the panes
    /// around it are the repository's and stay.
    ///
    /// [`TabsModel::leaving_tab`]: TabsModel::leaving_tab
    #[qsignal]
    pub(super) fn leaving_copy(&mut self, index: i32);

    /// …and the other side of it: the tab at `index` is standing in the
    /// copy that was asked for, and the session reading it is opening.
    #[qsignal]
    pub(super) fn stood_copy(&mut self, index: i32);

    /// Somebody asked to be shown the repository now in front, so the band
    /// travels to that tab's seat (デザイン規約 §タブの所作).
    ///
    /// Its own signal: `current_index_changed` is moved by the strip's
    /// own gestures too — a press, a carry putting the row somewhere
    /// else — and those are the reader putting the band where it
    /// stands, which this leaves alone. Emitted after the index has
    /// moved, so the strip reads a model that has already
    /// answered.
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

    /// Opens a plain filesystem path — a worktree row, the pill naming
    /// the copy holding a branch, `PGG_AUTO_OPEN`. A refusal here is
    /// shown on the tab's own failure screen, because nobody is standing
    /// in a picker to be sent back to
    /// (デザイン規約 §可否・警告の出し場所).
    #[qslot]
    fn open_repository_path(&mut self, path: String) {
        self.ask(path, false);
    }

    #[qslot]
    fn drain(&mut self) {
        for msg in self.asks.drain() {
            // One folder is asked about at a time, so the answer is the
            // one at the front (`TabsModel::asking`).
            let Some(ask) = self.asking.pop_front() else {
                continue;
            };
            self.land(&ask, msg);
        }
        self.start_asking();
        // After the next one is out, so a queue that still has folders
        // in it never reads as settled between two of them.
        self.settle_opening();
    }

    /// Puts back the tabs the last session had open.
    ///
    /// Only the active one gets a session here: restoring a window of tabs
    /// would otherwise spend one `git` startup per tab against the three
    /// second budget, for repositories nobody has looked at yet. The rest
    /// open when they are first selected.
    ///
    /// A path that is no longer a directory is dropped — it is not
    /// something the reader did. One that is still there but is no longer
    /// a repository keeps its tab and reports itself the usual way,
    /// because that one is worth seeing.
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
        for (position, saved_tab) in saved.tabs.iter().enumerate() {
            // Spelled for the screen on the way in, the same as the road
            // the picker takes (`TabsModel::ask`): the file is written
            // with `/` but nothing stops a hand from writing one that is
            // not, and the hover reads out whatever the tab kept.
            let copy = crate::urlpath::shown_path(&saved_tab.path);
            let repo = crate::urlpath::shown_path(&saved_tab.repo);
            // A linked copy that has gone stands the tab back in the
            // repository's own, the same fall this tab would take on
            // being looked at (デザイン規約 §タブの所作
            // 「立てない所へは立たない」). The tab is for the
            // repository, so only a repository that has gone too is
            // one the reader is not left holding.
            let copy = if std::path::Path::new(&copy).is_dir() {
                copy
            } else {
                repo.clone()
            };
            let copy_buf = std::path::PathBuf::from(&copy);
            if !copy_buf.is_dir() {
                tracing::info!(path = %copy, "restored tab dropped: not there any more");
                // Everything after it shifts left, and the active one with
                // it if it was to the right.
                if position < saved.active {
                    wanted = wanted.saturating_sub(1);
                }
                continue;
            }
            // A file can name one repository twice — written before the
            // strip refused duplicates, or spelling one folder two ways,
            // or naming two working copies of one repository. Putting
            // both back would restore the very thing opening now
            // declines to make, and git is not asked here: what the file
            // says is what a run wrote when it did ask
            // (`settings::TabRecord`).
            if let Landing::Show(held) | Landing::Switch(held) =
                landing_for(&self.items, &copy, &repo)
            {
                tracing::info!(path = %copy, "restored tab dropped: already open");
                if position == saved.active {
                    wanted_held = Some(held);
                } else if position < saved.active {
                    wanted = wanted.saturating_sub(1);
                }
                continue;
            }
            let title = title_of(&repo);
            let Some(Some(tab_id)) =
                Hub::with(|hub| hub.reserve_tab(copy_buf, std::path::PathBuf::from(&repo)))
            else {
                continue;
            };
            self.push(TabItem::standing(tab_id, title, repo, copy));
        }
        // Once, with the whole strip standing: a name settled against
        // half of it would be settled against tabs that are still to
        // arrive.
        self.settle_titles();
        if self.items.is_empty() {
            return;
        }
        self.current_index = wanted_held.unwrap_or(wanted).min(self.items.len() - 1) as i32;
        self.report();
        self.current_index_changed();
        self.front_tab_asked();
    }

    /// A tab's copy would not open, so it is stood back in the
    /// repository's own one (`TabsModel::stand_home`).
    ///
    /// Asked by the page that could not open — it is the one that
    /// hears git refuse, and it asks **before** it says anything about
    /// the refusal, so the reader is shown one screen and not a failure
    /// that is taken away again (`RepoTab::stand_home_asked`).
    #[qslot]
    fn stand_tab_home(&mut self, tab_id: i32) {
        self.stand_home(tab_id);
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
            // The namesake that made a tab spell out its parent may be
            // the one that just went, and the name goes back with it.
            self.settle_titles();
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
    pub(super) fn set_current_index(&mut self, index: i32) {
        if index != self.current_index && index >= -1 && index < self.items.len() as i32 {
            self.leave_front();
            self.current_index = index;
            self.report();
            self.current_index_changed();
        }
    }
}
