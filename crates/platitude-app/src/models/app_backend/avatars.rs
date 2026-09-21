//! Pictures put against authors: what the settings screen's `AVATARS`
//! chapter and the details pane's badge ask of the singleton.
//!
//! The slots themselves cannot leave the `#[qobject]` block (one type,
//! one file — `structure.md` §分割 Qt), so what left is their bodies.
//! Everything here goes through the hub, which owns the store.

use super::*;

impl AppBackend {
    /// Files a picture against an address. `file_url` comes from the
    /// picker, so it arrives as a URL.
    ///
    /// Writes at once. There is no Save on the card this is reached from
    /// and no Cancel to undo it — the gesture that takes one away is a
    /// hold (デザイン規約 §長押し).
    pub(super) fn file_avatar(&mut self, email: &str, name: &str, file_url: &str) {
        let source = crate::urlpath::file_url_to_path(file_url);
        let refused = Hub::with(|hub| hub.assign_avatar(email, name, &source)).flatten();
        // The three halves the card's line is written from: which refusal
        // it was, the numbers its sentence takes, and the operating
        // system's own words where the failure is one it made. The words
        // themselves are the screen's (`Words.avatarFailure`) —
        // app-ui.md「Rust に文言を置かない」.
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

    /// The picture for an address, or empty. For the one place that asks
    /// about a person it is not already showing: the settings card, when
    /// an avatar's own badge opened it.
    pub(super) fn avatar_url(&self, email: &str) -> String {
        Hub::with(|hub| hub.avatar_url(email)).unwrap_or_default()
    }

    fn reload_avatars(&mut self) {
        self.avatars = assignments();
        self.avatars_changed();
    }
}
