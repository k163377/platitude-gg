//! The two files platitude-gg keeps for itself: `settings.toml` holds what
//! a person decided, `state.toml` what the last session left behind.
//! Separate because a state flush landing on the settings would eat an
//! edit made in an editor while the app is running.
//!
//! Nothing in here can stop the application from starting. A bad file
//! costs only the keys it got wrong: every value is pulled out of a parsed
//! table one at a time and falls back on its own — a derived `Deserialize`
//! gives up the whole document over one mistyped value.

use std::path::PathBuf;

mod lock;
mod place;
mod prefs;
mod state;
mod store;
mod toml;

#[cfg(test)]
mod testkit;

// `self::`: the child `toml` shares the extern crate's name, and a bare
// `toml::` here is ambiguous with the extern prelude (E0659).
pub use self::toml::repo_key;
pub use lock::{Claim, Lock};
pub use place::{Build, Env, Platform};
pub use prefs::{Defaults, Settings, WorktreesReading};
pub use state::{AUTO_WIDTH, LayoutState, Sections, State, TabRecord, TabsState, WindowState};
pub use store::Store;

pub const DIR_NAME: &str = "platitude-gg";

pub const SETTINGS_FILE: &str = "settings.toml";
pub const STATE_FILE: &str = "state.toml";

/// The file whose handle says which process is using these two — one
/// nothing ever writes to. A lock on a content file would guard an orphan
/// after the first flush: both are replaced by `rename`
/// (`write_atomically`), which succeeds over an open handle on Windows
/// too.
pub const LOCK_FILE: &str = "lock";

const DEV_DIR: &str = "dev";

/// Written at the top of both files. Only a marker (every reader is
/// per-key tolerant): a later key rename can tell an old file by it.
pub const SCHEMA_VERSION: i64 = 1;

/// Points both files at one directory. Empty means "read nothing, write
/// nothing", which is what a test or a screenshot run wants.
pub const CONFIG_DIR_ENV: &str = "PGG_CONFIG_DIR";

/// Every knob read only under verification shares this prefix, and no
/// other variable starts with it: the product's own spell `PGG_`, xtask's
/// and the hooks' `PGGDEV_`, the build's inputs `PLATITUDE_`.
/// `xtask::app_env` spells the same prefix (xtask links no crates): change
/// one, change both.
const AUTOMATION_PREFIX: &str = "_PGG_";

/// Failure to write. Reading has no error type: the caller could only
/// carry on with defaults.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("could not write {}: {source}", path.display())]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
