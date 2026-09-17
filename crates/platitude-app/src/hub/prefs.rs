//! Settings, window state and avatars — the half of the hub that
//! reads and writes the store.

use super::*;

/// Addresses that have a picture, and where it is. Held by whoever is
/// building rows so the hub is borrowed once for a whole pass — the
/// graph draws two thousand commits at a time.
#[derive(Debug, Clone, Default)]
pub struct AvatarUrls {
    by_email: HashMap<String, String>,
}

impl AvatarUrls {
    /// The current assignments. Empty where there is nowhere to keep
    /// pictures.
    pub fn current() -> Self {
        Hub::with(|hub| hub.avatar_urls()).unwrap_or_default()
    }

    /// The URL for an address, or empty. The address is expected to have
    /// come through `avatar::key` already.
    pub fn url_of(&self, email: &str) -> String {
        self.by_email.get(email).cloned().unwrap_or_default()
    }
}

impl Hub {
    // -- settings and state -------------------------------------------------

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    /// The directory another process is holding, empty when this one got
    /// what it asked for.
    pub fn held_elsewhere(&self) -> &str {
        &self.held_elsewhere
    }

    /// Records the auto-fetch interval and puts it in force on every open
    /// tab. Written out at once.
    ///
    /// Takes a number that has already been through
    /// `session::auto_fetch_minutes`, which is where the ceiling lives.
    pub fn set_auto_fetch_minutes(&mut self, minutes: u32) {
        self.settings.defaults.auto_fetch_minutes = minutes;
        self.reapply_settings();
        if let Err(error) = self.store.save_settings(&self.settings) {
            tracing::warn!(%error, "settings not saved");
        }
    }

    /// Records how much history a graph opens with and puts it in force on
    /// every open tab, which restarts each of their walks
    /// (`RepoSession::set_log_limit`). `None` is the whole history.
    ///
    /// Takes a count that has already been through `session::log_limit`,
    /// which is where the floor lives.
    pub fn set_initial_commits(&mut self, commits: Option<u32>) {
        self.settings.defaults.initial_commits = commits;
        self.reapply_settings();
        if let Err(error) = self.store.save_settings(&self.settings) {
            tracing::warn!(%error, "settings not saved");
        }
    }

    /// Records how many git processes the application runs at once and
    /// puts it in force — on the one set of slots every session shares,
    /// so nothing has to be reapplied per tab
    /// (`platitude_core::process::Slots`). Written out at once.
    ///
    /// Takes a number that has already been through
    /// `process::concurrency`, which is where the range lives.
    pub fn set_git_concurrency(&mut self, concurrency: u32) {
        self.settings.defaults.git_concurrency = concurrency;
        self.executor
            .slots()
            .set_limits(platitude_core::process::Limits::of(concurrency));
        self.save_settings_now();
    }

    /// Records how often the other working copies of a repository are
    /// read for uncommitted work, and puts the one half of it the
    /// sessions hold in force — whether they are read at all
    /// (`RepoSession::set_copies_read`); the interval itself is the
    /// page's tick's to read off `AppBackend` (`RepoPage`). Written out
    /// at once.
    ///
    /// Takes a number that has already been through
    /// `session::copies_interval_secs`, which is where the range lives.
    pub fn set_copies_interval_secs(&mut self, secs: u32) {
        self.settings.defaults.copies_interval_secs = secs;
        self.reapply_settings();
        self.save_settings_now();
    }

    /// Records which git this computer runs. Empty is whichever one
    /// `PATH` resolves.
    ///
    /// **Written down.** The executor every session spawns through was
    /// settled at install (`resolve_git`), and a run that swapped it
    /// mid-flight would leave the tabs already open on the old binary and
    /// the next ones on the new — one repository, two gits, and no way to
    /// tell from the window which of them answered. The next start reads
    /// this file and is on one binary throughout.
    pub fn set_git_path(&mut self, path: String) {
        if self.settings.defaults.git_path == path {
            return;
        }
        self.settings.defaults.git_path = path;
        self.save_settings_now();
    }

    // -- avatars ------------------------------------------------------------

    /// A `file:` URL for the picture assigned to an address, or empty —
    /// no picture (the identicon draws then), or nowhere to keep pictures
    /// (a screenshot run).
    pub fn avatar_url(&self, email: &str) -> String {
        let (Some(dir), Some(file)) = (
            self.store.avatars_dir(),
            self.settings.avatars.file_of(email),
        ) else {
            return String::new();
        };
        crate::urlpath::file_url(&dir.join(file))
    }

    pub fn avatars(&self) -> &platitude_core::avatar::Avatars {
        &self.settings.avatars
    }

    /// Every assignment resolved to a URL in one go, so a pass over the
    /// graph asks the hub once for all of its rows.
    pub fn avatar_urls(&self) -> AvatarUrls {
        let Some(dir) = self.store.avatars_dir() else {
            return AvatarUrls::default();
        };
        AvatarUrls {
            by_email: self
                .settings
                .avatars
                .list()
                .iter()
                .map(|entry| {
                    (
                        entry.email.clone(),
                        crate::urlpath::file_url(&dir.join(&entry.file)),
                    )
                })
                .collect(),
        }
    }

    /// Files a picture against an address and writes the settings out.
    ///
    /// Answers with the refusal as the screen is written from it, or
    /// `None` where it worked (`avatar::AvatarRefusal` —
    /// app-ui.md「Rust に文言を置かない」).
    pub fn assign_avatar(
        &mut self,
        email: &str,
        name: &str,
        source: &std::path::Path,
    ) -> Option<platitude_core::avatar::AvatarRefusal> {
        let Some(dir) = self.store.avatars_dir() else {
            return Some(platitude_core::avatar::AvatarError::NoStore.refusal());
        };
        if let Err(error) = self.settings.avatars.assign(&dir, email, name, source) {
            // git's own words for what happened go to the log, where
            // every other failure's do; the screen is written from the
            // kind beside them.
            tracing::warn!(%error, "avatar not assigned");
            return Some(error.refusal());
        }
        self.save_settings_now();
        None
    }

    /// Takes the picture off an address. The person's own file is untouched.
    pub fn remove_avatar(&mut self, email: &str) {
        let Some(dir) = self.store.avatars_dir() else {
            return;
        };
        if self.settings.avatars.remove(&dir, email) {
            self.save_settings_now();
        }
    }

    /// Drops assignments whose image is no longer there. Runs once at
    /// startup.
    pub fn forget_missing_avatars(&mut self) {
        let Some(dir) = self.store.avatars_dir() else {
            return;
        };
        let gone = self.settings.avatars.forget_missing(&dir);
        if gone > 0 {
            tracing::info!(gone, "avatars whose image is missing");
            self.save_settings_now();
        }
    }

    fn save_settings_now(&mut self) {
        if let Err(error) = self.store.save_settings(&self.settings) {
            tracing::warn!(%error, "settings not saved");
        }
    }

    pub fn set_window_state(&mut self, window: platitude_core::settings::WindowState) {
        self.state.window = window;
    }

    pub fn set_layout_state(&mut self, layout: platitude_core::settings::LayoutState) {
        self.state.layout = layout;
    }

    pub fn set_tabs_state(&mut self, tabs: platitude_core::settings::TabsState) {
        self.state.tabs = tabs;
    }

    /// Writes the state out if it has moved since the last write.
    pub fn flush_state(&mut self) {
        if self.state == self.saved_state {
            return;
        }
        match self.store.save_state(&self.state) {
            Ok(()) => self.saved_state = self.state.clone(),
            // Left unsaved on purpose: the next tick tries again, and until
            // one succeeds the file on disk is still the last good one.
            Err(error) => tracing::warn!(%error, "state not saved"),
        }
    }
}
