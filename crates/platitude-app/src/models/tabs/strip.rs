//! The strip's own bookkeeping, off the Qt block: where a repository
//! already sits, what every tab is called against the ones standing
//! beside it, and what the hub is told once an act has landed.

use super::*;

impl TabsModel {
    /// Announces that the row in front is about to stop being it
    /// (`leaving_tab`). Silent with nothing in front, where there is no
    /// page to hand anything over.
    pub(super) fn leave_front(&mut self) {
        if self.current_index >= 0 {
            self.leaving_tab(self.current_index);
        }
    }

    /// Where the working copy at `path` already stands in the strip, if
    /// it does.
    ///
    /// Compared by `repo::open_key`: the same folder arrives spelled
    /// differently depending on the way in, and the worktree row — the
    /// row naming a copy already open — is the one that arrives in
    /// git's spelling every time.
    ///
    /// Resolves every open tab's path, so the cost is one filesystem
    /// lookup per tab. That is bounded by the cap on the tab list and is
    /// paid only when someone asks for a repository.
    pub(super) fn position_of(&self, path: &str) -> Option<usize> {
        let key = platitude_core::repo::open_key(path);
        self.items
            .iter()
            .position(|t| platitude_core::repo::open_key(&t.copy_path) == key)
    }

    /// Takes a folder somebody asked for: the strip answers straight
    /// away where it already stands in it, and asks git where it opens
    /// otherwise.
    ///
    /// **The fast answer is for the same folder, spelled the same or
    /// not** — a repository in the strip opened once already, and
    /// asking again would spend two processes to be told what the strip
    /// knows. Everything else goes to git, because neither a folder
    /// deeper in a tree nor a linked working copy looks like anything
    /// in the strip until it is asked about (`Hub::place_repo`).
    pub(super) fn ask(&mut self, path: String, picked: bool) {
        // Trimmed and spelled once, here, so the string the tab keeps is
        // the one it was compared by — and the one the hover reads out
        // (デザイン規約 §パスの区切り). Every road in hands over `/`
        // already (git answers with it, `repo_key` writes it, the picker
        // keeps it); the fold is what keeps the one that does not from
        // reaching the strip.
        let path = crate::urlpath::shown_path(path.trim());
        if path.is_empty() {
            return;
        }
        if let Some(position) = self.position_of(&path) {
            self.show(position);
            return;
        }
        self.asking.push_back(Ask { path, picked });
        if self.asking.len() == 1 {
            self.start_asking();
        }
        self.settle_opening();
    }

    /// Says whether anything is still waiting on git (`opening`).
    ///
    /// Called wherever the queue moves, which is the same three places
    /// it is touched: a folder asked for, an answer landed, and a queue
    /// dropped for having nothing to answer it.
    pub(super) fn settle_opening(&mut self) {
        let opening = !self.asking.is_empty();
        if opening != self.opening {
            self.opening = opening;
            self.opening_changed();
        }
    }

    /// Asks git about the folder at the front of the queue, if one is
    /// waiting and nothing is out.
    pub(super) fn start_asking(&mut self) {
        let Some(ask) = self.asking.front() else {
            return;
        };
        let path = std::path::PathBuf::from(&ask.path);
        if !self.attached {
            self.asks.attach(self.get_qml_method_invoker());
            self.attached = true;
        }
        let feed = Arc::clone(&self.asks);
        if Hub::with(|hub| hub.place_repo(path, feed)) != Some(true) {
            // No runtime to ask on is no runtime to open a tab with
            // either (`Hub::reserve_tab`), so there is nothing to fall
            // back to and nothing left that will ever answer.
            self.asking.clear();
            self.settle_opening();
        }
    }

    /// What git said about the folder at the front, applied to the
    /// strip.
    pub(super) fn land(&mut self, ask: &Ask, msg: OpenMsg) {
        match msg {
            OpenMsg::Placed { place } => {
                let copy = crate::urlpath::shown_path(&place.info.workdir.to_string_lossy());
                let repo = crate::urlpath::shown_path(&place.repo.to_string_lossy());
                match landing_for(&self.items, &copy, &repo) {
                    Landing::Show(at) => self.show(at),
                    Landing::Switch(at) => self.switch_copy(at, copy),
                    Landing::New => self.open_new(copy, repo),
                }
            }
            OpenMsg::Refused {
                path,
                near,
                kind,
                message,
            } => {
                if ask.picked {
                    tracing::info!(path = %path.display(), kind, "picked folder refused");
                    self.open_rejected(
                        crate::urlpath::shown_path(&path.to_string_lossy()),
                        kind.to_string(),
                        message,
                        near,
                    );
                } else {
                    // Nobody is standing in a picker to be sent back to:
                    // the folder gets its tab and the page says what
                    // became of it. It names itself, having no working
                    // copy to be named after.
                    self.open_new(ask.path.clone(), ask.path.clone());
                }
            }
        }
    }

    /// Puts a tab for `copy` at the end of the strip and moves to it.
    pub(super) fn open_new(&mut self, copy: String, repo: String) {
        let title = title_of(&repo);
        let Some(Some(tab_id)) = Hub::with(|hub| hub.open_tab(std::path::PathBuf::from(&copy)))
        else {
            return;
        };
        self.push(TabItem {
            tab_id,
            title,
            repo_path: repo,
            copy_path: copy,
        });
        self.settle_titles();
        self.leave_front();
        self.current_index = self.items.len() as i32 - 1;
        self.report();
        self.current_index_changed();
        self.front_tab_asked();
    }

    /// Stands the tab at `at` in another working copy of the repository
    /// it is already showing, and moves to it
    /// (デザイン規約 §タブの所作「同じリポジトリのタブは 1 枚」).
    ///
    /// **The row is taken out and put back, not written over.** A page
    /// is built per tab id and holds everything it read off it
    /// (`RepoPageStack`), so a row that kept its id would leave the page
    /// of the copy being left standing over the copy being arrived at.
    /// Taking the row out is what takes that page down, the same way
    /// closing a tab does.
    ///
    /// The name does not move: a tab is named after its repository, and
    /// that is the one thing this does not change. Where the reader is
    /// standing is the sidebar's to say (規約 §左メニューの所作).
    pub(super) fn switch_copy(&mut self, at: usize, copy: String) {
        let Some(item) = self.items.get(at) else {
            return;
        };
        let (leaving, title, repo) = (item.tab_id, item.title.clone(), item.repo_path.clone());
        // Before the hub is told, so the page being taken down is still
        // whole while it hands over its layout and its unsent words
        // (`leaving_tab`) — the same order a close keeps.
        self.leave_front();
        let Some(Some(tab_id)) =
            Hub::with(|hub| hub.switch_tab(leaving, std::path::PathBuf::from(&copy)))
        else {
            return;
        };
        self.remove(at);
        self.insert(
            at,
            TabItem {
                tab_id,
                title,
                repo_path: repo,
                copy_path: copy,
            },
        );
        self.current_index = at as i32;
        self.report();
        self.current_index_changed();
        self.front_tab_asked();
    }

    /// Moves the strip to the tab at `position` and asks the band to
    /// bring that seat into view — what answering "show me this
    /// repository" always ends with (デザイン規約 §タブの所作
    /// 「移った先が答え」).
    pub(super) fn show(&mut self, position: usize) {
        self.set_current_index(position as i32);
        self.front_tab_asked();
    }

    /// Names every tab against the strip it now stands in
    /// ([`tab_name::names_for`]).
    ///
    /// Called by opening, restoring and closing — the three acts that
    /// change which names are in the strip. **Not by a move**: the order
    /// is not what a name is settled against, and a `dataChanged` on the
    /// row being carried would be one the hand did not ask for.
    ///
    /// Rows that came out the same are left alone, so the ordinary case
    /// — a repository whose name nobody shares — costs no notification at
    /// all, and the strip re-measures only the tabs whose words moved
    /// (`TabStrip.settleTitleCap`).
    pub(super) fn settle_titles(&mut self) {
        let names = {
            let paths: Vec<&str> = self.items.iter().map(|t| t.repo_path.as_str()).collect();
            tab_name::names_for(&paths)
        };
        let mut runs = Vec::new();
        for (at, (item, name)) in self.items.iter_mut().zip(names).enumerate() {
            if item.title != name {
                item.title = name;
                push_run(&mut runs, at);
            }
        }
        self.notify_runs(runs);
    }

    /// Names the tab the front row is holding (`current_tab_id`).
    ///
    /// Called from [`TabsModel::report`], which every act on the strip
    /// ends with — so this cannot be left out of one. The signal goes out
    /// only on a change of tab, which is what makes a row closed to the
    /// left, or a tab carried past another, silent here: the strip
    /// renumbered, and the same repository is still in front.
    pub(super) fn settle_current(&mut self) {
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
    /// switching are single acts, so they report as they happen; the
    /// file itself is still only written by the
    /// flush.
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
    pub(super) fn report(&mut self) {
        self.settle_current();
        self.settle_open_repos();
        // Named the way the file names it. The store normalises separators
        // on the way out anyway, so handing it the raw path would leave the
        // state held here unequal to the one on disk — harmless today only
        // because the flush compares against what it last
        // wrote.
        //
        // **Both paths go down.** A restored tab is not opened until it
        // is looked at, so nothing would be left to ask which repository
        // a copy hangs off, and the strip would call the tab after the
        // copy until the reader clicked it.
        let tabs = self
            .items
            .iter()
            .map(|t| platitude_core::settings::TabRecord {
                path: platitude_core::settings::repo_key(&t.copy_path),
                repo: platitude_core::settings::repo_key(&t.repo_path),
            })
            .collect::<Vec<_>>();
        let active = usize::try_from(self.current_index).unwrap_or(0);
        Hub::with(|hub| hub.set_tabs_state(platitude_core::settings::TabsState { tabs, active }));
    }

    /// Lists the strip for the readers that want it whole (`open_repos`).
    ///
    /// From [`report`], which every act on the strip ends with — so a tab
    /// opened, closed, renamed against a new namesake or carried past its
    /// neighbour all land here, and those are the four things that can
    /// change what the list says. The path is the row's own spelling: it
    /// is what the reader hands back when they pick a name, and git is
    /// run in it.
    ///
    /// Silent when nothing came out different — the act that ends here is
    /// usually a switch, which moves neither a name nor an order.
    ///
    /// The path is the copy the tab is standing in, because that is the
    /// one git is run in; the name is the repository's, the same as the
    /// tab's.
    ///
    /// [`report`]: TabsModel::report
    fn settle_open_repos(&mut self) {
        let listed = OpenRepos::new(
            self.items
                .iter()
                .map(|item| OpenRepo {
                    name: item.title.clone(),
                    path: item.copy_path.clone(),
                })
                .collect(),
        );
        if listed != self.open_repos {
            self.open_repos = listed;
            self.open_repos_changed();
        }
    }
}

/// What a folder that opens does to the strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Landing {
    /// The very copy standing in this tab: the strip moves to it and
    /// nothing is read again.
    Show(usize),
    /// Another working copy of the repository this tab is showing: the
    /// strip moves to it and stands it in the copy asked for.
    Switch(usize),
    /// A repository the strip is not showing.
    New,
}

/// Where a folder that opens lands: the one place that answers "is this
/// repository already open?" (デザイン規約 §タブの所作 「判定は 1 か所に
/// 置く」), for every road in and for the tabs put back at startup.
///
/// `copy` is the working copy the folder is in and `repo` the
/// repository it hangs off ([`platitude_core::repo::Place`]), so a
/// folder deeper in a tree has already been folded into the copy around
/// it before this is asked. **The copy is looked for first**: standing a
/// tab in the copy it is already standing in would read the repository
/// again for no answer the reader asked for.
///
/// Compared by `repo::open_key`, which is what folds the spellings one
/// folder reaches the application in (git's, the picker's, a hand's).
pub(super) fn landing_for(items: &[TabItem], copy: &str, repo: &str) -> Landing {
    let key = platitude_core::repo::open_key;
    let standing = key(copy);
    if let Some(at) = items.iter().position(|t| key(&t.copy_path) == standing) {
        return Landing::Show(at);
    }
    let showing = key(repo);
    if let Some(at) = items.iter().position(|t| key(&t.repo_path) == showing) {
        return Landing::Switch(at);
    }
    Landing::New
}

/// The tab's label before the strip has been consulted: the repository's
/// own folder name, which is what it is called wherever nobody shares it.
///
/// Asked of the same rule the whole strip is settled by
/// ([`tab_name::names_for`]), so a tab is never named twice over — the
/// row is pushed with this and [`TabsModel::settle_titles`] grows it if
/// the strip it landed in has a namesake standing in it.
pub(super) fn title_of(path: &str) -> String {
    tab_name::names_for(&[path])
        .into_iter()
        .next()
        .unwrap_or_default()
}

/// Where the row at `current` ends up once the row at `from` has been
/// taken out and put down at `to`.
///
/// Everything between the two shifts by one, towards the place the moved
/// row left. `current` is a position: a strip with nothing in front of
/// it says -1, and no move gives it a tab.
pub(crate) fn index_after_move(current: i32, from: usize, to: usize) -> i32 {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A tab standing in `copy`, of the repository whose own working
    /// copy is at `repo`.
    fn tab(repo: &str, copy: &str) -> TabItem {
        TabItem {
            tab_id: 1,
            title: "repo".to_string(),
            repo_path: repo.to_string(),
            copy_path: copy.to_string(),
        }
    }

    /// The copy already open is the one landing that reads nothing
    /// again — asking for it twice is answered by moving to it.
    #[test]
    fn the_copy_a_tab_is_standing_in_is_that_tab() {
        let strip = [tab("C:/one", "C:/one"), tab("C:/two", "C:/two/wt")];
        assert_eq!(landing_for(&strip, "C:/one", "C:/one"), Landing::Show(0));
        assert_eq!(landing_for(&strip, "C:/two/wt", "C:/two"), Landing::Show(1));
    }

    /// Another working copy of a repository already in the strip: that
    /// tab stands in it, whichever way round the two copies are.
    #[test]
    fn another_copy_of_an_open_repository_moves_the_tab_into_it() {
        let strip = [tab("C:/one", "C:/one"), tab("C:/two", "C:/two/wt")];
        assert_eq!(
            landing_for(&strip, "C:/elsewhere/wt", "C:/one"),
            Landing::Switch(0),
            "a linked copy of the repository the first tab is showing"
        );
        assert_eq!(
            landing_for(&strip, "C:/two", "C:/two"),
            Landing::Switch(1),
            "and the repository's own copy, from a tab standing in a linked one"
        );
    }

    /// A repository nobody has open, however alike the folders are
    /// spelled.
    #[test]
    fn a_repository_the_strip_is_not_showing_opens() {
        let strip = [tab("C:/one", "C:/one")];
        assert_eq!(landing_for(&strip, "C:/three", "C:/three"), Landing::New);
        assert_eq!(landing_for(&[], "C:/one", "C:/one"), Landing::New);
    }

    /// Separators and a trailing one are not what tells two folders
    /// apart (`repo::open_key`), and a folder that is not there falls
    /// back to the fold the file is written with.
    #[test]
    fn one_folder_spelled_two_ways_is_one_tab() {
        let strip = [tab("C:/one", "C:/one")];
        assert_eq!(landing_for(&strip, r"C:\one", r"C:\one"), Landing::Show(0));
        assert_eq!(landing_for(&strip, "C:/one/", "C:/one/"), Landing::Show(0));
    }
}
