use super::*;

use crate::encode::{FIELD_SEP, RECORD_SEP};

/// Every assignment as `email\u{1e}name\u{1e}url`, records joined by
/// `\u{1f}`, sorted by address (the store keeps them that way) — the one
/// packed-record convention every list in this crate uses
/// (`encode::RECORD_SEP` / `FIELD_SEP`).
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

/// Directory the application's own configuration reads run in — the
/// user's home. git resolves configuration from a directory, and outside
/// a repository that is exactly the user's own (global + system)
/// configuration — what a first-run prompt is about, and what the
/// settings screen's `GLOBAL` identity chapter writes into
/// (`AppBackend::save_identity`). The process working directory would answer
/// the same *except* when the app is launched from a terminal standing
/// inside a checkout, where it would quietly fold that repository's
/// local values into the user's own.
pub(crate) fn app_workdir() -> std::path::PathBuf {
    std::env::home_dir()
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

impl Default for AppBackend {
    fn default() -> Self {
        // Whatever is driving this run, if anything is. A build without
        // the harness answers with an idle record and reads no
        // environment at all (`harness::knobs`), so every hook below
        // comes up in the arm a person at the window gets.
        let harness = crate::harness::knobs();
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
            auto_open: harness.open.clone(),
            shot_dir: harness.shot_dir.clone(),
            auto_watchdog_ms: harness.watchdog_ms,
            auto_select: harness.select,
            auto_scroll: harness.scroll,
            auto_perf: harness.perf,
            mem_report: crate::harness::memprobe::enabled(),
            auto_wip: harness.wip,
            plain_chrome: harness.plain_chrome,
            automated: platitude_core::settings::Env::system().automated(),
            auto_identity: harness.identity.clone(),
            auto_identity_save: harness.identity_save,
            scroll_to: harness.scroll_to.clone(),
            auto_act: harness.act.clone(),
            auto_act_arg: harness.act_arg.clone(),
            auto_fetch_minutes: Hub::with(|hub| hub.settings().defaults.auto_fetch_minutes as i32)
                .unwrap_or(platitude_core::session::AUTO_FETCH_DEFAULT_MINUTES as i32),
            auto_fetch_max: platitude_core::session::AUTO_FETCH_MAX_MINUTES as i32,
            initial_commits: Hub::with(|hub| hub.settings().defaults.initial_commits)
                .unwrap_or(Some(platitude_core::session::DEFAULT_LOG_LIMIT))
                .map_or(0, |count| i32::try_from(count).unwrap_or(i32::MAX)),
            initial_commits_min: platitude_core::session::MIN_LOG_LIMIT as i32,
            initial_commits_default: platitude_core::session::DEFAULT_LOG_LIMIT as i32,
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
