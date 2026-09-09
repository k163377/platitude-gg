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
pub use prefs::{Defaults, Settings};
pub use state::{AUTO_WIDTH, LayoutState, Sections, State, TabsState, WindowState};
pub use store::Store;

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

const DEV_DIR: &str = "dev";

/// Written at the top of both files. Every reader is per-key tolerant, so
/// this is not a gate — it is there so a later renaming of a key can tell
/// an old file from a new one instead of guessing.
pub const SCHEMA_VERSION: i64 = 1;

/// Points both files at one directory. Empty means "read nothing, write
/// nothing", which is what a test or a screenshot run wants.
pub const CONFIG_DIR_ENV: &str = "PGG_CONFIG_DIR";

/// Automation knobs all share this prefix, and a person never sets one.
const AUTOMATION_PREFIX: &str = "PGG_";

/// `PGG_*` variables that say nothing about who is driving. Turning the
/// logging up is something somebody does at their own window, and it must
/// not cost them their settings. `PGG_ALLOW_GUI` is the same shape from
/// the other end: it is how somebody says "I asked for a window" to the
/// pre-shell guard (CLAUDE.md ビルド・テスト), so it rides on the launch
/// that most needs the person's own tabs to come back.
const NOT_AUTOMATION: [&str; 3] = [CONFIG_DIR_ENV, "PGG_LOG", "PGG_ALLOW_GUI"];

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
