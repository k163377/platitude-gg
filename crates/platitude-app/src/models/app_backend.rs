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
    /// Smoke hook: one operation to run once the repository is loaded —
    /// a write, or a surface left standing for the overlay shot — and
    /// its argument. A bare verb rather than a script, so QML dispatches
    /// on equality; the argument passes through as the verb needs it.
    auto_act: String,
    auto_act_arg: String,
    /// Review hook: which of the three ways of drawing a conflicted file's
    /// combined diff to use (`markers` / `columns` / `sides`). Temporary —
    /// it exists so the three can be photographed side by side and one
    /// chosen; the loser goes, and so does this.
    conflict_style: String,
    /// Auto-fetch interval in minutes; 0 is off. Application-wide, because
    /// the answer is about how often this computer should talk to remotes.
    auto_fetch_minutes: i32,
    /// Ceiling the settings input enforces.
    auto_fetch_max: i32,
    check_feed: Arc<Feed<AppMsg>>,
}

fn with_window(pick: impl Fn(&platitude_core::settings::WindowState) -> i32) -> i32 {
    let fallback = platitude_core::settings::WindowState::default();
    Hub::with(|hub| pick(&hub.state().window)).unwrap_or_else(|| pick(&fallback))
}

fn with_layout(pick: impl Fn(&platitude_core::settings::LayoutState) -> i32) -> i32 {
    let fallback = platitude_core::settings::LayoutState::default();
    Hub::with(|hub| pick(&hub.state().layout)).unwrap_or_else(|| pick(&fallback))
}

fn with_flag(pick: impl Fn(&platitude_core::settings::LayoutState) -> bool) -> bool {
    let fallback = platitude_core::settings::LayoutState::default();
    Hub::with(|hub| pick(&hub.state().layout)).unwrap_or_else(|| pick(&fallback))
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
            // Smoke-test hook: "top" / "bottom" jumps the graph after
            // load; "nav-bottom" jumps the sidebar's branch list instead.
            scroll_to: std::env::var("PG_SCROLL_TO").unwrap_or_default(),
            auto_act: std::env::var("PG_AUTO_ACT").unwrap_or_default(),
            auto_act_arg: std::env::var("PG_AUTO_ACT_ARG").unwrap_or_default(),
            conflict_style: std::env::var("PG_CONFLICT_STYLE").unwrap_or_default(),
            auto_fetch_minutes: Hub::with(|hub| hub.settings().defaults.auto_fetch_minutes as i32)
                .unwrap_or(platitude_core::session::AUTO_FETCH_DEFAULT_MINUTES as i32),
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
    qproperty!("conflictStyle", Member = conflict_style, Constant);
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
        Hub::with(|hub| hub.set_auto_fetch_minutes(minutes.unsigned_abs()));
        self.settings_changed();
    }

    // -- window state -------------------------------------------------------
    //
    // Read once as a page or the window is built, not bound: these are
    // where something starts, and after that the UI owns the value. They
    // are slots rather than properties for the same reason (規約 §QML
    // バインディングはプロパティにしか反応しない — nothing here needs to
    // react). Reading them off the hub rather than off a copy taken at
    // startup is what makes a tab opened later pick up the layout that is
    // in force now instead of the one the app launched with.

    #[qslot]
    fn start_window_x(&self) -> i32 {
        Hub::with(|hub| hub.state().window.x.unwrap_or(i32::MIN)).unwrap_or(i32::MIN)
    }

    #[qslot]
    fn start_window_y(&self) -> i32 {
        Hub::with(|hub| hub.state().window.y.unwrap_or(i32::MIN)).unwrap_or(i32::MIN)
    }

    #[qslot]
    fn start_window_width(&self) -> i32 {
        with_window(|w| w.width)
    }

    #[qslot]
    fn start_window_height(&self) -> i32 {
        with_window(|w| w.height)
    }

    #[qslot]
    fn start_window_maximized(&self) -> bool {
        Hub::with(|hub| hub.state().window.maximized).unwrap_or(false)
    }

    #[qslot]
    fn start_sidebar_width(&self) -> i32 {
        with_layout(|l| l.sidebar_width)
    }

    #[qslot]
    fn start_sidebar_collapsed(&self) -> bool {
        with_flag(|l| l.sidebar_collapsed)
    }

    #[qslot]
    fn start_details_width(&self) -> i32 {
        with_layout(|l| l.details_width)
    }

    #[qslot]
    fn start_commands_height(&self) -> i32 {
        with_layout(|l| l.commands_height)
    }

    #[qslot]
    fn start_commands_shown(&self) -> bool {
        with_flag(|l| l.commands_shown)
    }

    #[qslot]
    fn start_tags_shown(&self) -> bool {
        with_flag(|l| l.tags_shown)
    }

    #[qslot]
    fn start_wip_tree(&self) -> bool {
        with_flag(|l| l.wip_tree)
    }

    #[qslot]
    fn start_details_tree(&self) -> bool {
        with_flag(|l| l.details_tree)
    }

    #[qslot]
    fn start_section(&self, name: String) -> bool {
        with_flag(|l| match name.as_str() {
            "branches" => l.sections.branches,
            "remotes" => l.sections.remotes,
            "worktree" => l.sections.worktree,
            "stashes" => l.sections.stashes,
            "tags" => l.sections.tags,
            _ => true,
        })
    }

    /// Where the window is now. Reported on the same timer as the layout:
    /// a drag across the desktop is as continuous as a pane drag.
    #[qslot]
    fn save_window(&self, x: i32, y: i32, width: i32, height: i32, maximized: bool) {
        Hub::with(|hub| {
            let previous = hub.state().window;
            hub.set_window_state(platitude_core::settings::WindowState {
                // A maximized window reports the size of the screen. Keeping
                // the last unmaximized one is what lets restoring down go
                // back to a window rather than to a full screen.
                x: if maximized { previous.x } else { Some(x) },
                y: if maximized { previous.y } else { Some(y) },
                width: if maximized { previous.width } else { width },
                height: if maximized { previous.height } else { height },
                maximized,
            });
        });
    }

    /// The layout, from the one place that can see all of it. Reporting
    /// each value as it changes would write on every frame of a splitter
    /// drag; the window says what it looks like on a timer instead, and
    /// the hub only writes a file when that differs from what is in one.
    ///
    /// Three calls rather than one because the layout has more parts than
    /// a Qt slot takes arguments. Each merges into what the hub holds.
    #[qslot]
    fn save_layout_sizes(&self, sidebar_width: i32, details_width: i32, commands_height: i32) {
        Hub::with(|hub| {
            let mut layout = hub.state().layout;
            layout.sidebar_width = sidebar_width;
            layout.details_width = details_width;
            layout.commands_height = commands_height;
            hub.set_layout_state(layout);
        });
    }

    #[qslot]
    fn save_layout_flags(
        &self,
        sidebar_collapsed: bool,
        commands_shown: bool,
        tags_shown: bool,
        wip_tree: bool,
        details_tree: bool,
    ) {
        Hub::with(|hub| {
            let mut layout = hub.state().layout;
            layout.sidebar_collapsed = sidebar_collapsed;
            layout.commands_shown = commands_shown;
            layout.tags_shown = tags_shown;
            layout.wip_tree = wip_tree;
            layout.details_tree = details_tree;
            hub.set_layout_state(layout);
        });
    }

    #[qslot]
    fn save_sections(
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

    /// Writes the state out if anything moved. Driven by a timer in the
    /// window and called once more as it closes.
    #[qslot]
    fn flush_state(&self) {
        Hub::with(|hub| hub.flush_state());
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
