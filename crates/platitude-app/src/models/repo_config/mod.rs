//! What one repository sets for itself, for a repository that need not be
//! the one in front.
//!
//! **Off the sessions on purpose.** Only the tab in front has a session
//! (`RepoPageStack`), and the settings screen offers every repository in
//! the strip — so the reads and the write here are spawned against a work
//! tree path, the way the application's own identity read is
//! (`AppBackend`), and nothing about them needs a repository to be open.
//! git takes its own lock on the file it writes, which is what makes that
//! safe beside a session's write queue.

use std::path::PathBuf;
use std::sync::Arc;

use qtbridge::{QObjectHolder, qobject};

use crate::hub::{Feed, Hub};

use super::qml_register;

mod qobject;
mod work;

/// What a spawned read or write sends back.
enum ConfigMsg {
    /// A read landed: what the repository's own file holds, and what git
    /// would use here as things stand.
    Read {
        /// The path the read was asked about. Carried back so an answer
        /// for a repository the reader has already moved off can be
        /// dropped rather than shown under the new name.
        path: String,
        local_name: String,
        local_email: String,
        effective_name: String,
        effective_email: String,
    },
    /// A read could not be made at all — git's own words.
    ReadFailed { path: String, message: String },
    /// A write finished. The two flags say which half git now reports as
    /// what was asked for; `error` carries git's words, and is empty for
    /// a write that failed nowhere.
    Written {
        path: String,
        error: String,
        name_saved: bool,
        email_saved: bool,
    },
}

pub struct RepoConfigModel {
    /// The repository being shown, as the strip spells it. Empty until
    /// the screen names one.
    repo_path: String,
    /// "idle" | "reading" | "ready" | "error" — the read's, not the
    /// write's. A save leaves the boxes standing and says how it went in
    /// the marks beside them.
    state: String,
    /// What this repository's own file holds; empty for a key it does not
    /// set, which is the same thing an empty box asks for.
    local_name: String,
    local_email: String,
    /// What git would put on a commit made here right now. The line under
    /// the boxes says it, because an empty box on its own cannot: the
    /// value it falls back to lives in a file this screen is not showing.
    effective_name: String,
    effective_email: String,
    /// A save is out. Nothing waits on it — the screen stays standing
    /// either way — but the button says so rather than looking unpressed.
    write_busy: bool,
    /// A save finished without both halves landing. It also carries "a
    /// save has been tried", which is what keeps the marks out of a screen
    /// that has only been read (the shape `AppBackend::identity_unsaved`
    /// keeps for the global pair).
    write_unsaved: bool,
    write_name_saved: bool,
    write_email_saved: bool,
    /// git's words from whichever of the two last had something to say.
    error: String,
    feed: Arc<Feed<ConfigMsg>>,
    attached: bool,
}

impl Default for RepoConfigModel {
    fn default() -> Self {
        Self {
            repo_path: String::new(),
            state: "idle".into(),
            local_name: String::new(),
            local_email: String::new(),
            effective_name: String::new(),
            effective_email: String::new(),
            write_busy: false,
            write_unsaved: false,
            write_name_saved: false,
            write_email_saved: false,
            error: String::new(),
            feed: Arc::new(Feed::default()),
            attached: false,
        }
    }
}

qml_register!(RepoConfigModel, "RepoConfigModel", singleton = false);
