//! The pair of files, and everything that reads or writes them.

use std::path::{Path, PathBuf};

use super::toml::{read_table, write_atomically};
use super::{
    Build, CONFIG_DIR_ENV, Env, Platform, SETTINGS_FILE, STATE_FILE, Settings, State, StoreError,
};

/// Where the two files are, or that there are none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Store {
    pub(super) settings_path: Option<PathBuf>,
    pub(super) state_path: Option<PathBuf>,
}

impl Store {
    /// The store this process should use, with a development build's own
    /// copy filled in the first time it runs.
    pub fn discover(build: Build) -> Self {
        Self::open(Platform::HOST, &Env::system(), build)
    }

    /// [`Store::discover`] with the platform and the environment handed
    /// over, so that what a development build does on its first run can be
    /// watched from a test.
    fn open(platform: Platform, env: &Env, build: Build) -> Self {
        let store = Self::locate(platform, env, build);
        if build.is_dev() {
            let shipped = Self::locate(platform, env, Build::SHIPPED);
            // Not when a directory was named: `locate` hands every build
            // the same one, and a store cannot be seeded from itself.
            if shipped != store {
                store.seed_from(&shipped);
            }
        }
        store
    }

    /// `PG_CONFIG_DIR` wins: a path puts both files in it, and an empty
    /// value asks for no files at all. With it unset, a build that says it
    /// is being driven still gets no files — otherwise a screenshot run
    /// would write its window geometry into the developer's real settings,
    /// and the next run would start from it.
    ///
    /// The build says so ([`Build::driven`]) rather than the environment
    /// being read here: only a binary carrying a verification harness can
    /// be driven at all, and what ships must not lose somebody their
    /// settings to a `PG_*` variable left in their shell.
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
        if build.driven {
            return Self::ephemeral();
        }
        Self::platform_paths(platform, env, build)
    }

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

    #[cfg(test)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use toml::{Table, Value};

    use crate::settings::testkit::dir_store;
    use crate::settings::{Claim, Defaults, LayoutState, Sections, TabsState, WindowState};
    #[test]
    fn round_trips_everything() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = dir_store(dir.path());

        let mut settings = Settings::default();
        settings.defaults.auto_fetch_minutes = 5;
        settings.defaults.network_timeout_secs = 300;
        settings.defaults.initial_commits = Some(4000);

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
            defaults: Defaults {
                auto_fetch_minutes: 5,
                network_timeout_secs: 300,
                initial_commits: Some(4000),
                git_path: "C:/tools/git/cmd/git.exe".into(),
            },
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

        let mut moved = dev.load_settings();
        moved.defaults.auto_fetch_minutes = 1;
        dev.save_settings(&moved).expect("save");
        dev.seed_from(&real);
        assert_eq!(dev.load_settings().defaults.auto_fetch_minutes, 1);
    }

    #[test]
    fn the_first_run_of_a_development_build_lands_beside_the_real_one() {
        let home = tempfile::tempdir().expect("tempdir");
        let base = home.path().to_string_lossy().replace('\\', "/");
        let env = Env::from_pairs(&[("APPDATA", &base), ("LOCALAPPDATA", &base)]);

        let real = Store::open(Platform::Windows, &env, Build::SHIPPED);
        let mut settings = Settings::default();
        settings.defaults.auto_fetch_minutes = 9;
        real.save_settings(&settings).expect("save");

        let dev = Store::open(
            Platform::Windows,
            &env,
            Build {
                tree: "solo",
                debug: true,
                ..Build::SHIPPED
            },
        );
        assert_ne!(dev, real, "a development build writes files of its own");
        assert_eq!(dev.load_settings().defaults.auto_fetch_minutes, 9);
        assert_eq!(
            real.load_settings().defaults.auto_fetch_minutes,
            9,
            "and takes nothing away from the build that shipped"
        );

        let held = real.claim();
        assert!(matches!(held, Claim::Ours(_)));
        assert!(matches!(dev.claim(), Claim::Ours(_)));
        drop(held);
    }

    #[test]
    fn an_ephemeral_store_writes_nothing() {
        let store = Store::ephemeral();
        store.save_state(&State::default()).expect("no-op");
        store.save_settings(&Settings::default()).expect("no-op");
        assert_eq!(store.load_state(), State::default());
    }
}
