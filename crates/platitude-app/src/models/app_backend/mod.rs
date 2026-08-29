//! The application singleton: git version gate + automation hooks.

use std::sync::Arc;

use platitude_core::version;
use qtbridge::{QObjectHolder, qobject};

use crate::hub::{Feed, Hub};

use super::qml_register;

mod avatars;
mod build_id;
mod lifecycle;
mod persist;
mod qobject;
mod start;

pub use build_id::build_tree;

use start::{packed_avatars, with_flag, with_layout, with_window};
// The directory the application's own configuration reads are made in.
// Shared with the settings screen's global level, which asks about the
// same file (`models::line_endings`).
pub(crate) use start::app_workdir;

enum AppMsg {
    GitOk {
        version: String,
        /// Whether that version is at or above the supported minimum
        /// (see `AppBackend::git_unsupported`).
        supported: bool,
    },
    GitMissing {
        message: String,
    },
    GitError {
        message: String,
    },
    /// Identity git would put on a new commit here.
    Identity {
        name: String,
        email: String,
    },
    /// The identity could not be read at all (not the same as unset).
    IdentityUnknown {
        message: String,
    },
    /// An identity write finished; `error` carries git's own message and
    /// the two flags say which half git now reports as what was asked for.
    IdentitySaved {
        error: Option<String>,
        name_saved: bool,
        email_saved: bool,
    },
}

pub struct AppBackend {
    git_state: String,
    git_version: String,
    /// The git that answered is older than [`platitude_core::version::MINIMUM_GIT`].
    /// Everything still runs — this is the band's fourth badge, not a gate:
    /// most of what the app asks for works on an older git, and the ones
    /// that do not fail with git's own words where they are asked for.
    git_unsupported: bool,
    /// The supported minimum, printed by the missing-git screen and by the
    /// badge's card. Read from core so neither can drift from the version
    /// the check actually compares against.
    minimum_git: String,
    git_error: String,
    /// "unknown" until the check runs, then "checking" / "missing" /
    /// "ready" / "error". Only "missing" opens the setup screen.
    identity_state: String,
    identity_name: String,
    identity_email: String,
    identity_error: String,
    identity_busy: bool,
    /// Which half of the last save git now reports as what was asked for.
    /// The two are how a half-written identity shows on screen instead of
    /// passing for a finished one, and they stay false until a save has
    /// finished — nothing has been asked for before that.
    identity_name_saved: bool,
    identity_email_saved: bool,
    /// A save finished without both halves landing. Separate from the two
    /// above because it also carries "a save has been tried", which is
    /// what keeps the marks and the toolbar badge out of a fresh window.
    identity_unsaved: bool,
    auto_open: String,
    shot_dir: String,
    /// Harness-only deadline. It never chooses when a screenshot is
    /// taken; it only keeps a broken causal run bounded.
    auto_watchdog_ms: i32,
    auto_select: bool,
    auto_scroll: bool,
    auto_perf: bool,
    /// `PG_MEM_REPORT=1`: the window drives the memory breakdown off a
    /// timer instead of leaving it to whoever remembers to ask.
    mem_report: bool,
    auto_wip: bool,
    /// Verification hook: take the shape the platforms that cannot merge
    /// the band into the title bar get — the two that cannot be run here.
    plain_chrome: bool,
    /// Whether something rather than somebody is driving this run
    /// (`Env::automated` — any `PG_*` knob but the three that say nothing
    /// about who is at the window). What reads it is the window: a run
    /// nobody is looking at keeps the size it was configured with instead
    /// of being fitted to a screen (`WindowShape.insideScreen`), and the screen
    /// the headless platform reports is 800x800.
    automated: bool,
    /// Screenshot hook: `"<name>|<email>"` prefills the identity screen.
    auto_identity: String,
    /// Screenshot hook: submit that prefilled identity straight away.
    auto_identity_save: bool,
    scroll_to: String,
    /// Smoke hook: one operation to run once the repository is loaded —
    /// a write, or a surface left standing for the overlay shot — and
    /// its argument. A bare verb rather than a script, so QML dispatches
    /// on equality; the argument passes through as the verb needs it.
    auto_act: String,
    auto_act_arg: String,
    /// Auto-fetch interval in minutes; 0 is off. Application-wide, because
    /// the answer is about how often this computer should talk to remotes.
    auto_fetch_minutes: i32,
    /// Ceiling the settings input enforces.
    auto_fetch_max: i32,
    /// Commits a graph opens with; 0 is the whole history, which the
    /// screen asks for with a box rather than a number. Application-wide
    /// for the same reason the interval is: the answer is about how much
    /// history this person reads at once.
    initial_commits: i32,
    /// Floor the settings input enforces. There is no ceiling of ours —
    /// the property's own type is the only one (`session::log_limit`).
    initial_commits_min: i32,
    /// What an empty input falls back to, and what the input shows behind
    /// a reader who has not typed one. Held rather than spelled out in
    /// QML: the number is core's (`session::DEFAULT_LOG_LIMIT`).
    initial_commits_default: i32,
    /// Assigned pictures, packed one per record: address, name, URL.
    /// The settings list is the only reader, and it is a handful of rows.
    avatars: String,
    /// git-style: empty means the last assignment worked.
    avatar_error: String,
    /// The patterns the picker offers, built from the kinds the store
    /// accepts so the dialog and the store cannot drift apart. Patterns
    /// only: the word in front of them is the dialog's, and words live in
    /// `qsTr()` on the QML side.
    avatar_patterns: String,
    /// The worktree this binary was built in, empty for the primary
    /// checkout — see [`build_tree`].
    build_tree: String,
    /// Another process is already using the files this one would have
    /// used, so this window is only here to say so and be dismissed.
    already_running: bool,
    /// The directory that other process is holding. Two builds are told
    /// apart by nothing else on screen, and the taskbar's launch entry
    /// gives no clue which one it started.
    held_elsewhere: String,
    check_feed: Arc<Feed<AppMsg>>,
}
