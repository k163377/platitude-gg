//! Settings, window state and avatars — the half of the hub that
//! reads and writes the store.

use super::*;

/// Addresses that have a picture, and where it is. Held by whoever is
/// building rows so the hub is borrowed once for a whole pass rather than
/// once per commit — the graph draws two thousand of those at a time.
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

    /// Records the auto-fetch interval and puts it in force. Written out
    /// at once rather than on the state timer.
    pub fn set_auto_fetch_minutes(&mut self, minutes: u32) {
        self.settings.defaults.auto_fetch_minutes = minutes;
        self.reapply_settings();
        if let Err(error) = self.store.save_settings(&self.settings) {
            tracing::warn!(%error, "settings not saved");
        }
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
    /// graph asks the hub once instead of once per row.
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
    /// Reports git-style: the error is the message, empty means it worked.
    pub fn assign_avatar(&mut self, email: &str, name: &str, source: &std::path::Path) -> String {
        let Some(dir) = self.store.avatars_dir() else {
            return platitude_core::avatar::AvatarError::NoStore.to_string();
        };
        if let Err(error) = self.settings.avatars.assign(&dir, email, name, source) {
            return error.to_string();
        }
        self.save_settings_now();
        String::new()
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
