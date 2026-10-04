//! The application singleton: the git and identity gates, app-wide
//! settings, avatars, and the window's saved shape.

use std::sync::Arc;

use platitude_core::version;
use qtbridge::{QmlObject, qobject};

use crate::hub::{Feed, Hub};

use super::qml_register;

mod avatars;
mod build_id;
mod identity;
mod lifecycle;
mod persist;
mod qobject;
mod start;

pub use build_id::{build_id, build_tree};

use start::{Assignments, assignments, section_open, with_flag, with_layout, with_window};
// Also used by the settings screen's `GLOBAL` identity chapter.
pub(crate) use start::app_workdir;

enum AppMsg {
    GitOk {
        version: String,
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
    /// The identity could not be read at all.
    IdentityUnknown {
        message: String,
    },
    /// One candidate git's answer. `path` is what was asked, so a stale
    /// answer can be dropped.
    GitPathProbed {
        path: String,
        probe: version::Probe,
        /// Whether the git asked is the binary this run spawns, whatever
        /// the spelling (`process::same_program`).
        names_the_run: bool,
    },
    /// The readback and verdict travel together: a drain between two
    /// messages would expose `ready` before `unsaved` and dismiss the gate.
    IdentitySaved(Result<platitude_core::identity::IdentityWrite, String>),
}

pub struct AppBackend {
    git_state: String,
    git_version: String,
    /// The git that answered is older than [`platitude_core::version::MINIMUM_GIT`].
    /// A badge, not a gate: what an older git cannot do fails with its own
    /// words where it is asked for.
    git_unsupported: bool,
    /// The supported minimum the missing-git screen and the badge print;
    /// core's, so it matches what the check compares against.
    minimum_git: String,
    git_error: String,
    /// "unknown" until the check runs, then "checking" / "missing" /
    /// "ready" / "error". Only "missing" opens the setup screen.
    identity_state: String,
    identity_name: String,
    identity_email: String,
    identity_error: String,
    identity_busy: bool,
    /// Which half of the last save git reports as written; both false
    /// until a save has finished.
    identity_name_saved: bool,
    identity_email_saved: bool,
    /// A save finished without both halves landing. Also means "a save has
    /// been tried", which keeps the marks and the toolbar badge out of a
    /// fresh window.
    identity_unsaved: bool,
    /// The window keeps the platform's own title bar above an ordinary
    /// tab row (`Main.captionMerged`). Read once as the window is created:
    /// a window that came up without a way to close it cannot be taken back.
    system_title_bar: bool,
    /// This build carries the verification harness, so the seats that load
    /// it by URL have something to load (`HarnessSeat.wanted`).
    harness_present: bool,
    /// Auto-fetch interval in minutes; 0 is off. Application-wide: the
    /// answer is about this computer, not a repository.
    auto_fetch_minutes: i32,
    /// Ceiling the settings input enforces.
    auto_fetch_max: i32,
    /// How many git processes run at once (`platitude_core::process::Slots`);
    /// application-wide, being about this machine. The ceiling and the
    /// empty box's fallback are core's.
    git_concurrency: i32,
    git_concurrency_max: i32,
    git_concurrency_default: i32,
    /// How often the other working copies are read for uncommitted work,
    /// in seconds; 0 is off. Floor and ceiling are core's
    /// (`session::copies_interval_secs`).
    copies_interval_secs: i32,
    copies_interval_min: i32,
    copies_interval_max: i32,
    copies_interval_ms: i32,
    /// Commits a graph opens with; 0 is the whole history (the screen's own
    /// box). Application-wide: it is about how much this person reads.
    initial_commits: i32,
    /// Floor the settings input enforces; the only ceiling is the type's
    /// (`session::log_limit`).
    initial_commits_min: i32,
    /// What an empty input falls back to and shows as its placeholder
    /// (`session::DEFAULT_LOG_LIMIT`).
    initial_commits_default: i32,
    /// The git this computer runs, as the settings file holds it: a path,
    /// or empty for whichever one `PATH` resolves. Application-wide: it is
    /// about the machine.
    git_path: String,
    /// What the git at [`Self::git_path`] last answered: "checking" while
    /// asked, then "ok" / "old" / "missing" / "failed" ([`version::Probe`]).
    /// The sentences are the screen's.
    git_path_state: String,
    /// The version string that git printed, empty where it printed none.
    git_path_version: String,
    /// git's or the OS's own words for a probe that failed, passed through.
    git_path_error: String,
    /// Where an empty box points: the git `PATH` resolves
    /// (`Hub::path_program`), so an emptied box compares as that binary.
    git_path_on_path: String,
    /// The box holds a git this run is not on, and that git answered. The
    /// one spelling of the rule: the button, its warning and the press's
    /// guard all read it, so they cannot disagree.
    git_path_offers_restart: bool,
    /// The last answer's word on whether the box names the running git
    /// (`AppMsg::GitPathProbed::names_the_run`). Valid only beside the
    /// state that answer set: a box rewritten since is "checking" again.
    git_path_names_the_run: bool,
    /// The window is to close and come back on the chosen git. Read by
    /// `Main.qml`, whose close is the one road out.
    restart_wanted: bool,
    /// Where the git this run spawns is — what the box shows behind an
    /// empty value. Constant: the binary is settled for the run
    /// (`Hub::git_program`).
    git_path_in_use: String,
    /// Assigned pictures (`Assignment`). Only the settings list reads it,
    /// and it is a handful of rows.
    avatars: Assignments,
    /// How the last assignment was refused: the refusal's kind, the numbers
    /// its sentence takes, and the OS's own words where it made the
    /// failure (`avatar::AvatarRefusal`). All empty means it worked. The
    /// sentence is `Words.avatarFailure`'s
    /// (rules-refs/app-ui.md「Rust に文言を置かない」).
    avatar_error_kind: String,
    avatar_error_facts: Vec<String>,
    avatar_error_said: String,
    /// The picker's patterns, built from the kinds the store accepts so the
    /// two cannot drift. Patterns only: the label is the dialog's.
    avatar_patterns: String,
    /// The corner's name for this binary (`build_id::BuildId`).
    build_tag: String,
    build_release: bool,
    build_commit: String,
    build_tree: String,
    /// Another process is already using the files this one would have
    /// used, so this window is only here to say so and be dismissed.
    already_running: bool,
    /// The directory that other process is holding — the only thing on
    /// screen that tells two builds apart.
    held_elsewhere: String,
    /// Whether the command log is up. Held for the run only: tabs hand it
    /// over like the pane widths (`PageLayout`), but a new launch comes up
    /// with the log down.
    commands_shown: bool,
    check_feed: Arc<Feed<AppMsg>>,
}
