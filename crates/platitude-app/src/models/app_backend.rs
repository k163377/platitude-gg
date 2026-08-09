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
    /// An identity write finished; `error` carries git's own message and
    /// the two flags say which half git now reports as what was asked for.
    IdentitySaved {
        error: Option<String>,
        name_saved: bool,
        email_saved: bool,
    },
}

pub struct AppBackend {
    git_state: String,
    git_version: String,
    /// The supported minimum, printed by the missing-git gate. Read from
    /// core so the screen cannot drift from the version actually enforced.
    minimum_git: String,
    git_error: String,
    /// "unknown" until the check runs, then "checking" / "missing" /
    /// "ready" / "error". Only "missing" opens the setup screen.
    identity_state: String,
    identity_name: String,
    identity_email: String,
    identity_error: String,
    identity_busy: bool,
    /// Which half of the last save git now reports as what was asked for.
    /// The two are how a half-written identity shows on screen instead of
    /// passing for a finished one, and they stay false until a save has
    /// finished — nothing has been asked for before that.
    identity_name_saved: bool,
    identity_email_saved: bool,
    /// A save finished without both halves landing. Separate from the two
    /// above because it also carries "a save has been tried", which is
    /// what keeps the marks and the toolbar badge out of a fresh window.
    identity_unsaved: bool,
    auto_open: String,
    shot_dir: String,
    auto_quit_ms: i32,
    auto_select: bool,
    auto_scroll: bool,
    auto_wip: bool,
    /// Verification hook: take the shape the platforms that cannot merge
    /// the band into the title bar get. Nobody here can run those two, so
    /// without a way to ask for their layout from this side it is only
    /// ever exercised by the people who cannot report back.
    plain_chrome: bool,
    /// Whether something rather than somebody is driving this run
    /// (`Env::automated` — any `PG_*` knob but the three that say nothing
    /// about who is at the window). What reads it is the window: a run
    /// nobody is looking at keeps the size it was configured with instead
    /// of being fitted to a screen (`Main.insideScreen`), and the screen
    /// the headless platform reports is 800x800.
    automated: bool,
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
    /// Auto-fetch interval in minutes; 0 is off. Application-wide, because
    /// the answer is about how often this computer should talk to remotes.
    auto_fetch_minutes: i32,
    /// Ceiling the settings input enforces.
    auto_fetch_max: i32,
    /// Assigned pictures, packed one per record: address, name, URL.
    /// The settings list is the only reader, and it is a handful of rows.
    avatars: String,
    /// git-style: empty means the last assignment worked.
    avatar_error: String,
    /// What the picker offers, built from the kinds the store accepts so
    /// the dialog and the store cannot drift apart.
    avatar_filters: String,
    /// The worktree this binary was built in, empty for the primary
    /// checkout — see [`build_tree`].
    build_tree: String,
    /// Another process is already using the files this one would have
    /// used, so this window is only here to say so and be dismissed.
    already_running: bool,
    /// The directory that other process is holding. Two builds are told
    /// apart by nothing else on screen, and the taskbar's launch entry
    /// gives no clue which one it started.
    held_elsewhere: String,
    check_feed: Arc<Feed<AppMsg>>,
}

/// The worktree this binary was built in, empty when it was not built in
/// one. Parallel sessions each build their own exe and any of them may put
/// a window on the screen, and the windows are otherwise identical — this
/// is what the corner of the right pane says to tell them apart, beside
/// the git version (CLAUDE.md ビルド・テスト).
///
/// It comes from the build path, not from the environment: the exe lives
/// in the tree that built it, so the mark travels with the file however it
/// is started, and a build from a plain checkout — every build anyone
/// outside this repository makes — carries none at all.
pub fn build_tree() -> String {
    tree_of(&env!("CARGO_MANIFEST_DIR").replace('\\', "/")).to_string()
}

/// The worktree directory named in `path`, which separates with `/`. Empty
/// when the path names none.
fn tree_of(path: &str) -> &str {
    const WORKTREES: &str = "/.claude/worktrees/";
    let Some(at) = path.find(WORKTREES).map(|at| at + WORKTREES.len()) else {
        return "";
    };
    let rest = &path[at..];
    &rest[..rest.find('/').unwrap_or(rest.len())]
}

/// Separators the packed records use, matching the graph's label records.
const FIELD_SEP: char = '\u{1f}';
const RECORD_SEP: char = '\u{1e}';

/// Every assignment as `email\u{1f}name\u{1f}url`, records joined by
/// `\u{1e}`, sorted by address (the store keeps them that way).
fn packed_avatars() -> String {
    Hub::with(|hub| {
        let urls = hub.avatar_urls();
        hub.avatars()
            .list()
            .iter()
            .map(|entry| {
                format!(
                    "{}{FIELD_SEP}{}{FIELD_SEP}{}",
                    entry.email,
                    entry.name,
                    urls.url_of(&entry.email)
                )
            })
            .collect::<Vec<_>>()
            .join(&RECORD_SEP.to_string())
    })
    .unwrap_or_default()
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
            minimum_git: platitude_core::version::minimum_string(),
            git_error: String::new(),
            identity_state: "unknown".into(),
            identity_name: String::new(),
            identity_email: String::new(),
            identity_error: String::new(),
            identity_busy: false,
            identity_name_saved: false,
            identity_email_saved: false,
            identity_unsaved: false,
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
            plain_chrome: std::env::var("PG_PLAIN_CHROME").as_deref() == Ok("1"),
            automated: platitude_core::settings::Env::system().automated(),
            auto_identity: std::env::var("PG_AUTO_IDENTITY").unwrap_or_default(),
            auto_identity_save: std::env::var("PG_AUTO_IDENTITY_SAVE").as_deref() == Ok("1"),
            // Smoke-test hook: "top" / "bottom" jumps the graph after
            // load; "nav-bottom" jumps the sidebar's branch list instead.
            scroll_to: std::env::var("PG_SCROLL_TO").unwrap_or_default(),
            auto_act: std::env::var("PG_AUTO_ACT").unwrap_or_default(),
            auto_act_arg: std::env::var("PG_AUTO_ACT_ARG").unwrap_or_default(),
            auto_fetch_minutes: Hub::with(|hub| hub.settings().defaults.auto_fetch_minutes as i32)
                .unwrap_or(platitude_core::session::AUTO_FETCH_DEFAULT_MINUTES as i32),
            auto_fetch_max: platitude_core::session::AUTO_FETCH_MAX_MINUTES as i32,
            avatars: packed_avatars(),
            avatar_error: String::new(),
            avatar_filters: format!(
                "Images ({})",
                platitude_core::avatar::EXTENSIONS
                    .iter()
                    .map(|e| format!("*.{e}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            build_tree: build_tree(),
            already_running: Hub::with(|hub| !hub.held_elsewhere().is_empty()).unwrap_or(false),
            held_elsewhere: Hub::with(|hub| hub.held_elsewhere().to_string()).unwrap_or_default(),
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
    qproperty!(
        "minimumGit",
        Member = minimum_git,
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
    qproperty!(
        "identityNameSaved",
        Member = identity_name_saved,
        Notify = identity_changed
    );
    qproperty!(
        "identityEmailSaved",
        Member = identity_email_saved,
        Notify = identity_changed
    );
    qproperty!(
        "identityUnsaved",
        Member = identity_unsaved,
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
    qproperty!("plainChrome", Member = plain_chrome, Constant);
    qproperty!("automated", Member = automated, Constant);
    qproperty!("scrollTo", Member = scroll_to, Constant);
    qproperty!("autoAct", Member = auto_act, Constant);
    qproperty!("autoActArg", Member = auto_act_arg, Constant);
    qproperty!(
        "autoFetchMinutes",
        Member = auto_fetch_minutes,
        Notify = settings_changed
    );
    qproperty!("autoFetchMaxMinutes", Member = auto_fetch_max, Constant);
    qproperty!("avatars", Member = avatars, Notify = avatars_changed);
    qproperty!(
        "avatarError",
        Member = avatar_error,
        Notify = avatars_changed
    );
    qproperty!("avatarFilters", Member = avatar_filters, Constant);
    qproperty!("buildTree", Member = build_tree, Constant);
    qproperty!("alreadyRunning", Member = already_running, Constant);
    qproperty!("heldElsewhere", Member = held_elsewhere, Constant);

    #[qsignal]
    fn git_state_changed(&mut self);

    #[qsignal]
    fn identity_changed(&mut self);

    #[qsignal]
    fn settings_changed(&mut self);

    /// An assignment was made or taken away. Every list already on screen
    /// re-reads itself off this — the graph rows, the details card and
    /// the settings list — because none of them can be told apart from
    /// the repository not having changed, which it did not.
    #[qsignal]
    fn avatars_changed(&mut self);

    /// Files a picture against an address. `file_url` comes from the
    /// picker, so it arrives as a URL rather than a path.
    ///
    /// Writes at once. There is no Save on the card this is reached from
    /// and no Cancel to undo it — the gesture that takes one away is a
    /// hold instead (デザイン規約 §長押し).
    #[qslot]
    fn assign_avatar(&mut self, email: String, name: String, file_url: String) {
        let source = crate::urlpath::file_url_to_path(&file_url);
        self.avatar_error =
            Hub::with(|hub| hub.assign_avatar(&email, &name, &source)).unwrap_or_default();
        if !self.avatar_error.is_empty() {
            tracing::warn!(error = %self.avatar_error, "avatar not assigned");
        }
        self.reload_avatars();
    }

    #[qslot]
    fn remove_avatar(&mut self, email: String) {
        Hub::with(|hub| hub.remove_avatar(&email));
        self.avatar_error.clear();
        self.reload_avatars();
    }

    /// The picture for an address, or empty. For the one place that asks
    /// about a person it is not already showing: the settings card, when
    /// an avatar's own badge opened it.
    #[qslot]
    fn avatar_url_for(&self, email: String) -> String {
        Hub::with(|hub| hub.avatar_url(&email)).unwrap_or_default()
    }

    fn reload_avatars(&mut self) {
        self.avatars = packed_avatars();
        self.avatars_changed();
    }

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

    /// Asks Windows not to round the window's corners. A no-op on the
    /// other two platforms, where the window manager is not doing it.
    #[qslot]
    fn square_window_corners(&self) {
        crate::winframe::square_corners();
    }

    /// Gives the window the app's own icon, which is what the title bar,
    /// the taskbar button and Alt+Tab read. Also Windows-only: elsewhere
    /// the icon travels with the desktop entry or the bundle.
    #[qslot]
    fn set_window_icon(&self) {
        crate::winframe::set_icon();
    }

    /// Puts back what the platform may do with the window on its own —
    /// Win+Arrow, the taskbar's menu, minimising from the taskbar button.
    /// Asking for none of the drawn buttons is what takes them away, and
    /// this app draws its own (P3-確認事項 §ウィンドウ chrome).
    #[qslot]
    fn keep_window_gestures(&self) {
        crate::winframe::keep_system_gestures();
        // And answer WM_NCHITTEST ourselves from here on: Qt 6.10's own
        // answer for this flag set synthesises input from a poll and
        // loses track of it, which is the dead first click and the
        // stuck hover (the whole story is on `take_frame_hit_test`).
        crate::winframe::take_frame_hit_test();
    }

    /// Where the band's empty run sits, in logical scene pixels — the
    /// one stretch of the window the hit test calls caption, which is
    /// what makes it drag, snap and answer a right-click with the
    /// window menu. The scene reports it; the platform does the rest.
    #[qslot]
    fn set_caption_strip(&self, x0: f64, x1: f64, bottom: f64) {
        crate::winframe::set_caption_strip(x0, x1, bottom);
    }

    /// Maximises the window or puts it back, through the platform rather
    /// than through `visibility`. Qt maximises a frameless window by
    /// resizing it, which leaves the platform with nothing to restore —
    /// so the button did nothing while the band's double-click worked
    /// (`winframe::set_maximized`).
    #[qslot]
    fn set_window_maximized(&self, maximized: bool) {
        crate::winframe::set_maximized(maximized);
    }

    /// Pulls a restored window back inside the screen it came up on, and
    /// answers whether it had to. The scene cannot do this itself: what
    /// has to fit is the frame, which is wider than the window says it
    /// is, and the work area is not a thing QML reports
    /// (`winframe::fit_to_work_area`).
    #[qslot]
    fn fit_window_to_screen(&self) -> bool {
        crate::winframe::fit_to_work_area()
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
    fn start_graph_labels_width(&self) -> i32 {
        with_layout(|l| l.graph_labels_width)
    }

    #[qslot]
    fn start_graph_lanes_width(&self) -> i32 {
        with_layout(|l| l.graph_lanes_width)
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
    fn save_layout_sizes(
        &self,
        sidebar_width: i32,
        details_width: i32,
        commands_height: i32,
        graph_labels_width: i32,
        graph_lanes_width: i32,
    ) {
        Hub::with(|hub| {
            let mut layout = hub.state().layout;
            layout.sidebar_width = sidebar_width;
            layout.details_width = details_width;
            layout.commands_height = commands_height;
            layout.graph_labels_width = graph_labels_width;
            layout.graph_lanes_width = graph_lanes_width;
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

    #[qslot]
    fn drain(&mut self) {
        let mut check_identity = false;
        let mut wrote = false;
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
qml_register!(AppBackend, "AppBackend", singleton = true);

#[cfg(test)]
mod tests {
    use super::tree_of;

    #[test]
    fn names_the_worktree_a_build_came_from() {
        assert_eq!(
            tree_of(
                "C:/Users/x/IdeaProjects/platitude-gg/.claude/worktrees/labels/crates/platitude-app"
            ),
            "labels"
        );
        assert_eq!(
            tree_of("/home/x/platitude-gg/.claude/worktrees/perf/crates/platitude-app"),
            "perf"
        );
    }

    #[test]
    fn leaves_every_other_build_unmarked() {
        // The primary checkout, and the plain checkout anyone outside this
        // repository builds from — neither carries a mark into what ships.
        assert_eq!(
            tree_of("C:/Users/x/IdeaProjects/platitude-gg/crates/platitude-app"),
            ""
        );
        assert_eq!(
            tree_of("/build/platitude-gg-0.0.0/crates/platitude-app"),
            ""
        );
    }
}
