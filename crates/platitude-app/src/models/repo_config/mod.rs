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
//!
//! **Every answer is matched to the ask it came from, by number.** The
//! path alone cannot tell them apart: a reader who went A → B → A has
//! made three asks about two paths, and the first one's answer arriving
//! last would show A's boxes as they were before B; two reads of one
//! path can land the other way round; and a save's answer can arrive
//! after the screen turned to another repository, or after a second save
//! was asked for. So a read carries the generation it was asked under
//! and a save the number it was given (`work::ReadTicket`,
//! `RepoConfigModel::begin_save`), and the drain takes only the answer to
//! the ask standing (`RepoConfigModel::absorb`). The ask after a read
//! cancels the read before it as well, so one still queued for a slot
//! never spawns (`platitude_core::process::Slots`).

use std::path::PathBuf;
use std::sync::Arc;

use qtbridge::{QObjectHolder, qobject};
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
        /// The ask this answers — the number the screen gave the read
        /// (`RepoConfigModel::read_generation`). An answer to an earlier
        /// ask is dropped, whatever path it names.
        generation: u64,
        /// The path the read was asked about, for the record.
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
        /// The save this answers — the number it was given at the ask
        /// (`RepoConfigModel::begin_save`). The answer to a save the
        /// screen has since moved off is dropped.
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
    /// A copy of `save_pending.is_some()` for QML to bind to.
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
    /// The read the screen is waiting on, numbered per ask — what an
    /// answer has to carry to be shown (module docs). Zero until the
    /// first ask.
    read_generation: u64,
    /// The token the standing read's commands run under, cancelled by
    /// the ask after it: a read that was still waiting for a slot then
    /// spawns nothing at all.
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
