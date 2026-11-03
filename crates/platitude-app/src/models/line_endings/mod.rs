//! What one repository's own git configuration says about line endings.
//!
//! That file and no other: the settings screen writes `core.autocrlf` into
//! the picked repository only (`LineEndingField.qml`), so this model asks git
//! `--local` about a worktree. Do not write `--global`, `core.eol` or
//! `.gitattributes` (user decision — ask before changing): rewriting the
//! reader's machine itself is outside this application.
//!
//! Off the sessions, as `repo_config` is: only the tab in front has one
//! (`RepoPageStack`) and the screen offers every repository in the strip,
//! so the reads and the write are spawned against a worktree's path. git's
//! own lock on the file makes that safe beside a session's write queue,
//! and an open session picks the write up on its next poll
//! (`RepoSession::forget_what_the_config_decides`).

use std::path::PathBuf;
use std::sync::Arc;

use qtbridge::{QmlObject, qobject};

use crate::hub::{Feed, Hub};

use super::qml_register;

mod qobject;
mod work;

/// What a spawned read or write sends back.
enum EolMsg {
    /// A read landed: what that file holds, and what git would use there.
    Read {
        /// The read's number (`LineEndingsModel::read_generation`): an
        /// older one is dropped, whatever path it names — the read before
        /// a pick's re-read can land after it.
        generation: u64,
        held: String,
        effective: String,
    },
    /// A read could not be made at all — git's own words.
    ReadFailed { generation: u64, message: String },
    /// A write finished; `error` is git's words, empty on success.
    Written { path: String, error: String },
}

pub struct LineEndingsModel {
    /// The worktree the reads and the write run in; empty until the
    /// screen names one (`"idle"`).
    repo_path: String,
    /// The read's `"idle" | "reading" | "ready" | "error"`; a pick keeps
    /// the field standing and reports in the line under it.
    state: String,
    /// What that file sets, in git's spelling (`eol::setting::AutoCrlf`);
    /// empty where it sets nothing — the value the empty row writes.
    held: String,
    /// What git would use in that repository right now, wherever it
    /// resolved the value from.
    effective: String,
    /// A pick is out; stops a second pick racing the first.
    busy: bool,
    /// git's words from whichever of the read and the write last had
    /// something to say; empty when neither did.
    error: String,
    feed: Arc<Feed<EolMsg>>,
    attached: bool,
    /// The number of the read the screen is waiting on (a latest read —
    /// the newest ask answers); zero until the first.
    read_generation: u64,
    /// The standing read's token, cancelled by the next ask.
    read_cancel: Option<tokio_util::sync::CancellationToken>,
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
            read_generation: 0,
            read_cancel: None,
        }
    }
}

qml_register!(LineEndingsModel, "LineEndingsModel", singleton = false);
