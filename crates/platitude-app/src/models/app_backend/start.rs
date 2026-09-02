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

/// Whether one sidebar section comes up open. A name nothing was saved
/// for opens: a section added later has no line in anybody's state file,
/// and closed-because-unheard-of is not what a first sight of it should
/// be.
pub(super) fn section_open(name: &str) -> bool {
    with_flag(|l| match name {
        "branches" => l.sections.branches,
        "remotes" => l.sections.remotes,
        "worktree" => l.sections.worktree,
        "stashes" => l.sections.stashes,
        "tags" => l.sections.tags,
        _ => true,
    })
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
        // The one thing here a run can change is the shape the window
        // comes up in, and it has to be known before the window exists. A
        // build without the harness answers from an idle record and reads
        // no environment at all (`harness::knobs`), so it comes up in the
        // arm a person at the window gets.
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
            system_title_bar: harness.plain_chrome,
            harness_present: cfg!(feature = "automation"),
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
