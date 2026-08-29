//! What one of git's configuration files says about line endings, for a
//! screen that offers the same question at two levels.
//!
//! **One type, standing at whichever level it was given.** The settings
//! screen writes `core.autocrlf` twice — once into the user's own file and
//! once into a chosen repository's — and the two differ in nothing but the
//! file: the same key, the same three answers, and an empty row that means
//! "not written here" at both. Two models for that would be one shape kept
//! in step by hand.
//!
//! **Off the sessions, for the reason `repo_config` is off them.** Only
//! the tab in front has a session (`RepoPageStack`) and the screen offers
//! every repository in the strip, so the reads and the write here are
//! spawned against a work tree path. git takes its own lock on the file it
//! writes, which is what makes that safe beside a session's write queue —
//! and an open session notices the written file on its next poll
//! (`RepoSession::forget_what_the_config_decides`), which is what stops a
//! repository that has just been told to convert from going on warning
//! about the files git now converts.

use std::path::PathBuf;
use std::sync::Arc;

use qtbridge::{QObjectHolder, qobject};

use crate::hub::{Feed, Hub};

use super::qml_register;

mod qobject;
mod work;

/// What a spawned read or write sends back.
enum EolMsg {
    /// A read landed: what that file holds, and — where the screen asked
    /// for it — what git would use in that repository as things stand.
    Read {
        /// The path the read was asked about, carried back so an answer
        /// for a repository the reader has already moved off can be
        /// dropped rather than shown under the new name.
        path: String,
        held: String,
        effective: String,
    },
    /// A read could not be made at all — git's own words.
    ReadFailed { path: String, message: String },
    /// A write finished. `error` carries git's words and is empty for one
    /// that failed nowhere.
    Written { path: String, error: String },
}

pub struct LineEndingsModel {
    /// Which of git's files this stands at: `"global"` or `"local"`.
    /// Written by the screen before it looks at anything.
    scope: String,
    /// The work tree the reads and the write are spawned in. Empty is the
    /// application's own directory, which is where git resolves the
    /// user's own configuration and nothing else's.
    repo_path: String,
    /// "idle" | "reading" | "ready" | "error" — the read's, not the
    /// write's. A pick leaves the field standing and says how it went in
    /// the line under it.
    state: String,
    /// What that file sets, spelled the way git spells it
    /// (`eol::setting::AutoCrlf`); empty for a level that sets nothing,
    /// which is what the empty row asks for.
    held: String,
    /// What git would use in the repository on screen right now. Empty
    /// where the screen did not ask (the global level has no repository to
    /// ask about).
    effective: String,
    /// A pick is out. Nothing waits on it — the field stays standing
    /// either way — but it stops a second pick from racing the first.
    busy: bool,
    /// git's words from whichever of the read and the write last had
    /// something to say.
    error: String,
    feed: Arc<Feed<EolMsg>>,
    attached: bool,
}

impl Default for LineEndingsModel {
    fn default() -> Self {
        Self {
            scope: "global".into(),
            repo_path: String::new(),
            state: "idle".into(),
            held: String::new(),
            effective: String::new(),
            busy: false,
            error: String::new(),
            feed: Arc::new(Feed::default()),
            attached: false,
        }
    }
}

qml_register!(LineEndingsModel, "LineEndingsModel", singleton = false);
