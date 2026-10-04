//! Writing the window, its layout and the live defaults back to the
//! settings store.

use super::*;

impl AppBackend {
    /// Sets how often every open repository fetches, in minutes; zero (the
    /// blank input) is off. The ceiling is core's
    /// (`session::auto_fetch_minutes`): a hand-written `settings.toml`
    /// sets the same field, so a limit here would be a second answer.
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

    /// Sets how much history every graph opens with; zero is the whole of
    /// it. The floor is core's (`session::log_limit`), as above.
    pub(super) fn apply_initial_commits(&mut self, commits: i32) {
        let asked = match commits.max(0).unsigned_abs() {
            0 => None,
            count => Some(platitude_core::session::log_limit(count)),
        };
        // Saturating: `settings.toml` can hold a count past `i32`.
        let commits = asked.map_or(0, |count| i32::try_from(count).unwrap_or(i32::MAX));
        if self.initial_commits == commits {
            return;
        }
        self.initial_commits = commits;
        Hub::with(|hub| hub.set_initial_commits(asked));
        self.settings_changed();
    }

    /// Sets how many git processes the application runs at once. The
    /// range is core's (`process::concurrency`), as above. Zero (the blank
    /// input) is the default: no git at all is not something to ask for.
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
    /// work, in seconds; zero (the blank input) is off. The range is core's
    /// (`session::copies_interval_secs`), as above.
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

    /// The settings screen has just opened: asks the path it shows for its
    /// version. On every open, because the file on disk can change without
    /// the path being retyped.
    pub(super) fn begin_git_path_screen(&mut self) {
        let path = self.git_path.clone();
        self.begin_git_path_check(path);
    }

    /// Records which git this computer runs, and asks that one for its
    /// version.
    ///
    /// Only a value that moved is asked again: re-asking withdraws the
    /// standing restart offer until the answer returns
    /// ([`Self::settle_restart_offer`]). Nor is one that did not move
    /// written: the held value is the file's with the screen's separators,
    /// and the way out of the screen writes every box back.
    ///
    /// Trimmed here as well as when read from the file
    /// (`settings::toml::text`), so the box and the file agree, separators
    /// aside, before the first save.
    pub(super) fn apply_git_path(&mut self, path: &str) {
        let path = path.trim();
        if self.git_path == path {
            return;
        }
        self.git_path = path.to_string();
        Hub::with(|hub| hub.set_git_path(path.to_string()));
        self.begin_git_path_check(path.to_string());
    }

    /// Works out whether there is a restart to offer: the box holds a git
    /// this run is not on, and that git answered. Called wherever either
    /// moves.
    pub(super) fn settle_restart_offer(&mut self) {
        self.git_path_offers_restart = !self.restart_wanted
            && !self.git_path_names_the_run
            && matches!(self.git_path_state.as_str(), "ok" | "old");
    }

    /// The button was held: the window is to close and come back on the
    /// box's git. Guarded by the offer the button is drawn from
    /// ([`Self::settle_restart_offer`]), so a hold that began before the
    /// answer, or on a box since emptied, does nothing.
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
                // A maximized window reports the screen's size; keep the
                // last normal geometry so restoring goes back to it.
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

    /// The log's flag stays in memory ([`AppBackend::commands_shown`]).
    pub(super) fn write_layout_flags(
        &mut self,
        sidebar_collapsed: bool,
        commands_shown: bool,
        tags_shown: bool,
        wip_tree: bool,
        details_tree: bool,
        diff_split: bool,
    ) {
        self.commands_shown = commands_shown;
        Hub::with(|hub| {
            let mut layout = hub.state().layout;
            layout.sidebar_collapsed = sidebar_collapsed;
            layout.tags_shown = tags_shown;
            layout.wip_tree = wip_tree;
            layout.details_tree = details_tree;
            layout.diff_split = diff_split;
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
