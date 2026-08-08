//! The two files platitude-gg keeps for itself.
//!
//! `settings.toml` holds what a person decided. `state.toml` holds what the
//! last session left behind. They are separate files because they are
//! written at completely different rates: dragging a pane rewrites the
//! state several times a minute, and one of those flushes landing on top of
//! the settings would eat an edit made in an editor while the app is
//! running. It also makes "delete `state.toml` to get the layout back"
//! something that can be said without also throwing the settings away.
//!
//! Nothing in here can stop the application from starting. A file that is
//! missing, unreadable, truncated or full of nonsense costs only the keys
//! it got wrong: every value is pulled out of a parsed table one at a time
//! and falls back on its own. A derived `Deserialize` cannot do that — one
//! mistyped value gives up the whole document (measured), which would turn
//! a stray keystroke in an editor into a reset of everything.

use std::path::{Path, PathBuf};

use toml::{Table, Value};

/// Directory the two files live in, under whichever base the platform uses.
pub const DIR_NAME: &str = "platitude-gg";

pub const SETTINGS_FILE: &str = "settings.toml";
pub const STATE_FILE: &str = "state.toml";

/// Written at the top of both files. Every reader is per-key tolerant, so
/// this is not a gate — it is there so a later renaming of a key can tell
/// an old file from a new one instead of guessing.
pub const SCHEMA_VERSION: i64 = 1;

/// Points both files at one directory. Empty means "read nothing, write
/// nothing", which is what a test or a screenshot run wants.
pub const CONFIG_DIR_ENV: &str = "PG_CONFIG_DIR";

/// Automation knobs all share this prefix, and a person never sets one.
const AUTOMATION_PREFIX: &str = "PG_";

/// Failure to write. Reading has no error type: it cannot fail loudly
/// enough to matter, and the caller has nothing to do about it but carry
/// on with defaults.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("could not write {}: {source}", path.display())]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Which platform's placement rules to apply. A parameter rather than a
/// `cfg!` so all three can be tested from any machine — two of the three
/// have no CI runner that reaches them until Phase 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    MacOs,
    /// Linux and anything else following the XDG base directory spec.
    Xdg,
}

impl Platform {
    /// The platform this build runs on.
    pub const HOST: Platform = if cfg!(windows) {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::MacOs
    } else {
        Platform::Xdg
    };
}

/// The environment the placement rules read. Captured once so the rules
/// stay a pure function of it, and tests can hand over one they built.
#[derive(Debug, Default, Clone)]
pub struct Env {
    vars: Vec<(String, String)>,
}

impl Env {
    pub fn system() -> Self {
        Self {
            vars: std::env::vars().collect(),
        }
    }

    pub fn from_pairs(pairs: &[(&str, &str)]) -> Self {
        Self {
            vars: pairs
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        }
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.vars
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// A non-empty variable, which is what every base directory needs to
    /// be: an empty `HOME` is not a home directory.
    fn filled(&self, key: &str) -> Option<&str> {
        self.get(key).filter(|v| !v.is_empty())
    }

    /// True when anything is driving this process. Every `PG_*` knob exists
    /// for automation only, so one of them being set is enough to know a
    /// person is not the one at the window.
    fn automated(&self) -> bool {
        self.vars
            .iter()
            .any(|(k, _)| k.starts_with(AUTOMATION_PREFIX) && k != CONFIG_DIR_ENV)
    }
}

/// Where the two files are, or that there are none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Store {
    settings_path: Option<PathBuf>,
    state_path: Option<PathBuf>,
}

impl Store {
    /// The store this process should use.
    pub fn discover() -> Self {
        Self::locate(Platform::HOST, &Env::system())
    }

    /// `PG_CONFIG_DIR` wins: a path puts both files in it, and an empty
    /// value asks for no files at all. With it unset, any other `PG_*`
    /// variable still means no files — otherwise a screenshot run would
    /// write its window geometry into the developer's real settings, and
    /// the next run would start from it.
    pub fn locate(platform: Platform, env: &Env) -> Self {
        if let Some(dir) = env.get(CONFIG_DIR_ENV) {
            if dir.is_empty() {
                return Self::ephemeral();
            }
            return Self::at(Path::new(dir));
        }
        if env.automated() {
            return Self::ephemeral();
        }
        Self::platform_paths(platform, env)
    }

    /// Both files in one directory.
    pub fn at(dir: &Path) -> Self {
        Self {
            settings_path: Some(dir.join(SETTINGS_FILE)),
            state_path: Some(dir.join(STATE_FILE)),
        }
    }

    /// Reads nothing and writes nothing.
    pub fn ephemeral() -> Self {
        Self {
            settings_path: None,
            state_path: None,
        }
    }

    /// Windows separates the two by roaming: settings follow a person to
    /// another machine, a window position must not. Linux has the same
    /// split spelled out in the XDG spec. macOS has no such convention, so
    /// both sit in Application Support.
    ///
    /// A base directory that is not in the environment is not guessed at —
    /// the store simply has no files rather than inventing a path to write
    /// into.
    fn platform_paths(platform: Platform, env: &Env) -> Self {
        let (settings_base, state_base) = match platform {
            Platform::Windows => (
                env.filled("APPDATA").map(PathBuf::from),
                env.filled("LOCALAPPDATA").map(PathBuf::from),
            ),
            Platform::MacOs => {
                let support = env
                    .filled("HOME")
                    .map(|home| Path::new(home).join("Library").join("Application Support"));
                (support.clone(), support)
            }
            Platform::Xdg => {
                let home = env.filled("HOME").map(PathBuf::from);
                let config = env
                    .filled("XDG_CONFIG_HOME")
                    .map(PathBuf::from)
                    .or_else(|| home.as_ref().map(|h| h.join(".config")));
                let state = env
                    .filled("XDG_STATE_HOME")
                    .map(PathBuf::from)
                    .or_else(|| home.as_ref().map(|h| h.join(".local").join("state")));
                (config, state)
            }
        };
        Self {
            settings_path: settings_base.map(|b| b.join(DIR_NAME).join(SETTINGS_FILE)),
            state_path: state_base.map(|b| b.join(DIR_NAME).join(STATE_FILE)),
        }
    }

    pub fn settings_path(&self) -> Option<&Path> {
        self.settings_path.as_deref()
    }

    pub fn state_path(&self) -> Option<&Path> {
        self.state_path.as_deref()
    }

    /// True when this store is not backed by any file.
    pub fn is_ephemeral(&self) -> bool {
        self.settings_path.is_none() && self.state_path.is_none()
    }

    pub fn load_settings(&self) -> Settings {
        Settings::from_table(&read_table(self.settings_path.as_deref()))
    }

    pub fn load_state(&self) -> State {
        State::from_table(&read_table(self.state_path.as_deref()))
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), StoreError> {
        let Some(path) = self.settings_path.as_deref() else {
            return Ok(());
        };
        write_atomically(path, &settings.to_table().to_string())
    }

    pub fn save_state(&self, state: &State) -> Result<(), StoreError> {
        let Some(path) = self.state_path.as_deref() else {
            return Ok(());
        };
        write_atomically(path, &state.to_table().to_string())
    }
}

// ---------------------------------------------------------------------------
// settings.toml
// ---------------------------------------------------------------------------

/// The values that apply to a repository. One set is the application's
/// defaults; a repository may override any of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoSettings {
    pub auto_fetch_minutes: u32,
    pub network_timeout_secs: u64,
}

impl Default for RepoSettings {
    fn default() -> Self {
        Self {
            auto_fetch_minutes: crate::session::AUTO_FETCH_DEFAULT_MINUTES,
            network_timeout_secs: crate::remote::DEFAULT_NETWORK_TIMEOUT.as_secs(),
        }
    }
}

/// What one repository asked to differ on. Absent fields take the default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoOverride {
    /// The work tree path, as [`repo_key`] writes it.
    pub key: String,
    pub auto_fetch_minutes: Option<u32>,
    pub network_timeout_secs: Option<u64>,
}

impl RepoOverride {
    pub fn new(path: &str) -> Self {
        Self {
            key: repo_key(path),
            auto_fetch_minutes: None,
            network_timeout_secs: None,
        }
    }

    fn is_empty(&self) -> bool {
        self.auto_fetch_minutes.is_none() && self.network_timeout_secs.is_none()
    }
}

/// `settings.toml`.
///
/// Per-repository overrides are only for values the application owns. What
/// git already remembers per repository — the identity on a commit, the
/// merge tool — stays in that repository's own config, where git put it and
/// where every other tool can see it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    pub defaults: RepoSettings,
    pub repos: Vec<RepoOverride>,
}

impl Settings {
    /// The values in force for one repository.
    pub fn for_repo(&self, path: &str) -> RepoSettings {
        let key = repo_key(path);
        let Some(over) = self.repos.iter().find(|r| r.key == key) else {
            return self.defaults.clone();
        };
        RepoSettings {
            auto_fetch_minutes: over
                .auto_fetch_minutes
                .unwrap_or(self.defaults.auto_fetch_minutes),
            network_timeout_secs: over
                .network_timeout_secs
                .unwrap_or(self.defaults.network_timeout_secs),
        }
    }

    fn from_table(table: &Table) -> Self {
        let fallback = RepoSettings::default();
        let defaults = match sub_table(table, "defaults") {
            Some(t) => RepoSettings {
                auto_fetch_minutes: minutes(t, "auto_fetch_minutes")
                    .unwrap_or(fallback.auto_fetch_minutes),
                network_timeout_secs: timeout_secs(t, "network_timeout_secs")
                    .unwrap_or(fallback.network_timeout_secs),
            },
            None => fallback,
        };
        let mut repos = Vec::new();
        if let Some(by_repo) = sub_table(table, "repo") {
            for (key, value) in by_repo {
                let Some(entry) = value.as_table() else {
                    continue;
                };
                let over = RepoOverride {
                    key: repo_key(key),
                    auto_fetch_minutes: minutes(entry, "auto_fetch_minutes"),
                    network_timeout_secs: timeout_secs(entry, "network_timeout_secs"),
                };
                if !over.is_empty() {
                    repos.push(over);
                }
            }
        }
        Self { defaults, repos }
    }

    fn to_table(&self) -> Table {
        let mut root = Table::new();
        root.insert("version".into(), Value::Integer(SCHEMA_VERSION));

        let mut defaults = Table::new();
        defaults.insert(
            "auto_fetch_minutes".into(),
            Value::Integer(self.defaults.auto_fetch_minutes.into()),
        );
        defaults.insert(
            "network_timeout_secs".into(),
            Value::Integer(clamp_to_i64(self.defaults.network_timeout_secs)),
        );
        root.insert("defaults".into(), Value::Table(defaults));

        let mut by_repo = Table::new();
        for over in &self.repos {
            if over.is_empty() || over.key.is_empty() {
                continue;
            }
            let mut entry = Table::new();
            if let Some(minutes) = over.auto_fetch_minutes {
                entry.insert("auto_fetch_minutes".into(), Value::Integer(minutes.into()));
            }
            if let Some(secs) = over.network_timeout_secs {
                entry.insert(
                    "network_timeout_secs".into(),
                    Value::Integer(clamp_to_i64(secs)),
                );
            }
            by_repo.insert(over.key.clone(), Value::Table(entry));
        }
        if !by_repo.is_empty() {
            root.insert("repo".into(), Value::Table(by_repo));
        }
        root
    }
}

// ---------------------------------------------------------------------------
// state.toml
// ---------------------------------------------------------------------------

/// Where the window was left. Position is optional because "never saved"
/// has to stay distinguishable from "saved at 0,0" — the first should let
/// the window manager place the window, the second should not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowState {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: i32,
    pub height: i32,
    pub maximized: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            x: None,
            y: None,
            width: 1440,
            height: 900,
            maximized: false,
        }
    }
}

/// Which sidebar sections are expanded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sections {
    pub branches: bool,
    pub remotes: bool,
    pub worktree: bool,
    pub stashes: bool,
    pub tags: bool,
}

impl Default for Sections {
    fn default() -> Self {
        Self {
            branches: true,
            remotes: true,
            worktree: true,
            stashes: true,
            tags: true,
        }
    }
}

/// One set for the whole application, not one per tab. Persisting per tab
/// would make the layout jump on every tab switch and then keep doing it
/// across restarts; what is worth remembering is the layout that was last
/// settled on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutState {
    pub sidebar_width: i32,
    pub sidebar_collapsed: bool,
    pub details_width: i32,
    pub commands_height: i32,
    pub commands_shown: bool,
    pub tags_shown: bool,
    pub wip_tree: bool,
    pub details_tree: bool,
    pub sections: Sections,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            sidebar_width: 260,
            sidebar_collapsed: false,
            details_width: 400,
            commands_height: 280,
            commands_shown: false,
            tags_shown: true,
            wip_tree: true,
            details_tree: true,
            sections: Sections::default(),
        }
    }
}

/// A cap on the tab list, so a file that has been hand-edited into
/// something enormous cannot make startup crawl.
const MAX_TABS: usize = 64;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TabsState {
    /// Work tree paths, in the order the tabs sat in.
    pub paths: Vec<String>,
    /// Index into `paths`. Always in range once loaded.
    pub active: usize,
}

/// `state.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    pub window: WindowState,
    pub layout: LayoutState,
    pub tabs: TabsState,
}

impl State {
    fn from_table(table: &Table) -> Self {
        let mut state = Self::default();

        if let Some(w) = sub_table(table, "window") {
            let d = WindowState::default();
            state.window = WindowState {
                x: coord(w, "x"),
                y: coord(w, "y"),
                width: int_in(w, "width", 320..=32_000, d.width),
                height: int_in(w, "height", 240..=32_000, d.height),
                maximized: flag(w, "maximized", d.maximized),
            };
        }

        if let Some(l) = sub_table(table, "layout") {
            let d = LayoutState::default();
            state.layout = LayoutState {
                sidebar_width: int_in(l, "sidebar_width", 180..=4_000, d.sidebar_width),
                sidebar_collapsed: flag(l, "sidebar_collapsed", d.sidebar_collapsed),
                details_width: int_in(l, "details_width", 300..=4_000, d.details_width),
                commands_height: int_in(l, "commands_height", 120..=4_000, d.commands_height),
                commands_shown: flag(l, "commands_shown", d.commands_shown),
                tags_shown: flag(l, "tags_shown", d.tags_shown),
                wip_tree: flag(l, "wip_tree", d.wip_tree),
                details_tree: flag(l, "details_tree", d.details_tree),
                sections: match sub_table(l, "sections") {
                    Some(s) => Sections {
                        branches: flag(s, "branches", d.sections.branches),
                        remotes: flag(s, "remotes", d.sections.remotes),
                        worktree: flag(s, "worktree", d.sections.worktree),
                        stashes: flag(s, "stashes", d.sections.stashes),
                        tags: flag(s, "tags", d.sections.tags),
                    },
                    None => d.sections,
                },
            };
        }

        if let Some(t) = sub_table(table, "tabs") {
            let paths: Vec<String> = t
                .get("paths")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .filter(|p| !p.is_empty())
                        .take(MAX_TABS)
                        .map(repo_key)
                        .collect()
                })
                .unwrap_or_default();
            let active = t
                .get("active")
                .and_then(Value::as_integer)
                .and_then(|v| usize::try_from(v).ok())
                .filter(|v| *v < paths.len())
                .unwrap_or(0);
            state.tabs = TabsState { paths, active };
        }

        state
    }

    fn to_table(&self) -> Table {
        let mut root = Table::new();
        root.insert("version".into(), Value::Integer(SCHEMA_VERSION));

        let mut window = Table::new();
        if let Some(x) = self.window.x {
            window.insert("x".into(), Value::Integer(x.into()));
        }
        if let Some(y) = self.window.y {
            window.insert("y".into(), Value::Integer(y.into()));
        }
        window.insert("width".into(), Value::Integer(self.window.width.into()));
        window.insert("height".into(), Value::Integer(self.window.height.into()));
        window.insert("maximized".into(), Value::Boolean(self.window.maximized));
        root.insert("window".into(), Value::Table(window));

        let l = &self.layout;
        let mut layout = Table::new();
        layout.insert(
            "sidebar_width".into(),
            Value::Integer(l.sidebar_width.into()),
        );
        layout.insert(
            "sidebar_collapsed".into(),
            Value::Boolean(l.sidebar_collapsed),
        );
        layout.insert(
            "details_width".into(),
            Value::Integer(l.details_width.into()),
        );
        layout.insert(
            "commands_height".into(),
            Value::Integer(l.commands_height.into()),
        );
        layout.insert("commands_shown".into(), Value::Boolean(l.commands_shown));
        layout.insert("tags_shown".into(), Value::Boolean(l.tags_shown));
        layout.insert("wip_tree".into(), Value::Boolean(l.wip_tree));
        layout.insert("details_tree".into(), Value::Boolean(l.details_tree));
        let mut sections = Table::new();
        sections.insert("branches".into(), Value::Boolean(l.sections.branches));
        sections.insert("remotes".into(), Value::Boolean(l.sections.remotes));
        sections.insert("worktree".into(), Value::Boolean(l.sections.worktree));
        sections.insert("stashes".into(), Value::Boolean(l.sections.stashes));
        sections.insert("tags".into(), Value::Boolean(l.sections.tags));
        layout.insert("sections".into(), Value::Table(sections));
        root.insert("layout".into(), Value::Table(layout));

        let mut tabs = Table::new();
        let paths: Vec<Value> = self
            .tabs
            .paths
            .iter()
            .filter(|p| !p.is_empty())
            .take(MAX_TABS)
            .map(|p| Value::String(repo_key(p)))
            .collect();
        let active =
            i64::try_from(self.tabs.active.min(paths.len().saturating_sub(1))).unwrap_or(0);
        tabs.insert("active".into(), Value::Integer(active));
        tabs.insert("paths".into(), Value::Array(paths));
        root.insert("tabs".into(), Value::Table(tabs));

        root
    }
}

// ---------------------------------------------------------------------------
// Reading and writing
// ---------------------------------------------------------------------------

/// The name a repository is filed under: separators the way git writes
/// them, and no trailing one. A repository that moves loses its overrides —
/// its path is the only name the application has for it.
///
/// Every path in both files goes through this, so the same repository reads
/// the same in a `[repo.…]` heading and in the tab list, and so nothing in
/// either file ever needs a backslash escape.
pub fn repo_key(path: &str) -> String {
    // `\\?\` and `\\.\` mean the backslashes to Windows itself: the prefix
    // is what turns off path parsing, and rewriting it addresses somewhere
    // else. Nothing else in a path cares which separator it gets.
    if path.starts_with(r"\\?\") || path.starts_with(r"\\.\") {
        return path.to_string();
    }
    let key = path.replace('\\', "/");
    let trimmed = key.trim_end_matches('/');
    // A drive root and the filesystem root are the slash.
    if trimmed.is_empty() || trimmed.ends_with(':') {
        key
    } else {
        trimmed.to_string()
    }
}

/// Whatever could be parsed. Missing, unreadable and malformed all read as
/// an empty table, which sends every key to its default.
fn read_table(path: Option<&Path>) -> Table {
    let Some(path) = path else {
        return Table::new();
    };
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Table::new(),
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "could not read; using defaults");
            return Table::new();
        }
    };
    match text.parse::<Table>() {
        Ok(table) => table,
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "could not parse; using defaults");
            Table::new()
        }
    }
}

/// Writes through a neighbouring temporary file and renames over the
/// target. A write cut short does not leave a file that fails to parse —
/// it leaves one that parses into *different values* (a truncated `180`
/// reads as `18`, measured), and no amount of per-key tolerance catches
/// that. The rename is what makes the old file survive a crash instead.
fn write_atomically(path: &Path, text: &str) -> Result<(), StoreError> {
    let failed = |source: std::io::Error| StoreError::Write {
        path: path.to_path_buf(),
        source,
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(failed)?;
    }
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let tmp = path.with_file_name(format!("{name}.{}.tmp", std::process::id()));

    let write = (|| -> std::io::Result<()> {
        use std::io::Write;
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()
    })();
    if let Err(error) = write {
        let _ = std::fs::remove_file(&tmp);
        return Err(failed(error));
    }

    // Replaces the target on every platform this ships to. If it does not
    // (on Windows something else may be holding the file open), the old
    // file is still there and intact — losing the newest layout beats
    // losing the file.
    if let Err(error) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(failed(error));
    }
    Ok(())
}

fn sub_table<'a>(table: &'a Table, key: &str) -> Option<&'a Table> {
    table.get(key).and_then(Value::as_table)
}

fn flag(table: &Table, key: &str, fallback: bool) -> bool {
    table.get(key).and_then(Value::as_bool).unwrap_or(fallback)
}

/// An integer that has to make sense. Out-of-range is treated exactly like
/// the wrong type: a saved pane width of zero is as unusable as `"wide"`.
fn int_in(table: &Table, key: &str, range: std::ops::RangeInclusive<i64>, fallback: i32) -> i32 {
    table
        .get(key)
        .and_then(Value::as_integer)
        .filter(|v| range.contains(v))
        .and_then(|v| i32::try_from(v).ok())
        .unwrap_or(fallback)
}

/// A window coordinate, which may legitimately be negative (a second
/// monitor to the left) but not absurd.
fn coord(table: &Table, key: &str) -> Option<i32> {
    table
        .get(key)
        .and_then(Value::as_integer)
        .filter(|v| (-32_000..=32_000).contains(v))
        .and_then(|v| i32::try_from(v).ok())
}

fn minutes(table: &Table, key: &str) -> Option<u32> {
    table
        .get(key)
        .and_then(Value::as_integer)
        .filter(|v| (0..=i64::from(crate::session::AUTO_FETCH_MAX_MINUTES)).contains(v))
        .and_then(|v| u32::try_from(v).ok())
}

fn timeout_secs(table: &Table, key: &str) -> Option<u64> {
    table
        .get(key)
        .and_then(Value::as_integer)
        .filter(|v| (1..=86_400).contains(v))
        .and_then(|v| u64::try_from(v).ok())
}

fn clamp_to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir_store(dir: &Path) -> Store {
        Store::at(dir)
    }

    #[test]
    fn round_trips_everything() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = dir_store(dir.path());

        let mut settings = Settings::default();
        settings.defaults.auto_fetch_minutes = 5;
        settings.defaults.network_timeout_secs = 300;
        settings.repos.push(RepoOverride {
            key: repo_key(r"C:\Users\me\huge-repo"),
            auto_fetch_minutes: Some(0),
            network_timeout_secs: None,
        });

        let state = State {
            window: WindowState {
                x: Some(-1200),
                y: Some(40),
                width: 1600,
                height: 980,
                maximized: true,
            },
            layout: LayoutState {
                sidebar_width: 320,
                sidebar_collapsed: true,
                details_width: 520,
                commands_height: 200,
                commands_shown: true,
                tags_shown: false,
                wip_tree: false,
                details_tree: true,
                sections: Sections {
                    branches: true,
                    remotes: false,
                    worktree: true,
                    stashes: false,
                    tags: true,
                },
            },
            tabs: TabsState {
                paths: vec!["C:/a".into(), "C:/b".into()],
                active: 1,
            },
        };

        store.save_settings(&settings).expect("save settings");
        store.save_state(&state).expect("save state");

        assert_eq!(store.load_settings(), settings);
        assert_eq!(store.load_state(), state);
    }

    /// The files are meant to be opened and edited, so what they look like
    /// is part of the interface, not an implementation detail.
    #[test]
    fn the_files_read_the_way_a_person_would_write_them() {
        let settings = Settings {
            defaults: RepoSettings {
                auto_fetch_minutes: 5,
                network_timeout_secs: 300,
            },
            repos: vec![RepoOverride {
                key: repo_key(r"C:\Users\me\huge-repo"),
                auto_fetch_minutes: Some(0),
                network_timeout_secs: None,
            }],
        };
        insta::assert_snapshot!("settings_file", settings.to_table().to_string());

        let state = State {
            window: WindowState {
                x: Some(120),
                y: Some(80),
                width: 1600,
                height: 980,
                maximized: false,
            },
            layout: LayoutState::default(),
            tabs: TabsState {
                paths: vec![
                    r"C:\Users\me\platitude-gg".into(),
                    r"C:\Users\me\other".into(),
                ],
                active: 1,
            },
        };
        insta::assert_snapshot!("state_file", state.to_table().to_string());
    }

    #[test]
    fn windows_paths_are_written_without_escaping() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = dir_store(dir.path());
        let state = State {
            tabs: TabsState {
                paths: vec![r"C:\Users\me\proj".into()],
                active: 0,
            },
            ..State::default()
        };
        store.save_state(&state).expect("save");
        let text = std::fs::read_to_string(dir.path().join(STATE_FILE)).expect("read");
        assert!(
            text.contains("\"C:/Users/me/proj\""),
            "separators as git writes them:\n{text}"
        );
        assert!(
            !text.contains('\\'),
            "nothing in the file needs an escape:\n{text}"
        );
        assert_eq!(
            store.load_state().tabs.paths,
            vec!["C:/Users/me/proj".to_string()],
            "and what comes back is what a repository is named elsewhere"
        );
    }

    #[test]
    fn an_extended_length_path_keeps_its_backslashes() {
        // The `\\?\` prefix is addressed to Windows, not to a reader: it
        // turns path parsing off, and rewriting it points somewhere else.
        let raw = r"\\?\C:\Users\me\proj";
        assert_eq!(repo_key(raw), raw);
    }

    #[test]
    fn one_bad_value_costs_only_that_key() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join(STATE_FILE),
            r#"
version = 1
[window]
width = "wide"
height = 1000
[layout]
sidebar_width = 0
details_width = 640
colour = "midnight"
"#,
        )
        .expect("write");

        let state = dir_store(dir.path()).load_state();
        let d = State::default();
        assert_eq!(state.window.width, d.window.width, "wrong type falls back");
        assert_eq!(state.window.height, 1000, "the good key survives");
        assert_eq!(
            state.layout.sidebar_width, d.layout.sidebar_width,
            "an unusable width falls back like a wrong type"
        );
        assert_eq!(state.layout.details_width, 640);
    }

    #[test]
    fn nonsense_file_starts_from_defaults() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(STATE_FILE), "this is not toml {{{").expect("write");
        assert_eq!(dir_store(dir.path()).load_state(), State::default());
    }

    #[test]
    fn missing_files_start_from_defaults() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = dir_store(&dir.path().join("never-created"));
        assert_eq!(store.load_state(), State::default());
        assert_eq!(store.load_settings(), Settings::default());
    }

    #[test]
    fn active_tab_outside_the_list_falls_back() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join(STATE_FILE),
            "[tabs]\nactive = 9\npaths = ['C:/a', '', 'C:/b']\n",
        )
        .expect("write");
        let tabs = dir_store(dir.path()).load_state().tabs;
        assert_eq!(tabs.paths, vec!["C:/a".to_string(), "C:/b".to_string()]);
        assert_eq!(tabs.active, 0);
    }

    #[test]
    fn a_repository_takes_its_own_values_and_the_defaults_for_the_rest() {
        let settings = Settings {
            defaults: RepoSettings {
                auto_fetch_minutes: 5,
                network_timeout_secs: 300,
            },
            repos: vec![RepoOverride {
                key: "C:/big".into(),
                auto_fetch_minutes: Some(0),
                network_timeout_secs: None,
            }],
        };
        let big = settings.for_repo(r"C:\big");
        assert_eq!(big.auto_fetch_minutes, 0, "separators do not matter");
        assert_eq!(big.network_timeout_secs, 300, "the rest is the default");
        assert_eq!(settings.for_repo("C:/other"), settings.defaults);
    }

    #[test]
    fn the_config_dir_variable_wins_and_empty_means_no_files() {
        let env = Env::from_pairs(&[(CONFIG_DIR_ENV, "/tmp/pg"), ("APPDATA", "/roaming")]);
        let store = Store::locate(Platform::Windows, &env);
        assert_eq!(
            store.settings_path(),
            Some(Path::new("/tmp/pg/settings.toml"))
        );
        assert_eq!(store.state_path(), Some(Path::new("/tmp/pg/state.toml")));

        let empty = Env::from_pairs(&[(CONFIG_DIR_ENV, ""), ("APPDATA", "/roaming")]);
        assert!(Store::locate(Platform::Windows, &empty).is_ephemeral());
    }

    #[test]
    fn an_automated_run_never_reaches_the_real_files() {
        let env = Env::from_pairs(&[("APPDATA", "/roaming"), ("PG_AUTO_ACT", "open-picker")]);
        assert!(
            Store::locate(Platform::Windows, &env).is_ephemeral(),
            "a driven process must not write where a person's settings are"
        );

        let told = Env::from_pairs(&[
            ("APPDATA", "/roaming"),
            ("PG_AUTO_ACT", "open-picker"),
            (CONFIG_DIR_ENV, "/tmp/run-7"),
        ]);
        assert_eq!(
            Store::locate(Platform::Windows, &told).state_path(),
            Some(Path::new("/tmp/run-7/state.toml")),
            "a run that names a directory gets it"
        );
    }

    #[test]
    fn each_platform_puts_the_files_where_it_keeps_them() {
        let windows = Store::locate(
            Platform::Windows,
            &Env::from_pairs(&[("APPDATA", r"C:\Roaming"), ("LOCALAPPDATA", r"C:\Local")]),
        );
        assert_eq!(
            windows.settings_path(),
            Some(Path::new(r"C:\Roaming\platitude-gg\settings.toml"))
        );
        assert_eq!(
            windows.state_path(),
            Some(Path::new(r"C:\Local\platitude-gg\state.toml")),
            "a window position must not roam to another machine"
        );

        let mac = Store::locate(Platform::MacOs, &Env::from_pairs(&[("HOME", "/Users/me")]));
        assert_eq!(
            mac.settings_path(),
            Some(Path::new(
                "/Users/me/Library/Application Support/platitude-gg/settings.toml"
            ))
        );

        let xdg = Store::locate(Platform::Xdg, &Env::from_pairs(&[("HOME", "/home/me")]));
        assert_eq!(
            xdg.settings_path(),
            Some(Path::new("/home/me/.config/platitude-gg/settings.toml"))
        );
        assert_eq!(
            xdg.state_path(),
            Some(Path::new("/home/me/.local/state/platitude-gg/state.toml"))
        );

        let told = Store::locate(
            Platform::Xdg,
            &Env::from_pairs(&[("HOME", "/home/me"), ("XDG_STATE_HOME", "/run/state")]),
        );
        assert_eq!(
            told.state_path(),
            Some(Path::new("/run/state/platitude-gg/state.toml"))
        );
    }

    #[test]
    fn nowhere_to_put_them_is_not_a_guess() {
        assert!(Store::locate(Platform::Xdg, &Env::default()).is_ephemeral());
    }

    #[test]
    fn an_ephemeral_store_writes_nothing() {
        let store = Store::ephemeral();
        store.save_state(&State::default()).expect("no-op");
        store.save_settings(&Settings::default()).expect("no-op");
        assert_eq!(store.load_state(), State::default());
    }

    #[test]
    fn a_replaced_file_never_appears_half_written() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = dir_store(dir.path());
        store.save_state(&State::default()).expect("first");

        let mut state = State::default();
        state.layout.details_width = 999;
        store.save_state(&state).expect("second");

        let left_behind: Vec<_> = std::fs::read_dir(dir.path())
            .expect("read_dir")
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".tmp"))
            .collect();
        assert!(
            left_behind.is_empty(),
            "no scratch left over: {left_behind:?}"
        );
        assert_eq!(store.load_state().layout.details_width, 999);
    }
}
