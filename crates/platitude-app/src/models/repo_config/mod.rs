//! What one repository sets for itself, for a repository that need not be
//! the one in front.
//!
//! Off the sessions on purpose: only the tab in front has a session
//! (`RepoPageStack`), and the settings screen offers every repository in
//! the strip — so the reads and the write are spawned against a work tree
//! path, as `AppBackend`'s identity read is. git's own lock on the file it
//! writes makes that safe beside a session's write queue.
//!
//! Every answer is matched to its ask by number, since the path cannot
//! tell them apart (A → B → A; two reads of one path landing out of
//! order; a save answering after the screen moved on, or after a second
//! save). A read carries its generation and a save its number
//! (`work::ReadTicket`, `RepoConfigModel::begin_save`), and the drain
//! takes only the answer to the ask standing (`RepoConfigModel::absorb`).
//! The next ask also cancels the read before it, so one still queued for
//! a slot never spawns (`platitude_core::process::Slots`).

use std::path::PathBuf;
use std::sync::Arc;

use qtbridge::{QmlObject, qobject};
use tokio_util::sync::CancellationToken;

use crate::hub::{Feed, Hub};

use super::qml_register;

mod qobject;
mod work;
#[cfg(test)]
mod work_tests;

/// What a spawned read or write sends back.
enum ConfigMsg {
    /// A read landed: what the repository's own file holds, and what git
    /// would use here as things stand.
    Read {
        /// The read's number (`RepoConfigModel::read_generation`); an
        /// older one is dropped, whatever path it names.
        generation: u64,
        /// For the log.
        path: String,
        local_name: String,
        local_email: String,
        effective_name: String,
        effective_email: String,
    },
    /// A read could not be made at all — git's own words.
    ReadFailed {
        generation: u64,
        path: String,
        message: String,
    },
    /// A write finished. The two flags say which half git now reports as
    /// what was asked for; `error` carries git's words, and is empty for
    /// a write that failed nowhere.
    Written {
        /// The save's number (`RepoConfigModel::begin_save`); an answer
        /// for any other save is dropped.
        save: u64,
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
    /// The read's state; a save reports in the marks instead.
    state: String,
    /// What this repository's own file holds; empty for a key it does not
    /// set.
    local_name: String,
    local_email: String,
    /// What git would put on a commit made here right now.
    effective_name: String,
    effective_email: String,
    /// A copy of `save_pending.is_some()` for QML to bind to.
    write_busy: bool,
    /// A save finished without both halves landing; false until a save is
    /// tried (as `AppBackend::identity_unsaved`).
    write_unsaved: bool,
    write_name_saved: bool,
    write_email_saved: bool,
    /// git's words from whichever of the two last had something to say.
    error: String,
    feed: Arc<Feed<ConfigMsg>>,
    attached: bool,
    /// The number of the read the screen is waiting on (module docs); zero
    /// until the first ask.
    read_generation: u64,
    /// The standing read's token, cancelled by the next ask.
    read_cancel: Option<CancellationToken>,
    /// The save that is out, by the number it was given, or `None`.
    save_pending: Option<u64>,
    /// Saves asked for so far — where the next number comes from.
    saves_asked: u64,
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
            read_generation: 0,
            read_cancel: None,
            save_pending: None,
            saves_asked: 0,
        }
    }
}

qml_register!(RepoConfigModel, "RepoConfigModel", singleton = false);
