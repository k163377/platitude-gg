//! The strip's bookkeeping, off the Qt block: where a repository already
//! sits, the tab names, and what the hub is told after each act.

use super::*;

impl TabsModel {
    pub(super) fn drain(&mut self) {
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

    pub(super) fn leave_front(&mut self) {
        if self.current_tab.is_some() {
            self.leaving_tab();
        }
    }

    /// Where the worktree at `path` already stands in the strip, by
    /// `repo::open_key` (each road in spells one folder differently).
    ///
    /// One filesystem lookup per tab, paid only when someone asks for a
    /// repository.
    pub(super) fn position_of(&self, path: &str) -> Option<usize> {
        let key = platitude_core::repo::open_key(path);
        self.items
            .iter()
            .position(|t| platitude_core::repo::open_key(&t.worktree_path) == key)
    }

    /// Takes a folder somebody asked for: moves to it where the strip
    /// already stands in it, and asks git where it opens otherwise
    /// (`Hub::place_repo`).
    ///
    /// Only the same folder is answered without git: a folder deeper in a
    /// tree or a linked worktree looks like nothing in the strip until git has
    /// placed it. Every road in comes through here and waits — the picker, a
    /// WORKTREES row, another worktree's pill, `_PGG_AUTO_OPEN` — and the
    /// queue (`asking`) is all that keeps `_PGG_AUTO_OPEN=a;b;c` in its order.
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
            self.asks.attach(crate::hub::Wake::drain(self, Self::drain));
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
                let worktree = crate::urlpath::shown_path(&place.info.workdir.to_string_lossy());
                let repo = crate::urlpath::shown_path(&place.repo.to_string_lossy());
                match landing_for(&self.items, &worktree, &repo) {
                    Landing::Show(at) => self.show(at),
                    Landing::Switch(at) => self.switch_worktree(at, worktree),
                    Landing::New => self.open_new(worktree, repo),
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

    /// Puts a tab for `worktree` at the end of the strip and moves to it.
    pub(super) fn open_new(&mut self, worktree: String, repo: String) {
        let title = title_of(&repo);
        let Some(Some(tab_id)) = Hub::with(|hub| {
            hub.open_tab(
                std::path::PathBuf::from(&worktree),
                std::path::PathBuf::from(&repo),
            )
        }) else {
            return;
        };
        self.push(TabItem::standing(tab_id, title, repo, worktree));
        self.settle_titles();
        self.leave_front();
        self.current_tab = Some(tab_id);
        self.report();
        self.ask_front();
    }

    /// Stands the tab at `at` in another worktree of the repository
    /// it shows, and moves to it
    /// (デザイン規約 §タブの所作「同じリポジトリのタブは 1 枚」).
    ///
    /// The row is rewritten in place. A tab already in front stays in front,
    /// which keeps its page (`RepoPageStack`): the worktrees share every
    /// commit and ref, so the graph stands while the session under it is
    /// swapped (`Hub::restand_tab`). One that was not comes forward and is
    /// built a page as any tab is. The title stays; only
    /// `TabItem::worktree_name` moves. Headless: `worktree-stand` /
    /// `carried-open`, whose `kept=` says the page was not built again.
    pub(super) fn switch_worktree(&mut self, at: usize, worktree: String) {
        self.stand_at(at, worktree, false);
    }

    /// Takes the tab holding `tab_id` out of the strip, and its session
    /// with it (`Hub::close_tab`).
    pub(super) fn close(&mut self, tab_id: i32) {
        let closing = self.items.iter().position(|t| t.tab_id == tab_id);
        let front = closing.is_some() && self.current_tab == Some(tab_id);
        // The front tab says its goodbyes (`leaving_tab`) before the hub
        // closes its session, while the page is still whole.
        if front {
            self.leave_front();
        }
        Hub::with(|hub| hub.close_tab(tab_id));
        let Some(pos) = closing else {
            return;
        };
        if front {
            // Its row lets go of the front before it goes: its delegates are
            // deleted later, and until then a page and a tab still flagged
            // would answer for a tab already closed.
            if let Some(item) = self.items.get_mut(pos) {
                item.current = false;
            }
            self.notify_runs(std::iter::once(pos..=pos));
        }
        self.remove(pos);
        // The closed tab may have been another's namesake.
        self.settle_titles();
        // A tab closed beside the front leaves it be.
        if front {
            self.current_tab = front_after_close(&self.items, pos);
        }
        self.report();
    }

    /// Takes the tab at `from` out of the strip and puts it down at `to`
    /// (デザイン規約 §タブの所作). A drag asks one neighbour at a time;
    /// the headless run carries the whole distance in one call.
    pub(super) fn carry(&mut self, from: usize, to: usize) {
        if from == to || from >= self.items.len() || to >= self.items.len() {
            return;
        }
        self.move_notified(from, to);
        // The tab in front stays in front; the row it sits in may not.
        self.report();
    }

    /// Stands the tab holding `tab_id` again, in `worktree`: the worktree it
    /// stood in, its folder renamed (the tab follows the new folder — user
    /// decision, ask before changing), or the same folder after a rename git
    /// refused, on a session of its own either way — the one it had stood
    /// down for the move (`RepoSession::move_worktree`). The same worktree
    /// both ways, so what was typed there goes with it (`Hub::carry_draft`):
    /// git's spelling of the old folder need not be the tab's.
    pub(super) fn follow_tab_worktree(&mut self, tab_id: i32, worktree: String) {
        let Some(at) = self.items.iter().position(|t| t.tab_id == tab_id) else {
            return;
        };
        self.stand_at(at, crate::urlpath::shown_path(&worktree), true);
    }

    /// [`Self::switch_worktree`] and [`Self::follow_tab_worktree`]'s one road.
    fn stand_at(&mut self, at: usize, worktree: String, carried: bool) {
        let Some(item) = self.items.get(at) else {
            return;
        };
        let standing = item.tab_id;
        // Every reader's road starts on the page in front; a path handed
        // in from outside can name another tab's repository, and that tab
        // has no page or session — it is only pointed elsewhere.
        let front = self.current_tab == Some(standing);
        if front {
            // Before the hub moves: drafts are filed under the worktree they
            // were written in (`Hub::hold_draft`).
            self.leaving_worktree();
        } else {
            self.leave_front();
        }
        if carried {
            Hub::with(|hub| hub.carry_draft(standing, &worktree));
        }
        if Hub::with(|hub| hub.restand_tab(standing, std::path::PathBuf::from(&worktree)))
            != Some(true)
        {
            return;
        }
        if let Some(item) = self.items.get_mut(at) {
            item.stand_in(worktree);
        }
        self.notify_runs(std::iter::once(at..=at));
        self.current_tab = Some(standing);
        self.report();
        if front {
            // After the hub, so the page restores the new worktree's words.
            self.stood_worktree();
        }
        self.ask_front();
    }

    /// Stands the tab holding `tab_id` back in the repository's own
    /// worktree, because the linked one would not open — gone, or no longer
    /// a repository (user decision — ask before changing). A tab names the
    /// repository, and one worktree gone is not the repository gone; only
    /// when that one will not open either does the tab show its failure
    /// screen (デザイン規約 §可否・警告の出し場所). One screen only: asked
    /// before the refusal is shown (`RepoTab` drain), so no failure flashes
    /// for a frame before the stand. Restored tabs fall back the same way
    /// (`TabsModel::restore_tabs`).
    ///
    /// Not through [`landing_for`] or git: the tab and its home worktree are
    /// already known. Silent for a tab already home — there is nowhere
    /// further back, so its failure is shown (`RepoTab` decides).
    pub(super) fn stand_home(&mut self, tab_id: i32) {
        let Some(at) = self.items.iter().position(|t| t.tab_id == tab_id) else {
            return;
        };
        let Some(item) = self.items.get(at) else {
            return;
        };
        if item.worktree_path == item.repo_path {
            return;
        }
        let home = item.repo_path.clone();
        self.switch_worktree(at, home);
    }

    /// Moves the strip to the tab at `position` and asks the band to
    /// bring that seat into view: the move is the answer, so it must be
    /// seen (デザイン規約 §タブの所作「必ず何かが起きる」).
    pub(super) fn show(&mut self, position: usize) {
        self.set_current_index(position as i32);
        self.ask_front();
    }

    /// Asks the band to bring the tab now in front into view
    /// (`front_tab_asked`); every road that asks has just put one there.
    pub(super) fn ask_front(&mut self) {
        if let Some(tab_id) = self.current_tab {
            self.front_tab_asked(tab_id);
        }
    }

    /// Names every tab against the strip it now stands in
    /// ([`tab_name::names_for`]), wherever the set of names changes:
    /// opening, restoring, closing.
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

    /// Reads everything else about the front off `current_tab`: the row it
    /// sits in (`current_index`), each row's flag (`TabItem::current`), and
    /// its repository and worktree names (`current_repo_name` /
    /// `current_worktree_name`). Called from [`TabsModel::report`], so every
    /// act on the strip ends here.
    ///
    /// Each is told only when it changes, so a renumbering (a close to the
    /// left, a carry past) tells `currentIndex` and no row's flag.
    pub(super) fn settle_current(&mut self) {
        let at = self
            .current_tab
            .and_then(|id| self.items.iter().position(|tab| tab.tab_id == id));
        let front = at.and_then(|at| self.items.get(at));
        let (repo_name, worktree_name) = front.map_or_else(Default::default, |tab| {
            (title_of(&tab.repo_path), tab.worktree_name.clone())
        });
        let index = at.map_or(-1, |at| at as i32);
        let moved = index != self.current_index;
        // Before any signal: whatever a signal wakes reads the place the
        // tab now sits in.
        self.current_index = index;
        // The names first: whatever reads the tab in front finds them
        // already said.
        if repo_name != self.current_repo_name || worktree_name != self.current_worktree_name {
            self.current_repo_name = repo_name;
            self.current_worktree_name = worktree_name;
            self.front_names_changed();
        }
        // The place before the flags: the window shows the page stack once
        // a tab is in front (`WindowBody`) and the stack shows the seat in
        // front by it (`RepoPageStack`), so a page is built visible.
        if moved {
            self.current_index_changed();
        }
        let mut runs = Vec::new();
        for (row, item) in self.items.iter_mut().enumerate() {
            let current = Some(row) == at;
            if item.current != current {
                item.current = current;
                push_run(&mut runs, row);
            }
        }
        self.notify_runs(runs);
    }

    /// Hands the hub the strip as it stands (the file is written only by
    /// the flush).
    ///
    /// Every act on the strip ends here, and this tells whatever moved
    /// about the front ([`settle_current`]); no act tells `currentIndex`
    /// itself.
    ///
    /// [`settle_current`]: TabsModel::settle_current
    pub(super) fn report(&mut self) {
        self.settle_current();
        self.settle_open_repos();
        // In the file's spelling (`repo_key`), so the state held here
        // equals the one on disk.
        //
        // Both paths go down: a restored tab is not opened until looked
        // at, so nothing else says which repository a worktree hangs off.
        let tabs = self
            .items
            .iter()
            .map(|t| platitude_core::settings::TabRecord {
                path: platitude_core::settings::repo_key(&t.worktree_path),
                repo: platitude_core::settings::repo_key(&t.repo_path),
            })
            .collect::<Vec<_>>();
        let active = usize::try_from(self.current_index).unwrap_or(0);
        Hub::with(|hub| hub.set_tabs_state(platitude_core::settings::TabsState { tabs, active }));
    }

    /// Lists the strip for the readers that want it whole (`open_repos`),
    /// from [`report`] so every act lands here: an opening, a close, a name
    /// settling and a carry along the strip all pass through it, and only
    /// it (`settle_titles` is not on the carry's road).
    ///
    /// The name is the tab's; the path is the worktree the tab stands in, in
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
                    path: item.worktree_path.clone(),
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
    /// The very worktree standing in this tab: the strip moves to it and
    /// nothing is read again.
    Show(usize),
    /// Another worktree of the repository this tab is showing: the
    /// strip moves to it and stands it in the worktree asked for.
    Switch(usize),
    /// A repository the strip is not showing.
    New,
}

/// Where a folder that opens lands: the one place that answers "is this
/// repository already open?" (デザイン規約 §タブの所作「同じリポジトリの
/// タブは 1 枚」), for every road in — the folder picker, a WORKTREES row,
/// the startup arguments — and for restored tabs.
///
/// The question goes to git (`Hub::place_repo`): which worktree a folder
/// is in, and whose that worktree is, cannot be read off the path's
/// spelling. A folder inside a worktree is that worktree (`repo/crates`
/// is `repo`). The two processes are paid first and the tab moves on the
/// answer: a tab opened and then folded flashes for a frame or two.
///
/// `worktree` and `repo` are git's [`platitude_core::repo::Place`], compared
/// by `repo::open_key`. The worktree is looked for first: switching a tab
/// into the worktree it already stands in would re-read the repository for
/// nothing. Headless: `open-again`.
pub(super) fn landing_for(items: &[TabItem], worktree: &str, repo: &str) -> Landing {
    let key = platitude_core::repo::open_key;
    let standing = key(worktree);
    if let Some(at) = items.iter().position(|t| key(&t.worktree_path) == standing) {
        return Landing::Show(at);
    }
    let showing = key(repo);
    if let Some(at) = items.iter().position(|t| key(&t.repo_path) == showing) {
        return Landing::Switch(at);
    }
    Landing::New
}

/// The tab that comes to the front once the tab in front, at row
/// `closed`, has been taken out of `items`: its right neighbour, now in that
/// row, or the last tab when it was the last; none when it was the only one.
fn front_after_close(items: &[TabItem], closed: usize) -> Option<i32> {
    items.get(closed).or(items.last()).map(|tab| tab.tab_id)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(repo: &str, worktree: &str) -> TabItem {
        TabItem::standing(
            1,
            "repo".to_string(),
            repo.to_string(),
            worktree.to_string(),
        )
    }

    #[test]
    fn only_a_linked_worktree_is_named_after_the_repository() {
        assert_eq!(tab("C:/one", "C:/one").worktree_name, "");
        assert_eq!(
            tab("C:/two", "C:/elsewhere/worktree").worktree_name,
            "worktree"
        );
    }

    #[cfg(windows)]
    #[test]
    fn a_windows_worktree_is_named_whichever_separator_wrote_it() {
        assert_eq!(
            tab("C:/two", r"C:\elsewhere\worktree").worktree_name,
            "worktree"
        );
    }

    #[test]
    fn one_folder_spelled_two_ways_names_no_worktree() {
        assert_eq!(tab("C:/one", "C:/one/").worktree_name, "");
        assert_eq!(tab("C:/one/", "C:/one").worktree_name, "");
    }

    #[test]
    fn the_worktree_a_tab_is_standing_in_is_that_tab() {
        let strip = [tab("C:/one", "C:/one"), tab("C:/two", "C:/two/worktree")];
        assert_eq!(landing_for(&strip, "C:/one", "C:/one"), Landing::Show(0));
        assert_eq!(
            landing_for(&strip, "C:/two/worktree", "C:/two"),
            Landing::Show(1)
        );
    }

    #[test]
    fn another_worktree_of_an_open_repository_moves_the_tab_into_it() {
        let strip = [tab("C:/one", "C:/one"), tab("C:/two", "C:/two/worktree")];
        assert_eq!(
            landing_for(&strip, "C:/elsewhere/worktree", "C:/one"),
            Landing::Switch(0),
            "a linked worktree of the repository the first tab is showing"
        );
        assert_eq!(
            landing_for(&strip, "C:/two", "C:/two"),
            Landing::Switch(1),
            "and the repository's own worktree, from a tab standing in a linked one"
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
