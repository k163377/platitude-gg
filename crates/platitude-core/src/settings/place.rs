//! Where the two files go on each platform, and what a build is.

use std::path::{Path, PathBuf};

use super::{
    AUTOMATION_PREFIX, DEV_DIR, DIR_NAME, NOT_AUTOMATION, SETTINGS_FILE, STATE_FILE, Store,
};

/// Which platform's placement rules to apply. A parameter rather than a
/// `cfg!` so all three can be tested from any machine — the 3-OS CI that
/// would otherwise reach them stays unrun until Phase 5 (CLAUDE.md).
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

    /// A non-empty variable, which is what every base directory needs to
    /// be: an empty `HOME` is not a home directory.
    fn filled(&self, key: &str) -> Option<&str> {
        self.get(key).filter(|v| !v.is_empty())
    }

    /// True when anything is driving this process.
    ///
    /// Asked by a build that carries a verification harness, and handed
    /// back as [`Build::driven`]. A shipped build never asks, so a `PGG_*`
    /// variable somebody happens to have exported costs them nothing.
    pub fn automated(&self) -> bool {
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
    /// Something rather than somebody is driving this run, so it gets no
    /// files at all — otherwise a screenshot run would write its window
    /// geometry into a person's real settings and the next run would open
    /// on it. [`Env::automated`] is how a caller works it out, but only a
    /// build carrying a verification harness ever asks: what ships passes
    /// `false` without reading the environment.
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

impl Store {
    /// Windows separates the two by roaming: settings follow a person to
    /// another machine, a window position must not. Linux has the same
    /// split spelled out in the XDG spec. macOS has no such convention, so
    /// both sit in Application Support.
    ///
    /// A base directory that is not in the environment is not guessed at —
    /// the store simply has no files rather than inventing a path to write
    /// into.
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

    /// The build that ships is the one that never says it, whatever is in
    /// the environment around it.
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

    /// What a caller hands to [`Build::driven`], which is the only place
    /// the environment still decides this.
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
