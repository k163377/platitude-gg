//! Applies a complete identity answer before the drain notifies QML.

use platitude_core::identity::IdentityWrite;

use super::AppBackend;

impl AppBackend {
    pub(super) fn read_identity(&mut self, name: String, email: String) {
        self.identity_state = if name.is_empty() || email.is_empty() {
            "missing"
        } else {
            "ready"
        }
        .into();
        self.identity_name = name;
        self.identity_email = email;
    }

    /// The readback cannot wake QML ahead of its save verdict. In
    /// particular, a half-save can read as a complete identity while the
    /// gate must stay open to explain the half that did not land.
    pub(super) fn finish_identity(&mut self, written: Result<IdentityWrite, String>) {
        self.identity_busy = false;
        match written {
            Ok(written) => {
                self.identity_name_saved = written.name_saved;
                self.identity_email_saved = written.email_saved;
                self.identity_unsaved = !written.is_saved();
                self.identity_error = written.message;
                self.read_identity(
                    written.identity.name.unwrap_or_default(),
                    written.identity.email.unwrap_or_default(),
                );
            }
            Err(message) => {
                self.identity_name_saved = false;
                self.identity_email_saved = false;
                self.identity_unsaved = true;
                self.identity_error = message;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hub::Feed;
    use crate::models::app_backend::AppMsg;
    use platitude_core::identity::Identity;

    fn waiting() -> AppBackend {
        let mut app = AppBackend::default();
        app.read_identity(String::new(), "old@example.com".into());
        app.identity_busy = true;
        app
    }

    fn answer(name_saved: bool, email_saved: bool) -> IdentityWrite {
        IdentityWrite {
            identity: Identity {
                name: Some("New name".into()),
                email: Some("old@example.com".into()),
            },
            name_saved,
            email_saved,
            message: String::new(),
        }
    }

    #[test]
    fn the_first_drain_of_a_half_save_keeps_the_identity_gate_wanted() {
        let mut app = waiting();
        let feed = Feed::default();
        feed.push(AppMsg::IdentitySaved(Ok(answer(true, false))));
        // Drain immediately after publication, before the producer can
        // publish anything else. There is no second half to wait for.
        let batch = feed.drain();
        assert_eq!(batch.len(), 1);
        let AppMsg::IdentitySaved(written) = batch.into_iter().next().unwrap() else {
            panic!("the readback arrived without its verdict");
        };
        app.finish_identity(written);
        assert_eq!(app.identity_state, "ready");
        assert!(app.identity_state == "missing" || app.identity_unsaved);
        assert!(app.identity_name_saved);
        assert!(!app.identity_email_saved);
        assert!(!app.identity_busy);
        assert!(feed.drain().is_empty());
    }

    #[test]
    fn a_complete_save_releases_the_gate_with_the_readback() {
        let mut app = waiting();
        app.finish_identity(Ok(answer(true, true)));
        assert_eq!(app.identity_state, "ready");
        assert_eq!(app.identity_name, "New name");
        assert!(!app.identity_unsaved);
        assert!(app.identity_name_saved && app.identity_email_saved);
        assert!(!app.identity_busy);
    }

    #[test]
    fn a_failed_save_preserves_the_known_identity_and_reports_the_error() {
        let mut app = waiting();
        app.finish_identity(Err("could not lock config file".into()));
        assert_eq!(app.identity_state, "missing");
        assert_eq!(app.identity_email, "old@example.com");
        assert!(app.identity_unsaved);
        assert!(!app.identity_name_saved && !app.identity_email_saved);
        assert!(!app.identity_busy);
        assert_eq!(app.identity_error, "could not lock config file");
    }
}
