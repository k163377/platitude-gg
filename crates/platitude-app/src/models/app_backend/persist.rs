//! Writing the window, its layout and the live defaults back to the
//! settings store.

use super::*;

impl AppBackend {
    /// Sets how often every open repository fetches, in minutes. Zero (the
    /// blank input) turns it off, and the ceiling is core's to apply
    /// (`session::auto_fetch_minutes`) — the field this writes is the one a
    /// hand-written `settings.toml` writes, so a limit spelled out here as
    /// well would be a second answer to the same question.
    pub(super) fn apply_auto_fetch_minutes(&mut self, minutes: i32) {
        let asked = platitude_core::session::auto_fetch_minutes(minutes.max(0).unsigned_abs());
        let minutes = asked as i32;
        if self.auto_fetch_minutes == minutes {
            return;
        }
        self.auto_fetch_minutes = minutes;
        Hub::with(|hub| hub.set_auto_fetch_minutes(asked));
        self.settings_changed();
    }

    /// Sets how much history every graph opens with. Zero is the whole of
    /// it: what the screen's own box asks for.
    ///
    /// The floor is core's to apply (`session::log_limit`) — the field
    /// this writes is the one a hand-written `settings.toml` writes, so a
    /// floor spelled out here as well would be a second answer to the
    /// same question.
    pub(super) fn apply_initial_commits(&mut self, commits: i32) {
        let asked = match commits.max(0).unsigned_abs() {
            0 => None,
            count => Some(platitude_core::session::log_limit(count)),
        };
        // Saturating: `settings.toml` can hold a count no `i32` can, and
        // the property is what the screen shows.
        let commits = asked.map_or(0, |count| i32::try_from(count).unwrap_or(i32::MAX));
        if self.initial_commits == commits {
            return;
        }
        self.initial_commits = commits;
        Hub::with(|hub| hub.set_initial_commits(asked));
        self.settings_changed();
    }

    /// Sets how many git processes the application runs at once. The
    /// range is core's to apply (`process::concurrency`) — the field this
    /// writes is the one a hand-written `settings.toml` writes, so a
    /// range spelled out here as well would be a second answer to the
    /// same question. Zero (the blank input) is the default: no git at
    /// all is not something a box can ask for.
    pub(super) fn apply_git_concurrency(&mut self, concurrency: i32) {
        let asked = match concurrency.max(0).unsigned_abs() {
            0 => platitude_core::process::default_concurrency(),
            count => platitude_core::process::concurrency(count),
        };
        let concurrency = asked as i32;
        if self.git_concurrency == concurrency {
            return;
        }
        self.git_concurrency = concurrency;
        Hub::with(|hub| hub.set_git_concurrency(asked));
        self.settings_changed();
    }

    /// Sets how often the other working copies are read for uncommitted
    /// work, in seconds. Zero (the blank input) turns it off, and the
    /// floor and ceiling are core's (`session::copies_interval_secs`),
    /// for the reason the interval above leaves its range to core.
    pub(super) fn apply_copies_interval_secs(&mut self, secs: i32) {
        let asked = platitude_core::session::copies_interval_secs(secs.max(0).unsigned_abs());
        let secs = asked as i32;
        if self.copies_interval_secs == secs {
            return;
        }
        self.copies_interval_secs = secs;
        self.copies_interval_ms = secs.saturating_mul(1000);
        Hub::with(|hub| hub.set_copies_interval_secs(asked));
        self.settings_changed();
    }

    /// The settings screen has just opened: the path it is about to show
    /// gets asked for its version.
    ///
    /// Asked on every open, because the answer is about a file on disk,
    /// which moves without anybody retyping the path — an installer that
    /// replaced it, a stick that was unplugged.
    pub(super) fn begin_git_path_screen(&mut self) {
        let path = self.git_path.clone();
        self.begin_git_path_check(path);
    }

    /// Records which git this computer runs, and asks that one for its
    /// version.
    ///
    /// **Only a value that moved is asked about again.** The answer
    /// already under the box is about the path still in it, and throwing
    /// it away for a fresh "Asking…" would leave the way out with nothing
    /// to decide the restart on ([`Self::leave_git_path_screen`]).
    ///
    /// The air is taken off here as well as on the way out of the file
    /// (`settings::toml::text`), so that the box and the file agree
    /// before the first save.
    pub(super) fn apply_git_path(&mut self, path: &str) {
        let path = path.trim();
        if self.git_path == path {
            return;
        }
        self.git_path = path.to_string();
        Hub::with(|hub| hub.set_git_path(path.to_string()));
        self.begin_git_path_check(path.to_string());
    }

    /// Works out whether the chapter has a restart to offer: the box is
    /// holding a path this run was not built on, and the git at it
    /// answered.
    ///
    /// Called wherever either of those moves — the box being written and
    /// the version coming back — because the offer is what both the
    /// button's shape and the warning beside it are drawn from.
    pub(super) fn settle_restart_offer(&mut self) {
        // **The binary itself** — the answer's own word on it
        // (`AppMsg::GitPathProbed::names_the_run`), which is only as
        // current as the state beside it: a box rewritten since is
        // "checking", and offers nothing until its own answer is in.
        self.git_path_offers_restart = !self.restart_wanted
            && !self.git_path_names_the_run
            && matches!(self.git_path_state.as_str(), "ok" | "old");
    }

    /// The reader has held the button down. The window is to close and
    /// come back on the git named in the box.
    ///
    /// **A press cannot outrun what the screen was offering.** The guard
    /// is the same one the button is drawn from ([`Self::settle_restart_offer`]),
    /// so a hold that began while a version was still coming back — or
    /// on a box since emptied — does nothing.
    pub(super) fn restart_now(&mut self) {
        if !self.git_path_offers_restart {
            return;
        }
        tracing::info!(path = %self.git_path, "a different git was applied; starting again");
        self.restart_wanted = true;
        self.settle_restart_offer();
        Hub::with(Hub::want_restart);
        self.git_path_changed();
    }

    pub(super) fn write_window(&self, x: i32, y: i32, width: i32, height: i32, maximized: bool) {
        Hub::with(|hub| {
            let previous = hub.state().window;
            hub.set_window_state(platitude_core::settings::WindowState {
                // A maximized window reports the size of the screen. Keeping
                // the last unmaximized one is what lets restoring down go
                // back to the window it was.
                x: if maximized { previous.x } else { Some(x) },
                y: if maximized { previous.y } else { Some(y) },
                width: if maximized { previous.width } else { width },
                height: if maximized { previous.height } else { height },
                maximized,
            });
        });
    }

    pub(super) fn write_layout_sizes(
        &self,
        sidebar_width: i32,
        details_width: i32,
        commands_height: i32,
        graph_labels_width: i32,
        graph_lanes_width: i32,
    ) {
        Hub::with(|hub| {
            let mut layout = hub.state().layout;
            layout.sidebar_width = sidebar_width;
            layout.details_width = details_width;
            layout.commands_height = commands_height;
            layout.graph_labels_width = graph_labels_width;
            layout.graph_lanes_width = graph_lanes_width;
            hub.set_layout_state(layout);
        });
    }

    /// The log's own flag stays in memory: it is held here for the
    /// length of the run, so the tab arriving finds what the tab leaving
    /// was showing and the next launch finds nothing
    /// ([`AppBackend::commands_shown`]).
    pub(super) fn write_layout_flags(
        &mut self,
        sidebar_collapsed: bool,
        commands_shown: bool,
        tags_shown: bool,
        wip_tree: bool,
        details_tree: bool,
    ) {
        self.commands_shown = commands_shown;
        Hub::with(|hub| {
            let mut layout = hub.state().layout;
            layout.sidebar_collapsed = sidebar_collapsed;
            layout.tags_shown = tags_shown;
            layout.wip_tree = wip_tree;
            layout.details_tree = details_tree;
            hub.set_layout_state(layout);
        });
    }

    pub(super) fn write_sections(
        &self,
        branches: bool,
        remotes: bool,
        worktree: bool,
        stashes: bool,
        tags: bool,
    ) {
        Hub::with(|hub| {
            let mut layout = hub.state().layout;
            layout.sections = platitude_core::settings::Sections {
                branches,
                remotes,
                worktree,
                stashes,
                tags,
            };
            hub.set_layout_state(layout);
        });
    }
}
