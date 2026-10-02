//! The strip's bookkeeping, off the Qt block: where a repository already
//! sits, the tab names, and what the hub is told after each act.

use super::*;

impl TabsModel {
    pub(super) fn leave_front(&mut self) {
        if self.current_index >= 0 {
            self.leaving_tab(self.current_index);
        }
    }

    /// Where the working copy at `path` already stands in the strip, by
    /// `repo::open_key` (each road in spells one folder differently).
    ///
    /// One filesystem lookup per tab, paid only when someone asks for a
    /// repository.
    pub(super) fn position_of(&self, path: &str) -> Option<usize> {
        let key = platitude_core::repo::open_key(path);
        self.items
            .iter()
            .position(|t| platitude_core::repo::open_key(&t.copy_path) == key)
    }

    /// Takes a folder somebody asked for: moves to it where the strip
    /// already stands in it, and asks git where it opens otherwise
    /// (`Hub::place_repo`).
    ///
    /// Only the same folder is answered without git: a folder deeper in a
    /// tree or a linked copy looks like nothing in the strip until git has
    /// placed it.
    pub(super) fn ask(&mut self, path: String, picked: bool) {
        // Trimmed and spelled once here, so the tab keeps the string it was
        // compared by and the hover reads (デザイン規約 §パスの区切り).
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

    /// Says whether anything is still waiting on git (`opening`); called
    /// wherever the queue moves.
    pub(super) fn settle_opening(&mut self) {
        let opening = !self.asking.is_empty();
        if opening != self.opening {
            self.opening = opening;
            self.opening_changed();
        }
    }

    /// Asks git about the folder at the front of the queue, if one is
    /// waiting; call only with nothing out (`TabsModel::asking`).
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
            // No runtime: nothing will ever answer, and no tab could open
            // either (`Hub::reserve_tab`).
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
                    // No picker to go back to: the folder gets a tab,
                    // named by itself, and the page shows the refusal.
                    self.open_new(ask.path.clone(), ask.path.clone());
                }
            }
        }
    }

    /// Puts a tab for `copy` at the end of the strip and moves to it.
    pub(super) fn open_new(&mut self, copy: String, repo: String) {
        let title = title_of(&repo);
        let Some(Some(tab_id)) = Hub::with(|hub| {
            hub.open_tab(
                std::path::PathBuf::from(&copy),
                std::path::PathBuf::from(&repo),
            )
        }) else {
            return;
        };
        self.push(TabItem::standing(tab_id, title, repo, copy));
        self.settle_titles();
        self.leave_front();
        self.current_index = self.items.len() as i32 - 1;
        self.report();
        self.current_index_changed();
        self.front_tab_asked();
    }

    /// Stands the tab at `at` in another working copy of the repository
    /// it shows, and moves to it
    /// (デザイン規約 §タブの所作「同じリポジトリのタブは 1 枚」).
    ///
    /// The row is rewritten in place and keeps its id, which keeps the
    /// page (`RepoPageStack`): the copies share every commit and ref, so
    /// the graph stands while the session under it is swapped
    /// (`Hub::restand_tab`). The title stays; only `TabItem::copy_name`
    /// moves.
    pub(super) fn switch_copy(&mut self, at: usize, copy: String) {
        let Some(item) = self.items.get(at) else {
            return;
        };
        let standing = item.tab_id;
        // Every reader's road starts on the page in front; a path handed
        // in from outside can name another tab's repository, and that tab
        // has no page or session — it is only pointed elsewhere.
        let front = usize::try_from(self.current_index).ok() == Some(at);
        if front {
            // Before the hub moves: drafts are filed under the copy they
            // were written in (`Hub::hold_draft`).
            self.leaving_copy(at as i32);
        } else {
            self.leave_front();
        }
        if Hub::with(|hub| hub.restand_tab(standing, std::path::PathBuf::from(&copy))) != Some(true)
        {
            return;
        }
        if let Some(item) = self.items.get_mut(at) {
            item.stand_in(copy);
        }
        self.notify_runs([(at, at)]);
        self.current_index = at as i32;
        self.report();
        self.current_index_changed();
        if front {
            // After the hub, so the page restores the new copy's words.
            self.stood_copy(at as i32);
        }
        self.front_tab_asked();
    }

    /// Stands the tab holding `tab_id` back in the repository's own
    /// working copy, because the linked one would not open
    /// (デザイン規約 §タブの所作「立てない所へは立たない」).
    ///
    /// Not through [`landing_for`] or git: the tab and its home copy are
    /// already known. Silent for a tab already home — there is nowhere
    /// further back, so its failure is shown (`RepoTab` decides).
    pub(super) fn stand_home(&mut self, tab_id: i32) {
        let Some(at) = self.items.iter().position(|t| t.tab_id == tab_id) else {
            return;
        };
        let Some(item) = self.items.get(at) else {
            return;
        };
        if item.copy_path == item.repo_path {
            return;
        }
        let home = item.repo_path.clone();
        self.switch_copy(at, home);
    }

    /// Moves the strip to the tab at `position` and asks the band to
    /// bring that seat into view (デザイン規約 §タブの所作
    /// 「移った先が答え」).
    pub(super) fn show(&mut self, position: usize) {
        self.set_current_index(position as i32);
        self.front_tab_asked();
    }

    /// Names every tab against the strip it now stands in
    /// ([`tab_name::names_for`]).
    ///
    /// Not called on a move: order does not settle a name, and a
    /// `dataChanged` on the carried row is one the hand did not ask for.
    /// Unchanged rows are not notified, so the strip re-measures only
    /// tabs whose words moved (`TabStrip.settleTitleCap`).
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

    /// Names the front row's tab (`current_tab_id`) and its repository and
    /// copy (`current_repo_name` / `current_copy_name`). Called from
    /// [`TabsModel::report`], so every act on the strip ends here.
    ///
    /// Each signal goes out only when what it names changes, so a
    /// renumbering (a close to the left, a carry past) is silent.
    pub(super) fn settle_current(&mut self) {
        let front = usize::try_from(self.current_index)
            .ok()
            .and_then(|at| self.items.get(at));
        let id = front.map_or(-1, |tab| tab.tab_id);
        let (repo_name, copy_name) = front.map_or_else(Default::default, |tab| {
            (title_of(&tab.repo_path), tab.copy_name.clone())
        });
        // The names before the row's own signal: whatever reads the tab
        // in front off it finds the names already said.
        if repo_name != self.current_repo_name || copy_name != self.current_copy_name {
            self.current_repo_name = repo_name;
            self.current_copy_name = copy_name;
            self.front_names_changed();
        }
        if id != self.current_tab_id {
            self.current_tab_id = id;
            self.current_index_changed();
        }
    }

    /// Hands the hub the strip as it stands (the file is written only by
    /// the flush).
    ///
    /// Call before the act's own `current_index_changed()`: this names the
    /// front tab ([`settle_current`]), and a notification with the row
    /// moved but the tab not yet named leaves "the page in front" reading
    /// nothing.
    ///
    /// [`settle_current`]: TabsModel::settle_current
    pub(super) fn report(&mut self) {
        self.settle_current();
        self.settle_open_repos();
        // In the file's spelling (`repo_key`), so the state held here
        // equals the one on disk.
        //
        // Both paths go down: a restored tab is not opened until looked
        // at, so nothing else says which repository a copy hangs off.
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

    /// Lists the strip for the readers that want it whole (`open_repos`),
    /// from [`report`] so every act lands here.
    ///
    /// The name is the tab's; the path is the copy the tab stands in, in
    /// the row's own spelling — what the reader hands back on picking a
    /// name, and where git is run.
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
/// 置く」), for every road in and for restored tabs.
///
/// `copy` and `repo` are git's [`platitude_core::repo::Place`], compared
/// by `repo::open_key`. The copy is looked for first: switching a tab into
/// the copy it already stands in would re-read the repository for nothing.
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

/// The tab's label on its own: the repository's folder name, by the rule
/// the whole strip is settled by ([`tab_name::names_for`]);
/// [`TabsModel::settle_titles`] grows it beside a namesake.
pub(super) fn title_of(path: &str) -> String {
    tab_name::names_for(&[path])
        .into_iter()
        .next()
        .unwrap_or_default()
}

/// Where the row at `current` ends up once the row at `from` has been
/// taken out and put down at `to`; -1 (nothing in front) stays -1.
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

    fn tab(repo: &str, copy: &str) -> TabItem {
        TabItem::standing(1, "repo".to_string(), repo.to_string(), copy.to_string())
    }

    #[test]
    fn only_a_linked_copy_is_named_after_the_repository() {
        assert_eq!(tab("C:/one", "C:/one").copy_name, "");
        assert_eq!(tab("C:/two", "C:/elsewhere/wt").copy_name, "wt");
    }

    #[cfg(windows)]
    #[test]
    fn a_windows_copy_is_named_whichever_separator_wrote_it() {
        assert_eq!(tab("C:/two", r"C:\elsewhere\wt").copy_name, "wt");
    }

    #[test]
    fn one_folder_spelled_two_ways_names_no_copy() {
        assert_eq!(tab("C:/one", "C:/one/").copy_name, "");
        assert_eq!(tab("C:/one/", "C:/one").copy_name, "");
    }

    #[test]
    fn the_copy_a_tab_is_standing_in_is_that_tab() {
        let strip = [tab("C:/one", "C:/one"), tab("C:/two", "C:/two/wt")];
        assert_eq!(landing_for(&strip, "C:/one", "C:/one"), Landing::Show(0));
        assert_eq!(landing_for(&strip, "C:/two/wt", "C:/two"), Landing::Show(1));
    }

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

    #[test]
    fn a_repository_the_strip_is_not_showing_opens() {
        let strip = [tab("C:/one", "C:/one")];
        assert_eq!(landing_for(&strip, "C:/three", "C:/three"), Landing::New);
        assert_eq!(landing_for(&[], "C:/one", "C:/one"), Landing::New);
    }

    /// The paths do not exist, so this binds `repo::open_key`'s fallback
    /// fold (`repo_key`).
    #[test]
    fn one_folder_spelled_two_ways_is_one_tab() {
        let strip = [tab("C:/one", "C:/one")];
        assert_eq!(landing_for(&strip, r"C:\one", r"C:\one"), Landing::Show(0));
        assert_eq!(landing_for(&strip, "C:/one/", "C:/one/"), Landing::Show(0));
    }
}
