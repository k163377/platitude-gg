//! Settings, window state and avatars — the half of the hub that
//! reads and writes the store.

use super::*;

/// Addresses that have a picture, and its URL. Taken once per row-building
/// pass, so the hub is borrowed once rather than per row.
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

    /// Records the auto-fetch interval, puts it in force on every open tab
    /// and saves. `minutes` has been through `session::auto_fetch_minutes`
    /// (the ceiling).
    pub fn set_auto_fetch_minutes(&mut self, minutes: u32) {
        self.settings.defaults.auto_fetch_minutes = minutes;
        self.reapply_settings();
        if let Err(error) = self.store.save_settings(&self.settings) {
            tracing::warn!(%error, "settings not saved");
        }
    }

    /// Records how much history a graph opens with and puts it in force on
    /// every open tab, restarting their walks (`RepoSession::set_log_limit`).
    /// `None` is the whole history. `commits` has been through
    /// `session::log_limit` (the floor).
    pub fn set_initial_commits(&mut self, commits: Option<u32>) {
        self.settings.defaults.initial_commits = commits;
        self.reapply_settings();
        if let Err(error) = self.store.save_settings(&self.settings) {
            tracing::warn!(%error, "settings not saved");
        }
    }

    /// Records how many git processes run at once, puts it in force on the
    /// slots every session shares (`platitude_core::process::Slots`) and
    /// saves. `concurrency` has been through `process::concurrency` (the
    /// range).
    pub fn set_git_concurrency(&mut self, concurrency: u32) {
        self.settings.defaults.git_concurrency = concurrency;
        self.executor
            .slots()
            .set_limits(platitude_core::process::Limits::of(concurrency));
        self.save_settings_now();
    }

    /// Records how often other working copies are read for uncommitted
    /// work and saves; sessions take only whether they are read at all
    /// (`apply_repo_settings`). `secs` has been through
    /// `session::copies_interval_secs` (the range).
    pub fn set_copies_interval_secs(&mut self, secs: u32) {
        self.settings.defaults.copies_interval_secs = secs;
        self.reapply_settings();
        self.save_settings_now();
    }

    /// Records which git to run; empty is `PATH`'s. Saved, not put in
    /// force: swapping the executor mid-run would leave open tabs on the
    /// old binary and new ones on the new. The next start reads it
    /// (`resolve_git`).
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

    /// Every assignment resolved to a URL (see [`AvatarUrls`]).
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
    /// Answers with the refusal the screen is written from, or `None`
    /// where it worked (`avatar::AvatarRefusal` —
    /// rules-refs/app-ui.md「Rust に文言を置かない」).
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
            // The error's words go to the log; the screen is written from
            // the kind.
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
            // Left unsaved: the next tick retries, and the file on disk
            // stays the last good one.
            Err(error) => tracing::warn!(%error, "state not saved"),
        }
    }
}
