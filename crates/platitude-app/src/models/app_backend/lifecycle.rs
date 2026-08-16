//! Starting the application up, and draining what answers.

use super::*;

impl AppBackend {
    pub(super) fn start_up(&mut self) {
        self.check_feed.attach(self.get_qml_method_invoker());
        let feed = Arc::clone(&self.check_feed);
        let spawned = Hub::with(|hub| {
            let Some(handle) = hub.runtime_handle() else {
                return false;
            };
            let executor = hub.executor();
            handle.spawn(async move {
                let cancel = tokio_util::sync::CancellationToken::new();
                let msg = match version::detect(&executor, &cancel).await {
                    Ok(v) => AppMsg::GitOk {
                        supported: v.supported(),
                        version: v.raw,
                    },
                    Err(e @ platitude_core::GitError::GitNotFound { .. }) => AppMsg::GitMissing {
                        message: e.to_string(),
                    },
                    Err(e) => AppMsg::GitError {
                        message: e.to_string(),
                    },
                };
                feed.push(msg);
            });
            true
        })
        .unwrap_or(false);
        if !spawned {
            self.git_state = "error".into();
            self.git_error = "internal: runtime unavailable".into();
            self.git_state_changed();
        }
    }

    pub(super) fn begin_identity_check(&mut self) {
        self.identity_state = "checking".into();
        let feed = Arc::clone(&self.check_feed);
        Hub::with(|hub| {
            let Some(handle) = hub.runtime_handle() else {
                return;
            };
            let executor = hub.executor();
            handle.spawn(async move {
                let cancel = tokio_util::sync::CancellationToken::new();
                let msg = match platitude_core::identity::load(&executor, &app_workdir(), &cancel)
                    .await
                {
                    Ok(config) => AppMsg::Identity {
                        name: config.identity.name.unwrap_or_default(),
                        email: config.identity.email.unwrap_or_default(),
                    },
                    Err(e) => AppMsg::IdentityUnknown {
                        message: e.to_string(),
                    },
                };
                feed.push(msg);
            });
        });
    }

    pub(super) fn write_identity(&mut self, name: String, email: String) {
        if self.identity_busy {
            return;
        }
        self.identity_busy = true;
        self.identity_error = String::new();
        // The marks describe the save that is starting, not the last one.
        self.identity_name_saved = false;
        self.identity_email_saved = false;
        self.identity_unsaved = false;
        self.identity_changed();
        let feed = Arc::clone(&self.check_feed);
        let spawned = Hub::with(|hub| {
            let Some(handle) = hub.runtime_handle() else {
                return false;
            };
            let executor = hub.executor();
            handle.spawn(async move {
                use platitude_core::identity::{self, ConfigScope};
                let cancel = tokio_util::sync::CancellationToken::new();
                let workdir = app_workdir();
                let written = identity::set_identity(
                    &executor,
                    &workdir,
                    &name,
                    &email,
                    ConfigScope::Global,
                    &cancel,
                )
                .await;
                match written {
                    Ok(written) => {
                        // What git answers, not what was typed: the write
                        // reads itself back, so a half that did not land
                        // and a repository-local setting sitting over the
                        // global one both show here.
                        let (name_saved, email_saved) = (written.name_saved, written.email_saved);
                        let message = written.message;
                        feed.push(AppMsg::Identity {
                            name: written.identity.name.unwrap_or_default(),
                            email: written.identity.email.unwrap_or_default(),
                        });
                        feed.push(AppMsg::IdentitySaved {
                            // git's own message, and only git's: a write
                            // that failed nowhere and still did not take
                            // is explained on screen, where it can be
                            // translated.
                            error: (!message.is_empty()).then_some(message),
                            name_saved,
                            email_saved,
                        });
                    }
                    Err(e) => feed.push(AppMsg::IdentitySaved {
                        error: Some(e.to_string()),
                        name_saved: false,
                        email_saved: false,
                    }),
                }
            });
            true
        })
        .unwrap_or(false);
        if !spawned {
            self.identity_busy = false;
            self.identity_error = "internal: runtime unavailable".into();
            self.identity_changed();
        }
    }

    pub(super) fn take_feed(&mut self) {
        let mut check_identity = false;
        let mut wrote = false;
        for msg in self.check_feed.drain() {
            match msg {
                AppMsg::GitOk { version, supported } => {
                    self.git_state = "ok".into();
                    self.git_version = version;
                    self.git_unsupported = !supported;
                    check_identity = true;
                }
                AppMsg::GitMissing { message } => {
                    self.git_state = "missing".into();
                    self.git_error = message;
                }
                AppMsg::GitError { message } => {
                    self.git_state = "error".into();
                    self.git_error = message;
                }
                AppMsg::Identity { name, email } => {
                    // Both halves are required; git refuses to commit with
                    // either one missing.
                    self.identity_state = if name.is_empty() || email.is_empty() {
                        "missing"
                    } else {
                        "ready"
                    }
                    .into();
                    self.identity_name = name;
                    self.identity_email = email;
                }
                AppMsg::IdentityUnknown { message } => {
                    // Not the same as unset: git could not answer, so the
                    // setup screen stays out of the way.
                    tracing::warn!(error = %message, "could not read the author identity");
                    self.identity_state = "error".into();
                    self.identity_error = message;
                }
                AppMsg::IdentitySaved {
                    error,
                    name_saved,
                    email_saved,
                } => {
                    self.identity_busy = false;
                    self.identity_name_saved = name_saved;
                    self.identity_email_saved = email_saved;
                    self.identity_unsaved = !(name_saved && email_saved);
                    if let Some(message) = error {
                        self.identity_error = message;
                    }
                    // A write that only half landed still changed the
                    // configuration, so the open repositories re-read it
                    // whichever way this one went.
                    wrote = true;
                }
            }
        }
        if check_identity {
            self.start_identity_check();
        }
        if wrote {
            // Open repositories hold their own copy of the configuration.
            Hub::with(|hub| hub.refresh_authors());
        }
        self.git_state_changed();
        self.identity_changed();
    }
}
