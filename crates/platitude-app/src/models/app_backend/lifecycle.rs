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

    /// Asks one candidate git for its version, without touching the git
    /// this run is already on (`version::probe` builds its own executor).
    pub(super) fn begin_git_path_check(&mut self, path: String) {
        self.git_path_state = "checking".into();
        self.git_path_version.clear();
        self.git_path_error.clear();
        // Withdraw the offer until this path answers: a kept offer could be
        // held over a path nobody has asked.
        self.settle_restart_offer();
        self.git_path_changed();
        let feed = Arc::clone(&self.check_feed);
        // An empty box means the git `PATH` resolves.
        let wanted = if path.is_empty() {
            self.git_path_on_path.clone()
        } else {
            path.clone()
        };
        let in_use = self.git_path_in_use.clone();
        let spawned = Hub::with(|hub| {
            let Some(handle) = hub.runtime_handle() else {
                return false;
            };
            handle.spawn(async move {
                let cancel = tokio_util::sync::CancellationToken::new();
                let probe = version::probe(&path, &cancel).await;
                // Blocking: the compare reads the filesystem, and a path on
                // a server that is not answering blocks as long as it does.
                let names_the_run = tokio::task::spawn_blocking(move || {
                    platitude_core::process::same_program(
                        std::path::Path::new(&wanted),
                        std::path::Path::new(&in_use),
                    )
                })
                .await
                .unwrap_or(false);
                feed.push(AppMsg::GitPathProbed {
                    path,
                    probe,
                    names_the_run,
                });
            });
            true
        })
        .unwrap_or(false);
        if !spawned {
            self.git_path_state = "failed".into();
            self.git_path_error = "internal: runtime unavailable".into();
            self.git_path_changed();
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
        // The marks describe the save that is starting.
        self.identity_name_saved = false;
        self.identity_email_saved = false;
        self.identity_unsaved = false;
        self.identity_changed();
        let feed = Arc::clone(&self.check_feed);
        // Held by the hub, so the window cannot close between the pair's
        // two `git config` writes (`hub::saves`).
        let spawned = Hub::with(|hub| {
            // Not the read executor: its timeout would kill a slow
            // `git config` and leave one half written (`Hub::save_executor`).
            let executor = hub.save_executor();
            hub.spawn_save(async move {
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
                feed.push(AppMsg::IdentitySaved(written.map_err(|e| e.to_string())));
            })
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
        let mut probed = false;
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
                    self.read_identity(name, email);
                }
                AppMsg::IdentityUnknown { message } => {
                    // Not "missing": git could not answer, so the setup
                    // screen stays away.
                    tracing::warn!(error = %message, "could not read the author identity");
                    self.identity_state = "error".into();
                    self.identity_error = message;
                }
                AppMsg::GitPathProbed {
                    path,
                    probe,
                    names_the_run,
                } => {
                    // Stale: the box moved on, and its own answer is coming.
                    if path != self.git_path {
                        continue;
                    }
                    probed = true;
                    self.git_path_names_the_run = names_the_run;
                    self.git_path_version = probe.version().to_string();
                    self.git_path_error.clear();
                    self.git_path_state = match probe {
                        version::Probe::Supported(_) => "ok",
                        version::Probe::Old(_) => "old",
                        version::Probe::Missing => "missing",
                        version::Probe::Failed { message } => {
                            self.git_path_error = message;
                            "failed"
                        }
                    }
                    .into();
                    // The answer is what offers the restart, an old git's
                    // too (デザイン規約 §設定の画面).
                    self.settle_restart_offer();
                }
                AppMsg::IdentitySaved(written) => {
                    self.finish_identity(written);
                    // Even a half-landed write changed the configuration.
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
        if probed {
            self.git_path_changed();
        }
        self.git_state_changed();
        self.identity_changed();
    }
}
