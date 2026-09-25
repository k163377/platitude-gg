use super::*;

use crate::encode::{Fields, Listed, Record, field};

/// One picture filed against an address; `url` is the `file:` URL it is
/// read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Assignment {
    pub(super) email: String,
    pub(super) name: String,
    pub(super) url: String,
}

/// Every assignment, sorted by address (the store keeps them that way).
pub(super) type Assignments = Listed<Assignment>;

impl Record for Assignment {
    fn to_map(&self) -> qtbridge::qtbridge_type_lib::QVariantMap {
        Fields::new()
            .put("email", &self.email)
            .put("name", &self.name)
            .put("url", &self.url)
            .done()
    }

    fn from_map(map: &qtbridge::qtbridge_type_lib::QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            email: field(map, "email")?,
            name: field(map, "name")?,
            url: field(map, "url")?,
        })
    }
}

pub(super) fn assignments() -> Assignments {
    Hub::with(|hub| {
        let urls = hub.avatar_urls();
        Assignments::new(
            hub.avatars()
                .list()
                .iter()
                .map(|entry| Assignment {
                    email: entry.email.clone(),
                    name: entry.name.clone(),
                    url: urls.url_of(&entry.email),
                })
                .collect(),
        )
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

/// Whether one sidebar section comes up open. An unknown name opens: a
/// section added later is in nobody's state file.
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
/// user's home, where git sees only the global + system configuration.
/// Not the working directory: launched from inside a checkout, that would
/// fold the repository's local values into the user's own.
pub(crate) fn app_workdir() -> std::path::PathBuf {
    std::env::home_dir()
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

fn copies_interval_secs() -> i32 {
    Hub::with(|hub| hub.settings().defaults.copies_interval_secs as i32)
        .unwrap_or(platitude_core::session::COPIES_INTERVAL_DEFAULT_SECS as i32)
}

impl Default for AppBackend {
    fn default() -> Self {
        // A run can change only the window's shape, which must be known
        // before the window exists; a build without the harness reads no
        // environment (`harness::knobs`).
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
            system_title_bar: harness.system_title_bar,
            harness_present: cfg!(feature = "automation"),
            auto_fetch_minutes: Hub::with(|hub| hub.settings().defaults.auto_fetch_minutes as i32)
                .unwrap_or(platitude_core::session::AUTO_FETCH_DEFAULT_MINUTES as i32),
            auto_fetch_max: platitude_core::session::AUTO_FETCH_MAX_MINUTES as i32,
            git_concurrency: Hub::with(|hub| hub.settings().defaults.git_concurrency as i32)
                .unwrap_or(platitude_core::process::default_concurrency() as i32),
            git_concurrency_max: platitude_core::process::MAX_CONCURRENCY as i32,
            git_concurrency_default: platitude_core::process::default_concurrency() as i32,
            copies_interval_secs: copies_interval_secs(),
            copies_interval_min: platitude_core::session::COPIES_INTERVAL_MIN_SECS as i32,
            copies_interval_max: platitude_core::session::COPIES_INTERVAL_MAX_SECS as i32,
            copies_interval_ms: copies_interval_secs().saturating_mul(1000),
            initial_commits: Hub::with(|hub| hub.settings().defaults.initial_commits)
                .unwrap_or(Some(platitude_core::session::DEFAULT_LOG_LIMIT))
                .map_or(0, |count| i32::try_from(count).unwrap_or(i32::MAX)),
            initial_commits_min: platitude_core::session::MIN_LOG_LIMIT as i32,
            initial_commits_default: platitude_core::session::DEFAULT_LOG_LIMIT as i32,
            git_path: Hub::with(|hub| hub.settings().defaults.git_path.clone()).unwrap_or_default(),
            git_path_on_path: Hub::with(|hub| hub.path_program()).unwrap_or_default(),
            git_path_offers_restart: false,
            git_path_names_the_run: false,
            restart_wanted: false,
            // Not asked until the screen opens, so a run that never opens
            // it spawns no probe.
            git_path_state: String::new(),
            git_path_version: String::new(),
            git_path_error: String::new(),
            git_path_in_use: Hub::with(|hub| hub.git_program().to_string()).unwrap_or_default(),
            avatars: assignments(),
            avatar_error_kind: String::new(),
            avatar_error_facts: Vec::new(),
            avatar_error_said: String::new(),
            avatar_patterns: platitude_core::avatar::EXTENSIONS
                .iter()
                .map(|e| format!("*.{e}"))
                .collect::<Vec<_>>()
                .join(" "),
            build_tree: build_tree(),
            already_running: Hub::with(|hub| !hub.held_elsewhere().is_empty()).unwrap_or(false),
            held_elsewhere: Hub::with(|hub| hub.held_elsewhere().to_string()).unwrap_or_default(),
            commands_shown: false,
            check_feed: Arc::new(Feed::default()),
        }
    }
}
