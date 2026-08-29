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

    // Every tab, packed — see the member. Its own signal rather than the
    // row one: this changes on a name settling and on a carry across the
    // strip, neither of which moves the tab in front.
    qproperty!(
        "openRepos",
        Member = open_repos,
        Notify = open_repos_changed
    );

    #[qsignal]
    pub(super) fn current_index_changed(&mut self);

    #[qsignal]
    pub(super) fn open_repos_changed(&mut self);

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
        let title = title_of(&path);
        let Some(Some(tab_id)) = Hub::with(|hub| hub.open_tab(path_buf)) else {
            return;
        };
        self.push(TabItem {
            tab_id,
            title,
            repo_path: path,
        });
        self.settle_titles();
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
            let title = title_of(path);
            let Some(Some(tab_id)) = Hub::with(|hub| hub.reserve_tab(path_buf)) else {
                continue;
            };
            self.push(TabItem {
                tab_id,
                title,
                repo_path: path.clone(),
            });
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
    fn set_current_index(&mut self, index: i32) {
        if index != self.current_index && index >= -1 && index < self.items.len() as i32 {
            self.leave_front();
            self.current_index = index;
            self.report();
            self.current_index_changed();
        }
    }
}
