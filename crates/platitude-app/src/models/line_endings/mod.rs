//! What one repository's own git configuration says about line endings.
//!
//! **That file and no other.** The settings screen writes `core.autocrlf`
//! into the repository somebody picked and nowhere else (規約 §設定の画面):
//! the machine's own configuration is not an application's to rewrite, and
//! that is exactly where the other git GUIs' accidents come from. So this
//! model is handed a work tree and asks git `--local` about it — there is
//! no level above for it to stand at.
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
    /// A read landed: what that file holds, and what git would use in that
    /// repository as things stand.
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
    /// The work tree the reads and the write are spawned in, which is the
    /// repository whose own file they are about. Empty until the screen
    /// has named one, which is what `"idle"` below says.
    repo_path: String,
    /// "idle" | "reading" | "ready" | "error" — the read's, not the
    /// write's. A pick leaves the field standing and says how it went in
    /// the line under it.
    state: String,
    /// What that file sets, spelled the way git spells it
    /// (`eol::setting::AutoCrlf`); empty for a repository that sets
    /// nothing of its own, which is what the empty row asks for.
    held: String,
    /// What git would use in that repository right now, wherever it
    /// resolved the value from.
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
