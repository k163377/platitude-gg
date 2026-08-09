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

/// The file whose handle says which process is using these two.
///
/// A file of its own, and one nothing ever writes to. Locking a content
/// file instead would come apart on the first flush: both are replaced by
/// `rename` (`write_atomically`), and a lock held on a file that is then
/// replaced guards an orphan. Measured on Windows — the rename succeeds
/// over the open handle, and the next process locks the new file without a
/// word; POSIX renames over open files as a matter of course, so it goes
/// the same way there.
pub const LOCK_FILE: &str = "lock";

/// What a development build's directory is called, under [`DIR_NAME`].
const DEV_DIR: &str = "dev";

/// Written at the top of both files. Every reader is per-key tolerant, so
/// this is not a gate — it is there so a later renaming of a key can tell
/// an old file from a new one instead of guessing.
pub const SCHEMA_VERSION: i64 = 1;

/// Points both files at one directory. Empty means "read nothing, write
/// nothing", which is what a test or a screenshot run wants.
pub const CONFIG_DIR_ENV: &str = "PG_CONFIG_DIR";

/// Automation knobs all share this prefix, and a person never sets one.
const AUTOMATION_PREFIX: &str = "PG_";

/// `PG_*` variables that say nothing about who is driving. Turning the
/// logging up is something somebody does at their own window, and it must
/// not cost them their settings. `PG_ALLOW_GUI` is the same shape from
/// the other end: it is how somebody says "I asked for a window" to the
/// pre-shell guard (CLAUDE.md ビルド・テスト), so it rides on the launch
/// that most needs the person's own tabs to come back.
const NOT_AUTOMATION: [&str; 3] = [CONFIG_DIR_ENV, "PG_LOG", "PG_ALLOW_GUI"];

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
            .any(|(k, _)| k.starts_with(AUTOMATION_PREFIX) && !NOT_AUTOMATION.contains(&k.as_str()))
    }
}

/// Which build is asking for a store.
///
/// A build made in a worktree, or one with debug assertions on, keeps its
/// own copy of the two files. Several of those are up at once on this
/// machine, next to the one being used for real work (CLAUDE.md
/// ビルド・テスト), and the files are what two processes fight over — so
/// giving each build its own is both what lets them run side by side and
/// what keeps a development run from costing somebody the tabs and the
/// layout they were in. What ships reaches the real files, and nothing
/// else does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Build<'a> {
    /// The worktree the binary was built in, empty for a plain checkout
    /// (`models::build_tree` reads it out of the build path).
    pub tree: &'a str,
    /// Built with debug assertions on.
    pub debug: bool,
}

impl Build<'_> {
    /// What ships: a plain checkout, optimised.
    pub const SHIPPED: Build<'static> = Build {
        tree: "",
        debug: false,
    };

    /// True for a build that keeps its own copy.
    pub fn is_dev(&self) -> bool {
        !self.tree.is_empty() || self.debug
    }

    /// The directory this build's files live in, under the platform's
    /// base. The tree names it, because that is the one thing that tells
    /// two development builds apart — the same tree built both ways is
    /// still one build to a person, and shares.
    fn dir(&self) -> PathBuf {
        let base = PathBuf::from(DIR_NAME);
        if !self.tree.is_empty() {
            base.join(format!("{DEV_DIR}-{}", self.tree))
        } else if self.debug {
            base.join(DEV_DIR)
        } else {
            base
        }
    }
}

/// Where the two files are, or that there are none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Store {
    settings_path: Option<PathBuf>,
    state_path: Option<PathBuf>,
}

impl Store {
    /// The store this process should use, with a development build's own
    /// copy filled in the first time it runs.
    pub fn discover(build: Build) -> Self {
        let env = Env::system();
        let store = Self::locate(Platform::HOST, &env, build);
        if build.is_dev() {
            let shipped = Self::locate(Platform::HOST, &env, Build::SHIPPED);
            if shipped != store {
                store.seed_from(&shipped);
            }
        }
        store
    }

    /// `PG_CONFIG_DIR` wins: a path puts both files in it, and an empty
    /// value asks for no files at all. With it unset, any other `PG_*`
    /// variable still means no files — otherwise a screenshot run would
    /// write its window geometry into the developer's real settings, and
    /// the next run would start from it.
    ///
    /// A named directory is a named directory, whichever build is asking:
    /// two runs sharing one `--config-dir` are how the saved layout is
    /// tested at all, and a build that quietly went somewhere else would
    /// answer a different question than the one asked.
    pub fn locate(platform: Platform, env: &Env, build: Build) -> Self {
        if let Some(dir) = env.get(CONFIG_DIR_ENV) {
            if dir.is_empty() {
                return Self::ephemeral();
            }
            return Self::at(Path::new(dir));
        }
        if env.automated() {
            return Self::ephemeral();
        }
        Self::platform_paths(platform, env, build)
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
    fn platform_paths(platform: Platform, env: &Env, build: Build) -> Self {
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
        let dir = build.dir();
        Self {
            settings_path: settings_base.map(|b| b.join(&dir).join(SETTINGS_FILE)),
            state_path: state_base.map(|b| b.join(&dir).join(STATE_FILE)),
        }
    }

    pub fn settings_path(&self) -> Option<&Path> {
        self.settings_path.as_deref()
    }

    pub fn state_path(&self) -> Option<&Path> {
        self.state_path.as_deref()
    }

    /// Where assigned pictures are kept: beside `settings.toml`, since that
    /// is the file that indexes them. A store with no settings file has
    /// nowhere to put them either.
    pub fn avatars_dir(&self) -> Option<PathBuf> {
        self.settings_path
            .as_deref()
            .and_then(Path::parent)
            .map(|dir| dir.join(crate::avatar::DIR_NAME))
    }

    /// True when this store is not backed by any file.
    pub fn is_ephemeral(&self) -> bool {
        self.settings_path.is_none() && self.state_path.is_none()
    }

    /// Starts a development build off as a copy of the real files, so that
    /// the first run of one opens on the tabs and the layout the person was
    /// already in rather than on an empty window.
    ///
    /// Once only, and only into a store that holds neither file: after that
    /// the copy is its own, and the two go their separate ways. Nothing
    /// here can stop the application — an empty store of one's own is a
    /// working store.
    pub fn seed_from(&self, source: &Store) {
        let (Some(settings), Some(state)) = (self.settings_path(), self.state_path()) else {
            return;
        };
        if settings.exists() || state.exists() {
            return;
        }
        if let Err(error) = self.copy_from(source) {
            tracing::warn!(%error, "this build starts with an empty store of its own");
        }
    }

    fn copy_from(&self, source: &Store) -> std::io::Result<()> {
        for (from, to) in [
            (source.settings_path(), self.settings_path()),
            (source.state_path(), self.state_path()),
        ] {
            let (Some(from), Some(to)) = (from, to) else {
                continue;
            };
            if !from.is_file() {
                continue;
            }
            if let Some(dir) = to.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::copy(from, to)?;
        }
        // The pictures the settings index sit beside them (`avatar`), and
        // an index whose files did not come along draws rows that never
        // fill. One level: the directory holds files named for their own
        // bytes and nothing else.
        let (Some(from), Some(to)) = (source.avatars_dir(), self.avatars_dir()) else {
            return Ok(());
        };
        if !from.is_dir() {
            return Ok(());
        }
        std::fs::create_dir_all(&to)?;
        for entry in std::fs::read_dir(&from)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                std::fs::copy(entry.path(), to.join(entry.file_name()))?;
            }
        }
        Ok(())
    }

    /// Where the lock sits: beside the state file, which is the one two
    /// processes overwrite in turn — and which is per-machine, where the
    /// settings may be a roaming profile that follows a person to another
    /// computer. `None` for a store with no files.
    pub fn lock_path(&self) -> Option<PathBuf> {
        let beside = self
            .state_path
            .as_deref()
            .or(self.settings_path.as_deref())?;
        Some(beside.with_file_name(LOCK_FILE))
    }

    /// Asks for sole use of these files.
    ///
    /// The answer separates "somebody else has it" from "the question
    /// could not be asked": a redirected profile or a network share can
    /// leave file locking unanswered, and a lock nobody can take must
    /// never become the reason a window will not open.
    pub fn claim(&self) -> Claim {
        let Some(path) = self.lock_path() else {
            // A store with no files has nothing for a second process to
            // overwrite. Every automated run is this one.
            return Claim::Ours(Lock { _file: None });
        };
        if let Some(dir) = path.parent()
            && let Err(error) = std::fs::create_dir_all(dir)
        {
            return Claim::Unknown(error);
        }
        let file = match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
        {
            Ok(file) => file,
            Err(error) => return Claim::Unknown(error),
        };
        match file.try_lock() {
            Ok(()) => Claim::Ours(Lock { _file: Some(file) }),
            Err(std::fs::TryLockError::WouldBlock) => Claim::Taken,
            Err(std::fs::TryLockError::Error(error)) => Claim::Unknown(error),
        }
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

/// Sole use of a store, for as long as this value is alive.
///
/// The kernel owns it: dropping the handle releases it, and so does the
/// process ending, however it ends. There is no stale file to clean up
/// after a crash, and no identifier written anywhere that could outlive
/// the process that wrote it.
#[derive(Debug)]
pub struct Lock {
    /// Never read. Holding the handle open *is* the lock.
    _file: Option<std::fs::File>,
}

/// What came back from [`Store::claim`].
#[derive(Debug)]
pub enum Claim {
    /// Nobody else is using these files. Hold on to it.
    Ours(Lock),
    /// Another process is using them.
    Taken,
    /// The lock could not be asked for. Carry on: a filesystem that will
    /// not answer is not a second application.
    Unknown(std::io::Error),
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
    /// Pictures put against authors. Kept here rather than in a third file
    /// because the split between the two is a write-rate one: an avatar is
    /// assigned about as often as an interval is changed, and neither
    /// happens while a pane is being dragged.
    pub avatars: crate::avatar::Avatars,
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
        let avatars = table
            .get("avatar")
            .and_then(Value::as_array)
            .map(|values| crate::avatar::Avatars::from_values(values))
            .unwrap_or_default();
        Self {
            defaults,
            repos,
            avatars,
        }
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
        if !self.avatars.is_empty() {
            root.insert("avatar".into(), Value::Array(self.avatars.to_values()));
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
    /// Chip and lane columns inside the graph, or [`AUTO_WIDTH`] while
    /// nobody has moved that divider — which is worth keeping apart from
    /// a number, because a session that never touched it goes on
    /// following the pane's default even when that default changes.
    pub graph_labels_width: i32,
    pub graph_lanes_width: i32,
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
            graph_labels_width: AUTO_WIDTH,
            graph_lanes_width: AUTO_WIDTH,
            commands_height: 280,
            commands_shown: false,
            tags_shown: true,
            wip_tree: true,
            details_tree: true,
            sections: Sections::default(),
        }
    }
}

/// "No width was chosen here" — see [`LayoutState::graph_labels_width`].
pub const AUTO_WIDTH: i32 = -1;

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
                graph_labels_width: width_or_auto(l, "graph_labels_width"),
                graph_lanes_width: width_or_auto(l, "graph_lanes_width"),
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
            "graph_labels_width".into(),
            Value::Integer(l.graph_labels_width.into()),
        );
        layout.insert(
            "graph_lanes_width".into(),
            Value::Integer(l.graph_lanes_width.into()),
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

/// A column width inside the graph: `AUTO_WIDTH` for "nobody moved this
/// one", or a width wide enough to be one. Everything between the two —
/// a stored 3, a stored 0 — is as unusable as a string would be, and
/// falls back the same way (規約 §読みは Table からキーごとに取る).
fn width_or_auto(table: &Table, key: &str) -> i32 {
    /// Narrower than this and the column cannot hold what it is for: one
    /// lane, or a chip clipped to nothing.
    const NARROWEST: i64 = 24;
    table
        .get(key)
        .and_then(Value::as_integer)
        .filter(|v| *v == i64::from(AUTO_WIDTH) || (NARROWEST..=4_000).contains(v))
        .and_then(|v| i32::try_from(v).ok())
        .unwrap_or(AUTO_WIDTH)
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
                graph_labels_width: 190,
                graph_lanes_width: 300,
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
            avatars: crate::avatar::Avatars::from_values(&[toml::Value::Table({
                let mut t = Table::new();
                t.insert("email".into(), Value::String("ada@example.com".into()));
                t.insert("name".into(), Value::String("Ada Lovelace".into()));
                t.insert(
                    "file".into(),
                    Value::String("3f2a1c4e5b6d7089000004d2.png".into()),
                );
                t
            })]),
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

    /// Saving normalises paths, so a value built with native separators is
    /// not what comes back. Callers hand over [`repo_key`] output for that
    /// reason; this pins the one round trip it takes to settle, so nothing
    /// downstream can start comparing a held state against the file and
    /// find a difference every time.
    #[test]
    fn a_raw_path_settles_after_one_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = dir_store(dir.path());
        let raw = State {
            tabs: TabsState {
                paths: vec![r"C:\Users\me\proj".into()],
                active: 0,
            },
            ..State::default()
        };
        store.save_state(&raw).expect("save");
        let settled = store.load_state();
        assert_ne!(settled, raw, "the separators moved");

        store.save_state(&settled).expect("save again");
        assert_eq!(store.load_state(), settled, "and then stop moving");
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

    /// The graph's two columns carry a value below every real width that
    /// still means something: nobody has moved this divider. It has to
    /// survive the same range check that throws out a stored 3.
    #[test]
    fn an_untouched_graph_divider_is_not_a_width() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join(STATE_FILE),
            r#"
version = 1
[layout]
graph_labels_width = -1
graph_lanes_width = 3
"#,
        )
        .expect("write");

        let state = dir_store(dir.path()).load_state();
        assert_eq!(
            state.layout.graph_labels_width, AUTO_WIDTH,
            "-1 is a value, not a bad one"
        );
        assert_eq!(
            state.layout.graph_lanes_width, AUTO_WIDTH,
            "a column too narrow to hold a lane falls back like a wrong type"
        );

        // And the pair a person can actually leave behind comes back.
        let mut moved = state;
        moved.layout.graph_labels_width = 190;
        moved.layout.graph_lanes_width = 300;
        let store = dir_store(dir.path());
        store.save_state(&moved).expect("save");
        let back = store.load_state().layout;
        assert_eq!(
            (back.graph_labels_width, back.graph_lanes_width),
            (190, 300)
        );
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
            avatars: crate::avatar::Avatars::default(),
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
        let store = Store::locate(Platform::Windows, &env, Build::SHIPPED);
        assert_eq!(
            store.settings_path(),
            Some(Path::new("/tmp/pg/settings.toml"))
        );
        assert_eq!(store.state_path(), Some(Path::new("/tmp/pg/state.toml")));

        let empty = Env::from_pairs(&[(CONFIG_DIR_ENV, ""), ("APPDATA", "/roaming")]);
        assert!(Store::locate(Platform::Windows, &empty, Build::SHIPPED).is_ephemeral());
    }

    #[test]
    fn an_automated_run_never_reaches_the_real_files() {
        let env = Env::from_pairs(&[("APPDATA", "/roaming"), ("PG_AUTO_ACT", "open-picker")]);
        assert!(
            Store::locate(Platform::Windows, &env, Build::SHIPPED).is_ephemeral(),
            "a driven process must not write where a person's settings are"
        );

        let told = Env::from_pairs(&[
            ("APPDATA", "/roaming"),
            ("PG_AUTO_ACT", "open-picker"),
            (CONFIG_DIR_ENV, "/tmp/run-7"),
        ]);
        assert_eq!(
            Store::locate(Platform::Windows, &told, Build::SHIPPED).state_path(),
            Some(Path::new("/tmp/run-7/state.toml")),
            "a run that names a directory gets it"
        );
    }

    #[test]
    fn turning_the_logging_up_does_not_cost_you_your_settings() {
        let env = Env::from_pairs(&[
            ("APPDATA", "C:/Roaming"),
            ("LOCALAPPDATA", "C:/Local"),
            ("PG_LOG", "info"),
        ]);
        assert_eq!(
            Store::locate(Platform::Windows, &env, Build::SHIPPED).settings_path(),
            Some(Path::new("C:/Roaming/platitude-gg/settings.toml")),
            "PG_LOG says how loud to be, not who is driving"
        );
    }

    #[test]
    fn asking_for_a_window_does_not_cost_you_your_tabs() {
        // The pre-shell guard is told "the user asked for a window" by a
        // variable in front of the launch (CLAUDE.md ビルド・テスト), and
        // that launch is the one that most needs the person's own tabs to
        // come back — so it must not read as a driven run.
        let env = Env::from_pairs(&[
            ("APPDATA", "C:/Roaming"),
            ("LOCALAPPDATA", "C:/Local"),
            ("PG_ALLOW_GUI", "1"),
        ]);
        assert_eq!(
            Store::locate(Platform::Windows, &env, Build::SHIPPED).state_path(),
            Some(Path::new("C:/Local/platitude-gg/state.toml")),
            "the window a person asked for opens on the tabs they left"
        );
    }

    #[test]
    fn each_platform_puts_the_files_where_it_keeps_them() {
        // The Windows bases are spelled with forward slashes on purpose.
        // `join` punctuates with the separator of the host the test runs on,
        // so a `C:\Roaming` fixture comes back as `C:\Roaming/platitude-gg/…`
        // on Linux — where a backslash is an ordinary character, not a
        // separator — and the assertion could only ever hold on one of the
        // three operating systems. Windows reads both separators, so this is
        // the spelling every host agrees on, and what is under test is which
        // base directory each file lands in, not how a path is punctuated.
        let windows = Store::locate(
            Platform::Windows,
            &Env::from_pairs(&[("APPDATA", "C:/Roaming"), ("LOCALAPPDATA", "C:/Local")]),
            Build::SHIPPED,
        );
        assert_eq!(
            windows.settings_path(),
            Some(Path::new("C:/Roaming/platitude-gg/settings.toml"))
        );
        assert_eq!(
            windows.state_path(),
            Some(Path::new("C:/Local/platitude-gg/state.toml")),
            "a window position must not roam to another machine"
        );

        let mac = Store::locate(
            Platform::MacOs,
            &Env::from_pairs(&[("HOME", "/Users/me")]),
            Build::SHIPPED,
        );
        assert_eq!(
            mac.settings_path(),
            Some(Path::new(
                "/Users/me/Library/Application Support/platitude-gg/settings.toml"
            ))
        );

        let xdg = Store::locate(
            Platform::Xdg,
            &Env::from_pairs(&[("HOME", "/home/me")]),
            Build::SHIPPED,
        );
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
            Build::SHIPPED,
        );
        assert_eq!(
            told.state_path(),
            Some(Path::new("/run/state/platitude-gg/state.toml"))
        );
    }

    #[test]
    fn nowhere_to_put_them_is_not_a_guess() {
        assert!(Store::locate(Platform::Xdg, &Env::default(), Build::SHIPPED).is_ephemeral());
    }

    #[test]
    fn a_development_build_never_writes_the_real_files() {
        let env = Env::from_pairs(&[("APPDATA", "C:/Roaming"), ("LOCALAPPDATA", "C:/Local")]);
        let debug = Store::locate(
            Platform::Windows,
            &env,
            Build {
                tree: "",
                debug: true,
            },
        );
        assert_eq!(
            debug.settings_path(),
            Some(Path::new("C:/Roaming/platitude-gg/dev/settings.toml"))
        );

        let worktree = Store::locate(
            Platform::Windows,
            &env,
            Build {
                tree: "solo",
                debug: true,
            },
        );
        assert_eq!(
            worktree.state_path(),
            Some(Path::new("C:/Local/platitude-gg/dev-solo/state.toml")),
            "the tree names it, so two of them can be up at once"
        );
        assert_eq!(
            worktree,
            Store::locate(
                Platform::Windows,
                &env,
                Build {
                    tree: "solo",
                    debug: false
                }
            ),
            "one tree is one build to a person, however it was compiled"
        );

        let xdg = Store::locate(
            Platform::Xdg,
            &Env::from_pairs(&[("HOME", "/home/me")]),
            Build {
                tree: "labels",
                debug: false,
            },
        );
        assert_eq!(
            xdg.settings_path(),
            Some(Path::new(
                "/home/me/.config/platitude-gg/dev-labels/settings.toml"
            ))
        );
    }

    #[test]
    fn a_named_directory_is_the_same_one_for_every_build() {
        // Two runs sharing one `--config-dir` are how the saved layout is
        // tested; a build that went somewhere else of its own accord would
        // quietly answer a different question.
        let env = Env::from_pairs(&[(CONFIG_DIR_ENV, "/tmp/run-7"), ("APPDATA", "C:/Roaming")]);
        assert_eq!(
            Store::locate(
                Platform::Windows,
                &env,
                Build {
                    tree: "solo",
                    debug: true
                }
            ),
            Store::locate(Platform::Windows, &env, Build::SHIPPED)
        );
    }

    #[test]
    fn a_development_build_starts_from_a_copy_and_then_goes_its_own_way() {
        let dir = tempfile::tempdir().expect("tempdir");
        let real = dir_store(&dir.path().join("real"));
        let mut settings = Settings::default();
        settings.defaults.auto_fetch_minutes = 7;
        real.save_settings(&settings).expect("save");
        real.save_state(&State::default()).expect("save");
        let avatars = real.avatars_dir().expect("avatars");
        std::fs::create_dir_all(&avatars).expect("mkdir");
        std::fs::write(avatars.join("abc.png"), b"picture").expect("write");

        let dev = dir_store(&dir.path().join("dev"));
        dev.seed_from(&real);
        assert_eq!(
            dev.load_settings().defaults.auto_fetch_minutes,
            7,
            "the first run opens on what the person had"
        );
        assert!(
            dev.avatars_dir()
                .expect("avatars")
                .join("abc.png")
                .is_file(),
            "an index whose pictures stayed behind draws rows that never fill"
        );

        // From here the two are strangers: a second seeding must not undo
        // what the development build has done since.
        let mut moved = dev.load_settings();
        moved.defaults.auto_fetch_minutes = 1;
        dev.save_settings(&moved).expect("save");
        dev.seed_from(&real);
        assert_eq!(dev.load_settings().defaults.auto_fetch_minutes, 1);
    }

    #[test]
    fn only_one_process_at_a_time_holds_a_store() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = dir_store(dir.path());
        let first = store.claim();
        assert!(matches!(first, Claim::Ours(_)), "nobody else has it");
        assert!(
            matches!(store.claim(), Claim::Taken),
            "a second asker is turned away"
        );

        // Dropping the handle is the whole release mechanism — which is
        // also what happens when a process is killed.
        drop(first);
        assert!(matches!(store.claim(), Claim::Ours(_)));
    }

    #[test]
    fn a_store_with_no_files_is_never_taken() {
        // Every automated run lands here, and two of them run at once.
        let store = Store::ephemeral();
        let first = store.claim();
        assert!(store.lock_path().is_none());
        assert!(matches!(first, Claim::Ours(_)));
        assert!(matches!(store.claim(), Claim::Ours(_)));
    }

    #[test]
    fn the_lock_is_not_one_of_the_two_files() {
        // Both are replaced by rename on every write, and a lock on a
        // replaced file guards an orphan (measured — see `LOCK_FILE`).
        let dir = tempfile::tempdir().expect("tempdir");
        let store = dir_store(dir.path());
        let held = store.claim();
        store.save_state(&State::default()).expect("save");
        store.save_settings(&Settings::default()).expect("save");
        assert!(
            matches!(store.claim(), Claim::Taken),
            "a flush must not hand the store to the next process"
        );
        drop(held);
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
