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

pub use self::toml::repo_key;
pub use lock::{Claim, Lock};
pub use place::{Build, Env, Platform};
pub use prefs::{CopiesReading, Defaults, Settings};
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

/// Automation knobs all share this prefix, and a person never sets one.
const AUTOMATION_PREFIX: &str = "PGG_";

/// `PGG_*` variables that say nothing about who is driving: a person turns
/// the logging up at their own window, `PGG_ALLOW_GUI` is a person asking
/// for a window (CLAUDE.md ビルド・テスト), and `PGG_STEP` only addresses
/// a stop inside a gate's container. Kept by hand in step with
/// `xtask::app_env`.
const NOT_AUTOMATION: [&str; 4] = [CONFIG_DIR_ENV, "PGG_LOG", "PGG_ALLOW_GUI", "PGG_STEP"];

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
