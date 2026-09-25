//! Pictures put against authors: the bodies of the slots the settings
//! screen's `AVATARS` chapter and the details pane's badge call.

use super::*;

impl AppBackend {
    /// Files a picture against an address (`file_url` is the picker's).
    /// Writes at once: the card has no Save, and taking one away is a hold
    /// (デザイン規約 §長押し).
    pub(super) fn file_avatar(&mut self, email: &str, name: &str, file_url: &str) {
        let source = crate::urlpath::file_url_to_path(file_url);
        let refused = Hub::with(|hub| hub.assign_avatar(email, name, &source)).flatten();
        let (kind, facts, said) = match refused {
            Some(refusal) => (refusal.kind.to_string(), refusal.facts, refusal.said),
            None => (String::new(), Vec::new(), String::new()),
        };
        self.avatar_error_kind = kind;
        self.avatar_error_facts = facts;
        self.avatar_error_said = said;
        self.reload_avatars();
    }

    pub(super) fn unfile_avatar(&mut self, email: &str) {
        Hub::with(|hub| hub.remove_avatar(email));
        self.avatar_error_kind.clear();
        self.avatar_error_facts.clear();
        self.avatar_error_said.clear();
        self.reload_avatars();
    }

    /// The picture for an address, or empty — for the settings card opened
    /// from a badge, which is not already showing the person.
    pub(super) fn avatar_url(&self, email: &str) -> String {
        Hub::with(|hub| hub.avatar_url(email)).unwrap_or_default()
    }

    fn reload_avatars(&mut self) {
        self.avatars = assignments();
        self.avatars_changed();
    }
}
