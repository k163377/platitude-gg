use super::*;

/// Separators the packed records use. Note the graph's label records
/// (`encode::FIELD_SEP` / `RECORD_SEP`) assign the same two characters
/// the other way round — each side matches its own reader, not the other.
const FIELD_SEP: char = '\u{1f}';
const RECORD_SEP: char = '\u{1e}';

/// Every assignment as `email\u{1f}name\u{1f}url`, records joined by
/// `\u{1e}`, sorted by address (the store keeps them that way).
pub(super) fn packed_avatars() -> String {
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

pub(super) fn with_window(pick: impl Fn(&platitude_core::settings::WindowState) -> i32) -> i32 {
    let fallback = platitude_core::settings::WindowState::default();
    Hub::with(|hub| pick(&hub.state().window)).unwrap_or_else(|| pick(&fallback))
}

pub(super) fn with_layout(pick: impl Fn(&platitude_core::settings::LayoutState) -> i32) -> i32 {
    let fallback = platitude_core::settings::LayoutState::default();
    Hub::with(|hub| pick(&hub.state().layout)).unwrap_or_else(|| pick(&fallback))
}

pub(super) fn with_flag(pick: impl Fn(&platitude_core::settings::LayoutState) -> bool) -> bool {
    let fallback = platitude_core::settings::LayoutState::default();
    Hub::with(|hub| pick(&hub.state().layout)).unwrap_or_else(|| pick(&fallback))
}

/// Directory the identity check runs in. git resolves configuration from a
/// directory, and outside a repository that is exactly the user's own
/// (global + system) configuration — what a first-run prompt is about.
pub(super) fn app_workdir() -> std::path::PathBuf {
    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
}

impl Default for AppBackend {
    fn default() -> Self {
        Self {
            git_state: "checking".into(),
            git_version: String::new(),
            git_unsupported: false,
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
            auto_watchdog_ms: std::env::var("PG_AUTO_WATCHDOG_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            auto_select: std::env::var("PG_AUTO_SELECT").as_deref() == Ok("1"),
            auto_scroll: std::env::var("PG_AUTO_SCROLL").as_deref() == Ok("1"),
            mem_report: crate::memprobe::enabled(),
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
            avatar_patterns: platitude_core::avatar::EXTENSIONS
                .iter()
                .map(|e| format!("*.{e}"))
                .collect::<Vec<_>>()
                .join(" "),
            build_tree: build_tree(),
            already_running: Hub::with(|hub| !hub.held_elsewhere().is_empty()).unwrap_or(false),
            held_elsewhere: Hub::with(|hub| hub.held_elsewhere().to_string()).unwrap_or_default(),
            check_feed: Arc::new(Feed::default()),
        }
    }
}
