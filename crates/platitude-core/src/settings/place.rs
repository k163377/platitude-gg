//! Where the two files go on each platform, and what a build is.

use std::path::{Path, PathBuf};

use super::{
    AUTOMATION_PREFIX, DEV_DIR, DIR_NAME, NOT_AUTOMATION, SETTINGS_FILE, STATE_FILE, Store,
};

/// Which platform's placement rules to apply. A parameter, so all three
/// can be tested from any machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    MacOs,
    /// Linux and anything else following the XDG base directory spec.
    Xdg,
}

impl Platform {
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

    #[cfg(test)]
    pub fn from_pairs(pairs: &[(&str, &str)]) -> Self {
        Self {
            vars: pairs
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        }
    }

    pub(super) fn get(&self, key: &str) -> Option<&str> {
        self.vars
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// A non-empty variable: an empty `HOME` is not a home directory.
    fn filled(&self, key: &str) -> Option<&str> {
        self.get(key).filter(|v| !v.is_empty())
    }

    /// True when anything is driving this process. Asked only by a build
    /// carrying a verification harness ([`Build::driven`]), so a `PGG_*`
    /// variable somebody exported costs a shipped build nothing.
    pub fn automated(&self) -> bool {
        self.vars
            .iter()
            .any(|(k, _)| k.starts_with(AUTOMATION_PREFIX) && !NOT_AUTOMATION.contains(&k.as_str()))
    }
}

/// Which build is asking for a store.
///
/// A build made in a worktree, or one with debug assertions on, keeps its
/// own copy of the two files, so development builds run side by side
/// without costing a person the tabs and layout of the real one. What
/// ships reaches the real files, and nothing else does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Build<'a> {
    /// The worktree the binary was built in, empty for a plain checkout
    /// (`models::build_tree` reads it out of the build path).
    pub tree: &'a str,
    pub debug: bool,
    /// A machine is driving this run, so it gets no files at all — a
    /// screenshot run would otherwise write its window geometry into a
    /// person's real settings. What ships passes `false` without reading
    /// the environment ([`Env::automated`]).
    pub driven: bool,
}

impl Build<'_> {
    pub const SHIPPED: Build<'static> = Build {
        tree: "",
        debug: false,
        driven: false,
    };

    pub fn is_dev(&self) -> bool {
        !self.tree.is_empty() || self.debug
    }

    /// The directory this build's files live in, under the platform's
    /// base. The tree names it: the same tree built both ways is one build
    /// to a person, and shares.
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

impl Store {
    /// Windows and XDG split the two by roaming (settings follow a person
    /// to another machine, a window position stays); macOS has no such
    /// convention. A base directory the environment does not name leaves
    /// the store with no files at all.
    pub(super) fn platform_paths(platform: Platform, env: &Env, build: Build) -> Self {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::CONFIG_DIR_ENV;

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

    /// The shipped build never says this, whatever the environment holds.
    const DRIVEN: Build<'static> = Build {
        driven: true,
        ..Build::SHIPPED
    };

    #[test]
    fn a_driven_run_never_reaches_the_real_files() {
        let env = Env::from_pairs(&[("APPDATA", "/roaming")]);
        assert!(
            Store::locate(Platform::Windows, &env, DRIVEN).is_ephemeral(),
            "a driven process must not write where a person's settings are"
        );
        assert_eq!(
            Store::locate(Platform::Windows, &env, Build::SHIPPED).settings_path(),
            Some(Path::new("/roaming/platitude-gg/settings.toml")),
            "and the same environment costs a build nobody is driving nothing"
        );

        let told = Env::from_pairs(&[("APPDATA", "/roaming"), (CONFIG_DIR_ENV, "/tmp/run-7")]);
        assert_eq!(
            Store::locate(Platform::Windows, &told, DRIVEN).state_path(),
            Some(Path::new("/tmp/run-7/state.toml")),
            "a run that names a directory gets it"
        );
    }

    /// What a harness build hands to [`Build::driven`].
    #[test]
    fn a_knob_is_what_says_something_is_driving() {
        assert!(Env::from_pairs(&[("PGG_AUTO_ACT", "open-picker")]).automated());
        assert!(
            !Env::from_pairs(&[("APPDATA", "C:/Roaming"), ("PGG_LOG", "info")]).automated(),
            "PGG_LOG says how loud to be, not who is driving"
        );
        assert!(
            !Env::from_pairs(&[("APPDATA", "C:/Roaming"), ("PGG_ALLOW_GUI", "1")]).automated(),
            "the window a person asked for opens on the tabs they left"
        );
    }

    #[test]
    fn each_platform_puts_the_files_where_it_keeps_them() {
        // Windows bases in forward slashes on purpose: `join` uses the
        // host's separator and Linux reads a backslash as an ordinary
        // character, so only this spelling holds on every host.
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
                ..Build::SHIPPED
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
                ..Build::SHIPPED
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
                    debug: false,
                    ..Build::SHIPPED
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
                ..Build::SHIPPED
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
        let env = Env::from_pairs(&[(CONFIG_DIR_ENV, "/tmp/run-7"), ("APPDATA", "C:/Roaming")]);
        assert_eq!(
            Store::locate(
                Platform::Windows,
                &env,
                Build {
                    tree: "solo",
                    debug: true,
                    ..Build::SHIPPED
                }
            ),
            Store::locate(Platform::Windows, &env, Build::SHIPPED)
        );
    }
}
