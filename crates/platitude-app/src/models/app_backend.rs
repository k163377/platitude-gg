use std::sync::Arc;

use platitude_core::version;
use qtbridge::{QObjectHolder, qobject};

use crate::hub::{Feed, Hub};

use super::qml_register;

// ---------------------------------------------------------------------------
// AppBackend (singleton): git version gate + automation hooks
// ---------------------------------------------------------------------------

enum AppMsg {
    GitOk {
        version: String,
    },
    GitMissing {
        message: String,
    },
    GitUnsupported {
        message: String,
    },
    GitError {
        message: String,
    },
    /// Identity git would put on a new commit here.
    Identity {
        name: String,
        email: String,
    },
    /// The identity could not be read at all (not the same as unset).
    IdentityUnknown {
        message: String,
    },
    /// An identity write finished; `error` carries git's own message.
    IdentitySaved {
        error: Option<String>,
    },
}

pub struct AppBackend {
    git_state: String,
    git_version: String,
    git_error: String,
    /// "unknown" until the check runs, then "checking" / "missing" /
    /// "ready" / "error". Only "missing" opens the setup screen.
    identity_state: String,
    identity_name: String,
    identity_email: String,
    identity_error: String,
    identity_busy: bool,
    auto_open: String,
    shot_dir: String,
    auto_quit_ms: i32,
    auto_select: bool,
    auto_scroll: bool,
    auto_wip: bool,
    /// Screenshot hook: `"<name>|<email>"` prefills the identity screen.
    auto_identity: String,
    /// Screenshot hook: submit that prefilled identity straight away.
    auto_identity_save: bool,
    scroll_to: String,
    /// Smoke hook: one write operation to run once the repository is
    /// loaded, and its argument. A bare verb rather than a script, so QML
    /// dispatches on equality and parses nothing.
    auto_act: String,
    auto_act_arg: String,
    /// Auto-fetch interval in minutes; 0 is off. Application-wide, and not
    /// persisted yet — settings storage is Phase 4, so this starts at the
    /// default every launch.
    auto_fetch_minutes: i32,
    /// Ceiling the settings input enforces.
    auto_fetch_max: i32,
    check_feed: Arc<Feed<AppMsg>>,
}

/// Directory the identity check runs in. git resolves configuration from a
/// directory, and outside a repository that is exactly the user's own
/// (global + system) configuration — what a first-run prompt is about.
fn app_workdir() -> std::path::PathBuf {
    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
}

impl Default for AppBackend {
    fn default() -> Self {
        Self {
            git_state: "checking".into(),
            git_version: String::new(),
            git_error: String::new(),
            identity_state: "unknown".into(),
            identity_name: String::new(),
            identity_email: String::new(),
            identity_error: String::new(),
            identity_busy: false,
            auto_open: std::env::var("PG_AUTO_OPEN").unwrap_or_default(),
            shot_dir: std::env::var("PG_SHOT_DIR")
                .unwrap_or_default()
                .replace('\\', "/"),
            auto_quit_ms: std::env::var("PG_AUTO_QUIT_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            auto_select: std::env::var("PG_AUTO_SELECT").as_deref() == Ok("1"),
            auto_scroll: std::env::var("PG_AUTO_SCROLL").as_deref() == Ok("1"),
            auto_wip: std::env::var("PG_AUTO_WIP").as_deref() == Ok("1"),
            auto_identity: std::env::var("PG_AUTO_IDENTITY").unwrap_or_default(),
            auto_identity_save: std::env::var("PG_AUTO_IDENTITY_SAVE").as_deref() == Ok("1"),
            // Smoke-test hook: "top" / "bottom" jumps the graph after load.
            scroll_to: std::env::var("PG_SCROLL_TO").unwrap_or_default(),
            auto_act: std::env::var("PG_AUTO_ACT").unwrap_or_default(),
            auto_act_arg: std::env::var("PG_AUTO_ACT_ARG").unwrap_or_default(),
            auto_fetch_minutes: platitude_core::session::AUTO_FETCH_DEFAULT_MINUTES as i32,
            auto_fetch_max: platitude_core::session::AUTO_FETCH_MAX_MINUTES as i32,
            check_feed: Arc::new(Feed::default()),
        }
    }
}

#[qobject(ConvertToCamelCase, NoQmlElement)]
impl AppBackend {
    qproperty!("gitState", Member = git_state, Notify = git_state_changed);
    qproperty!(
        "gitVersion",
        Member = git_version,
        Notify = git_state_changed
    );
    qproperty!("gitError", Member = git_error, Notify = git_state_changed);
    qproperty!(
        "identityState",
        Member = identity_state,
        Notify = identity_changed
    );
    qproperty!(
        "identityName",
        Member = identity_name,
        Notify = identity_changed
    );
    qproperty!(
        "identityEmail",
        Member = identity_email,
        Notify = identity_changed
    );
    qproperty!(
        "identityError",
        Member = identity_error,
        Notify = identity_changed
    );
    qproperty!(
        "identityBusy",
        Member = identity_busy,
        Notify = identity_changed
    );
    qproperty!("autoIdentity", Member = auto_identity, Constant);
    qproperty!("autoIdentitySave", Member = auto_identity_save, Constant);
    qproperty!("autoOpen", Member = auto_open, Constant);
    qproperty!("shotDir", Member = shot_dir, Constant);
    qproperty!("autoQuitMs", Member = auto_quit_ms, Constant);
    qproperty!("autoSelect", Member = auto_select, Constant);
    qproperty!("autoScroll", Member = auto_scroll, Constant);
    qproperty!("autoWip", Member = auto_wip, Constant);
    qproperty!("scrollTo", Member = scroll_to, Constant);
    qproperty!("autoAct", Member = auto_act, Constant);
    qproperty!("autoActArg", Member = auto_act_arg, Constant);
    qproperty!(
        "autoFetchMinutes",
        Member = auto_fetch_minutes,
        Notify = settings_changed
    );
    qproperty!("autoFetchMaxMinutes", Member = auto_fetch_max, Constant);

    #[qsignal]
    fn git_state_changed(&mut self);

    #[qsignal]
    fn identity_changed(&mut self);

    #[qsignal]
    fn settings_changed(&mut self);

    /// Sets how often every open repository fetches, in minutes. Zero (the
    /// blank input) turns it off; anything above the ceiling is clamped,
    /// because past an hour the automatic fetch has no point left.
    #[qslot]
    fn set_auto_fetch_minutes(&mut self, minutes: i32) {
        let max = platitude_core::session::AUTO_FETCH_MAX_MINUTES as i32;
        let minutes = minutes.clamp(0, max);
        if self.auto_fetch_minutes == minutes {
            return;
        }
        self.auto_fetch_minutes = minutes;
        let interval = (minutes > 0).then(|| std::time::Duration::from_secs(minutes as u64 * 60));
        Hub::with(|hub| hub.set_auto_fetch(interval));
        self.settings_changed();
    }

    /// Benchmark/automation reporting channel (QML → tracing).
    #[qslot]
    fn report(&self, message: String) {
        tracing::info!(target: "bench", "{message}");
    }

    /// Starts the git version check (call once from QML on startup).
    #[qslot]
    fn initialize(&mut self) {
        self.check_feed.attach(self.get_qml_method_invoker());
        let feed = Arc::clone(&self.check_feed);
        let spawned = Hub::with(|hub| {
            let Some(handle) = hub.runtime_handle() else {
                return false;
            };
            let executor = hub.executor();
            handle.spawn(async move {
                let cancel = tokio_util::sync::CancellationToken::new();
                let msg = match version::ensure_supported(&executor, &cancel).await {
                    Ok(v) => AppMsg::GitOk { version: v.raw },
                    Err(e @ platitude_core::GitError::GitNotFound { .. }) => AppMsg::GitMissing {
                        message: e.to_string(),
                    },
                    Err(e @ platitude_core::GitError::UnsupportedVersion { .. }) => {
                        AppMsg::GitUnsupported {
                            message: e.to_string(),
                        }
                    }
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

    /// Reads the identity git would record on a commit. Runs once the
    /// version gate has passed — with no usable git there is nothing to ask
    /// about, and nothing to ask it with.
    fn start_identity_check(&mut self) {
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

    /// Records `user.name` / `user.email` in the user's own configuration.
    ///
    /// Global is deliberate: the answer is about the person, not one
    /// project. Values are validated by the core (a line break would turn
    /// the rest of the config file into another setting), and git's own
    /// message comes back as `identityError`.
    #[qslot]
    fn save_identity(&mut self, name: String, email: String) {
        if self.identity_busy {
            return;
        }
        self.identity_busy = true;
        self.identity_error = String::new();
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
                    Ok(()) => {
                        // Read back rather than echo: a repository-local
                        // setting can still override what was just written.
                        match identity::load(&executor, &workdir, &cancel).await {
                            Ok(config) => feed.push(AppMsg::Identity {
                                name: config.identity.name.unwrap_or_default(),
                                email: config.identity.email.unwrap_or_default(),
                            }),
                            Err(e) => feed.push(AppMsg::IdentityUnknown {
                                message: e.to_string(),
                            }),
                        }
                        feed.push(AppMsg::IdentitySaved { error: None });
                    }
                    Err(e) => feed.push(AppMsg::IdentitySaved {
                        error: Some(e.to_string()),
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

    #[qslot]
    fn drain(&mut self) {
        let mut check_identity = false;
        let mut saved = false;
        for msg in self.check_feed.drain() {
            match msg {
                AppMsg::GitOk { version } => {
                    self.git_state = "ok".into();
                    self.git_version = version;
                    check_identity = true;
                }
                AppMsg::GitMissing { message } => {
                    self.git_state = "missing".into();
                    self.git_error = message;
                }
                AppMsg::GitUnsupported { message } => {
                    self.git_state = "unsupported".into();
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
                AppMsg::IdentitySaved { error } => {
                    self.identity_busy = false;
                    match error {
                        Some(message) => self.identity_error = message,
                        None => saved = true,
                    }
                }
            }
        }
        if check_identity {
            self.start_identity_check();
        }
        if saved {
            // Open repositories hold their own copy of the configuration.
            Hub::with(|hub| hub.refresh_authors());
        }
        self.git_state_changed();
        self.identity_changed();
    }
}
qml_register!(AppBackend, "AppBackend", singleton = true);
